mod env;
mod error;
mod eval;
mod interpreter;
mod parse;
mod tokenize;

use std::env as std_env;
use std::io::{self, Write};
use std::time::Instant;

use interpreter::Interpreter;
use parse::SymbolicExpression;
use tokenize::tokenize;

fn repl() {
    let mut interp = Interpreter::new();
    interp.eval_file("std.scm").unwrap();
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
        if line.is_empty() {
            continue;
        }
        let result = interp.eval_str(&line);
        match result {
            Ok(result) => println!("out: {}", result),
            Err(err) => println!("{}", err),
        };
    }
}

fn benchmark() {
    let code_strings = vec![
        "(fib 30)",
        "(reduce + (map (lambda (x) (* x x)) (range 1000)))",
        "(reducei + (map (lambda (x) (* x x)) (range 1000)))",
        "(reducei + (mapi (lambda (x) (* x x)) (range 1000)))",
        "(reduce + (map (lambda (x) (* x x)) (range 10000)))",
        "(reducei + (map (lambda (x) (* x x)) (range 10000)))",
        "(reducei + (mapi (lambda (x) (* x x)) (range 10000)))",
    ];

    for code_string in code_strings {
        let mut interp = Interpreter::new();
        interp.eval_file("std.scm").unwrap();

        let now = Instant::now();
        {
            let _ = interp.eval_str(code_string);
        }
        let elapsed = now.elapsed();
        println!("{} took: {:.2?}", code_string, elapsed);
    }
}

fn test() {
    let code = "(mapi (lambda (x) (* x x)) (range 1000))";
    let mut interp = Interpreter::new();
    interp.eval_file("std.scm").unwrap();
    let expression = parse::parse(&mut tokenize(code));
    println!("{}", expression);
    println!("{}", interp.eval_str("mapi").unwrap());
}

fn run_file(filename: &str) {
    let mut interp = Interpreter::new();
    let result = interp.eval_file(filename);
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
    } else if args[1] == "--test" {
        test();
    } else {
        run_file(args[1].as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_define_function() {
        let mut interp = Interpreter::new();
        interp.eval_str("(define pi 3.141592653)").unwrap();
        interp.eval_str("(define circle-area (lambda (r) (* pi (* r r))))").unwrap();
        assert_eq!(
            interp.eval_str("(circle-area 3)").unwrap(),
            SymbolicExpression::Float(28.274333877)
        );
        assert_eq!(
            interp.eval_str("(circle-area 3)").unwrap(),
            SymbolicExpression::Float(28.274333877)
        );
    }

    #[test]
    fn account_state() {
        let mut interp = Interpreter::new();
        let code = "
        (define make-account
            (lambda (balance)
              (lambda (amt)
                  (begin (set! balance (+ balance amt))
                          balance))))
        ";
        interp.eval_str(code).unwrap();
        let code = "(define account (make-account 100.00))";
        interp.eval_str(code).unwrap();
        let code = "(account -20.00)";
        assert_eq!(
            interp.eval_str(code).unwrap(),
            SymbolicExpression::Float(80.0)
        );
        assert_eq!(
            interp.eval_str(code).unwrap(),
            SymbolicExpression::Float(60.0)
        );
    }

    #[test]
    fn fib() {
        let code =
            "(define fib (lambda (n) (cond ((< n 2) 1) (#t (+ (fib (- n 1)) (fib (- n 2)))))))";
        let mut interp = Interpreter::new();
        interp.eval_str(code).unwrap();
        assert_eq!(
            interp.eval_str("(fib 0)").unwrap(),
            SymbolicExpression::Int(1)
        );
        assert_eq!(
            interp.eval_str("(fib 1)").unwrap(),
            SymbolicExpression::Int(1)
        );
        assert_eq!(
            interp.eval_str("(fib 2)").unwrap(),
            SymbolicExpression::Int(2)
        );
        assert_eq!(
            interp.eval_str("(fib 9)").unwrap(),
            SymbolicExpression::Int(55)
        );
    }

    #[test]
    fn test_let() {
        let code = "(let ((a 5) (b (+ 5 a))) (+ a b))";
        let mut interp = Interpreter::new();
        assert_eq!(
            interp.eval_str(code).unwrap(),
            SymbolicExpression::Int(15)
        );
    }

    #[test]
    fn tail_recursive_sum() {
        let mut interp = Interpreter::new();
        // Define tail-recursive sum: sum-iter(n, acc) = if n==0 then acc else sum-iter(n-1, acc+n)
        interp.eval_str(
            "(define sum-iter (lambda (n acc) (if (= n 0) acc (sum-iter (- n 1) (+ acc n)))))",
        )
        .unwrap();
        // This would stack overflow without TCO
        assert_eq!(
            interp.eval_str("(sum-iter 10000 0)").unwrap(),
            SymbolicExpression::Int(50005000)
        );
    }

    #[test]
    fn type_error_in_arithmetic() {
        let mut interp = Interpreter::new();
        assert!(interp.eval_str("(+ 1 \"hello\")").is_err());
        assert!(interp.eval_str("(- 1 \"hello\")").is_err());
        assert!(interp.eval_str("(* 1 \"hello\")").is_err());
        assert!(interp.eval_str("(/ 1 \"hello\")").is_err());
    }

    #[test]
    fn arity_error_in_math_ops() {
        let mut interp = Interpreter::new();
        assert!(interp.eval_str("(exp)").is_err());
        assert!(interp.eval_str("(pow 2)").is_err());
    }
}
