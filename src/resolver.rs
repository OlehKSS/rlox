use std::collections::HashMap;
use std::vec;

use super::parser::{Expr, Stmt};
use super::scanner::Token;
use super::utility::error;

struct VariableState {
    pub name: Token,
    pub defined: bool,
    pub used: bool,
}

// Semantic analysis pass
pub struct Resolver {
    scopes: Vec<HashMap<String, VariableState>>,
    locals: HashMap<usize, usize>,
    errors: Vec<String>,
}

impl Resolver {
    pub fn new() -> Self {
        Resolver {
            scopes: vec![],
            locals: HashMap::new(),
            errors: vec![],
        }
    }

    pub fn resolve(mut self, statements: &Vec<Stmt>) -> Result<HashMap<usize, usize>, Vec<String>> {
        self.resolve_stmts(statements);

        if self.errors.is_empty() {
            // TODO: Should we hide locals behind a pointer to avoid cloning?
            Result::Ok(self.locals)
        } else {
            Result::Err(self.errors)
        }
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        if let Option::Some(scope) = self.scopes.pop() {
            for (_, state) in scope {
                if !state.used {
                    self.errors.push(error(
                        &format!("Local variable '{}' is not used", state.name.lexeme),
                        &state.name,
                    ));
                }
            }
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Block { statements } => {
                self.begin_scope();
                self.resolve_stmts(statements);
                self.end_scope();
            }
            Stmt::Break => (),
            Stmt::Continue => (),
            Stmt::Expression { expression } => self.resolve_expr(expression),
            Stmt::Function {
                name,
                parameters,
                body,
            } => {
                self.declare(name);
                self.define(name);
                self.resolve_function(parameters, body);
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => self.resolve_if_stmt(condition, then_branch, else_branch.as_deref()),
            Stmt::Print { expression } => self.resolve_expr(expression),
            Stmt::Return { value, .. } => self.resolve_expr(value),
            Stmt::Var { name, initializer } => {
                // Split declaration and definition of variables to prevent
                // referencing a variable in its initializer
                // var a = "outer";
                // {
                //  var a = a;
                // }
                self.declare(name);
                if let Option::Some(expr) = initializer {
                    self.resolve_expr(expr);
                }
                self.define(name);
            }
            Stmt::While {
                condition,
                body,
                increment,
            } => {
                self.resolve_expr(condition);
                self.resolve_stmt(body);

                if let Option::Some(expr) = increment {
                    self.resolve_expr(expr);
                }
            }
        }
    }

    fn resolve_stmts(&mut self, statements: &Vec<Stmt>) {
        for stmt in statements {
            self.resolve_stmt(stmt);
        }
    }

    fn resolve_if_stmt(
        &mut self,
        condition: &Expr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
    ) {
        self.resolve_expr(condition);
        self.resolve_stmt(then_branch);

        if let Option::Some(stmt) = else_branch {
            self.resolve_stmt(stmt);
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Assign { id, name, value } => self.resolve_assign_expr(*id, name, value),
            Expr::Binary { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            Expr::Call {
                callee, arguments, ..
            } => self.resolve_call_expr(callee, arguments),
            Expr::Grouping { expression } => self.resolve_expr(expression),
            Expr::Literal { .. } => (),
            Expr::Logical { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right)
            }
            Expr::Unary { right, .. } => self.resolve_expr(right),
            Expr::Variable { id, name } => self.resolve_var_expr(*id, name),
        }
    }

    fn resolve_function(&mut self, parameters: &Vec<Token>, body: &Vec<Stmt>) {
        self.begin_scope();

        for param in parameters {
            self.declare(param);
            self.define(param);
        }

        self.resolve_stmts(body);
        self.end_scope();
    }

    fn resolve_var_expr(&mut self, id: usize, name: &Token) {
        if let Option::Some(scope) = self.scopes.last_mut() {
            if let Option::Some(state) = scope.get(&name.lexeme) {
                if !state.defined {
                    self.errors.push(error(
                        "Cannot read local variable in its own initializer.",
                        name,
                    ));
                }
            }
        }

        self.resolve_local(id, name);
    }

    fn resolve_assign_expr(&mut self, id: usize, name: &Token, value: &Expr) {
        self.resolve_expr(value);
        self.resolve_local(id, name);
    }

    fn resolve_call_expr(&mut self, callee: &Expr, arguments: &Vec<Expr>) {
        self.resolve_expr(callee);

        for arg in arguments {
            self.resolve_expr(arg);
        }
    }

    fn resolve_local(&mut self, id: usize, name: &Token) {
        for i in (0..self.scopes.len()).rev() {
            if let Option::Some(state) = self.scopes[i].get_mut(&name.lexeme) {
                state.used = true;
                self.locals.insert(id, self.scopes.len() - 1 - i);
                return;
            }
        }
    }

    fn declare(&mut self, name: &Token) {
        if let Option::Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name.lexeme.clone(),
                VariableState {
                    name: name.clone(),
                    defined: false,
                    used: false,
                },
            );
        }
    }

    fn define(&mut self, name: &Token) {
        if let Option::Some(scope) = self.scopes.last_mut() {
            if let Option::Some(state) = scope.get_mut(&name.lexeme) {
                state.defined = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::scanner::Scanner;

    #[test]
    fn test_unused_local_variable() {
        let source = "{ var a = 1; }";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        
        let resolver = Resolver::new();
        let result = resolver.resolve(&statements);
        
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("is not used"));
    }

    #[test]
    fn test_read_in_own_initializer() {
        let source = "var a = \"outer\"; { var a = a; }";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        
        let resolver = Resolver::new();
        let result = resolver.resolve(&statements);
        
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("Cannot read local variable in its own initializer"));
    }

    #[test]
    fn test_correct_resolution() {
        let source = "var a = \"global\"; { var b = a; print b; }";
        let mut scanner = Scanner::new(source);
        let (tokens, _) = scanner.scan_tokens();
        let mut parser = Parser::new(tokens.clone());
        let statements = parser.parse().unwrap();
        
        let resolver = Resolver::new();
        let result = resolver.resolve(&statements);
        
        assert!(result.is_ok()); // Should have no errors because 'b' is used
    }
}
