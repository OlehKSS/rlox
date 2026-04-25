use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::scanner::{LiteralType, Token};

#[derive(Debug)]
pub struct Environment {
    values: HashMap<String, LiteralType>,
    enclosing: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            values: HashMap::new(),
            enclosing: Option::None,
        }
    }

    pub fn new_with_enclosing(enclosing: Rc<RefCell<Environment>>) -> Self {
        Environment {
            values: HashMap::new(),
            enclosing: Option::Some(enclosing),
        }
    }

    pub fn define(&mut self, name: &str, value: &LiteralType) {
        self.values.insert(name.to_string(), value.clone());
    }

    pub fn get(&self, token: &Token) -> Result<LiteralType, String> {
        if let Option::Some(value) = self.values.get(&token.lexeme) {
            return Result::Ok(value.clone());
        }

        if let Option::Some(enclosing) = &self.enclosing {
            return enclosing.borrow().get(&token);
        }

        Result::Err(format!("Undefined variable '{}'.", token.lexeme))
    }

    pub fn assign(&mut self, name: &Token, value: &LiteralType) -> Result<(), String> {
        if self.values.contains_key(&name.lexeme) {
            self.values.insert(name.lexeme.clone(), value.clone());
            return Result::Ok(());
        }

        if let Option::Some(enclosing) = &self.enclosing {
            return enclosing.borrow_mut().assign(name, value);
        }

        Result::Err(format!("Undefined variable '{}'.", name.lexeme))
    }
}
