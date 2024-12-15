#![warn(clippy::unwrap_used)]
#![allow(clippy::upper_case_acronyms)]
#![allow(unused_imports)]

use std::{
    error::Error,
    io::{Read, Write},
};

use crate::grammar::NT;

mod grammar;
mod parser;
mod parser_generator;
mod tokeniser;

fn main() -> Result<(), Box<dyn Error>> {
    let mut gram_str = String::new();
    std::io::stdin()
        .read_to_string(&mut gram_str)
        .expect("Failed to read");
    let tokens = tokeniser::tokenize(gram_str.as_str());
    println!("Tokenization complete!");
    std::io::stdout().flush()?;
    let ast = parser::parse(tokens)?;
    println!("{}", &ast.to_ebnf());
    println!("{}", ast.to_chomsky());
    // dbg!(parser_generator::follow(
    //     &NT::Non("F".into()),
    //     &ast.to_chomsky()
    // )?);
    let parser = parser_generator::get_parser(ast.to_chomsky())?;
    dbg!(parser("12+12"));

    Ok(())
}
