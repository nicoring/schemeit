use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::error::{InterpreterError, Result};
use crate::SymbolicExpression;

type Bindings = HashMap<String, SymbolicExpression>;
type FrameLink = Rc<RefCell<Frame>>;

#[derive(Debug, Clone, PartialEq)]
struct Frame {
    bindings: Bindings,
    outer: Option<FrameLink>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Env {
    current_frame: FrameLink,
}

#[derive(Debug, Clone)]
pub struct VariableNotFoundError;

impl Frame {
    fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            outer: None,
        }
    }

    fn with_outer(outer: FrameLink) -> Self {
        Self {
            bindings: HashMap::new(),
            outer: Some(outer),
        }
    }

    fn define_symbol(&mut self, symbol: &str, value: SymbolicExpression) {
        self.bindings.insert(symbol.to_owned(), value);
    }

    fn find_symbol(&self, symbol: &str) -> Option<SymbolicExpression> {
        self.bindings.get(symbol).cloned().or_else(|| {
            self.outer
                .as_ref()
                .and_then(|outer| outer.borrow().find_symbol(symbol))
        })
    }

    fn set_symbol(&mut self, symbol: &str, new_value: SymbolicExpression) -> Result<()> {
        match self.bindings.get_mut(symbol) {
            Some(value) => {
                *value = new_value;
                Ok(())
            }
            None => match self.outer.as_ref() {
                Some(outer) => outer.borrow_mut().set_symbol(symbol, new_value),
                None => Err(InterpreterError::VariableNotFound(symbol.to_string())),
            },
        }
    }
}

impl Env {
    pub fn new() -> Self {
        Env {
            current_frame: Rc::new(RefCell::new(Frame::new())),
        }
    }

    fn with_frame(frame: Frame) -> Self {
        Env {
            current_frame: Rc::new(RefCell::new(frame)),
        }
    }

    pub fn add_frame(&mut self) {
        let new_frame = Frame::with_outer(self.current_frame.clone());
        self.current_frame = Rc::new(RefCell::new(new_frame));
    }

    pub fn pop_frame(&mut self) {
        let new_current_frame = self.current_frame.borrow().outer.clone().expect(
            "should have outer frame, seems like you are trying to remove the global frame",
        );
        self.current_frame = new_current_frame;
    }

    pub fn get_lambda_env(&self) -> Env {
        let new_frame = Frame::with_outer(self.current_frame.clone());
        Env::with_frame(new_frame)
    }

    pub fn find_symbol(&self, symbol: &str) -> Result<SymbolicExpression> {
        self.current_frame
            .borrow()
            .find_symbol(symbol)
            .ok_or(InterpreterError::VariableNotFound(symbol.to_string()))
    }

    pub fn define_symbol(&mut self, symbol: &str, value: SymbolicExpression) {
        self.current_frame
            .as_ref()
            .borrow_mut()
            .define_symbol(symbol, value);
    }

    pub fn set_symbol(&mut self, symbol: &str, new_value: SymbolicExpression) -> Result<()> {
        self.current_frame
            .as_ref()
            .borrow_mut()
            .set_symbol(symbol, new_value)
    }
}

#[cfg(test)]
mod tests {
    use super::Env;
    use crate::error::Result;
    use crate::SymbolicExpression;

    type SE = SymbolicExpression;

    #[test]
    fn global_frame() -> Result<()> {
        let mut global_env = Env::new();

        let name = "a";

        global_env.define_symbol(name, SE::Nil);
        assert_eq!(global_env.find_symbol(name)?, SE::Nil);

        global_env.set_symbol(name, SE::Bool(true))?;
        assert_eq!(global_env.find_symbol(name)?, SE::Bool(true));
        Ok(())
    }

    #[test]
    fn multiple_frames() -> Result<()> {
        let mut env = Env::new();
        let a = "a";
        let b = "b";
        let c = "c";

        env.define_symbol(a, SE::Nil);
        assert_eq!(env.find_symbol(a)?, SE::Nil);

        env.define_symbol(b, SE::Str("b1".to_string()));
        assert_eq!(env.find_symbol(b)?, SE::Str("b1".to_string()));

        env.add_frame();

        env.define_symbol(a, SE::Int(2));
        assert_eq!(env.find_symbol(a)?, SE::Int(2));

        env.set_symbol(b, SE::Str("b2".to_string()))?;
        assert_eq!(env.find_symbol(b)?, SE::Str("b2".to_string()));

        env.define_symbol(c, SE::Str("c".to_string()));
        assert_eq!(env.find_symbol(c)?, SE::Str("c".to_string()));

        env.pop_frame();

        assert_eq!(env.find_symbol(a)?, SE::Nil);
        assert_eq!(env.find_symbol(b)?, SE::Str("b2".to_string()));
        assert!(env.find_symbol(c).is_err());
        Ok(())
    }

    #[test]
    fn lambda_env() -> Result<()> {
        let mut env = Env::new();
        let a = "a";

        env.add_frame();
        env.define_symbol(a, SE::Int(1));

        let mut lambda_env = env.get_lambda_env();
        assert_eq!(lambda_env.find_symbol(a)?, SE::Int(1));

        lambda_env.set_symbol(a, SE::Int(2))?;
        assert_eq!(lambda_env.find_symbol(a)?, SE::Int(2));
        assert_eq!(env.find_symbol(a)?, SE::Int(2));

        env.pop_frame();
        assert_eq!(lambda_env.find_symbol(a)?, SE::Int(2));
        assert!(env.find_symbol(a).is_err());
        Ok(())
    }
}
