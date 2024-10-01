use crate::grammar::{GrammarEBNF, EBNF};
use crate::tokeniser::Token;
use std::collections::{HashMap, VecDeque};

pub fn parse(tokens: Vec<Token>) -> Result<GrammarEBNF, String> {
    let mut deq: VecDeque<Token> = tokens.into();
    let mut rules: HashMap<String, Vec<EBNF>> = HashMap::new();
    while !deq.is_empty() {
        let (nonterm, rule) = parse_rule(&mut deq)?;
        rules.entry(nonterm).or_default().push(rule);
    }

    Ok(GrammarEBNF { rules })
}

fn parse_rule(tokens: &mut VecDeque<Token>) -> Result<(String, EBNF), String> {
    let Token::Nonterminal(nonterm) = tokens
        .pop_front()
        .ok_or("Nonterminal on lhs of rule expected.")?
    else {
        return Err("Nonterminal on lhs of rule expected.".into());
    };
    if !matches!(tokens.pop_front(), Some(Token::Equal)) {
        return Err("Equal expected.".into());
    }
    let rule = parse_rhs(tokens)?;
    if parse_semicolon(tokens).is_err() {
        Err(format!("No semicolon after rule: {:?}", rule))
    } else {
        Ok((nonterm, rule))
    }
}

fn parse_rhs(tokens: &mut VecDeque<Token>) -> Result<EBNF, String> {
    parse_alternation(tokens)
}

fn parse_term(tokens: &mut VecDeque<Token>) -> Result<EBNF, String> {
    if let Some(token) = tokens.front() {
        match token {
            Token::LParen => {
                tokens.pop_front();
                let r = parse_rhs(tokens)?;
                if !matches!(tokens.pop_front(), Some(Token::RParen)) {
                    Err("No matching ')' found.".into())
                } else {
                    Ok(r)
                }
            }
            Token::LBracket => {
                tokens.pop_front();
                let r = parse_rhs(tokens)?;
                if !matches!(tokens.pop_front(), Some(Token::RBracket)) {
                    Err("No matching ')' found.".into())
                } else {
                    Ok(EBNF::Optional(Box::new(r)))
                }
            }
            Token::LBrace => {
                tokens.pop_front();
                let r = parse_rhs(tokens)?;
                if !matches!(tokens.pop_front(), Some(Token::RBrace)) {
                    Err("No matching '}' found.".into())
                } else {
                    Ok(EBNF::Iteration(Box::new(r)))
                }
            }
            Token::Nonterminal(_) => {
                let Some(Token::Nonterminal(pc)) = tokens.pop_front() else {
                    unreachable!()
                };
                Ok(EBNF::Non(pc))
            }
            Token::Terminal(_) => {
                let Some(Token::Terminal(pc)) = tokens.pop_front() else {
                    unreachable!()
                };
                Ok(EBNF::Term(pc))
            }
            Token::Epsilon => {
                tokens.pop_front();
                Ok(EBNF::Epsilon)
            }
            Token::RBrace => Err("lol".into()),
            idk => Err(format!("Unexpected '{:?}', {:?}", idk, tokens)),
        }
    } else {
        Err("wtf2".into())
    }
}

fn parse_factor(tokens: &mut VecDeque<Token>) -> Result<EBNF, String> {
    let term = parse_term(tokens)?;

    if let Some(token) = tokens.front() {
        match token {
            Token::Asterix => {
                tokens.pop_front();
                Ok(EBNF::Iteration(Box::new(term)))
            }
            Token::Plus => {
                tokens.pop_front();
                Ok(EBNF::OneOrMore(Box::new(term)))
            }
            _ => Ok(term),
        }
    } else {
        Err("idk co sem napsat".into())
    }
}

fn parse_alternation(tokens: &mut VecDeque<Token>) -> Result<EBNF, String> {
    let factor = parse_concatanation(tokens)?;
    if let Some(Token::Or) = tokens.front() {
        tokens.pop_front();
        Ok(EBNF::Or(
            Box::new(factor),
            Box::new(parse_alternation(tokens)?),
        ))
    } else {
        Ok(factor)
    }
}

fn parse_concatanation(tokens: &mut VecDeque<Token>) -> Result<EBNF, String> {
    let factor = parse_factor(tokens)?;
    if let Some(t) = tokens.front() {
        if matches!(t, Token::Semicolon) {
            Ok(factor)
        } else {
            let c = parse_concatanation(tokens);
            match c {
                Ok(c) => Ok(EBNF::Concat(Box::new(factor), Box::new(c))),
                Err(_) => Ok(factor),
            }
        }
    } else {
        Ok(factor)
    }
}

fn parse_semicolon(tokens: &mut VecDeque<Token>) -> Result<(), String> {
    if let Some(Token::Semicolon) = tokens.pop_front() {
        Ok(())
    } else {
        Err("Semicolon not found.".into())
    }
}
