use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    error::Error,
    fmt::{Debug, Display},
    iter,
    rc::Rc,
};

use crate::grammar::{self, GrammarChomsky, GrammarEBNF, Rstr, NT};

fn first(
    input: &NT,
    first_table: &BTreeMap<Rstr, BTreeSet<Rstr>>,
) -> Result<BTreeSet<Rstr>, Box<dyn Error>> {
    Ok(match input {
        NT::Term(f) => BTreeSet::from([f.clone(); 1]),
        NT::Epsilon => BTreeSet::from([Rc::from(""); 1]),
        NT::Non(n) => first_table
            .get(n)
            .ok_or("No first for nonterminal")?
            .clone(),
    })
}

fn get_first_table(
    grammar: &GrammarChomsky,
) -> Result<(Vec<BTreeSet<Rstr>>, BTreeMap<Rstr, BTreeSet<Rstr>>), Box<dyn Error>>
{
    let mut table = Vec::new();
    table.resize(grammar.rules.len(), BTreeSet::new());
    let mut symbol_table: BTreeMap<Rstr, BTreeSet<Rstr>> = BTreeMap::new();
    let mut changed = true;
    while changed {
        changed = false;
        for (i, (l, r)) in grammar.rules.iter().enumerate() {
            for nt in r {
                match nt {
                    NT::Term(f) => {
                        changed = table[i].insert(f.clone()) || changed;
                        if !f.is_empty() {
                            break;
                        }
                    }
                    NT::Non(n) => {
                        if let Some(first) = symbol_table.get(n) {
                            if !first
                                .is_subset(&BTreeSet::from([Rstr::from("")]))
                            {
                                changed = first
                                    .iter()
                                    .cloned()
                                    .map(|x| table[i].insert(x))
                                    .any(|x| x || changed);
                                if !first.contains("") {
                                    break;
                                }
                            }
                        }
                    }
                    NT::Epsilon => {
                        table[i].insert("".into());
                    }
                }
            }
            symbol_table
                .entry(l.clone())
                .or_default()
                .extend(table[i].iter().cloned());
        }
    }
    Ok((table, symbol_table))
}

fn get_follow_table(
    grammar: &GrammarChomsky,
    first_table: &BTreeMap<Rstr, BTreeSet<Rstr>>,
) -> Result<BTreeMap<Rstr, BTreeSet<Rstr>>, Box<dyn Error>> {
    let mut follow_table: BTreeMap<Rstr, BTreeSet<Rstr>> = BTreeMap::new();
    follow_table.insert(
        grammar.start_nonterm.clone(),
        BTreeSet::from([Rstr::from(""); 1]),
    );
    let mut changed = true;
    while changed {
        changed = false;
        for (l, r) in grammar.rules.iter() {
            let mut last = vec![NT::Epsilon];
            for nt in r {
                for la in last.iter() {
                    if let NT::Non(n) = la {
                        changed = first(nt, first_table)?
                            .iter()
                            .filter(|x| !x.is_empty())
                            .map(|x| {
                                follow_table
                                    .entry(n.clone())
                                    .or_default()
                                    .insert(x.clone())
                            })
                            .any(|x| x || changed);
                    }
                }
                if first(nt, first_table)?.contains("") {
                    last.push(nt.clone())
                } else {
                    last = vec![nt.clone()];
                }
            }
            for la in last {
                if let NT::Non(n) = la {
                    if let Some(f) = follow_table.get(l) {
                        let f = f.clone();
                        changed = f
                            .iter()
                            .map(|x| {
                                follow_table
                                    .entry(n.clone())
                                    .or_default()
                                    .insert(x.clone())
                            })
                            .any(|x| x || changed);
                    }
                }
            }
        }
    }
    Ok(follow_table)
}

type ParseTable = BTreeMap<Rstr, BTreeMap<Rstr, usize>>;

