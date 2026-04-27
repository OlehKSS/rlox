use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::callable::{Callable, LoxCallable, LoxFunction, NativeFunction};
use super::environment::Environment;
use super::parser::{Expr, Stmt};
use super::scanner::{LiteralType, Token, TokenType};
use super::utility::error;

use std::time::{SystemTime, UNIX_EPOCH};

pub type LoxValue = LiteralType;

pub struct Interpreter {
    pub return_flag: bool,
    pub return_value: LiteralType,
    pub globals: Rc<RefCell<Environment>>,
    environment: Rc<RefCell<Environment>>,
    break_flag: bool,
    continue_flag: bool,
    locals: HashMap<usize, usize>, // Map Expr.id to its depth
}

impl Interpreter {
    pub fn new() -> Self {
        let mut globals = Environment::new();
        let clock_func = Callable::Native(Rc::new(NativeFunction::new(0, || {
            let start = SystemTime::now();
            let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("System failure");
            let time_ms = since_the_epoch.as_millis() as f64;

            Ok(LiteralType::NumberValue(time_ms))
        })));
        globals.define("clock", &LiteralType::Callable(clock_func));

        let globals = Rc::new(RefCell::new(globals));

        Interpreter {
            globals: globals.clone(),
            environment: globals.clone(),
            break_flag: false,
            continue_flag: false,
            return_flag: false,
            return_value: LiteralType::NoneValue,
            locals: HashMap::new(),
        }
    }

    pub fn interpret(&mut self, statements: &Vec<Stmt>, repl: bool) {
        for stmt in statements {
            let res = self.execute_statement(stmt, repl);

            if let Result::Err(runtime_error) = &res {
                std::eprintln!("Runtime error: {}", runtime_error);
            }
        }
    }

    pub fn resolve(&mut self, locals: HashMap<usize, usize>) {
        self.locals = locals;
    }

