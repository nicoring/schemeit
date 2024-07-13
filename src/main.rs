mod env;
mod error;
mod eval;
mod parse;
mod tokenize;

use std::env as std_env;
use std::fs;
use std::io::{self, Write};

use env::Env;
use error::Result;
use eval::eval;
use parse::{parse, SymbolicExpression};
use tokenize::tokenize;

fn eval_str(env: &mut Env, code: &str) -> Result<SymbolicExpression> {
    let mut tokens = tokenize(code);
    tokens.pop_front();
    let expression = parse(&mut tokens);
    eval(env, &expression)
}

fn eval_file(env: &mut Env, filename: &str) -> Result<SymbolicExpression> {
    let contents = fs::read_to_string(filename).expect("Should have been able to read the file");
    eval_str(env, &contents)
}

fn repl() {
    let mut env = Env::new();
    eval_file(&mut env, "test.scm").unwrap();
    loop {
        print!("repl> ");
        io::stdout().flush().unwrap();
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .expect("Failed to read line");

        line = line.trim().to_string();
        if line == "exit" {
            return;
        };
        if line == "" {
            continue;
        }
        let result = eval_str(&mut env, &line);
        match result {
            Ok(result) => println!("out: {}", result),
            Err(err) => println!("{}", err),
        };
    }
}

fn benchmark() {
    let mut env = Env::new();
    eval_file(&mut env, "test.scm").unwrap();
    use std::time::Instant;
    let now = Instant::now();
    {
        let _ = eval_str(&mut env, "(fib 30");
    }
    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}

fn run_file(filename: &str) {
    let mut env = Env::new();
    let result = eval_file(&mut env, filename);
    match result {
        Ok(result) => println!("out: {}", result),
        Err(err) => println!("{}", err),
    };
}

fn main() {
    let args: Vec<String> = std_env::args().collect();
    if args.len() == 1 {
        repl();
    } else if args[1] == "--benchmark" {
        benchmark();
    } else {
        run_file(args[1].as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_define_function() {
        let mut env = Env::new();
        eval_str(&mut env, "(define pi 3.141592653)").unwrap();
        eval_str(&mut env, "(define circle-area (lambda (r) (* pi (* r r))))").unwrap();
        assert_eq!(
            eval_str(&mut env, "(circle-area 3)").unwrap(),
            SymbolicExpression::Float(28.274333877)
        );
        assert_eq!(
            eval_str(&mut env, "(circle-area 3)").unwrap(),
            SymbolicExpression::Float(28.274333877)
        );
    }

    #[test]
    fn account_state() {
        let mut env = Env::new();
        let code = "
        (define make-account
            (lambda (balance)
              (lambda (amt)
                  (begin (set! balance (+ balance amt))
                          balance))))
        ";
        eval_str(&mut env, code).unwrap();
        let code = "(define account (make-account 100.00))";
        eval_str(&mut env, code).unwrap();
        let code = "(account -20.00)";
        assert_eq!(
            eval_str(&mut env, code).unwrap(),
            SymbolicExpression::Float(80.0)
        );
        assert_eq!(
            eval_str(&mut env, code).unwrap(),
            SymbolicExpression::Float(60.0)
        );
    }

    #[test]
    fn fib() {
        let code =
            "(define fib (lambda (n) (cond ((< n 2) 1) (#t (+ (fib (- n 1)) (fib (- n 2)))))))";
        let mut env = Env::new();
        eval_str(&mut env, code).unwrap();
        assert_eq!(
            eval_str(&mut env, "(fib 0)").unwrap(),
            SymbolicExpression::Int(1)
        );
        assert_eq!(
            eval_str(&mut env, "(fib 1)").unwrap(),
            SymbolicExpression::Int(1)
        );
        assert_eq!(
            eval_str(&mut env, "(fib 2)").unwrap(),
            SymbolicExpression::Int(2)
        );
        assert_eq!(
            eval_str(&mut env, "(fib 9)").unwrap(),
            SymbolicExpression::Int(55)
        );
    }

    #[test]
    fn test_let() {
        let code = "(let ((a 5) (b (+ 5 a))) (+ a b))";
        let mut env = Env::new();
        assert_eq!(
            eval_str(&mut env, code).unwrap(),
            SymbolicExpression::Int(15)
        );
    }
}
