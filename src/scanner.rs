use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use super::callable::Callable;
use super::callable::LoxInstance;

pub struct Scanner {
    source: String,
    tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub ttype: TokenType,
    pub lexeme: String,
    pub literal: LiteralType,
    pub line: usize,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenType {
    // Single character tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    // One or two character tokens
    Bang, // !
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals
    Identifier,
    String,
    Number,

    // Keywords
    And,
    Break,
    Class,
    Continue,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Static,
    Super,
    This,
    True,
    Var,
    While,

    Eof,
}

#[derive(Debug, PartialEq, Clone)]
pub enum LiteralType {
    StringValue(String),
    NumberValue(f64),
    BoolValue(bool),
    Callable(Callable),
    Instance(Rc<RefCell<LoxInstance>>),
    NoneValue,
}

impl Scanner {
    pub fn new(source: &str) -> Self {
        Scanner {
            source: String::from(source),
            tokens: vec![],
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_tokens(&mut self) -> (&Vec<Token>, Vec<String>) {
        let mut errors: Vec<String> = Vec::new();

        while !self.is_at_end() {
            self.start = self.current;

            if let Err(error_message) = self.scan_token() {
                errors.push(error_message);
            }
        }

        self.tokens.push(Token {
            ttype: TokenType::Eof,
            lexeme: String::new(),
            literal: LiteralType::NoneValue,
            line: self.line,
        });

        (&self.tokens, errors)
    }

    fn scan_token(&mut self) -> Result<(), String> {
        let c = self.advance();

        match c {
            // Single char lexemes
            '(' => self.add_token(TokenType::LeftParen),
            ')' => self.add_token(TokenType::RightParen),
            '{' => self.add_token(TokenType::LeftBrace),
            '}' => self.add_token(TokenType::RightBrace),
            ',' => self.add_token(TokenType::Comma),
            '.' => self.add_token(TokenType::Dot),
            '-' => self.add_token(TokenType::Minus),
            '+' => self.add_token(TokenType::Plus),
            ';' => self.add_token(TokenType::Semicolon),
            '*' => self.add_token(TokenType::Star),
            // Multichar lexemes
            '!' => {
                let token_type = if self.match_current_char('=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                };
                self.add_token(token_type);
            }
            '=' => {
                let token_type = if self.match_current_char('=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                };
                self.add_token(token_type);
            }
            '<' => {
                let token_type = if self.match_current_char('=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                };
                self.add_token(token_type);
            }
            '>' => {
                let token_type = if self.match_current_char('=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                };
                self.add_token(token_type);
            }
            '/' => {
                if self.match_current_char('/') {
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                } else {
                    self.add_token(TokenType::Slash);
                }
            }
            // Skip whitespace
            ' ' | '\r' | '\t' => (),
            // Newline
            '\n' => self.line += 1,
            // Literals
            '"' => self.string(),
            '0'..='9' => self.number(),
            c if c.is_ascii_alphabetic() || c == '_' => {
                self.identifier();
            }
            _ => {
                return Result::Err(
                    format!("Unsupported character '{}' at line {}", c, self.line).to_string(),
                );
            }
        };

        Result::Ok(())
    }

    fn match_current_char(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }

        let current_char = self.source.as_bytes()[self.current] as char;
        if current_char != expected {
            return false;
        }

        self.current += 1;
        true
    }

    fn advance(&mut self) -> char {
        let current_char = self.source.as_bytes()[self.current] as char;
        self.current += 1;
        current_char
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        self.source.as_bytes()[self.current] as char
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            return '\0';
        }
        self.source.as_bytes()[self.current + 1] as char
    }

    fn add_token(&mut self, token_type: TokenType) {
        self.add_token_literal(token_type, LiteralType::NoneValue);
    }

    fn add_token_literal(&mut self, token_type: TokenType, literal: LiteralType) {
        let text = self.source[self.start..self.current].to_string();
        self.tokens.push(Token {
            ttype: token_type,
            lexeme: text,
            literal: literal,
            line: self.line,
        });
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn string(&mut self) {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            // TODO: Report error via Lox.error?
            panic!("Unterminated string.");
        }

        self.advance(); // The closing "
        // Trim the surrounding quotes
        let start = self.start + 1;
        let end = self.current - 1;
        let value = self.source[start..end].to_string();
        self.add_token_literal(TokenType::String, LiteralType::StringValue(value));
    }

    fn number(&mut self) {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance(); // Consume "."

            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        let value: f64 = self.source[self.start..self.current]
            .parse()
            .expect("Failed to parse number");

        self.add_token_literal(TokenType::Number, LiteralType::NumberValue(value));
    }

    fn identifier(&mut self) {
        while self.peek().is_ascii_alphanumeric() || self.peek() == '_' {
            self.advance();
        }

        let text = &self.source[self.start..self.current];
        let token_type = match text {
            "and" => TokenType::And,
            "break" => TokenType::Break,
            "class" => TokenType::Class,
            "continue" => TokenType::Continue,
            "else" => TokenType::Else,
            "false" => TokenType::False,
            "for" => TokenType::For,
            "fun" => TokenType::Fun,
            "if" => TokenType::If,
            "nil" => TokenType::Nil,
            "or" => TokenType::Or,
            "print" => TokenType::Print,
            "return" => TokenType::Return,
            "static" => TokenType::Static,
            "super" => TokenType::Super,
            "this" => TokenType::This,
            "true" => TokenType::True,
            "var" => TokenType::Var,
            "while" => TokenType::While,
            _ => TokenType::Identifier,
        };

        self.add_token(token_type);
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.literal == LiteralType::NoneValue {
            write!(f, "{:?} {}", self.ttype, self.lexeme)
        } else {
            write!(f, "{:?} {} {}", self.ttype, self.lexeme, self.literal)
        }
    }
}

impl fmt::Display for LiteralType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            LiteralType::StringValue(value) => {
                write!(f, "{}", value)
            }
            LiteralType::NumberValue(value) => {
                write!(f, "{}", value)
            }
            LiteralType::BoolValue(value) => {
                write!(f, "{}", value)
            }
            LiteralType::Callable(callable) => {
                write!(f, "{}", callable)
            }
            LiteralType::Instance(instance) => {
                write!(f, "{}", instance.borrow())
            }
            LiteralType::NoneValue => {
                write!(f, "")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_character_tokens() {
        let mut scanner = Scanner::new("(){},.-+;*");
        let (tokens, errors) = scanner.scan_tokens();

        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 11); // 10 characters + EOF
        assert_eq!(tokens[0].ttype, TokenType::LeftParen);
        assert_eq!(tokens[1].ttype, TokenType::RightParen);
        assert_eq!(tokens[2].ttype, TokenType::LeftBrace);
        assert_eq!(tokens[3].ttype, TokenType::RightBrace);
        assert_eq!(tokens[4].ttype, TokenType::Comma);
        assert_eq!(tokens[5].ttype, TokenType::Dot);
        assert_eq!(tokens[6].ttype, TokenType::Minus);
        assert_eq!(tokens[7].ttype, TokenType::Plus);
        assert_eq!(tokens[8].ttype, TokenType::Semicolon);
        assert_eq!(tokens[9].ttype, TokenType::Star);
        assert_eq!(tokens[10].ttype, TokenType::Eof);
    }

    #[test]
    fn test_multi_character_tokens() {
        let mut scanner = Scanner::new("!= ! == = <= < >= > / //");
        let (tokens, errors) = scanner.scan_tokens();

        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 10); // 9 tokens + EOF, comment // is ignored
        assert_eq!(tokens[0].ttype, TokenType::BangEqual);
        assert_eq!(tokens[0].lexeme, "!=");
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[1].ttype, TokenType::Bang);
        assert_eq!(tokens[2].ttype, TokenType::EqualEqual);
        assert_eq!(tokens[3].ttype, TokenType::Equal);
        assert_eq!(tokens[4].ttype, TokenType::LessEqual);
        assert_eq!(tokens[5].ttype, TokenType::Less);
        assert_eq!(tokens[6].ttype, TokenType::GreaterEqual);
        assert_eq!(tokens[7].ttype, TokenType::Greater);
        assert_eq!(tokens[8].ttype, TokenType::Slash);
        assert_eq!(tokens[8].lexeme, "/");
        assert_eq!(tokens[8].line, 1);
        assert_eq!(tokens[9].ttype, TokenType::Eof);
    }

    #[test]
    fn test_new_line() {
        let mut scanner = Scanner::new("\n\n\n");
        let (tokens, errors) = scanner.scan_tokens();

        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].ttype, TokenType::Eof);
        assert_eq!(tokens[0].line, 4);
    }