    fn execute_statement(&mut self, statement: &Stmt, repl: bool) -> Result<(), String> {
        match statement {
            Stmt::Block { statements } => {
                let block_env = Rc::new(RefCell::new(Environment::new_with_enclosing(
                    self.environment.clone(),
                )));
                self.execute_block(statements, block_env)
            }
            Stmt::Break => {
                self.break_flag = true;
                Result::Ok(())
            }
            Stmt::Continue => {
                self.continue_flag = true;
                Result::Ok(())
            }
            Stmt::Expression { expression } => {
                if repl {
                    self.print_statement(expression)?;
                } else {
                    self.evaluate(expression)?;
                }
                Result::Ok(())
            }
            Stmt::Function {
                name,
                parameters,
                body,
            } => {
                let function = Callable::Function(Rc::new(LoxFunction::new(
                    name,
                    parameters,
                    body,
                    self.environment.clone(),
                )));
                self.environment
                    .borrow_mut()
                    .define(&name.lexeme, &LoxValue::Callable(function));
                Result::Ok(())
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => self.if_statement(condition, then_branch, else_branch.as_deref()),
            Stmt::Print { expression } => self.print_statement(expression),
            Stmt::Return { keyword, value } => self.return_statement(keyword, value),
            Stmt::While {
                condition,
                body,
                increment,
            } => self.while_statement(condition, body, increment),
            Stmt::Var { name, initializer } => {
                let value = match &initializer {
                    Option::Some(expr) => self.evaluate(expr)?,
                    Option::None => LiteralType::NoneValue,
                };
                self.environment.borrow_mut().define(&name.lexeme, &value);
                Result::Ok(())
            }
        }
    }

    fn if_statement(
        &mut self,
        condition: &Expr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
    ) -> Result<(), String> {
        let cond_value = self.evaluate(condition)?;
        if is_truthy(&cond_value) {
            self.execute_statement(then_branch, false)?;
        } else if let Some(else_stmt) = else_branch {
            self.execute_statement(else_stmt, false)?;
        }

        Result::Ok(())
    }

    fn print_statement(&mut self, expr: &Expr) -> Result<(), String> {
        let value = self.evaluate(expr)?;
        std::println!("{}", stringify(&value));
        Result::Ok(())
    }

    fn return_statement(&mut self, _keyword: &Token, value: &Expr) -> Result<(), String> {
        self.return_value = self.evaluate(value)?;
        self.return_flag = true;
        Result::Ok(())
    }

    fn while_statement(
        &mut self,
        condition: &Expr,
        body: &Stmt,
        increment: &Option<Expr>,
    ) -> Result<(), String> {
        let mut cond_value = self.evaluate(condition)?;
        while is_truthy(&cond_value) {
            self.execute_statement(body, false)?;
            if self.break_flag {
                self.break_flag = false;
                return Result::Ok(());
            }
            if self.return_flag {
                return Result::Ok(());
            }
            // Shim allowing for-loop support
            if let Option::Some(expr) = increment {
                self.evaluate(expr)?;
            }
            self.continue_flag = false;
            cond_value = self.evaluate(condition)?;
        }

        Result::Ok(())
    }

    pub fn execute_block(
        &mut self,
        statements: &Vec<Stmt>,
        environment: Rc<RefCell<Environment>>,
    ) -> Result<(), String> {
        let previous = self.environment.clone();
        self.environment = environment;

        for stmt in statements {
            let res = self.execute_statement(stmt, false);

            if let Result::Err(_) = &res {
                self.environment = previous;
                return res;
            }
            if self.break_flag || self.continue_flag || self.return_flag {
                break;
            }
        }

        self.environment = previous;
        Result::Ok(())
    }

    fn evaluate(&mut self, expr: &Expr) -> Result<LoxValue, String> {
        match expr {
            Expr::Assign { id, name, value } => self.evaluate_assign(*id, name, value),
            Expr::Binary {
                left,
                right,
                operator,
            } => self.evaluate_binary(left, right, operator),
            Expr::Call {
                callee,
                right_parenthesis,
                arguments,
            } => self.evaluate_call(callee, right_parenthesis, arguments),
            Expr::Grouping { expression } => self.evaluate(expression),
            Expr::Literal { value } => Result::Ok(value.clone()),
            Expr::Logical {
                left,
                right,
                operator,
            } => self.evaluate_logical(left, right, operator),
            Expr::Unary { right, operator } => self.evaluate_unary(right, operator),
            Expr::Variable { id, name } => self.look_up_variable(name, *id),
        }
    }

    fn evaluate_assign(
        &mut self,
        id: usize,
        name: &Token,
        value: &Expr,
    ) -> Result<LoxValue, String> {
        let rvalue = self.evaluate(value)?;
        let distance = self.locals.get(&id);

        if let Option::Some(d) = distance {
            self.environment.borrow_mut().assign_at(name, &rvalue, *d)?;
        } else {
            self.globals.borrow_mut().assign(name, &rvalue)?;
        }

        Result::Ok(rvalue)
    }

    fn evaluate_logical(
        &mut self,
        left: &Expr,
        right: &Expr,
        operator: &Token,
    ) -> Result<LoxValue, String> {
        let left = self.evaluate(left)?;

        // Short-circuiting of logical operators
        if operator.ttype == TokenType::Or {
            if is_truthy(&left) {
                return Result::Ok(left);
            }
        } else {
            if !is_truthy(&left) {
                return Result::Ok(left);
            }
        }

        let right_value = self.evaluate(right)?;
        Result::Ok(right_value)
    }

    fn evaluate_unary(&mut self, right: &Expr, operator: &Token) -> Result<LoxValue, String> {
        let right_value = self.evaluate(right)?;

        match operator.ttype {
            TokenType::Minus => {
                if let LoxValue::NumberValue(value) = right_value {
                    return Result::Ok(LoxValue::NumberValue(-1.0 * value));
                } else {
                    return Result::Err(format!(
                        "evaluate_unary expected numeric value, got {:?}.\n[line {}]",
                        right_value, operator.line
                    ));
                }
            }
            TokenType::Bang => {
                return Result::Ok(LoxValue::BoolValue(!is_truthy(&right_value)));
            }
            _ => {
                return Result::Err(format!(
                    "Unsupported unary operator {:?}.\n[line {}]",
                    operator.ttype, operator.line
                ));
            }
        }
    }

    fn evaluate_binary(
        &mut self,
        left: &Expr,
        right: &Expr,
        operator: &Token,
    ) -> Result<LoxValue, String> {
        let left_value = self.evaluate(left)?;
        let right_value = self.evaluate(right)?;

        match operator.ttype {
            TokenType::BangEqual => {
                return Result::Ok(LoxValue::BoolValue(!is_equal(&left_value, &right_value)));
            }
            TokenType::EqualEqual => {
                return Result::Ok(LoxValue::BoolValue(is_equal(&left_value, &right_value)));
            }
            _ => (),
        }

        if let LoxValue::StringValue(left_string) = &left_value {
            if let LoxValue::StringValue(right_string) = &right_value {
                if operator.ttype == TokenType::Plus {
                    return Result::Ok(LoxValue::StringValue(
                        left_string.to_owned() + right_string,
                    ));
                }
            }
        }

        if let LoxValue::NumberValue(left_number) = left_value {
            if let LoxValue::NumberValue(right_number) = right_value {
                match operator.ttype {
                    // Arithmetic operators
                    TokenType::Minus => {
                        return Result::Ok(LoxValue::NumberValue(left_number - right_number));
                    }
                    TokenType::Plus => {
                        return Result::Ok(LoxValue::NumberValue(left_number + right_number));
                    }
                    TokenType::Slash => {
                        return Result::Ok(LoxValue::NumberValue(left_number / right_number));
                    }
                    TokenType::Star => {
                        return Result::Ok(LoxValue::NumberValue(left_number * right_number));
                    }
                    // Comparison operators
                    TokenType::Greater => {
                        return Result::Ok(LoxValue::BoolValue(left_number > right_number));
                    }
                    TokenType::GreaterEqual => {
                        return Result::Ok(LoxValue::BoolValue(left_number >= right_number));
                    }
                    TokenType::Less => {
                        return Result::Ok(LoxValue::BoolValue(left_number < right_number));
                    }
                    TokenType::LessEqual => {
                        return Result::Ok(LoxValue::BoolValue(left_number <= right_number));
                    }
                    _ => (),
                }
            }
        }

        return Result::Err(format!(
            "Unsupported operands {:?}, {:?}, {:?}.\n[line {}]",
            operator, left_value, right_value, operator.line
        ));
    }

    fn evaluate_call(
        &mut self,
        callee: &Expr,
        right_parenthesis: &Token,
        arguments: &Vec<Expr>,
    ) -> Result<LoxValue, String> {
        let callee_value = self.evaluate(callee)?;
        let mut args: Vec<LoxValue> = vec![];
        for expr in arguments {
            let arg_value = self.evaluate(expr)?;
            args.push(arg_value);
        }

        if let LoxValue::Callable(function) = callee_value {
            if arguments.len() != (function.arity() as usize) {
                return Result::Err(error(
                    &format!(
                        "Expected {} arguments but got {}",
                        function.arity(),
                        arguments.len()
                    ),
                    right_parenthesis,
                ));
            }

            return function.call(self, &args);
        }

        Result::Err(error(
            "Can only call functions and classes.",
            right_parenthesis,
        ))
    }

    fn look_up_variable(&self, name: &Token, id: usize) -> Result<LiteralType, String> {
        if let Option::Some(distance) = self.locals.get(&id) {
            self.environment.borrow().get_at(name, *distance)
        } else {
            self.globals.borrow().get(name)
        }
    }
}

/// false and nil are falsey, and everything else is truthy
fn is_truthy(literal: &LiteralType) -> bool {
    match literal {
        LiteralType::NoneValue => false,
        LiteralType::BoolValue(value) => value.clone(),
        _ => true,
    }
}

fn is_equal(a: &LiteralType, b: &LiteralType) -> bool {
    if *a == LiteralType::NoneValue {
        return *b == LiteralType::NoneValue;
    }

    a == b
}

fn stringify(lox_value: &LoxValue) -> String {
    match lox_value {
        LoxValue::NoneValue => "nil".to_string(),
        LoxValue::NumberValue(value) => {
            let mut text = value.to_string();
            if text.ends_with(".0") {
                text = text[0..text.len() - 2].to_string()
            }
            return text;
        }
        LoxValue::BoolValue(value) => value.to_string(),
        LoxValue::StringValue(value) => value.clone(),
        LoxValue::Callable(callable) => callable.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::parser::Parser;
    use super::super::scanner::Scanner;
    use super::*;
    #[test]
    fn test_evaluate_unary_bool() {
        let right_bool = Expr::Literal {
            value: LiteralType::BoolValue(true),
        };
        let operator_bang = Token {
            ttype: TokenType::Bang,
            lexeme: "!".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };

        let mut intp = Interpreter::new();
        let result_bool = intp.evaluate_unary(&right_bool, &operator_bang);
        assert_eq!(result_bool.unwrap(), LoxValue::BoolValue(false));
    }

    #[test]
    fn test_evaluate_unary_num() {
        let operator_minus = Token {
            ttype: TokenType::Minus,
            lexeme: "-".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };
        let right_number = Expr::Literal {
            value: LiteralType::NumberValue(42.0),
        };

        let mut intp = Interpreter::new();
        let result_number = intp.evaluate_unary(&right_number, &operator_minus);
        assert_eq!(result_number.unwrap(), LoxValue::NumberValue(-42.0));
    }

    #[test]
    fn test_evaluate_binary_str() {
        let operator_plus = Token {
            ttype: TokenType::Plus,
            lexeme: "+".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };
        let left_str = Expr::Literal {
            value: LiteralType::StringValue("ab".to_string()),
        };
        let right_str = Expr::Literal {
            value: LiteralType::StringValue("c".to_string()),
        };

        let mut intp = Interpreter::new();
        let result_str = intp.evaluate_binary(&left_str, &right_str, &operator_plus);
        assert_eq!(
            result_str.unwrap(),
            LoxValue::StringValue("abc".to_string())
        );
    }

    fn create_num_expr(val: f64) -> Expr {
        Expr::Literal {
            value: LiteralType::NumberValue(val),
        }
    }

    fn create_op(ttype: TokenType, lexeme: &str) -> Token {
        Token {
            ttype,
            lexeme: lexeme.to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        }
    }

    #[test]
    fn test_evaluate_binary_plus() {
        let op = create_op(TokenType::Plus, "+");
        let mut intp = Interpreter::new();
        let result = intp.evaluate_binary(&create_num_expr(2.0), &create_num_expr(3.0), &op);
        assert_eq!(result.unwrap(), LoxValue::NumberValue(5.0));
    }

    #[test]
    fn test_evaluate_binary_minus() {
        let op = create_op(TokenType::Minus, "-");
        let mut intp = Interpreter::new();
        let result = intp.evaluate_binary(&create_num_expr(5.0), &create_num_expr(3.0), &op);
        assert_eq!(result.unwrap(), LoxValue::NumberValue(2.0));
    }

    #[test]
    fn test_evaluate_binary_star() {
        let op = create_op(TokenType::Star, "*");
        let mut intp = Interpreter::new();
        let result = intp.evaluate_binary(&create_num_expr(5.0), &create_num_expr(3.0), &op);
        assert_eq!(result.unwrap(), LoxValue::NumberValue(15.0));
    }

    #[test]
    fn test_evaluate_binary_slash() {
        let op = create_op(TokenType::Slash, "/");
        let mut intp = Interpreter::new();
        let result = intp.evaluate_binary(&create_num_expr(6.0), &create_num_expr(3.0), &op);
        assert_eq!(result.unwrap(), LoxValue::NumberValue(2.0));
    }

    #[test]
    fn test_evaluate_binary_greater() {
        let op = create_op(TokenType::Greater, ">");
        let mut intp = Interpreter::new();
        let result1 = intp.evaluate_binary(&create_num_expr(5.0), &create_num_expr(3.0), &op);
        assert_eq!(result1.unwrap(), LoxValue::BoolValue(true));

        let result2 = intp.evaluate_binary(&create_num_expr(3.0), &create_num_expr(5.0), &op);
        assert_eq!(result2.unwrap(), LoxValue::BoolValue(false));
    }

    #[test]
    fn test_evaluate_binary_greater_equal() {
        let op = create_op(TokenType::GreaterEqual, ">=");
        let mut intp = Interpreter::new();
        let result1 = intp.evaluate_binary(&create_num_expr(5.0), &create_num_expr(5.0), &op);
        assert_eq!(result1.unwrap(), LoxValue::BoolValue(true));

        let result2 = intp.evaluate_binary(&create_num_expr(4.0), &create_num_expr(5.0), &op);
        assert_eq!(result2.unwrap(), LoxValue::BoolValue(false));
    }

    #[test]
    fn test_evaluate_binary_less() {
        let op = create_op(TokenType::Less, "<");
        let mut intp = Interpreter::new();
        let result1 = intp.evaluate_binary(&create_num_expr(3.0), &create_num_expr(5.0), &op);
        assert_eq!(result1.unwrap(), LoxValue::BoolValue(true));

        let result2 = intp.evaluate_binary(&create_num_expr(5.0), &create_num_expr(3.0), &op);
        assert_eq!(result2.unwrap(), LoxValue::BoolValue(false));
    }

    #[test]
    fn test_evaluate_binary_less_equal() {
        let op = create_op(TokenType::LessEqual, "<=");
        let mut intp = Interpreter::new();
        let result1 = intp.evaluate_binary(&create_num_expr(5.0), &create_num_expr(5.0), &op);
        assert_eq!(result1.unwrap(), LoxValue::BoolValue(true));

        let result2 = intp.evaluate_binary(&create_num_expr(6.0), &create_num_expr(5.0), &op);
        assert_eq!(result2.unwrap(), LoxValue::BoolValue(false));
    }

    #[test]
    fn test_evaluate_logical_or() {
        let op = create_op(TokenType::Or, "or");
        let mut intp = Interpreter::new();
        let false_expr = Expr::Literal {
            value: LoxValue::BoolValue(false),
        };
        let result1 = intp.evaluate_logical(&create_num_expr(5.0), &false_expr, &op);
        assert_eq!(result1.unwrap(), LoxValue::NumberValue(5.0));
        let result2 = intp.evaluate_logical(&false_expr, &false_expr, &op);
        assert_eq!(result2.unwrap(), LoxValue::BoolValue(false));
    }

    #[test]
    fn test_evaluate_logical_and() {
        let op = create_op(TokenType::And, "and");
        let mut intp = Interpreter::new();
        let nil_expr = Expr::Literal {
            value: LoxValue::NoneValue,
        };
        let true_expr = Expr::Literal {
            value: LoxValue::StringValue(String::new()),
        };
        let result1 = intp.evaluate_logical(&true_expr, &true_expr, &op);
        assert_eq!(result1.unwrap(), LoxValue::StringValue(String::new()));
        let result2 = intp.evaluate_logical(&nil_expr, &true_expr, &op);
        assert_eq!(result2.unwrap(), LoxValue::NoneValue);
    }

    #[test]
    fn test_global_variables() {
        let mut intp = Interpreter::new();
        let var_a = Token {
            ttype: TokenType::Var,
            lexeme: "a".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };

        intp.environment
            .borrow_mut()
            .define(&var_a.lexeme, &LiteralType::NumberValue(1.0));

        let var_a_expr = Expr::Variable {
            id: 0,
            name: var_a.clone(),
        };

        assert_eq!(
            intp.evaluate(&var_a_expr).unwrap(),
            LiteralType::NumberValue(1.0)
        );

        let literal = Box::new(Expr::Literal {
            value: LiteralType::NumberValue(42.0),
        });
        let assign_expr = Expr::Assign {
            id: 1,
            name: var_a,
            value: literal,
        };

        intp.evaluate(&assign_expr).unwrap();

        assert_eq!(
            intp.evaluate(&var_a_expr).unwrap(),
            LiteralType::NumberValue(42.0)
        );
    }

    #[test]
    fn test_variable_scope() {
        let source = "var a = 42; { var b = -1;  a = b; }";
        let mut scanner = Scanner::new(source);
        let (tokens, lex_errors) = scanner.scan_tokens();

        assert!(lex_errors.is_empty());
        assert_eq!(tokens[0].ttype, TokenType::Var);
        assert_eq!(tokens[1].ttype, TokenType::Identifier);

        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();

        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();

        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);

        assert_eq!(
            intp.environment.borrow_mut().get(&tokens[1]).unwrap(),
            LiteralType::NumberValue(-1.0)
        );
    }

    #[test]
    fn test_if_statement() {
        let source = "var a = 1; if (true) { a = 2; } else { a = 3; }";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);
        assert_eq!(
            intp.environment.borrow_mut().get(&tokens[1]).unwrap(),
            LiteralType::NumberValue(2.0)
        );
    }

