use crate::env::Env;
use crate::tokenize::Token;
use std::collections::VecDeque;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Operation {
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
    Let,
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
            "let" => Some(Operation::Let),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolicExpression {
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

impl PartialOrd for SymbolicExpression {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Str(left), Self::Str(right)) => left.partial_cmp(right),
            (Self::Float(left), Self::Float(right)) => left.partial_cmp(right),
            (Self::Int(left), Self::Int(right)) => left.partial_cmp(right),
            (Self::Int(left), Self::Float(right)) => (*left as f64).partial_cmp(right),
            (Self::Float(left), Self::Int(right)) => left.partial_cmp(&(*right as f64)),
            _ => None,
        }
    }
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

pub fn parse(tokens: &mut VecDeque<Token>) -> SymbolicExpression {
    let mut values = Vec::new();
    while let Some(token) = tokens.pop_front() {
        let value = match token {
            Token::RightParanthesis => break,
            Token::LeftParanthesis => parse(tokens),
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
