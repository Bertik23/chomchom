#![warn(clippy::unwrap_used)]

use std::{
    error::Error,
    io::{Read, Write},
};

mod grammar;
mod parser;
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
    Ok(())
}
