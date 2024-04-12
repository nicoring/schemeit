mod env;

use std::collections::VecDeque;
use std::fmt::Display;
use std::fs;
use std::io::{self, Write};
use std::slice::Iter;

use env::Env;

#[derive(Debug, Clone, PartialEq)]
enum Operation {
    Add,
    Substract,
    Divide,
    Multiply,
    Pow,
    Exp,
    Car,
    Cdr,
    Cons,
    List,
    Begin,
    Module,
    Cond,
    If,
    Eq,
    Smaller,
    Greater,
    SmallerOrEqual,
    GreaterOrEqual,
    Define,
    Set,
    Lambda,
    Quote,
}

impl Operation {
    fn get(operation_name: &str) -> Option<Operation> {
        match operation_name {
            "+" => Some(Operation::Add),
            "-" => Some(Operation::Substract),
            "*" => Some(Operation::Multiply),
            "/" => Some(Operation::Divide),
            "pow" => Some(Operation::Pow),
            "exp" => Some(Operation::Exp),
            "car" => Some(Operation::Car),
            "cdr" => Some(Operation::Cdr),
            "cons" => Some(Operation::Cons),
            "list" => Some(Operation::List),
            "begin" => Some(Operation::Begin),
            "module" => Some(Operation::Module),
            "cond" => Some(Operation::Cond),
            "if" => Some(Operation::If),
            "=" => Some(Operation::Eq),
            "<" => Some(Operation::Smaller),
            ">" => Some(Operation::Greater),
            "<=" => Some(Operation::SmallerOrEqual),
            ">=" => Some(Operation::GreaterOrEqual),
            "define" => Some(Operation::Define),
            "set!" => Some(Operation::Set),
            "lambda" => Some(Operation::Lambda),
            "quote" => Some(Operation::Quote),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum SymbolicExpression {
    Str(String),
    Symbol(String),
    Float(f64),
    Int(i128),
    Bool(bool),
    Cons {
        head: Box<SymbolicExpression>,
        tail: Box<SymbolicExpression>,
    },
    Nil,
    Expression(Vec<SymbolicExpression>),
    Lambda {
        parameters: Vec<String>,
        env: Env,
        body: Box<SymbolicExpression>,
    },
    Operation(Operation),
}

impl Display for SymbolicExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Float(value) => write!(f, "{}", value),
            Self::Int(value) => write!(f, "{}", value),
            Self::Str(value) => write!(f, "{}", value),
            Self::Cons { head, tail } => write!(f, "({} . {})", head, tail),
            Self::Symbol(value) => write!(f, "#{}", value),
            Self::Bool(value) => write!(f, "{}", if *value { "#t" } else { "#f" }),
            Self::Nil => write!(f, "#nil"),
            Self::Expression(values) => write!(f, "({:?})", values),
            Self::Lambda {
                parameters, body, ..
            } => {
                write!(f, "(lambda ({:?}) ({:?}))", parameters, body)
            }
            Self::Operation(operation) => write!(f, "{:?}", operation),
        }
    }
}

#[derive(Debug)]
enum Token {
    LeftParanthesis,
    RightParanthesis,
    Int(i128),
    Float(f64),
    String(String),
    Symbol(String),
}

fn tokenize(code: &str) -> VecDeque<Token> {
    code.replace("(", " ( ")
        .replace(")", " ) ")
        .split_whitespace()
        .map(|token| match token {
            "(" => Token::LeftParanthesis,
            ")" => Token::RightParanthesis,
            _ => {
                if let Ok(int) = token.parse::<i128>() {
                    Token::Int(int)
                } else if let Ok(float) = token.parse::<f64>() {
                    Token::Float(float)
                } else if token.starts_with("\"") & token.ends_with("\"") {
                    Token::String(token.to_string())
                } else {
                    Token::Symbol(token.to_string())
                }
            }
        })
        .collect()
}

fn parse_token_list(tokens: &mut VecDeque<Token>) -> SymbolicExpression {
    let mut values = Vec::new();
    while let Some(token) = tokens.pop_front() {
        let value = match token {
            Token::RightParanthesis => break,
            Token::LeftParanthesis => parse_token_list(tokens),
            Token::Float(value) => SymbolicExpression::Float(value),
            Token::Int(value) => SymbolicExpression::Int(value),
            Token::String(value) => SymbolicExpression::Str(value),
            Token::Symbol(value) => match value.as_str() {
                "#nil" => SymbolicExpression::Nil,
                "#t" => SymbolicExpression::Bool(true),
                "#f" => SymbolicExpression::Bool(false),
                _ => {
                    if let Some(operation) = Operation::get(&value) {
                        SymbolicExpression::Operation(operation)
                    } else {
                        SymbolicExpression::Symbol(value)
                    }
                }
            },
        };
        values.push(value);
    }
    SymbolicExpression::Expression(values)
}

