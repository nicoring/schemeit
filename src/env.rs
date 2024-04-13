use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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

    fn define_symbol(&mut self, symbol: String, value: SymbolicExpression) {
        self.bindings.insert(symbol, value);
    }

    fn find_symbol(&self, symbol: &String) -> Option<SymbolicExpression> {
        self.bindings.get(symbol).cloned().or_else(|| {
            self.outer
                .as_ref()
                .and_then(|outer| outer.borrow().find_symbol(symbol))
        })
    }

    fn set_symbol(&mut self, symbol: &String, new_value: SymbolicExpression) {
        match self.bindings.get_mut(symbol) {
            Some(value) => {
                *value = new_value;
            }
            None => {
                self.outer
                    .as_ref()
                    .unwrap_or_else(|| panic!("should have variable '{}' defined", symbol))
                    .borrow_mut()
                    .set_symbol(symbol, new_value);
            }
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

    pub fn find_symbol(&self, symbol: &String) -> Option<SymbolicExpression> {
        self.current_frame.borrow().find_symbol(symbol)
    }

    pub fn define_symbol(&mut self, symbol: &String, value: SymbolicExpression) {
        self.current_frame
            .as_ref()
            .borrow_mut()
            .define_symbol(symbol.clone(), value);
    }

    pub fn set_symbol(&mut self, symbol: &String, new_value: SymbolicExpression) {
        self.current_frame
            .as_ref()
            .borrow_mut()
            .set_symbol(symbol, new_value);
    }
}

#[cfg(test)]
mod test {
    use super::Env;
    use crate::SymbolicExpression;

    type SE = SymbolicExpression;

    #[test]
    fn global_frame() {
        let mut global_env = Env::new();

        let name = "a".to_string();

        global_env.define_symbol(&name, SE::Nil);
        assert_eq!(global_env.find_symbol(&name), Some(SE::Nil));

        global_env.set_symbol(&name, SE::Bool(true));
        assert_eq!(global_env.find_symbol(&name), Some(SE::Bool(true)));
    }

    #[test]
    fn multiple_frames() {
        let mut env = Env::new();
        let a = "a".to_string();
        let b = "b".to_string();
        let c = "c".to_string();

        env.define_symbol(&a, SE::Nil);
        assert_eq!(env.find_symbol(&a), Some(SE::Nil));

        env.define_symbol(&b, SE::Str("b1".to_string()));
        assert_eq!(env.find_symbol(&b), Some(SE::Str("b1".to_string())));

        env.add_frame();

        env.define_symbol(&a, SE::Int(2));
        assert_eq!(env.find_symbol(&a), Some(SE::Int(2)));

        env.set_symbol(&b, SE::Str("b2".to_string()));
        assert_eq!(env.find_symbol(&b), Some(SE::Str("b2".to_string())));

        env.define_symbol(&c, SE::Str("c".to_string()));
        assert_eq!(env.find_symbol(&c), Some(SE::Str("c".to_string())));

        env.pop_frame();

        assert_eq!(env.find_symbol(&a), Some(SE::Nil));
        assert_eq!(env.find_symbol(&b), Some(SE::Str("b2".to_string())));
        assert_eq!(env.find_symbol(&c), None);
    }

    #[test]
    fn lambda_env() {
        let mut env = Env::new();
        let a = "a".to_string();

        env.add_frame();
        env.define_symbol(&a, SE::Int(1));

        let mut lambda_env = env.get_lambda_env();
        assert_eq!(lambda_env.find_symbol(&a), Some(SE::Int(1)));

        lambda_env.set_symbol(&a, SE::Int(2));
        assert_eq!(lambda_env.find_symbol(&a), Some(SE::Int(2)));
        assert_eq!(env.find_symbol(&a), Some(SE::Int(2)));

        env.pop_frame();
        assert_eq!(lambda_env.find_symbol(&a), Some(SE::Int(2)));
        assert_eq!(env.find_symbol(&a), None);
    }
}
