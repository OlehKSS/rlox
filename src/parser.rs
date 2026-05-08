use core::fmt;

use super::scanner::{LiteralType, Token, TokenType};
use super::utility::error;

/// program      -> declaration* EOF ;
/// declaration  -> classDecl | funDecl | varDecl | statement ;
/// classDecl    -> "class" IDENTIFIER ( "<" IDENTIFIER )? "{" function* "}" ;
/// funDecl      -> "fun" function;
/// function     -> IDENTIFIER "(" parameters? ")" block ;
/// parameters   -> IDENTIFIER ( "," IDENTIFIER )* ;
/// varDecl      -> "var" IDENTIFIER ( "=" expression )? ";" ;
/// statement    -> exprStmt
///             | forStmt
///             | ifStmt
///             | printStmt
///             | whileStmt
///             | breakStmt
///             | continueStmt
///             | returnStmt
///             | block ;
/// exprStmt     -> expression ";" ;
/// forStmt      -> "for" "(" varDecl | exprStmt | ";" ) expression? ";" expression? ")" statement ;
/// ifStmt       -> "if" "(" expression ")" statement ( "else" statement )? ;
/// printStmt    -> "print" expression ";" ;
/// whileStmt    -> "while" "(" expression ")" statement ;
/// breakStmt    -> "break" ";"
/// continueStmt -> "continue" ;
/// returnStmt   -> "return" expression? ";" ;
/// block        -> "{" declaration* "}" ;
///
/// expression   -> assignment ;
/// assignment   -> ( call "." )? IDENTIFIER "=" assignment | logic_or ; // Property setter or variable assignment
/// logic_or     -> logic_and ( "or" logic_and )* ;
/// logic_and    -> equality ( "and" equality )* ;
/// equality     -> comparison ( ( "!=" | "==" ) comparison )* ;
/// comparison   -> term ( ( ">" | ">=" | "<" | "<=" ) term )* ;
/// term         -> factor ( ( "-" | "+" ) factor )* ;
/// factor       -> unary ( ( "/" | "*" ) unary )* ;
/// unary        -> ( "!" | "-" ) unary | call ;
/// call         -> primary ( "(" arguments? ")" | "." IDENTIFIER )* ; // Call or property access
/// arguments    -> expression ( "," expression )* ;
/// primary      -> NUMBER | STRING | "true" | "false" | "nil" | "(" expression ")" | IDENTIFIER
///                 | "super" "." IDENTIFIER ; // super cannot be used on its own, only for method access