fn parse(code: &str) -> SymbolicExpression {
    let mut tokens = tokenize(code);
    tokens.pop_front();
    let result = parse_token_list(&mut tokens);
    result
}

fn eval_operation<'a>(
    env: &mut Env,
    operation: Operation,
    expression_iter: &mut impl DoubleEndedIterator<Item = &'a SymbolicExpression>,
) -> SymbolicExpression {
    let mut eval_w_env = |expression| eval(env, expression);

    match operation {
        Operation::Add => expression_iter
            .map(eval_w_env)
            .reduce(|acc, elem| match (acc, elem) {
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Float(elem_value)) => {
                    SymbolicExpression::Float(acc_value + elem_value)
                }
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Int(elem_value)) => {
                    SymbolicExpression::Float(acc_value + elem_value as f64)
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Float(elem_value)) => {
                    SymbolicExpression::Float(acc_value as f64 + elem_value)
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Int(elem_value)) => {
                    SymbolicExpression::Int(acc_value + elem_value)
                }
                _ => panic!("alarm reduce"),
            })
            .unwrap(),
        Operation::Substract => expression_iter
            .map(eval_w_env)
            .reduce(|acc, elem| match (acc, elem) {
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Float(elem_value)) => {
                    SymbolicExpression::Float(acc_value - elem_value)
                }
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Int(elem_value)) => {
                    SymbolicExpression::Float(acc_value - elem_value as f64)
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Float(elem_value)) => {
                    SymbolicExpression::Float(acc_value as f64 - elem_value)
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Int(elem_value)) => {
                    SymbolicExpression::Int(acc_value - elem_value)
                }
                _ => panic!("alarm reduce"),
            })
            .unwrap(),
        Operation::Multiply => expression_iter
            .map(eval_w_env)
            .reduce(|acc, elem| match (acc, elem) {
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Float(elem_value)) => {
                    SymbolicExpression::Float(acc_value * elem_value)
                }
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Int(elem_value)) => {
                    SymbolicExpression::Float(acc_value * elem_value as f64)
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Float(elem_value)) => {
                    SymbolicExpression::Float(acc_value as f64 * elem_value)
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Int(elem_value)) => {
                    SymbolicExpression::Int(acc_value * elem_value)
                }
                _ => panic!("alarm reduce"),
            })
            .unwrap(),
        Operation::Divide => expression_iter
            .map(eval_w_env)
            .reduce(|acc, elem| match (acc, elem) {
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Float(elem_value)) => {
                    SymbolicExpression::Float(acc_value / elem_value)
                }
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Int(elem_value)) => {
                    SymbolicExpression::Float(acc_value / elem_value as f64)
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Float(elem_value)) => {
                    SymbolicExpression::Float(acc_value as f64 / elem_value)
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Int(elem_value)) => {
                    SymbolicExpression::Float(acc_value as f64 / elem_value as f64)
                }
                _ => panic!("alarm reduce"),
            })
            .unwrap(),
        Operation::Exp => match expression_iter.map(eval_w_env).next().unwrap() {
            SymbolicExpression::Float(value) => SymbolicExpression::Float(value.exp()),
            SymbolicExpression::Int(value) => SymbolicExpression::Float((value as f64).exp()),
            value => panic!("exp on {}", value),
        },
        Operation::Pow => {
            let mut evaluated_arguments = expression_iter.map(eval_w_env);
            let value_first = evaluated_arguments.next().unwrap();
            let value_second = evaluated_arguments.next().unwrap();
            match (value_first, value_second) {
                (SymbolicExpression::Float(first), SymbolicExpression::Float(second)) => {
                    SymbolicExpression::Float(first.powf(second))
                }
                (SymbolicExpression::Float(first), SymbolicExpression::Int(second)) => {
                    SymbolicExpression::Float(first.powi(second as i32))
                }
                (SymbolicExpression::Int(first), SymbolicExpression::Float(second)) => {
                    SymbolicExpression::Float((first as f64).powf(second))
                }
                (SymbolicExpression::Int(first), SymbolicExpression::Int(second)) => {
                    SymbolicExpression::Int(first.pow(second as u32))
                }
                _ => panic!("alarm pow"),
            }
        }
        Operation::Begin => {
            env.add_frame();
            let result = expression_iter.map(|el| eval(env, el)).last().unwrap();
            env.pop_frame();
            result
        }
        Operation::Module => {
            expression_iter.for_each(|el| {
                eval_w_env(el);
            });
            SymbolicExpression::Nil
        }
        Operation::Cons => {
            let mut args = expression_iter.map(eval_w_env);
            let head = Box::new(args.next().unwrap());
            let tail = Box::new(args.next().unwrap());
            SymbolicExpression::Cons { head, tail }
        }
        Operation::List => {
            expression_iter
                .map(eval_w_env)
                .rfold(SymbolicExpression::Nil, |acc, elem| {
                    SymbolicExpression::Cons {
                        head: Box::new(elem),
                        tail: Box::new(acc),
                    }
                })
        }
        Operation::Car => match expression_iter.map(eval_w_env).next().unwrap() {
            SymbolicExpression::Cons { head, .. } => *head,
            _ => panic!("car on non cons type"),
        },
        Operation::Cdr => match expression_iter.map(eval_w_env).next().unwrap() {
            SymbolicExpression::Cons { tail, .. } => *tail,
            _ => panic!("car on non cons type"),
        },
        Operation::Eq => {
            let mut evaluated_arguments = expression_iter.map(eval_w_env);
            let first = evaluated_arguments.next().unwrap();
            SymbolicExpression::Bool(evaluated_arguments.all(|each| {
                match (&first, &each) {
                    (
                        SymbolicExpression::Float(first_value),
                        SymbolicExpression::Float(second_value),
                    ) => first_value == second_value,
                    (
                        SymbolicExpression::Float(first_value),
                        SymbolicExpression::Int(second_value),
                    ) => *first_value == (*second_value as f64),
                    (
                        SymbolicExpression::Int(first_value),
                        SymbolicExpression::Float(second_value),
                    ) => (*first_value as f64) == *second_value,
                    (
                        SymbolicExpression::Int(first_value),
                        SymbolicExpression::Int(second_value),
                    ) => first_value == second_value,
                    (
                        SymbolicExpression::Str(first_value),
                        SymbolicExpression::Str(second_value),
                    ) => first_value == second_value,
                    (
                        SymbolicExpression::Bool(first_value),
                        SymbolicExpression::Bool(second_value),
                    ) => first_value == second_value,
                    // (SymbolicExpression::Cons {head: first_head, tail: first_tail},
                    //     SymbolicExpression::Cons { head: second_head, tail: second_tail})
                    //     => todo!(),
                    (SymbolicExpression::Nil, SymbolicExpression::Nil) => true,
                    _ => false,
                }
            }))
        }
        Operation::Smaller => {
            let mut evaluated_arguments = expression_iter.map(eval_w_env);
            let first = evaluated_arguments.next().unwrap();
            SymbolicExpression::Bool(evaluated_arguments.all(|each| match (&first, &each) {
                (
                    SymbolicExpression::Float(first_value),
                    SymbolicExpression::Float(second_value),
                ) => first_value < second_value,
                (SymbolicExpression::Float(first_value), SymbolicExpression::Int(second_value)) => {
                    *first_value < (*second_value as f64)
                }
                (SymbolicExpression::Int(first_value), SymbolicExpression::Float(second_value)) => {
                    (*first_value as f64) < *second_value
                }
                (SymbolicExpression::Int(first_value), SymbolicExpression::Int(second_value)) => {
                    first_value < second_value
                }
                (SymbolicExpression::Str(first_value), SymbolicExpression::Str(second_value)) => {
                    first_value < second_value
                }
                (SymbolicExpression::Bool(first_value), SymbolicExpression::Bool(second_value)) => {
                    first_value < second_value
                }
                _ => false,
            }))
        }
        Operation::Greater => {
            let mut evaluated_arguments = expression_iter.map(eval_w_env);
            let first = evaluated_arguments.next().unwrap();
            SymbolicExpression::Bool(evaluated_arguments.all(|each| match (&first, &each) {
                (
                    SymbolicExpression::Float(first_value),
                    SymbolicExpression::Float(second_value),
                ) => first_value > second_value,
                (SymbolicExpression::Float(first_value), SymbolicExpression::Int(second_value)) => {
                    *first_value > (*second_value as f64)
                }
                (SymbolicExpression::Int(first_value), SymbolicExpression::Float(second_value)) => {
                    (*first_value as f64) > *second_value
                }
                (SymbolicExpression::Int(first_value), SymbolicExpression::Int(second_value)) => {
                    first_value > second_value
                }
                (SymbolicExpression::Str(first_value), SymbolicExpression::Str(second_value)) => {
                    first_value > second_value
                }
                (SymbolicExpression::Bool(first_value), SymbolicExpression::Bool(second_value)) => {
                    first_value > second_value
                }
                _ => false,
            }))
        }
        Operation::GreaterOrEqual => {
            let mut evaluated_arguments = expression_iter.map(eval_w_env);
            let first = evaluated_arguments.next().unwrap();
            SymbolicExpression::Bool(evaluated_arguments.all(|each| match (&first, &each) {
                (
                    SymbolicExpression::Float(first_value),
                    SymbolicExpression::Float(second_value),
                ) => first_value >= second_value,
                (SymbolicExpression::Float(first_value), SymbolicExpression::Int(second_value)) => {
                    *first_value >= (*second_value as f64)
                }
                (SymbolicExpression::Int(first_value), SymbolicExpression::Float(second_value)) => {
                    (*first_value as f64) >= *second_value
                }
                (SymbolicExpression::Int(first_value), SymbolicExpression::Int(second_value)) => {
                    first_value >= second_value
                }
                (SymbolicExpression::Str(first_value), SymbolicExpression::Str(second_value)) => {
                    first_value >= second_value
                }
                (SymbolicExpression::Bool(first_value), SymbolicExpression::Bool(second_value)) => {
                    first_value >= second_value
                }
                _ => false,
            }))
        }
        Operation::SmallerOrEqual => {
            let mut evaluated_arguments = expression_iter.map(eval_w_env);
            let first = evaluated_arguments.next().unwrap();
            SymbolicExpression::Bool(evaluated_arguments.all(|each| match (&first, &each) {
                (
                    SymbolicExpression::Float(first_value),
                    SymbolicExpression::Float(second_value),
                ) => first_value <= second_value,
                (SymbolicExpression::Float(first_value), SymbolicExpression::Int(second_value)) => {
                    *first_value <= (*second_value as f64)
                }
                (SymbolicExpression::Int(first_value), SymbolicExpression::Float(second_value)) => {
                    (*first_value as f64) <= *second_value
                }
                (SymbolicExpression::Int(first_value), SymbolicExpression::Int(second_value)) => {
                    first_value <= second_value
                }
                (SymbolicExpression::Str(first_value), SymbolicExpression::Str(second_value)) => {
                    first_value <= second_value
                }
                (SymbolicExpression::Bool(first_value), SymbolicExpression::Bool(second_value)) => {
                    first_value <= second_value
                }
                _ => false,
            }))
        }
        Operation::If => {
            let predicate = eval_w_env(expression_iter.next().unwrap());
            match predicate {
                SymbolicExpression::Bool(true) => eval_w_env(expression_iter.next().unwrap()),
                SymbolicExpression::Bool(false) => {
                    eval_w_env(expression_iter.skip(1).next().unwrap())
                }
                _ => panic!("predicate must evaluate to boolean"),
            }
        }
        Operation::Cond => expression_iter
            .find_map(|expression| match expression {
                SymbolicExpression::Expression(values) => {
                    let predicate = eval_w_env(&values[0]);
                    match predicate {
                        SymbolicExpression::Bool(true) => Some(eval_w_env(&values[1])),
                        SymbolicExpression::Bool(false) => None,
                        _ => panic!("predicate must evaluate to boolean"),
                    }
                }
                other => panic!("Invalid args to cond: {}", other),
            })
            .unwrap(),
        Operation::Quote => expression_iter.next().expect("Nothing to quote").clone(),
        Operation::Define => {
            let name = match expression_iter.next() {
                Some(SymbolicExpression::Symbol(value)) => value,
                _ => panic!("Invalid args to define"),
            };
            let value = eval_w_env(expression_iter.next().unwrap());
            env.define_symbol(name, value);
            SymbolicExpression::Nil
        }
        Operation::Set => {
            let name = match expression_iter.next() {
                Some(SymbolicExpression::Symbol(value)) => value,
                _ => panic!("Invalid args to set!"),
            };
            let value = eval_w_env(expression_iter.next().unwrap());
            env.set_symbol(name, value);
            SymbolicExpression::Nil
        }
        Operation::Lambda => {
            let parameters = match expression_iter.next().unwrap() {
                SymbolicExpression::Expression(values) => {
                    values.into_iter().map(|each| match each {
                        SymbolicExpression::Symbol(name) => name.to_owned(),
                        _ => panic!("non symbol arg in lambda {}", each),
                    })
                }
                _ => panic!("invalid arg list for lambda"),
            }
            .collect();

            let body: Box<SymbolicExpression> = Box::new(expression_iter.next().unwrap().clone());
            let lambda_env = env.get_lambda_env();
            SymbolicExpression::Lambda {
                parameters,
                env: lambda_env,
                body,
            }
        }
    }
}

