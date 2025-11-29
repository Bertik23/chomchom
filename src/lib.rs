use std::error::Error;

use crate::parser_generator::{TokenIteratorType, TokenReq};

pub mod grammar;
pub mod parser;
pub mod parser_generator;
pub mod tokeniser;

pub fn get_parser<'a, TokenT: TokenReq>(
    grammar_file: &str,
    tokeniser: impl Fn(&'a str) -> TokenIteratorType<'a, TokenT>,
) -> impl Fn(&'a str) -> Result<parser_generator::AST<TokenT>, Box<dyn Error>> {
    let gram_str =
        std::fs::read_to_string(grammar_file).expect("Failed to read grammar.");
    let tokens = tokeniser::tokenize(gram_str.as_str());
    let ast = parser::parse(tokens).expect("Failed to parse grammar");
    parser_generator::get_parser(ast.to_chomsky(), tokeniser)
        .expect("Failed to generate parser")
}
