use crate::grammar::{GrammarEBNF, Rstr, EBNF};
use crate::tokeniser::Token;
use std::collections::{BTreeMap, VecDeque};

pub fn parse(tokens: Vec<Token>) -> Result<GrammarEBNF, String> {
    let mut deq: VecDeque<Token> = tokens.into();
    let mut rules: BTreeMap<Rstr, Vec<EBNF>> = BTreeMap::new();
    let mut start_nonterm: Option<Rstr> = None;
    while !deq.is_empty() {
        let (nonterm, rule) = parse_rule(&mut deq)?;
        if start_nonterm.is_none() {
            start_nonterm = Some(nonterm.clone());
        }
        rules.entry(nonterm).or_default().push(rule);
    }

    Ok(GrammarEBNF {
        start_nonterm: start_nonterm
            .unwrap_or("No starting nonterminal.".into()),
        rules,
    })
}

fn parse_rule(tokens: &mut VecDeque<Token>) -> Result<(Rstr, EBNF), String> {
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
        Err("Expected one of Asterix or Plus, found EOF".into())
    }
}

fn flaten_or(x: EBNF) -> Vec<EBNF> {
    match x {
        EBNF::Or(or) => or.into_iter().flat_map(flaten_or).collect(),
        els => vec![els],
    }
}

fn parse_alternation(tokens: &mut VecDeque<Token>) -> Result<EBNF, String> {
    let factor = parse_concatanation(tokens)?;
    if let Some(Token::Or) = tokens.front() {
        tokens.pop_front();
        let r = vec![factor, parse_alternation(tokens)?];
        Ok(EBNF::Or(r.into_iter().flat_map(flaten_or).collect()))
    } else {
        Ok(factor)
    }
}

fn flaten_cat(x: EBNF) -> Vec<EBNF> {
    match x {
        EBNF::Concat(cat) => cat.into_iter().flat_map(flaten_cat).collect(),
        els => vec![els],
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
                Ok(c) => {
                    let cs = vec![factor, c];
                    Ok(EBNF::Concat(
                        cs.into_iter().flat_map(flaten_cat).collect(),
                    ))
                }
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
