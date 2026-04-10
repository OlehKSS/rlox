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

    pub fn define(&mut self, name: &str, value: &LiteralType) {
        self.values.insert(name.to_string(), value.clone());
    }

    pub fn get(&self, token: &Token) -> Result<LiteralType, String> {
        if let Option::Some(value) = self.values.get(&token.lexeme) {
            return Result::Ok(value.clone());
        }

        Result::Err(format!("Undefined variable '{}'.", token.lexeme))
    }
}
