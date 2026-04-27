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

    pub fn get_at(&self, token: &Token, distance: usize) -> Result<LiteralType, String> {
        let value = if distance == 0 {
            self.values.get(&token.lexeme).cloned()
        } else {
            let env = self.ancestor(distance)?;
            env.borrow().values.get(&token.lexeme).cloned()
        };

        if let Option::Some(val) = value {
            return Result::Ok(val);
        } else {
            return Result::Err(format!("Undefined variable '{}'", token.lexeme));
        }
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

    pub fn assign_at(
        &mut self,
        name: &Token,
        value: &LiteralType,
        distance: usize,
    ) -> Result<(), String> {
        if distance == 0 {
            self.values.insert(name.lexeme.clone(), value.clone());
        } else {
            let env = self.ancestor(distance)?;
            env.borrow_mut()
                .values
                .insert(name.lexeme.clone(), value.clone());
        }

        Result::Ok(())
    }

    fn ancestor(&self, distance: usize) -> Result<Rc<RefCell<Environment>>, String> {
        let mut env = self.enclosing.clone();

        for _ in 1..distance {
            if let Option::Some(enclosing) = env {
                env = enclosing.borrow().enclosing.clone();
            } else {
                return Result::Err(format!("Cannot find an environment at distance {distance}"));
            }
        }

        if let Option::Some(enclosing) = env {
            Result::Ok(enclosing)
        } else {
            Result::Err(format!("Cannot find an environment at distance {distance}"))
        }
    }
}
