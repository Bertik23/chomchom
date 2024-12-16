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

pub fn follow(
    input: &NT,
    grammar: &GrammarChomsky,
    first_table: &BTreeMap<String, BTreeSet<String>>,
) -> Result<HashSet<String>, Box<dyn Error>> {
    let mut f = HashSet::new();
    let mut folow = false;
    for (l, r) in grammar.rules.iter() {
        for nt in r {
            if folow {
                let fir = match nt {
                    NT::Non(n) => first_table
                        .get(n)
                        .ok_or("No first for this nonterm")?
                        .clone(),
                    NT::Term(t) => BTreeSet::from([t.clone(); 1]),
                    NT::Epsilon => unreachable!(),
                };
                if !fir.contains(&"".to_string()) {
                    folow = false;
                }
                for ff in fir {
                    f.insert(ff.to_string());
                }
            }
            if nt == input {
                folow = true;
            }
        }
        if folow && &NT::Non(l.clone()) != input {
            for ff in follow(&NT::Non(l.clone()), grammar, first_table)? {
                f.insert(ff);
            }
        }
    }
    Ok(f)
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
            let mut last = NT::Epsilon;
            for nt in r {
                if let NT::Non(n) = last {
                    changed = first(nt, first_table)?
                        .iter()
                        .map(|x| {
                            follow_table
                                .entry(n.clone())
                                .or_default()
                                .insert(x.clone())
                        })
                        .any(|x| x || changed);
                }
                last = nt.clone();
            }
            if let NT::Non(n) = last {
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
    // let mut follow_table = BTreeMap::new();
    // for nt in grammar.nonterminals.iter() {
    //     follow_table.insert(
    //         nt.clone(),
    //         follow(&NT::Non(nt.clone()), grammar, &symbol_first_table)?,
    //     );
    // }
    let follow_table = get_follow_table(grammar, &symbol_first_table)?;
    println!("Got second table.");
    let mut pt = ParseTable::new();
    for (i, (l, _)) in grammar.rules.iter().enumerate() {
        let use_follow = first_table[i].contains(&"".to_string());
        let tp_iter = first_table[i]
            .iter()
            .cloned()
            .zip(iter::repeat(i).take(first_table.len()));
        if use_follow {
            let ft =
                follow_table.get(l).ok_or(format!("No follow for {}", l))?;
            pt.entry(l.clone()).or_default().extend(
                tp_iter.chain(
                    ft.iter().cloned().zip(iter::repeat(i).take(ft.len())),
                ),
            );
        } else {
            pt.entry(l.clone()).or_default().extend(tp_iter);
        }
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

pub fn get_parser(
    grammar: GrammarChomsky,
) -> Result<impl Fn(&str) -> Result<AST, Box<dyn Error>>, Box<dyn Error>> {
    let parse_table = gen_parsetable(&grammar)?;
    println!("{:?}", parse_table);
    Ok(move |str: &str| {
        let mut stack =
            vec![StackObject::Nonterm(grammar.start_nonterm.clone())];
        let mut input = str.chars().map(|x| x.to_string());
        let mut rules = vec![];
        let mut i = input.next().ok_or("Not from language, empty")?;
        let mut node_stack = vec![AST::Node {
            name: "chomchom_root".to_string(),
            children: vec![],
        }];
        dbg!(&stack);
        dbg!(&grammar.start_nonterm);
        while !stack.is_empty() {
            match stack.pop().ok_or("Not from language")? {
                StackObject::Nonterm(non) => {
                    dbg!(&non);
                    let rul = parse_table
                    .get(&non)
                    .ok_or(format!("Ivalid parsetable. No rules for {}", non))?
                    .get(&i)
                    .ok_or(
                        format!("Not from language. Don't know which rule to use. Got {}. Expected one of {:?}",
                            i,
                            parse_table.get(&non).ok_or("Invalid parsetable")?.keys()),
                    )?;
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
                            "Not from language. Term '{}' not expected.",
                            term
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
        dbg!(&node_stack);
        println!("{:?}", rules);
        Ok(node_stack.pop().ok_or("Invalid language.")?)
    })
}
