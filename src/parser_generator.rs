use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    error::Error,
    iter,
};

use crate::grammar::{self, GrammarChomsky, GrammarEBNF, NT};

fn first(
    input: &NT,
    first_table: &BTreeMap<String, BTreeSet<String>>,
) -> Result<BTreeSet<String>, Box<dyn Error>> {
    Ok(match input {
        NT::Term(f) => BTreeSet::from([f.clone(); 1]),
        NT::Epsilon => BTreeSet::from(["".to_string(); 1]),
        NT::Non(n) => first_table
            .get(n)
            .ok_or("No first for nonterminal")?
            .clone(),
    })
}

fn get_first_table(
    grammar: &GrammarChomsky,
) -> Result<
    (Vec<BTreeSet<String>>, BTreeMap<String, BTreeSet<String>>),
    Box<dyn Error>,
> {
    let mut table = Vec::new();
    table.resize(grammar.rules.len(), BTreeSet::new());
    let mut symbol_table: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
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
                                .is_subset(&BTreeSet::from(["".to_string()]))
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
    first_table: &BTreeMap<String, BTreeSet<String>>,
) -> Result<BTreeMap<String, BTreeSet<String>>, Box<dyn Error>> {
    let mut follow_table: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    follow_table.insert(
        grammar.start_nonterm.clone(),
        BTreeSet::from(["".to_string(); 1]),
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

type ParseTable = BTreeMap<String, BTreeMap<String, usize>>;

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
        let use_follow = first_table[i].contains(&"".to_string());
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
    Node { name: String, children: Vec<AST> },
    Token(String),
}

#[derive(Debug)]
enum StackObject {
    Term(String),
    Nonterm(String),
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

enum Token<'a> {
    EOF,
    String(&'a String),
}

struct TokenIter<'a> {
    str: &'a str,
    curr: String,
    terminals: Vec<String>,
    pos: usize,
    str_pos: usize,
    line: usize,
    input_str: &'a str,
}

impl<'a> TokenIter<'a> {
    fn new(string: &str, terminals: Vec<String>) -> TokenIter {
        TokenIter {
            str: string,
            curr: "".to_string(),
            terminals,
            pos: 0,
            str_pos: 0,
            line: 0,
            input_str: string,
        }
    }
}

impl<'a> Iterator for TokenIter<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        for term in self.terminals.iter().rev() {
            if term.is_empty() {
                continue;
            }
            if let Some(r) = self.str.strip_prefix(term.as_str()) {
                self.str = r;
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
                return Some(term.clone());
            }
        }
        None
    }
}

fn print_arrow(pos: usize, len: usize) -> String {
    " ".repeat(pos) + &"^".repeat(len.max(1))
}

fn get_tokenizer(grammar: &GrammarChomsky) -> impl Fn(&str) -> TokenIter {
    let mut terminals = Vec::new();
    terminals.extend(grammar.terminals.iter().cloned());
    terminals.sort_by_key(|x| x.len());
    move |str: &str| TokenIter::new(str, terminals.clone())
}

pub fn get_parser(
    grammar: GrammarChomsky,
) -> Result<impl Fn(&str) -> Result<AST, Box<dyn Error>>, Box<dyn Error>> {
    let parse_table = gen_parsetable(&grammar)?;
    // println!("{:?}", parse_table);
    Ok(move |str: &str| {
        let mut stack =
            vec![StackObject::Nonterm(grammar.start_nonterm.clone())];
        // let mut input = str.chars().map(|x| x.to_string());
        let mut rules = vec![];
        let mut input = get_tokenizer(&grammar)(str);
        let mut i = input.next().ok_or("Not from language, empty")?;
        let mut node_stack = vec![AST::Node {
            name: "chomchom_root".to_string(),
            children: vec![],
        }];
        // dbg!(&stack);
        // dbg!(&grammar.start_nonterm);
        while !stack.is_empty() {
            match stack.pop().ok_or("Not from language")? {
                StackObject::Nonterm(non) => {
                    // dbg!(&non);
                    let rul = parse_table
                    .get(&non)
                    .ok_or(format!("Ivalid parsetable. No rules for {}", non))?
                    .get(&i)
                    .ok_or(
                        format!("Unexpected token. Got {}. Expected one of {:?}. Line: {}, Pos: {}\n{}\n{}",
                            if !i.is_empty() {i.as_str()} else {"EOF"},
                            parse_table.get(&non).ok_or("Invalid parsetable")?.keys(), input.line,
                            input.pos,
                            &input.input_str[(input.str_pos.saturating_sub(100)).max(0)..(input.str_pos.saturating_add(100)).min(input.input_str.len()-1)],
                            print_arrow(input.pos, i.len()),
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
                    if i != *term {
                        return Err(format!(
                            "Not from language. Term '{}' not expected. Expected {}. Rest: {}, Stack: {:?}",
                            i,
                            term,
                            input.str,
                            stack,
                        )
                        .into());
                    }
                    if let AST::Node { children, .. } =
                        node_stack.last_mut().ok_or("Empty stack?")?
                    {
                        children.push(AST::Token(term));
                    }
                    i = input.next().unwrap_or("".into());
                }
                StackObject::Epsilon => {}
                StackObject::PopNode => {
                    let n = node_stack.pop().ok_or("Empty stack?")?;
                    if let Some(AST::Node { children, .. }) =
                        node_stack.last_mut()
                    {
                        children.push(n);
                    } else {
                        panic!("aaa");
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
