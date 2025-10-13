#![warn(clippy::unwrap_used)]
#![allow(clippy::upper_case_acronyms)]
#![allow(unused_imports)]

use std::{
    error::Error,
    io::{Read, Write},
};

use clap::Parser;

#[derive(Parser)]
struct Args {
    grammar: String,
}

use chomchom::grammar::NT;

use chomchom::{parser, parser_generator, tokeniser};

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let gram_str =
        std::fs::read_to_string(args.grammar).expect("Failed to read grammar.");
    // let mut gram_str = String::new();
    // std::io::stdin()
    //     .read_to_string(&mut gram_str)
    //     .expect("Failed to read");
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
    let mut sentense = String::new();
    std::io::stdin()
        .read_to_string(&mut sentense)
        .expect("Failed to read");
    let parsed = match parser(&sentense) {
        Err(err) => {
            println!("Error: {}", err);
            return Ok(());
        }
        Ok(p) => p,
    };
    dbg!(parsed);

    Ok(())
}
