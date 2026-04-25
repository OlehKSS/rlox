use std::cell::RefCell;
use std::rc::Rc;

use crate::environment::Environment;

use super::interpreter::{Interpreter, LoxValue};
use super::parser::Stmt;
use super::scanner::Token;

pub trait LoxCallable {
    fn arity(&self) -> u8;
    fn call(
        &self,
        interpreter: &mut Interpreter,
        arguments: &Vec<LoxValue>,
    ) -> Result<LoxValue, String>;
    fn to_string(&self) -> String;
}

#[derive(Debug, Clone)]
pub enum Callable {
    Native(Rc<NativeFunction>),
    Function(Rc<LoxFunction>),
}

#[derive(Debug, Clone)]
pub struct NativeFunction {
    arity: u8,
    func: fn() -> Result<LoxValue, String>,
}

#[derive(Debug, Clone)]
pub struct LoxFunction {
    name: Token,
    parameters: Vec<Token>,
    body: Vec<Stmt>,
    closure: Rc<RefCell<Environment>>,
}

impl LoxCallable for Callable {
    fn arity(&self) -> u8 {
        match self {
            Callable::Native(f) => f.arity(),
            Callable::Function(f) => f.arity(),
        }
    }

    fn call(
        &self,
        interpreter: &mut Interpreter,
        arguments: &Vec<LoxValue>,
    ) -> Result<LoxValue, String> {
        match self {
            Callable::Native(f) => f.call(interpreter, arguments),
            Callable::Function(f) => f.call(interpreter, arguments),
        }
    }

    fn to_string(&self) -> String {
        match self {
            Callable::Native(f) => f.to_string(),
            Callable::Function(f) => f.to_string(),
        }
    }
}

impl PartialEq for Callable {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl NativeFunction {
    pub fn new(arity: u8, func: fn() -> Result<LoxValue, String>) -> Self {
        NativeFunction { arity, func }
    }
}

impl LoxCallable for NativeFunction {
    fn arity(&self) -> u8 {
        self.arity
    }

    fn call(
        &self,
        _interpreter: &mut Interpreter,
        _arguments: &Vec<LoxValue>,
    ) -> Result<LoxValue, String> {
        (self.func)()
    }

    fn to_string(&self) -> String {
        "<native fn>".to_string()
    }
}

impl LoxFunction {
    pub fn new(
        name: &Token,
        parameters: &Vec<Token>,
        body: &Vec<Stmt>,
        closure: Rc<RefCell<Environment>>,
    ) -> Self {
        // TODO: Can we elimate cloning here?
        LoxFunction {
            name: name.clone(),
            parameters: parameters.clone(),
            body: body.clone(),
            closure: closure,
        }
    }
}

impl LoxCallable for LoxFunction {
    fn arity(&self) -> u8 {
        self.parameters.len() as u8
    }

    fn call(
        &self,
        interpreter: &mut Interpreter,
        arguments: &Vec<LoxValue>,
    ) -> Result<LoxValue, String> {
        let mut env = Environment::new_with_enclosing(self.closure.clone());

        for i in 0..self.parameters.len() {
            env.define(&self.parameters[i].lexeme, &arguments[i]);
        }

        interpreter.execute_block(&self.body, Rc::new(RefCell::new(env)))?;

        if interpreter.return_flag {
            let return_value = interpreter.return_value.clone();
            interpreter.return_flag = false;
            interpreter.return_value = LoxValue::NoneValue;
            Result::Ok(return_value)
        } else {
            Result::Ok(LoxValue::NoneValue)
        }
    }

    fn to_string(&self) -> String {
        format!("<fn {}>", self.name.lexeme)
    }
}