fn gen_parsetable(
    grammar: &GrammarChomsky,
) -> Result<ParseTable, Box<dyn Error>> {
    // let mut first_table = vec![];
    // for (_l, r) in grammar.rules.iter() {
    //     first_table
    //         .push(first(r.first().ok_or("Non LL(1) gramamr.")?, grammar)?);
    // }
    // dbg!(&first_table);
    // dbg!(get_first_table(grammar));
    let (first_table, symbol_first_table) = get_first_table(grammar)?;
    println!("Got table.");
    // dbg!(&first_table, &symbol_first_table);
    // let mut follow_table = BTreeMap::new();
    // for nt in grammar.nonterminals.iter() {
    //     follow_table.insert(
    //         nt.clone(),
    //         follow(&NT::Non(nt.clone()), grammar, &symbol_first_table)?,
    //     );
    // }
    let follow_table = get_follow_table(grammar, &symbol_first_table)?;
    println!("Got second table.");
    // dbg!(&follow_table);
    let mut pt = ParseTable::new();
    for (i, (l, _)) in grammar.rules.iter().enumerate() {
        let use_follow = first_table[i].contains(&Rstr::from(""));
        let tp_iter = first_table[i]
            .iter()
            .filter(|x| !x.is_empty())
            .cloned()
            .zip(iter::repeat(i));
        if use_follow {
            let ft =
                follow_table.get(l).ok_or(format!("No follow for {}", l))?;
            pt.entry(l.clone())
                .or_default()
                .extend(ft.iter().cloned().zip(iter::repeat(i)).chain(tp_iter));
        } else {
            pt.entry(l.clone()).or_default().extend(tp_iter);
        }
        // dbg!(&pt);
    }
    // dbg!(first_table);
    // dbg!(follow_table);
    Ok(pt)
}

#[derive(Debug)]
pub enum AST {
    Node { name: Rstr, children: Vec<AST> },
    Token(Box<dyn TokenReq>),
}

enum StackObject {
    Term(Rstr),
    Nonterm(Rstr),
    Epsilon,
    PopNode,
}

impl From<NT> for StackObject {
    fn from(value: NT) -> Self {
        match value {
            NT::Non(a) => StackObject::Nonterm(a),
            NT::Term(a) => StackObject::Term(a),
            NT::Epsilon => StackObject::Epsilon,
        }
    }
}

#[derive(Debug)]
enum TokenType {
    EOF,
    String(Rstr),
}

impl From<String> for Token {
    fn from(value: String) -> Self {
        Token {
            token: TokenType::String(Rstr::from(value)),
            line: 0,
            column: 0,
            str_pos: 0,
        }
    }
}

impl Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenType::EOF => write!(f, "EOF"),
            TokenType::String(s) => write!(f, "{}", s),
        }
    }
}

impl Display for dyn TokenTrait {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Debug for dyn TokenTrait {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Token {{ token: {}, line: {}, column: {}, str_pos: {} }}",
            self.as_str(),
            self.line(),
            self.column(),
            self.str_pos()
        )
    }
}

impl Debug for dyn TokenReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Token {{ token: {}, line: {}, column: {}, str_pos: {} }}",
            self.as_str(),
            self.line(),
            self.column(),
            self.str_pos()
        )
    }
}

#[derive(Debug)]
struct Token {
    token: TokenType,
    line: usize,
    column: usize,
    str_pos: usize,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.token)
    }
}

impl TokenTrait for Token {
    fn as_str(&self) -> Rstr {
        match &self.token {
            TokenType::EOF => Rstr::from("EOF"),
            TokenType::String(s) => s.clone(),
        }
    }
    fn column(&self) -> usize {
        self.column
    }
    fn line(&self) -> usize {
        self.line
    }
    fn str_pos(&self) -> usize {
        self.str_pos
    }
}

pub trait TokenTrait {
    fn as_str(&self) -> Rstr;
    fn column(&self) -> usize;
    fn line(&self) -> usize;
    fn str_pos(&self) -> usize;
}

impl PartialEq<dyn TokenTrait> for Token {
    fn eq(&self, other: &dyn TokenTrait) -> bool {
        self.as_str() == other.as_str()
            && self.column() == other.column()
            && self.line() == other.line()
            && self.str_pos() == other.str_pos()
    }
}

pub trait TokenReq: TokenTrait + std::cmp::PartialEq<dyn TokenTrait> {}

impl TokenReq for Token {}

struct TokenIter {
    str: Rstr,
    curr: Rstr,
    terminals: Vec<Rstr>,
    pos: usize,
    str_pos: usize,
    line: usize,
    input_str: Rstr,
}

impl TokenIter {
    fn new(string: Rstr, terminals: Vec<Rstr>) -> TokenIter {
        TokenIter {
            str: string.clone(),
            curr: Rstr::from(""),
            terminals,
            pos: 0,
            str_pos: 0,
            line: 0,
            input_str: string,
        }
    }
}

impl<'a> Iterator for TokenIter {
    type Item = Box<dyn TokenReq>;

    fn next(&mut self) -> Option<Self::Item> {
        for term in self.terminals.iter().rev() {
            if term.is_empty() {
                continue;
            }
            if let Some(r) = self.str.strip_prefix(term.as_ref()) {
                self.str = r.into();
                println!("Matched: {}", term);
                self.curr = term.clone();
                for c in term.chars() {
                    self.pos += 1;
                    self.str_pos += 1;
                    if c == '\n' {
                        self.line += 1;
                        self.pos = 0;
                    }
                }
                return Some(Box::new(Token {
                    token: TokenType::String(term.clone()),
                    line: self.line,
                    column: self.pos - term.len(),
                    str_pos: self.str_pos - term.len(),
                }));
            }
        }
        None
    }
}

