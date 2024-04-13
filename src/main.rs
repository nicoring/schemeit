mod env;
mod eval;
mod parse;
mod tokenize;

use std::fs;
use std::io::{self, Write};

use env::Env;
use eval::eval;
use parse::{parse, SymbolicExpression};
use tokenize::tokenize;

fn run_str(env: &mut Env, code: &str) -> SymbolicExpression {
    let mut tokens = tokenize(code);
    tokens.pop_front();
    let expression = parse(&mut tokens);
    let result = eval(env, &expression);
    result
}

fn run_file(env: &mut Env, filename: &str) -> SymbolicExpression {
    let contents = fs::read_to_string(filename).expect("Should have been able to read the file");
    run_str(env, &contents)
}

fn repl() {
    let mut env = Env::new();
    run_file(&mut env, "test.scm");
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
        let result = run_str(&mut env, &line);
        println!("out: {}", result);
    }
}

fn main() {
    repl();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_define_function() {
        let mut env = Env::new();
        run_str(&mut env, "(define pi 3.141592653)");
        run_str(&mut env, "(define circle-area (lambda (r) (* pi (* r r))))");
        assert_eq!(
            run_str(&mut env, "(circle-area 3)"),
            SymbolicExpression::Float(28.274333877)
        );
        assert_eq!(
            run_str(&mut env, "(circle-area 3)"),
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
        run_str(&mut env, &code);
        let code = "(define account (make-account 100.00))";
        run_str(&mut env, code);
        let code = "(account -20.00)";
        assert_eq!(run_str(&mut env, code), SymbolicExpression::Float(80.0));
        assert_eq!(run_str(&mut env, code), SymbolicExpression::Float(60.0));
    }

    #[test]
    fn fib() {
        let code =
            "(define fib (lambda (n) (cond ((< n 2) 1) (#t (+ (fib (- n 1)) (fib (- n 2)))))))";
        let mut env = Env::new();
        run_str(&mut env, code);
        assert_eq!(run_str(&mut env, "(fib 0)"), SymbolicExpression::Int(1));
        assert_eq!(run_str(&mut env, "(fib 1)"), SymbolicExpression::Int(1));
        assert_eq!(run_str(&mut env, "(fib 2)"), SymbolicExpression::Int(2));
        assert_eq!(run_str(&mut env, "(fib 9)"), SymbolicExpression::Int(55));
    }

    #[test]
    fn test_let() {
        let code = "(let ((a 5) (b (+ 5 a))) (+ a b))";
        let mut env = Env::new();
        assert_eq!(run_str(&mut env, code), SymbolicExpression::Int(15));
    }
}
