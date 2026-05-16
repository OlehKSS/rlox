use std::cell::RefCell;
use std::rc::Rc;

use super::scanner::{LiteralType, Token};

#[derive(Debug)]
pub struct Environment {
    values: Vec<(String, LiteralType)>,
    pub enclosing: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            values: Vec::new(),
            enclosing: Option::None,
        }
    }

    pub fn new_with_enclosing(enclosing: Rc<RefCell<Environment>>) -> Self {
        Environment {
            values: Vec::new(),
            enclosing: Option::Some(enclosing),
        }
    }

    pub fn define(&mut self, name: &str, value: &LiteralType) {
        if let Some(pos) = self.values.iter().position(|(k, _)| k == name) {
            self.values[pos].1 = value.clone();
        } else {
            self.values.push((name.to_string(), value.clone()));
        }
    }

    pub fn get(&self, token: &Token) -> Result<LiteralType, String> {
        if let Some((_, value)) = self.values.iter().find(|(k, _)| k == &token.lexeme) {
            return Result::Ok(value.clone());
        }

        if let Option::Some(enclosing) = &self.enclosing {
            return enclosing.borrow().get(&token);
        }

        Result::Err(format!("Undefined variable '{}'.", token.lexeme))
    }

    pub fn get_at(&self, name: &str, distance: usize) -> Result<LiteralType, String> {
        if distance == 0 {
            if let Some((_, value)) = self.values.iter().find(|(k, _)| k == name) {
                return Result::Ok(value.clone());
            }
        } else {
            let env = self.ancestor(distance)?;
            if let Some((_, value)) = &env.borrow().values.iter().find(|(k, _)| k == name) {
                return Result::Ok(value.clone());
            }
        }

        return Result::Err(format!("Undefined variable '{}'", name));
    }

    pub fn assign(&mut self, name: &Token, value: &LiteralType) -> Result<(), String> {
        if let Some(pos) = self.values.iter().position(|(k, _)| k == &name.lexeme) {
            self.values[pos].1 = value.clone();
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
            if let Some(pos) = self.values.iter().position(|(k, _)| k == &name.lexeme) {
                self.values[pos].1 = value.clone();
                return Result::Ok(());
            }
        } else {
            let env = self.ancestor(distance)?;
            if let Some(pos) = env
                .borrow()
                .values
                .iter()
                .position(|(k, _)| k == &name.lexeme)
            {
                env.borrow_mut().values[pos].1 = value.clone();
                return Result::Ok(());
            }
        }
        // Technically redundant
        // Resolver pass has already definitively proven that the variable exists at that exact lexical scope
        Result::Err(format!("Undefined variable '{}'.", name.lexeme))
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
