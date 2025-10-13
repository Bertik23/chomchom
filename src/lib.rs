use std::error::Error;

pub mod grammar;
pub mod parser;
pub mod parser_generator;
pub mod tokeniser;

pub fn get_parser(
    grammar_file: &str,
) -> impl Fn(&str) -> Result<parser_generator::AST, Box<dyn Error>> {
    let gram_str =
        std::fs::read_to_string(grammar_file).expect("Failed to read grammar.");
    let tokens = tokeniser::tokenize(gram_str.as_str());
    let ast = parser::parse(tokens).expect("Failed to parse grammar");
    parser_generator::get_parser(ast.to_chomsky())
        .expect("Failed to generate parser")
}