fn print_arrow(pos: usize, len: usize) -> String {
    " ".repeat(pos) + &"^".repeat(len.max(1))
}

pub type TokenIteratorType = Box<dyn Iterator<Item = Box<dyn TokenReq>>>;

fn get_tokenizer<'a>(
    grammar: &GrammarChomsky,
) -> impl Fn(&'a str) -> TokenIteratorType {
    let mut terminals = Vec::new();
    terminals.extend(grammar.terminals.iter().cloned());
    terminals.sort_by_key(|x| x.len());
    move |str: &'a str| Box::new(TokenIter::new(str.into(), terminals.clone()))
}

pub type GetTokeniserType = Box<dyn Fn(&str) -> TokenIteratorType>;

pub fn get_parser(
    grammar: GrammarChomsky,
    tokenizer: Option<GetTokeniserType>,
) -> Result<impl Fn(&str) -> Result<AST, Box<dyn Error>>, Box<dyn Error>> {
    let parse_table = gen_parsetable(&grammar)?;
    // println!("{:?}", parse_table);
    Ok(move |input_str: &str| {
        let mut stack =
            vec![StackObject::Nonterm(grammar.start_nonterm.clone())];
        // let mut input = str.chars().map(|x| x.to_string());
        let mut rules = vec![];
        let mut input = tokenizer.as_ref().map_or_else(
            || get_tokenizer(&grammar)(input_str),
            |t| t(input_str),
        );
        let mut i = input.next().ok_or("Not from language, empty")?;
        let mut node_stack = vec![AST::Node {
            name: Rstr::from("chomchom_root"),
            children: vec![],
        }];
        // dbg!(&stack);
        // dbg!(&grammar.start_nonterm);
        while !stack.is_empty() {
            // dbg!(stack.last());
            // dbg!(&i, &input);
            match stack.pop().ok_or("Not from language")? {
                StackObject::Nonterm(non) => {
                    // dbg!(&non);
                    let rul = parse_table
                    .get(&non)
                    .ok_or(format!("Ivalid parsetable. No rules for {}", non))?
                    .get(i.as_str().as_ref())
                    .ok_or(
                        format!("Unexpected token. Got `{}`. Expected one of {:?}. Line: {}, Pos: {}\n{}\n{}",
                            i.as_str(),
                            parse_table.get(&non).ok_or("Invalid parsetable")?.keys(), i.line(),
                            i.column(),
                            input_str, //[(i.str_pos().saturating_sub(100)).max(0)..(i.str_pos().saturating_add(100)).min(input_str.len()-1)],
                            print_arrow(i.column(), i.as_str().len()),
                        ),
                    )?;
                    // println!("Using Rule {}", rul);
                    rules.push(rul);
                    if !non.starts_with('_') {
                        stack.push(StackObject::PopNode);
                        node_stack.push(AST::Node {
                            name: non,
                            children: Vec::new(),
                        })
                    }
                    stack.extend(
                        grammar.rules[*rul]
                            .1
                            .iter()
                            .rev()
                            .cloned()
                            .map(|x| x.into()),
                    );
                }
                StackObject::Term(term) => {
                    if term.as_ref() != i.as_str().as_ref() {
                        return Err(format!(
                            "Not from language. Term '{}' not expected. Expected {}. Rest: {}, Stack:",
                            i.as_str(),
                            term,
                            input_str[i.str_pos()..].to_string(),
                        )
                        .into());
                    }
                    if let AST::Node { children, .. } =
                        node_stack.last_mut().ok_or("Empty stack?")?
                    {
                        children.push(AST::Token(i));
                    }
                    i = input.next().unwrap_or(Box::new(Token {
                        token: TokenType::EOF,
                        line: 0,
                        column: 0,
                        str_pos: 0,
                    }));
                }
                StackObject::Epsilon => {
                    dbg!("Epsilon");
                }
                StackObject::PopNode => {
                    dbg!("PopNode");
                    let n = node_stack.pop().ok_or("Empty stack?")?;
                    if let Some(AST::Node { children, .. }) =
                        node_stack.last_mut()
                    {
                        children.push(n);
                    } else {
                        panic!("Cannot pop emptry stack");
                        // node_stack.push(n);
                    }
                }
            }
        }
        if stack.is_empty() && input.next().is_some() {
            return Err("Not from language too long.".into());
        }
        // println!("{:?}", rules);
        Ok(node_stack.pop().ok_or("Invalid language.")?)
    })
}
