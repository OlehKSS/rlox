use super::scanner::{Token, TokenType};

pub fn error(message: &str, token: &Token) -> String {
    if token.ttype == TokenType::Eof {
        format!("[line {}] Error at end: {message}", token.line)
    } else {
        format!(
            "[line {}] Error at '{}': {message}",
            token.line, token.lexeme
        )
    }
}
