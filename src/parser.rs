use core::fmt;

use super::scanner::{LiteralType, Token, TokenType};

#[derive(Debug, Clone)]
pub enum Stmt {
    Expression {
        expression: Box<Expr>,
    },
    Print {
        expression: Box<Expr>,
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Binary {
        left: Box<Expr>,
        right: Box<Expr>,
        operator: Token,
    },
    Grouping {
        expression: Box<Expr>,
    },
    Literal {
        value: LiteralType,
    },
    Unary {
        right: Box<Expr>,
        operator: Token,
    },
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Expr::Binary {
                left,
                right,
                operator,
            } => {
                let subexpr = parenthesize(&operator.lexeme, &[&left, &right]);
                write!(f, "{}", subexpr)
            }
            Expr::Grouping { expression } => {
                let subexpr = parenthesize("group", &[&expression]);
                write!(f, "{}", subexpr)
            }
            Expr::Literal { value } => {
                if *value == LiteralType::NoneValue {
                    write!(f, "nil")
                } else {
                    write!(f, "{}", value)
                }
            }
            Expr::Unary { right, operator } => {
                let subexpr = parenthesize(&operator.lexeme, &[&right]);
                write!(f, "{}", subexpr)
            }
        }
    }
}

fn parenthesize(name: &str, exprs: &[&Expr]) -> String {
    let mut out = String::new();

    out.push('(');
    out.push_str(name);

    for expr in exprs {
        out.push(' ');
        out.push_str(&expr.to_string());
    }

    out.push(')');

    out
}

