mod interpreter;
mod parser;
mod scanner;

use std::env;
use std::fs;
use std::io::{self, Write};

use interpreter::interpret;
use parser::Parser;
use scanner::Scanner;

pub const EXIT_USAGE: i32 = 64;
pub const EXIT_DATAERR: i32 = 65;
pub const EXIT_SOFTWARE: i32 = 70;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut interpreter = Lox::new();

    if args.len() > 2 {
        println!("Usage: jlox [script]");
        std::process::exit(EXIT_USAGE);
    } else if args.len() == 2 {
        interpreter.run_file(&args[1]);
    } else {
        interpreter.run_prompt();
    }
}

struct Lox {
    had_error: bool,
    had_runtime_error: bool,
}

impl Lox {
    fn new() -> Self {
        Lox {
            had_error: false,
            had_runtime_error: false,
        }
    }

    fn run_file(&mut self, script_path: &str) {
        match fs::read_to_string(script_path) {
            Ok(file_contents) => {
                self.run(&file_contents);

                if self.had_error {
                    std::process::exit(EXIT_DATAERR);
                }
                if self.had_runtime_error {
                    std::process::exit(EXIT_SOFTWARE);
                }
            }
            Err(error) => {
                eprintln!("Failed to read file '{script_path}', {error}");
            }
        }
    }

    fn run_prompt(&mut self) {
        let mut input = String::new();

        loop {
            print!("> ");
            io::stdout().flush().unwrap();
            input.clear();

            match io::stdin().read_line(&mut input) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        // EOF reached (e.g., Ctrl+D)
                        break;
                    }

                    let line = input.trim();
                    self.run(line);
                    self.had_error = false;
                }
                Err(error) => {
                    eprintln!("Error reading input: {}", error);
                }
            }
        }
    }

    fn run(&mut self, source: &str) {
        let mut scanner = Scanner::new(source);
        let (tokens, errors) = scanner.scan_tokens();

        if !errors.is_empty() {
            self.had_error = true;
            for err in errors {
                eprintln!("{}", err);
            }
            return;
        }

        let mut parser = Parser::new(tokens.clone());
        let parse_result = parser.parse();

        if let Result::Err(emsg) = parse_result {
            eprintln!("{}", emsg);
            return;
        }

        interpret(&parse_result.unwrap());
    }

    fn error(&mut self, line: i64, message: &str) {
        self.report(line, "", message);
    }

    fn report(&mut self, line: i64, source: &str, message: &str) {
        println!("[line {line}] Error {source}: {message}");
        self.had_error = true;
    }
}
