use crate::env::Env;
use crate::tokenize::Token;
use std::collections::VecDeque;
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
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

/// A cons cell with custom Drop to handle deeply nested lists iteratively.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsCell {
    pub head: Box<SymbolicExpression>,
    pub tail: Box<SymbolicExpression>,
}

impl Drop for ConsCell {
    fn drop(&mut self) {
        use std::mem::ManuallyDrop;

        // Iteratively drop the tail chain to prevent stack overflow.
        // We use ManuallyDrop to prevent recursive Drop calls on inner ConsCells.
        let mut current = std::mem::replace(&mut self.tail, Box::new(SymbolicExpression::Nil));

        loop {
            // Take the value out of the box
            let inner = std::mem::replace(&mut *current, SymbolicExpression::Nil);

            match inner {
                SymbolicExpression::Cons(cell) => {
                    // Wrap in ManuallyDrop to prevent automatic Drop
                    let cell = ManuallyDrop::new(cell);
                    // SAFETY: We're manually handling the drop of cell's fields.
                    // After this, cell is left in an undefined state but won't be dropped.
                    unsafe {
                        // Read and drop head
                        let head = std::ptr::read(&cell.head);
                        drop(head);
                        // Read tail for next iteration
                        current = std::ptr::read(&cell.tail);
                    }
                    // cell is ManuallyDrop, so no Drop is triggered
                }
                _ => break, // Not a Cons, done
            }
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
    Cons(ConsCell),
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
            Self::Cons(ConsCell { head, tail }) => write!(f, "({} . {})", head, tail),
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