/// Statements
#[derive(Debug, Clone)]
pub enum Stmt {
    Block {
        statements: Vec<Stmt>,
    },
    Break,
    Class {
        name: Token,
        superclass: Option<Box<Expr>>,
        methods: Vec<Stmt>,
    },
    Continue,
    Expression {
        expression: Box<Expr>,
    },
    Function {
        name: Token,
        parameters: Vec<Token>,
        body: Vec<Stmt>,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    Print {
        expression: Box<Expr>,
    },
    Return {
        keyword: Token,
        value: Box<Expr>,
    },
    Var {
        name: Token,
        initializer: Option<Box<Expr>>,
    },
    While {
        condition: Box<Expr>,
        body: Box<Stmt>,
        increment: Option<Expr>,
    },
}

/// Expressions
#[derive(Debug, Clone)]
pub enum Expr {
    Assign {
        id: usize,
        name: Token,
        value: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        right: Box<Expr>,
        operator: Token,
    },
    Call {
        callee: Box<Expr>,
        right_parenthesis: Token,
        arguments: Vec<Expr>,
    },
    // Class field access
    Get {
        name: Token,
        object: Box<Expr>,
    },
    Grouping {
        expression: Box<Expr>,
    },
    Literal {
        value: LiteralType,
    },
    Logical {
        left: Box<Expr>,
        right: Box<Expr>,
        operator: Token,
    },
    Set {
        name: Token,
        object: Box<Expr>,
        value: Box<Expr>,
    },
    Super {
        id: usize,
        keyword: Token,
        method: Token,
    },
    This {
        id: usize,
        keyword: Token,
    },
    Unary {
        right: Box<Expr>,
        operator: Token,
    },
    Variable {
        id: usize,
        name: Token,
    },
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Expr::Assign { name, value, .. } => {
                write!(f, "{} = {}", name.lexeme, value.to_string())
            }
            Expr::Binary {
                left,
                right,
                operator,
            } => {
                let subexpr = parenthesize(&operator.lexeme, &[&left, &right]);
                write!(f, "{}", subexpr)
            }
            Expr::Call {
                callee, arguments, ..
            } => {
                let args_str = arguments
                    .iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");

                write!(f, "{}({})", callee, args_str)
            }
            Expr::Get { name, object } => {
                write!(f, "{}.{}", object, name.lexeme)
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
            Expr::Logical {
                left,
                right,
                operator,
            } => {
                let subexpr = parenthesize(&operator.lexeme, &[&left, &right]);
                write!(f, "{}", subexpr)
            }
            Expr::Set {
                name,
                object,
                value,
            } => {
                write!(f, "{}.{} = {}", object, name.lexeme, value)
            }
            Expr::Super { method, .. } => write!(f, "super.{}", method.lexeme),
            Expr::This { .. } => write!(f, "this"),
            Expr::Unary { right, operator } => {
                let subexpr = parenthesize(&operator.lexeme, &[&right]);
                write!(f, "{}", subexpr)
            }
            Expr::Variable { name, .. } => {
                write!(f, "var {}", name.lexeme)
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

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    is_loop_open: bool,
    next_id: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens: tokens,
            current: 0,
            is_loop_open: false,
            next_id: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>, Vec<String>> {
        let mut statements: Vec<Stmt> = Vec::new();
        let mut error_messages: Vec<String> = Vec::new();

        while !self.is_at_end() {
            match &self.declaration() {
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

    fn declaration(&mut self) -> Result<Stmt, String> {
        if self.match_token_type(&[TokenType::Class]) {
            return self.class_declaration();
        }
        if self.match_token_type(&[TokenType::Fun]) {
            return self.function_declaration("function");
        }
        if self.match_token_type(&[TokenType::Var]) {
            return self.var_declaration();
        }

        self.statement()
        // TODO: synchronize() when errors happen
    }

    fn class_declaration(&mut self) -> Result<Stmt, String> {
        let name = self
            .consume(TokenType::Identifier, "Expected class name.")?
            .clone();

        let superclass = if self.match_token_type(&[TokenType::Less]) {
            self.consume(TokenType::Identifier, "Expected superclass name.")?;
            Option::Some(Box::new(Expr::Variable {
                id: self.get_next_id(),
                name: self.previous().clone(),
            }))
        } else {
            Option::None
        };

        self.consume(TokenType::LeftBrace, "Expected '{' before class body.")?;

        let mut methods = vec![];

        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            let fun = self.function_declaration("method")?;
            methods.push(fun);
        }

        self.consume(TokenType::RightBrace, "Expected '}' after class body")?;

        Result::Ok(Stmt::Class {
            name,
            superclass,
            methods,
        })
    }

    fn function_declaration(&mut self, kind: &str) -> Result<Stmt, String> {
        let name = self
            .consume(TokenType::Identifier, &format!("Expected {kind} name."))?
            .clone();
        self.consume(
            TokenType::LeftParen,
            &format!("Expected '(' after {kind} name."),
        )?;

        let mut parameters: Vec<Token> = vec![];

        if !self.check(TokenType::RightParen) {
            loop {
                if parameters.len() >= 255 {
                    return Result::Err("Cannot have more than 255 parameters.".to_string());
                }

                let param_name = self.consume(TokenType::Identifier, "Expected parameter name.")?;
                parameters.push(param_name.clone());

                if !self.match_token_type(&[TokenType::Comma]) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expected '(' after parameters.")?;
        self.consume(
            TokenType::LeftBrace,
            &format!("Expected '{{' before {kind} body."),
        )?;
        let body = self.block_statement()?;

        Result::Ok(Stmt::Function {
            name: name,
            parameters,
            body,
        })
    }

    fn var_declaration(&mut self) -> Result<Stmt, String> {
        let name = self
            .consume(TokenType::Identifier, "Expected variable name.")?
            .clone();
        let mut initializer = Option::None;

        if self.match_token_type(&[TokenType::Equal]) {
            let expr = self.expression()?;
            initializer = Option::Some(Box::new(expr));
        }

        self.consume(
            TokenType::Semicolon,
            "Expected ';' after variable declaration.",
        )?;
        return Result::Ok(Stmt::Var {
            name: name,
            initializer: initializer,
        });
    }

    fn statement(&mut self) -> Result<Stmt, String> {
        if self.match_token_type(&[TokenType::Break]) {
            return self.break_statement();
        }
        if self.match_token_type(&[TokenType::Continue]) {
            return self.continue_statement();
        }
        if self.match_token_type(&[TokenType::For]) {
            return self.for_statement();
        }
        if self.match_token_type(&[TokenType::If]) {
            return self.if_statement();
        }
        if self.match_token_type(&[TokenType::Print]) {
            return self.print_statement();
        }
        if self.match_token_type(&[TokenType::Return]) {
            return self.return_statement();
        }
        if self.match_token_type(&[TokenType::While]) {
            return self.while_statement();
        }

        if self.match_token_type(&[TokenType::LeftBrace]) {
            let stmts = self.block_statement()?;
            return Result::Ok(Stmt::Block { statements: stmts });
        }

        self.expression_statement()
    }

    fn break_statement(&mut self) -> Result<Stmt, String> {
        self.consume(TokenType::Semicolon, "Missing ';' after 'break'.")?;
        if self.is_loop_open {
            Result::Ok(Stmt::Break)
        } else {
            Result::Err("'break' outside loop.".to_string())
        }
    }

    fn continue_statement(&mut self) -> Result<Stmt, String> {
        self.consume(TokenType::Semicolon, "Missing ';' after 'continue'.")?;
        if self.is_loop_open {
            Result::Ok(Stmt::Continue)
        } else {
            Result::Err("'continue' is outside loop".to_string())
        }
    }

    fn for_statement(&mut self) -> Result<Stmt, String> {
        self.consume(TokenType::LeftParen, "Expected '(' after 'for'.")?;

        let initializer = if self.match_token_type(&[TokenType::Semicolon]) {
            Option::None
        } else if self.match_token_type(&[TokenType::Var]) {
            let var_stmt = self.var_declaration()?;
            Option::Some(var_stmt)
        } else {
            let expr_stmt = self.expression_statement()?;
            Option::Some(expr_stmt)
        };

        let condition = if !self.check(TokenType::Semicolon) {
            let expr = self.expression()?;
            expr
        } else {
            Expr::Literal {
                value: LiteralType::BoolValue(true),
            }
        };
        self.consume(TokenType::Semicolon, "Expect ';' after loop condition.")?;

        let increment = if !self.check(TokenType::RightParen) {
            let expr = self.expression()?;
            Option::Some(expr)
        } else {
            Option::None
        };

        self.consume(TokenType::RightParen, "Expected ')' after 'for' clauses.")?;

        let is_outer_loop_open = self.is_loop_open;
        self.is_loop_open = true;
        let mut body = self.statement()?;
        self.is_loop_open = is_outer_loop_open;

        // Synthesize syntax tree nodes that expres the semantics of the for loop
        body = Stmt::While {
            condition: Box::new(condition),
            body: Box::new(body),
            increment: increment,
        };

        if let Option::Some(expr) = initializer {
            body = Stmt::Block {
                statements: vec![expr, body],
            };
        }

        Result::Ok(body)
    }

    fn if_statement(&mut self) -> Result<Stmt, String> {
        self.consume(TokenType::LeftParen, "Expected '(' after 'if'.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expected ')' after if condition.")?;

        let then_branch = self.statement()?;
        let else_branch = if self.match_token_type(&[TokenType::Else]) {
            let else_branch = self.statement()?;
            Option::Some(Box::new(else_branch))
        } else {
            Option::None
        };

        Result::Ok(Stmt::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: else_branch,
        })
    }

    fn print_statement(&mut self) -> Result<Stmt, String> {
        let expr = self.expression()?;
        self.consume(TokenType::Semicolon, "Expected ';' after value.")?;

        Result::Ok(Stmt::Print {
            expression: Box::new(expr),
        })
    }

    fn return_statement(&mut self) -> Result<Stmt, String> {
        let keyword = self.previous().clone();
        let value = if self.check(TokenType::Semicolon) {
            Expr::Literal {
                value: LiteralType::NoneValue,
            }
        } else {
            self.expression()?
        };

        self.consume(TokenType::Semicolon, "Expected ';' after return values")?;

        Result::Ok(Stmt::Return {
            keyword,
            value: Box::new(value),
        })
    }

    fn while_statement(&mut self) -> Result<Stmt, String> {
        let is_outer_loop_open = self.is_loop_open;
        self.is_loop_open = true;
        self.consume(TokenType::LeftParen, "Expected '(' after 'while'.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expected ')' after condition")?;
        let body = self.statement()?;
        self.is_loop_open = is_outer_loop_open;

        Result::Ok(Stmt::While {
            condition: Box::new(condition),
            body: Box::new(body),
            increment: Option::None,
        })
    }

    fn expression_statement(&mut self) -> Result<Stmt, String> {
        let expr = self.expression()?;
        self.consume(TokenType::Semicolon, "Expected ';' after expression.")?;

        Result::Ok(Stmt::Expression {
            expression: Box::new(expr),
        })
    }

    fn block_statement(&mut self) -> Result<Vec<Stmt>, String> {
        let mut statements: Vec<Stmt> = Vec::new();

        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            let decl = self.declaration()?;
            statements.push(decl);
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block")?;

        Result::Ok(statements)
    }

    fn expression(&mut self) -> Result<Expr, String> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, String> {
        let expr = self.or()?;

        if self.match_token_type(&[TokenType::Equal]) {
            let equals_pos = self.previous_index();
            let value = self.assignment()?;
            // The parser returns getter for the left-side expression
            // Convert it to setter, since it is indicated by '='
            if let Expr::Variable { name, .. } = expr {
                return Result::Ok(Expr::Assign {
                    id: self.get_next_id(),
                    name: name,
                    value: Box::new(value),
                });
            }

            if let Expr::Get { name, object } = expr {
                return Result::Ok(Expr::Set {
                    name,
                    object,
                    value: Box::new(value),
                });
            }

            return Result::Err(error(
                "Invalid assignment target.",
                &self.tokens[equals_pos],
            ));
        }

        Result::Ok(expr)
    }

    fn or(&mut self) -> Result<Expr, String> {
        let expr = self.and()?;

        while self.match_token_type(&[TokenType::Or]) {
            let operator = self.previous().clone();
            let right = self.and()?;
            return Result::Ok(Expr::Logical {
                left: Box::new(expr),
                right: Box::new(right),
                operator: operator,
            });
        }

        Result::Ok(expr)
    }

    fn and(&mut self) -> Result<Expr, String> {
        let expr = self.equality()?;

        while self.match_token_type(&[TokenType::And]) {
            let operator = self.previous().clone();
            let right = self.equality()?;
            return Result::Ok(Expr::Logical {
                left: Box::new(expr),
                right: Box::new(right),
                operator: operator,
            });
        }

        Result::Ok(expr)
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
            self.call()
        }
    }

    fn call(&mut self) -> Result<Expr, String> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token_type(&[TokenType::LeftParen]) {
                expr = self.finish_call(&expr)?;
            }
            if self.match_token_type(&[TokenType::Dot]) {
                let name =
                    self.consume(TokenType::Identifier, "Expected property name after '.'.")?;
                expr = Expr::Get {
                    name: name.clone(),
                    object: Box::new(expr),
                };
            } else {
                break;
            }
        }

        Result::Ok(expr)
    }

    fn finish_call(&mut self, callee: &Expr) -> Result<Expr, String> {
        let mut arguments: Vec<Expr> = Vec::new();

        if !self.check(TokenType::RightParen) {
            loop {
                if arguments.len() >= 255 {
                    return Result::Err("Cannot have more than 255 arguments.".to_string());
                }

                let expr = self.expression()?;
                arguments.push(expr);

                if !self.match_token_type(&[TokenType::Comma]) {
                    break;
                }
            }
        }

        let right_parenthesis =
            self.consume(TokenType::RightParen, "Expected ')' after arguments.")?;

        Result::Ok(Expr::Call {
            callee: Box::new(callee.clone()),
            right_parenthesis: right_parenthesis.clone(),
            arguments,
        })
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
        if self.match_token_type(&[TokenType::Super]) {
            let keyword = self.previous().clone();
            self.consume(TokenType::Dot, "Expected '.' after 'super'.")?;
            let method = self
                .consume(TokenType::Identifier, "Expected superclass method name.")?
                .clone();
            return Result::Ok(Expr::Super {
                id: self.get_next_id(),
                keyword,
                method: method,
            });
        }
        if self.match_token_type(&[TokenType::This]) {
            return Result::Ok(Expr::This {
                id: self.get_next_id(),
                keyword: self.previous().clone(),
            });
        }
        if self.match_token_type(&[TokenType::Identifier]) {
            return Result::Ok(Expr::Variable {
                id: self.get_next_id(),
                name: self.previous().clone(),
            });
        }
        if self.match_token_type(&[TokenType::LeftParen]) {
            let expr = self.expression()?;
            self.consume(TokenType::RightParen, "Expect ')' after expression")?;
            return Result::Ok(Expr::Grouping {
                expression: Box::new(expr),
            });
        }

        self.advance();
        Result::Err(error(
            &format!("Unexpected primary token {:?}", self.previous().ttype),
            self.previous(),
        ))
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

        Result::Err(error(message, &self.peek()))
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

    fn previous_index(&self) -> usize {
        self.current - 1
    }

    fn is_at_end(&self) -> bool {
        self.peek().ttype == TokenType::Eof
    }

    fn get_next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::super::scanner::{Scanner, TokenType};
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
        let mut scanner = Scanner::new("-123 * (45.67);");
        let (tokens, errors) = scanner.scan_tokens();

        assert!(errors.is_empty());

        let mut parser = Parser::new(tokens.clone());
        let ptree = parser.parse().unwrap();

        assert_eq!(ptree.len(), 1);

        if let Stmt::Expression { expression } = &ptree[0] {
            assert_eq!(expression.to_string(), "(* (- 123) (group 45.67))");
        } else {
            assert!(false);
        }
    }
}