fn eval_lambda(
    env: &mut Env,
    lambda_env: &mut Env,
    parameters: &Vec<String>,
    body: &SymbolicExpression,
    expression_iter: &mut Iter<'_, SymbolicExpression>,
) -> SymbolicExpression {
    lambda_env.add_frame();
    parameters
        .iter()
        .zip(expression_iter)
        .for_each(|(param, expression)| {
            let value = eval(env, expression);
            lambda_env.define_symbol(param, value);
        });

    let result = eval(lambda_env, body);
    lambda_env.pop_frame();
    result
}

fn eval_expression(env: &mut Env, expression: &Vec<SymbolicExpression>) -> SymbolicExpression {
    let mut expression_iter = expression.into_iter();

    let first_expression = eval(env, expression_iter.next().unwrap());

    match first_expression {
        SymbolicExpression::Operation(operation) => {
            eval_operation(env, operation, &mut expression_iter)
        }
        SymbolicExpression::Lambda {
            parameters,
            env: mut lambda_env,
            body,
        } => eval_lambda(
            env,
            &mut lambda_env,
            &parameters,
            &body,
            &mut expression_iter,
        ),
        _ => panic!("invalid first argument in expression {}", first_expression),
    }
}

fn eval(env: &mut Env, expression: &SymbolicExpression) -> SymbolicExpression {
    match expression {
        SymbolicExpression::Symbol(name) => env
            .find_symbol(name)
            .expect(format!("could not find symbol {}", name).as_str()),
        SymbolicExpression::Expression(expression) => eval_expression(env, expression),
        value => value.clone(),
    }
}

