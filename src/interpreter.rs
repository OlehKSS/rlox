use super::environment::Environment;
use super::parser::{Expr, Stmt};
use super::scanner::{LiteralType, Token, TokenType};

type LoxValue = LiteralType;

pub struct Interpreter {
    environment: Environment,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            environment: Environment::new(),
        }
    }

    pub fn interpret(&mut self, statements: &Vec<Stmt>) {
        for stmt in statements {
            let mut runtime_error = String::new();

            match &stmt {
                Stmt::Expression { expression } => {
                    let res = self.evaluate(expression);

                    if let Result::Err(emsg) = res {
                        runtime_error = emsg;
                    }
                }
                Stmt::Print { expression } => {
                    let res = self.print_statement(expression);

                    if let Result::Err(emsg) = res {
                        runtime_error = emsg;
                    }
                }
                Stmt::Var { name, initializer } => {
                    let mut value = LiteralType::NoneValue;
                    if let Option::Some(expr) = initializer {
                        let res = self.evaluate(expr);

                        match res {
                            Result::Ok(expr_value) => {
                                value = expr_value;
                            }
                            Result::Err(emsg) => {
                                std::eprintln!("Runtime error: {}", emsg);
                                continue;
                            }
                        };
                    }

                    self.environment.define(&name, &value);
                }
            };

            if !runtime_error.is_empty() {
                std::eprintln!("Runtime error: {}", runtime_error);
            }
        }
    }

    fn print_statement(&mut self, expr: &Expr) -> Result<(), String> {
        let value = self.evaluate(expr)?;
        std::println!("{}", stringify(&value));
        Result::Ok(())
    }

    fn evaluate(&mut self, expr: &Expr) -> Result<LoxValue, String> {
        match expr {
            Expr::Assign { name, value } => {
                let rvalue = self.evaluate(value)?;
                self.environment.assign(name, &rvalue)?;
                Result::Ok(rvalue)
            }
            Expr::Binary {
                left,
                right,
                operator,
            } => self.evaluate_binary(left, right, operator),
            Expr::Grouping { expression } => self.evaluate(expression),
            Expr::Literal { value } => Result::Ok(value.clone()),
            Expr::Unary { right, operator } => self.evaluate_unary(right, operator),
            Expr::Variable { name } => self.environment.get(&name),
        }
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
    fn test_global_variables() {
        let mut intp = Interpreter::new();
        let var_a = Token {
            ttype: TokenType::Var,
            lexeme: "a".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };

        intp.environment
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
}
