use std::env;
use std::fs;

use crate::lex::Lexer;
use crate::parse::Parser;
mod lex;

mod parse;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("expected source file as argument 0!");
        std::process::exit(1);
    }

    let contents = fs::read_to_string(&args[1])
        .expect("read failed");

    let mut lexer: Lexer = Lexer::new(&contents);
    lexer.lex();

    let mut errors = 0; for token in &lexer.result.tokens { 
        if let lex::TokenType::Illegal = token.token_type { 
            eprintln!(
                "Illegal token found at: line {} column {}!",
                token.span.line, token.span.column
            ); 
            errors += 1; 
        } 
    } 
    
    if errors > 0 { 
        eprintln!("{errors} errors found! Exiting..."); 
        std::process::exit(1); 
    }

    //println!("{:#?}", lexer.result);

    let mut parser: Parser = Parser::new(lexer.result.tokens);
    parser.parse();
    println!("{}", parser.result);
}