    #[test]
    fn test_string_literals() {
        let mut scanner = Scanner::new("\"abc\"");
        let (tokens, errors) = scanner.scan_tokens();

        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].ttype, TokenType::String);
        assert_eq!(tokens[0].lexeme, "\"abc\"");
        assert_eq!(
            tokens[0].literal,
            LiteralType::StringValue(String::from("abc"))
        );
        assert_eq!(tokens[1].ttype, TokenType::Eof);
    }

    #[test]
    fn test_keywords_identifiers_literals() {
        let mut scanner = Scanner::new("var foo = 123;");
        let (tokens, errors) = scanner.scan_tokens();

        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 6); // 5 tokens + EOF

        assert_eq!(tokens[0].ttype, TokenType::Var);

        assert_eq!(tokens[1].ttype, TokenType::Identifier);
        assert_eq!(tokens[1].lexeme, "foo");

        assert_eq!(tokens[2].ttype, TokenType::Equal);

        assert_eq!(tokens[3].ttype, TokenType::Number);
        assert_eq!(tokens[3].literal, LiteralType::NumberValue(123.0));

        assert_eq!(tokens[4].ttype, TokenType::Semicolon);

        assert_eq!(tokens[5].ttype, TokenType::Eof);
    }

    #[test]
    fn test_keywords() {
        let mut scanner = Scanner::new(
            "and class else false for fun if nil or print return super this true var while",
        );
        let (tokens, errors) = scanner.scan_tokens();

        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 17); // 16 keywords + EOF
        assert_eq!(tokens[0].ttype, TokenType::And);
        assert_eq!(tokens[1].ttype, TokenType::Class);
        assert_eq!(tokens[2].ttype, TokenType::Else);
        assert_eq!(tokens[3].ttype, TokenType::False);
        assert_eq!(tokens[4].ttype, TokenType::For);
        assert_eq!(tokens[5].ttype, TokenType::Fun);
        assert_eq!(tokens[6].ttype, TokenType::If);
        assert_eq!(tokens[7].ttype, TokenType::Nil);
        assert_eq!(tokens[8].ttype, TokenType::Or);
        assert_eq!(tokens[9].ttype, TokenType::Print);
        assert_eq!(tokens[10].ttype, TokenType::Return);
        assert_eq!(tokens[11].ttype, TokenType::Super);
        assert_eq!(tokens[12].ttype, TokenType::This);
        assert_eq!(tokens[13].ttype, TokenType::True);
        assert_eq!(tokens[14].ttype, TokenType::Var);
        assert_eq!(tokens[15].ttype, TokenType::While);
        assert_eq!(tokens[16].ttype, TokenType::Eof);
    }

    #[test]
    fn test_errors() {
        let mut scanner = Scanner::new("and `");
        let (tokens, errors) = scanner.scan_tokens();
        assert_eq!(errors.is_empty(), false);
        assert_eq!(tokens.len(), 2);
    }
}
