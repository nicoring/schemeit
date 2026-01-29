use std::fs;

use crate::env::Env;
use crate::error::Result;
use crate::eval::eval;
use crate::parse::{parse, SymbolicExpression};
use crate::tokenize::tokenize;

pub struct Interpreter {
    env: Env,
}

impl Interpreter {
    pub fn new() -> Self {
        Self { env: Env::new() }
    }

    pub fn eval_str(&mut self, code: &str) -> Result<SymbolicExpression> {
        let mut tokens = tokenize(code);
        let expression = parse(&mut tokens);
        // parse wraps everything in an Expression; unwrap single-element top-level
        let expression = match expression {
            SymbolicExpression::Expression(ref exprs) if exprs.len() == 1 => exprs[0].clone(),
            _ => expression,
        };
        eval(&mut self.env, &expression)
    }

    pub fn eval_file(&mut self, path: &str) -> Result<SymbolicExpression> {
        let code = fs::read_to_string(path)?;
        self.eval_str(&code)
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
