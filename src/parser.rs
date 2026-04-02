use core::fmt;
use std::path::Display;

use super::scanner::{LiteralType, Token};

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
}
