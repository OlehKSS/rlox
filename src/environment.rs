use ::std::collections::HashMap;

use super::scanner::{LiteralType, Token};

pub struct Environment {
    values: HashMap<String, LiteralType>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            values: HashMap::new(),
        }
    }

    pub fn define(&mut self, name: &Token, value: &LiteralType) {
        self.values.insert(name.lexeme.clone(), value.clone());
    }

    pub fn get(&self, token: &Token) -> Result<LiteralType, String> {
        if let Option::Some(value) = self.values.get(&token.lexeme) {
            return Result::Ok(value.clone());
        }

        Result::Err(format!("Undefined variable '{}'.", token.lexeme))
    }

    pub fn assign(&mut self, name: &Token, value: &LiteralType) -> Result<(), String> {
        if self.values.contains_key(&name.lexeme) {
            self.values.insert(name.lexeme.clone(), value.clone());
            return Result::Ok(());
        }

        Result::Err(format!("Undefined variable '{}'.", name.lexeme))
    }
}