fn eval_str(env: &mut Env, code: &str) -> SymbolicExpression {
    let expression = parse(code);
    let result = eval(env, &expression);
    result
}

fn eval_file(env: &mut Env, filename: &str) -> SymbolicExpression {
    let contents = fs::read_to_string(filename).expect("Should have been able to read the file");
    eval_str(env, &contents)
}

fn repl() {
    let mut env = Env::new();
    eval_file(&mut env, "test.scm");
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
        let result = eval_str(&mut env, &line);
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
        eval_str(&mut env, "(define pi 3.141592653)");
        eval_str(&mut env, "(define circle-area (lambda (r) (* pi (* r r))))");
        assert_eq!(
            eval_str(&mut env, "(circle-area 3)"),
            SymbolicExpression::Float(28.274333877)
        );
        assert_eq!(
            eval_str(&mut env, "(circle-area 3)"),
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
        eval_str(&mut env, &code);
        let code = "(define account (make-account 100.00))";
        eval_str(&mut env, code);
        let code = "(account -20.00)";
        assert_eq!(eval_str(&mut env, code), SymbolicExpression::Float(80.0));
        assert_eq!(eval_str(&mut env, code), SymbolicExpression::Float(60.0));
    }

    #[test]
    fn fib() {
        let code =
            "(define fib (lambda (n) (cond ((< n 2) 1) (#t (+ (fib (- n 1)) (fib (- n 2)))))))";
        let mut env = Env::new();
        eval_str(&mut env, code);
        assert_eq!(eval_str(&mut env, "(fib 0)"), SymbolicExpression::Int(1));
        assert_eq!(eval_str(&mut env, "(fib 1)"), SymbolicExpression::Int(1));
        assert_eq!(eval_str(&mut env, "(fib 2)"), SymbolicExpression::Int(2));
        assert_eq!(eval_str(&mut env, "(fib 9)"), SymbolicExpression::Int(55));
    }
}
