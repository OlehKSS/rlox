use std::cell::RefCell;
use std::rc::Rc;

use super::environment::Environment;
use super::parser::{Expr, Stmt};
use super::scanner::{LiteralType, Token, TokenType};

type LoxValue = LiteralType;

pub struct Interpreter {
    environment: Rc<RefCell<Environment>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            environment: Rc::new(RefCell::new(Environment::new())),
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

    fn execute_statement(&mut self, statement: &Stmt, repl: bool) -> Result<(), String> {
        match statement {
            Stmt::Block { statements } => self.execute_block(statements),
            Stmt::Expression { expression } => {
                if repl {
                    self.print_statement(expression)?;
                } else {
                    self.evaluate(expression)?;
                }
                Result::Ok(())
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => self.if_statement(condition, then_branch, else_branch.as_deref()),
            Stmt::Print { expression } => self.print_statement(expression),
            Stmt::While { condition, body } => self.while_statement(condition, body),
            Stmt::Var { name, initializer } => {
                let value = match &initializer {
                    Option::Some(expr) => self.evaluate(expr)?,
                    Option::None => LiteralType::NoneValue,
                };
                self.environment.borrow_mut().define(&name, &value);
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

    fn while_statement(&mut self, condition: &Expr, body: &Stmt) -> Result<(), String> {
        let mut cond_value = self.evaluate(condition)?;
        while is_truthy(&cond_value) {
            self.execute_statement(body, false)?;
            cond_value = self.evaluate(condition)?;
        }

        Result::Ok(())
    }

    fn execute_block(&mut self, statements: &Vec<Stmt>) -> Result<(), String> {
        let previous = self.environment.clone();
        self.environment = Rc::new(RefCell::new(Environment::new_with_enclosing(
            previous.clone(),
        )));

        for stmt in statements {
            let res = self.execute_statement(stmt, false);

            if let Result::Err(_) = &res {
                self.environment = previous;
                return res;
            }
        }

        self.environment = previous;
        Result::Ok(())
    }

    fn evaluate(&mut self, expr: &Expr) -> Result<LoxValue, String> {
        match expr {
            Expr::Assign { name, value } => {
                let rvalue = self.evaluate(value)?;
                self.environment.borrow_mut().assign(name, &rvalue)?;
                Result::Ok(rvalue)
            }
            Expr::Binary {
                left,
                right,
                operator,
            } => self.evaluate_binary(left, right, operator),
            Expr::Grouping { expression } => self.evaluate(expression),
            Expr::Literal { value } => Result::Ok(value.clone()),
            Expr::Logical {
                left,
                right,
                operator,
            } => self.evaluate_logical(left, right, operator),
            Expr::Unary { right, operator } => self.evaluate_unary(right, operator),
            Expr::Variable { name } => self.environment.borrow().get(&name),
        }
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
            .define(&var_a, &LiteralType::NumberValue(1.0));

        let var_a_expr = Expr::Variable {
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

        let mut intp = Interpreter::new();
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
        let mut intp = Interpreter::new();
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
        let mut intp = Interpreter::new();
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
        let mut intp = Interpreter::new();
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
        let mut intp = Interpreter::new();
        intp.interpret(&statements, false);
        assert_eq!(
            intp.environment.borrow_mut().get(&tokens[1]).unwrap(),
            LiteralType::NumberValue(3.0)
        );
    }
}