    #[test]
    fn test_if_else_statement() {
        let source = "var a = 1; if (false) { a = 2; } else { a = 3; }";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);
        assert_eq!(
            intp.environment.borrow_mut().get(&tokens[1]).unwrap(),
            LiteralType::NumberValue(3.0)
        );
    }

    #[test]
    fn test_while_loop() {
        let source = "var a = 0; while (a < 3) { a = a + 1; }";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);
        assert_eq!(
            intp.environment.borrow_mut().get(&tokens[1]).unwrap(),
            LiteralType::NumberValue(3.0)
        );
    }

    #[test]
    fn test_for_loop() {
        let source = "var a = 0; for (var i = 0; i < 3; i = i + 1) { a = a + i; }";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);
        assert_eq!(
            intp.environment.borrow_mut().get(&tokens[1]).unwrap(),
            LiteralType::NumberValue(3.0)
        );
    }

    #[test]
    fn test_while_loop_break() {
        let source = "var a = 0; while (a < 3) { a = a + 1; if (a == 2) break; }";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);
        assert_eq!(
            intp.environment.borrow_mut().get(&tokens[1]).unwrap(),
            LiteralType::NumberValue(2.0)
        );
    }

    #[test]
    fn test_while_loop_continue() {
        let source =
            "var a = 0; var b = 0; while (a < 3) { a = a + 1; if (a == 2) continue; b = b + 1; }";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);
        assert_eq!(
            intp.environment.borrow_mut().get(&tokens[6]).unwrap(),
            LiteralType::NumberValue(2.0)
        );
    }

    #[test]
    fn test_for_loop_break() {
        let source =
            "var a = 0; for (var i = 0; i < 5; i = i + 1) { if (i == 3) break; a = a + 1; }";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);
        assert_eq!(
            intp.environment.borrow_mut().get(&tokens[1]).unwrap(),
            LiteralType::NumberValue(3.0)
        );
    }

    #[test]
    fn test_for_loop_continue() {
        let source =
            "var a = 0; for (var i = 0; i < 5; i = i + 1) { if (i == 3) continue; a = a + 1; }";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);
        assert_eq!(
            intp.environment.borrow_mut().get(&tokens[1]).unwrap(),
            LiteralType::NumberValue(4.0)
        );
    }

    #[test]
    fn test_function_call_and_return() {
        let source = "fun add(a, b) { return a + b; } var result = add(3, 4);";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);

        let token_result = Token {
            ttype: TokenType::Identifier,
            lexeme: "result".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };
        assert_eq!(
            intp.environment.borrow().get(&token_result).unwrap(),
            LiteralType::NumberValue(7.0)
        );
    }

    #[test]
    fn test_function_closure() {
        let source = "var a = 1; fun makeAdder() { var b = 2; fun add(c) { return a + b + c; } return add; } var adder = makeAdder(); var result = adder(3);";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);

        let token_result = Token {
            ttype: TokenType::Identifier,
            lexeme: "result".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };
        assert_eq!(
            intp.environment.borrow().get(&token_result).unwrap(),
            LiteralType::NumberValue(6.0)
        );
    }

    #[test]
    fn test_function_return_inside_loop() {
        let source = "fun findTarget(target) { var i = 0; while (i < 10) { if (i == target) { return i; } i = i + 1; } return -1; } var result = findTarget(5);";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);

        let token_result = Token {
            ttype: TokenType::Identifier,
            lexeme: "result".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };
        assert_eq!(
            intp.environment.borrow().get(&token_result).unwrap(),
            LiteralType::NumberValue(5.0)
        );
    }

    #[test]
    fn test_function_recursive() {
        let source = "fun fib(n) { if (n <= 1) return n; return fib(n - 2) + fib(n - 1); } var result = fib(5);";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);

        let token_result = Token {
            ttype: TokenType::Identifier,
            lexeme: "result".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };
        assert_eq!(
            intp.environment.borrow().get(&token_result).unwrap(),
            LiteralType::NumberValue(5.0)
        );
    }

    #[test]
    fn test_closure_lexical_scoping() {
        let source = r#"
            var a = "global";
            var result1;
            var result2;
            {
                fun showA() {
                    return a;
                }
                result1 = showA();
                var a = "block";
                print a;
                result2 = showA();
            }
        "#;
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);

        let token_result1 = Token {
            ttype: TokenType::Identifier,
            lexeme: "result1".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };
        let token_result2 = Token {
            ttype: TokenType::Identifier,
            lexeme: "result2".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };
        assert_eq!(
            intp.environment.borrow().get(&token_result1).unwrap(),
            LiteralType::StringValue("global".to_string())
        );
        assert_eq!(
            intp.environment.borrow().get(&token_result2).unwrap(),
            LiteralType::StringValue("global".to_string())
        );
    }

    #[test]
    fn test_closure_variable_assignment() {
        let source = r#"
            var globalSet;
            var globalGet;
            fun main() {
                var a = "initial";
                fun set() { a = "updated"; }
                fun get() { return a; }
                globalSet = set;
                globalGet = get;
            }
            main();
            globalSet();
            var result = globalGet();
        "#;
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        let resolver = crate::resolver::Resolver::new();
        let locals = resolver.resolve(&statements).unwrap();
        let mut intp = Interpreter::new();
        intp.resolve(locals);
        intp.interpret(&statements, false);

        let token_result = Token {
            ttype: TokenType::Identifier,
            lexeme: "result".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };
        assert_eq!(
            intp.environment.borrow().get(&token_result).unwrap(),
            LiteralType::StringValue("updated".to_string())
        );
    }
}