// program → statement* EOF ;
// statement → exprStmt | printStmt ;
// exprStmt → expression ";" ;
// printStmt → "print" expression ";" ;
// expression → equality ;
// equality   → comparison ( ( "!=" | "==" ) comparison )* ;
// comparison → term ( ( ">" | ">=" | "<" | "<=" ) term )* ;
// term       → factor ( ( "-" | "+" ) factor )* ;
// factor     → unary ( ( "/" | "*" ) unary )* ;
// unary      → ( "!" | "-" ) unary | primary ;
// primary    → NUMBER | STRING | "true" | "false" | "nil" | "(" expression ")" ;
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens: tokens,
            current: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>, Vec<String>> {
        let mut statements: Vec<Stmt> = Vec::new();
        let mut error_messages: Vec<String> = Vec::new();

        while !self.is_at_end() {
            match &self.statement() {
                Result::Ok(stmt) => statements.push(stmt.clone()),
                Result::Err(emsg) => error_messages.push(emsg.clone()),
            }
        }

        if error_messages.is_empty() {
            Result::Ok(statements)
        } else {
            Result::Err(error_messages)
        }
    }

    fn statement(&mut self) -> Result<Stmt, String> {
        if self.match_token_type(&[TokenType::Print]) {
            return self.print_statement();
        }

        self.expression_statment()
    }

    fn print_statement(&mut self) -> Result<Stmt, String> {
        let expr=  self.expression()?;
        self.consume(TokenType::Semicolon, "Expected ';' after value.")?;
        
        Result::Ok(Stmt::Print { expression: Box::new(expr) })
    }

    fn expression_statment(&mut self) -> Result<Stmt, String> {
        let expr = self.expression()?;
        self.consume(TokenType::Semicolon, "Expected ';' after expression.")?;

        Result::Ok(Stmt::Expression { expression: Box::new(expr) })
    }

    fn expression(&mut self) -> Result<Expr, String> {
        self.equality()
    }

    fn equality(&mut self) -> Result<Expr, String> {
        let mut expr = self.comparison()?;

        while self.match_token_type(&[TokenType::BangEqual, TokenType::EqualEqual]) {
            let operator = self.previous().clone();
            let expr_right = self.comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                right: Box::new(expr_right),
                operator,
            };
        }

        Result::Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr, String> {
        let mut expr = self.term()?;

        while self.match_token_type(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            let operator = self.previous().clone();
            let expr_right = self.term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                right: Box::new(expr_right),
                operator,
            };
        }

        Result::Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, String> {
        let mut expr = self.factor()?;

        while self.match_token_type(&[TokenType::Plus, TokenType::Minus]) {
            let operator = self.previous().clone();
            let expr_right = self.comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                right: Box::new(expr_right),
                operator,
            }
        }

        Result::Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr, String> {
        let mut expr = self.unary()?;

        while self.match_token_type(&[TokenType::Star, TokenType::Slash]) {
            let operator = self.previous().clone();
            let expr_right = self.unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                right: Box::new(expr_right),
                operator,
            }
        }

        Result::Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.match_token_type(&[TokenType::Bang, TokenType::Minus]) {
            let operator = self.previous().clone();
            let expr_right = self.unary()?;
            Result::Ok(Expr::Unary {
                right: Box::new(expr_right),
                operator,
            })
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> Result<Expr, String> {
        if self.match_token_type(&[TokenType::False]) {
            return Result::Ok(Expr::Literal {
                value: LiteralType::BoolValue(false),
            });
        }
        if self.match_token_type(&[TokenType::True]) {
            return Result::Ok(Expr::Literal {
                value: LiteralType::BoolValue(true),
            });
        }
        if self.match_token_type(&[TokenType::Nil]) {
            return Result::Ok(Expr::Literal {
                value: LiteralType::NoneValue,
            });
        }
        if self.match_token_type(&[TokenType::Number, TokenType::String]) {
            return Result::Ok(Expr::Literal {
                value: self.previous().literal.clone(),
            });
        }
        if self.match_token_type(&[TokenType::LeftParen]) {
            let expr = self.expression()?;
            self.consume(TokenType::RightParen, "Expect ')' after expression")?;
            return Result::Ok(Expr::Grouping {
                expression: Box::new(expr),
            });
        }

        let token_type = self.advance().ttype;
        Result::Err(self.error(&format!("Unexpected primary token {:?}", token_type)))
    }

    fn match_token_type(&mut self, token_types: &[TokenType]) -> bool {
        for ttype in token_types {
            if self.check(*ttype) {
                self.advance();
                return true;
            }
        }

        return false;
    }

    fn check(&self, token_type: TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }

        self.peek().ttype == token_type
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> Result<&Token, String> {
        if self.check(token_type) {
            return Result::Ok(&self.advance());
        }

        Result::Err(self.error(message))
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }

        self.previous()
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn is_at_end(&self) -> bool {
        self.peek().ttype == TokenType::Eof
    }

    fn error(&self, message: &str) -> String {
        if self.peek().ttype == TokenType::Eof {
            format!("[line {}] Error at end: {message}", self.peek().line)
        } else {
            format!(
                "[line {}] Error at '{}': {message}",
                self.peek().line,
                self.peek().lexeme
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::scanner::TokenType;
    use super::*;

    #[test]
    fn test_expr_printing() {
        let op_minus = Token {
            ttype: TokenType::Minus,
            lexeme: "-".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };
        let expr_lit1 = Expr::Literal {
            value: LiteralType::NumberValue(123.0),
        };
        let expr_unary = Expr::Unary {
            right: Box::new(expr_lit1),
            operator: op_minus,
        };

        let op_star = Token {
            ttype: TokenType::Star,
            lexeme: "*".to_string(),
            literal: LiteralType::NoneValue,
            line: 1,
        };
        let expr_lit2 = Box::new(Expr::Literal {
            value: LiteralType::NumberValue(45.67),
        });
        let expr_grouping = Expr::Grouping {
            expression: expr_lit2,
        };

        let expression = Expr::Binary {
            left: Box::new(expr_unary),
            right: Box::new(expr_grouping),
            operator: op_star,
        };

        assert_eq!(expression.to_string(), "(* (- 123) (group 45.67))");
    }

    #[test]
    fn test_expr_parsing() {
        let mut scanner = Scanner::new("-123 * (45.67)");
        let (tokens, errors) = scanner.scan_tokens();

        assert!(errors.is_empty());

        let mut parser = Parser::new(tokens.clone());
        let ptree = parser.parse().unwrap();

        assert_eq!(ptree.to_string(), "(* (- 123) (group 45.67))");
    }
}
