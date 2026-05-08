use core::fmt;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::environment::Environment;
use crate::scanner::LiteralType;
use crate::utility::error;

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
}

#[derive(Debug, Clone)]
pub enum Callable {
    Native(Rc<NativeFunction>),
    Function(Rc<LoxFunction>),
    Class(Rc<LoxClass>),
}

#[derive(Debug, Clone)]
pub struct NativeFunction {
    arity: u8,
    func: fn() -> Result<LoxValue, String>,
}

#[derive(Debug, Clone)]
pub struct LoxFunction {
    name: Token,
    parameters: Rc<Vec<Token>>,
    body: Rc<Vec<Stmt>>,
    closure: Rc<RefCell<Environment>>,
    is_initializer: bool,
}

#[derive(Debug, Clone)]
pub struct LoxClass {
    name: Token,
    superclass: Option<Rc<LoxClass>>,
    methods: HashMap<String, Rc<LoxFunction>>,
}

#[derive(Debug, Clone)]
pub struct LoxInstance {
    class: Rc<LoxClass>,
    fields: HashMap<String, LoxValue>,
}

impl LoxCallable for Callable {
    fn arity(&self) -> u8 {
        match self {
            Callable::Native(f) => f.arity(),
            Callable::Function(f) => f.arity(),
            Callable::Class(c) => c.arity(),
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
            Callable::Class(c) => {
                let instance = LoxInstance::new(c);
                let instance = Rc::new(RefCell::new(instance));
                if let Option::Some(initialzer) = c.find_method("init") {
                    initialzer
                        .bind(instance.clone())
                        .call(interpreter, arguments)?;
                }

                Result::Ok(LoxValue::Instance(instance))
            }
        }
    }
}

impl fmt::Display for Callable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Callable::Native(fun) => write!(f, "{}", fun),
            Callable::Function(fun) => write!(f, "{}", fun),
            Callable::Class(c) => write!(f, "{}", c),
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
}

impl fmt::Display for NativeFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<native fn>")
    }
}

impl LoxFunction {
    pub fn new(
        name: &Token,
        parameters: &Vec<Token>,
        body: &Vec<Stmt>,
        closure: Rc<RefCell<Environment>>,
        is_initializer: bool,
    ) -> Self {
        LoxFunction {
            name: name.clone(),
            parameters: Rc::new(parameters.clone()),
            body: Rc::new(body.clone()),
            closure: closure,
            is_initializer: is_initializer,
        }
    }

    pub fn bind(&self, instance: Rc<RefCell<LoxInstance>>) -> Rc<LoxFunction> {
        let env = Rc::new(RefCell::new(Environment::new_with_enclosing(
            self.closure.clone(),
        )));
        env.borrow_mut()
            .define("this", &LiteralType::Instance(instance));
        Rc::new(LoxFunction {
            name: self.name.clone(),
            parameters: self.parameters.clone(),
            body: self.body.clone(),
            closure: env,
            is_initializer: self.is_initializer,
        })
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

            if self.is_initializer {
                return self.closure.borrow_mut().get_at("this", 0);
            }

            return Result::Ok(return_value);
        }

        // init() methods always return this, even when directly called
        if self.is_initializer {
            return self.closure.borrow_mut().get_at("this", 0);
        }

        Result::Ok(LoxValue::NoneValue)
    }
}

impl fmt::Display for LoxFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<fn {}>", self.name.lexeme)
    }
}

impl LoxClass {
    pub fn new(
        name: &Token,
        superclass: Option<Rc<LoxClass>>,
        methods: &HashMap<String, LoxFunction>,
    ) -> Self {
        let methods = methods
            .iter()
            .map(|(method_name, method)| (method_name.clone(), Rc::new(method.clone())))
            .collect::<HashMap<String, Rc<LoxFunction>>>();
        LoxClass {
            name: name.clone(),
            superclass,
            methods: methods.clone(),
        }
    }

    pub fn find_method(&self, name: &str) -> Option<Rc<LoxFunction>> {
        if self.methods.contains_key(name) {
            return self.methods.get(name).cloned();
        }

        if let Option::Some(superclass) = &self.superclass {
            return superclass.find_method(name);
        }

        Option::None
    }
}

impl LoxCallable for LoxClass {
    fn arity(&self) -> u8 {
        if let Option::Some(initializer) = self.find_method("init") {
            initializer.arity()
        } else {
            0
        }
    }

    fn call(
        &self,
        _interpreter: &mut Interpreter,
        _arguments: &Vec<LoxValue>,
    ) -> Result<LoxValue, String> {
        // Instantiation is handled directly in Callable::call where we have the Rc<LoxClass>
        unreachable!("LoxClass::call should not be called directly")
    }
}

impl fmt::Display for LoxClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<class {}>", self.name.lexeme)
    }
}

impl LoxInstance {
    pub fn new(class: &Rc<LoxClass>) -> Self {
        LoxInstance {
            class: class.clone(),
            fields: HashMap::new(),
        }
    }

    pub fn get(instance: &Rc<RefCell<LoxInstance>>, name: &Token) -> Result<LoxValue, String> {
        let inst_ref = instance.borrow();

        if let Option::Some(value) = inst_ref.fields.get(&name.lexeme) {
            return Result::Ok(value.clone());
        }

        if let Option::Some(method) = inst_ref.class.find_method(&name.lexeme) {
            return Result::Ok(LoxValue::Callable(Callable::Function(
                method.bind(instance.clone()),
            )));
        }

        Result::Err(error(
            &format!("Undefined property '{}'.", name.lexeme),
            name,
        ))
    }

    pub fn set(&mut self, name: &Token, value: &LoxValue) {
        self.fields.insert(name.lexeme.clone(), value.clone());
    }
}

impl fmt::Display for LoxInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<instance {}>", self.class)
    }
}

impl PartialEq for LoxInstance {
    fn eq(&self, other: &Self) -> bool {
        // Compare by reference (pointer equality) since instances are mutable
        std::ptr::eq(self, other)
    }
}
