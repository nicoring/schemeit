use std::error;
use std::fmt;

use crate::parse::SymbolicExpression;

pub type Result<T> = std::result::Result<T, InterpreterError>;

#[derive(Debug)]
pub enum InterpreterError {
    VariableNotFound(String),
    SyntaxError(SymbolicExpression),
    RuntimeError(String),
    ValueError(String),
    ArgumentError(String),
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VariableNotFound(name) => {
                write!(f, "RuntimeError: variable {} not found", name)
            }
            Self::SyntaxError(exp) => write!(f, "SyntaxError {}", exp),
            Self::RuntimeError(explanation) => write!(f, "RuntimeError: {}", explanation),
            Self::ValueError(explanation) => write!(f, "ValueError: {}", explanation),
            Self::ArgumentError(explanation) => write!(f, "ArgumentError: {}", explanation),
        }
    }
}

impl error::Error for InterpreterError {}
