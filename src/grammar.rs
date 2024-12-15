use std::{
    collections::{BTreeMap, HashSet},
    fmt::Display,
    io::IsTerminal,
};

fn termcolor(r: u8, g: u8, b: u8) -> String {
    if std::io::stdout().is_terminal() {
        format!("\x1b[38;2;{};{};{}m", r, g, b)
    } else {
        "".into()
    }
}

fn termreset() -> String {
    if std::io::stdout().is_terminal() {
        "\x1b[0m".to_string()
    } else {
        "".into()
    }
}

fn num_to_alphastr(mut num: u32) -> String {
    let mut result = "".to_string();
    num += 1;

    while num > 0 {
        let remainder = (num % 26) as u8;
        result.push((b'A' + remainder - if num >= 26 { 0 } else { 1 }) as char);
        num /= 26;
        if num == 0 {
            break;
        }
    }

    result.push('_');

    result.chars().rev().collect()
}

fn capitalize_first_letter(s: &str) -> String {
    let mut c = s.chars();

    match c.next() {
        None => String::new(), // Handle empty string case
        Some(first_char) => {
            first_char.to_uppercase().collect::<String>() + c.as_str()
        }
    }
}

#[derive(Debug)]
pub enum EBNF {
    Epsilon,
    Non(String),
    Term(String),
    Iteration(Box<EBNF>),
    OneOrMore(Box<EBNF>),
    Optional(Box<EBNF>),
    Or(Vec<EBNF>),
    Concat(Vec<EBNF>),
}

#[derive(Debug)]
pub struct GrammarEBNF {
    pub start_nonterm: String,
    pub rules: BTreeMap<String, Vec<EBNF>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NT {
    Epsilon,
    Non(String),
    Term(String),
}

#[derive(Debug)]
pub struct GrammarChomsky {
    pub rules: Vec<(String, Vec<NT>)>,
    pub start_nonterm: String,
    pub nonterminals: HashSet<String>,
}

impl GrammarChomsky {
    fn from_rules(
        start_nonterm: String,
        rules: Vec<(String, Vec<NT>)>,
    ) -> Self {
        let nonterminals: HashSet<String> = rules
            .iter()
            .map(|(l, _)| l.clone())
            .chain(rules.iter().flat_map(|(_, r)| {
                r.iter().filter_map(|x| {
                    if let NT::Non(nt) = x {
                        Some(nt.clone())
                    } else {
                        None
                    }
                })
            }))
            .collect();

        for n in nonterminals.iter() {
            if !rules.iter().any(|(l, _)| l == n) {
                panic!("Invalid gramar")
            }
        }

        GrammarChomsky {
            rules,
            start_nonterm,
            nonterminals,
        }
    }
}

impl GrammarEBNF {
    pub fn to_ebnf(&self) -> String {
        let mut out = String::new();
        for rule in self.rules.iter() {
            for r in rule.1 {
                out = format!("{}\n{} = {}", out, rule.0, r.to_ebnf());
            }
        }
        out
    }
    pub fn to_chomsky(&self) -> GrammarChomsky {
        let mut new_rules = vec![];
        let mut new_nonterms = 0;
        for rule in self.rules.iter() {
            for r in rule.1.iter() {
                let nr = r.to_chomsky(&mut new_rules, &mut new_nonterms);
                new_rules.push((capitalize_first_letter(rule.0), nr))
            }
        }
        GrammarChomsky::from_rules(self.start_nonterm.clone(), new_rules)
    }
}

impl Display for GrammarChomsky {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (l, r) in self.rules.iter() {
            write!(f, "{} ->", l)?;
            for nt in r.iter() {
                write!(f, " {}", nt)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl Display for NT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NT::Epsilon => {
                write!(f, "{}ε{}", termcolor(100, 250, 100), termreset())
            }
            NT::Non(a) => {
                write!(f, "{}{}{}", termcolor(250, 100, 100), a, termreset())
            }
            NT::Term(a) => {
                write!(f, "{}{}{}", termcolor(100, 250, 100), a, termreset())
            }
        }
    }
}

impl EBNF {
    fn to_ebnf(&self) -> String {
        match self {
            EBNF::Epsilon => "ε".into(),
            EBNF::Concat(a) => {
                format!(
                    "( {} )",
                    a.iter()
                        .map(|x| x.to_ebnf())
                        .collect::<Vec<String>>()
                        .join(" ")
                )
            }
            EBNF::OneOrMore(a) => format!("( {} ) +", a.to_ebnf()),
            EBNF::Optional(a) => format!("[ {} ]", a.to_ebnf()),
            EBNF::Or(a) => {
                format!(
                    "( {} )",
                    a.iter()
                        .map(|x| x.to_ebnf())
                        .collect::<Vec<String>>()
                        .join(" | ")
                )
            }
            EBNF::Iteration(a) => format!("{{ {} }}", a.to_ebnf()),
            EBNF::Non(a) => a.to_string(),
            EBNF::Term(a) => format!("\"{}\"", a),
        }
    }
    fn to_chomsky(
        &self,
        rules: &mut Vec<(String, Vec<NT>)>,
        nonterm_counter: &mut u32,
    ) -> Vec<NT> {
        match self {
            EBNF::Epsilon => vec![NT::Epsilon],
            EBNF::Non(a) => vec![NT::Non(capitalize_first_letter(a))],
            EBNF::Term(a) => vec![NT::Term(a.clone())],
            EBNF::Concat(a) => {
                let mut v = vec![];
                for a in a {
                    v.extend(a.to_chomsky(rules, nonterm_counter));
                }
                v
            }
            EBNF::Or(a) => {
                let new_nt = num_to_alphastr(*nonterm_counter);
                *nonterm_counter += 1;
                for a in a {
                    let na = a.to_chomsky(rules, nonterm_counter);
                    rules.push((new_nt.clone(), na));
                }
                vec![NT::Non(new_nt)]
            }
            EBNF::Optional(a) => {
                let na = a.to_chomsky(rules, nonterm_counter);
                let new_nt = num_to_alphastr(*nonterm_counter);
                *nonterm_counter += 1;
                rules.push((new_nt.clone(), na));
                rules.push((new_nt.clone(), vec![NT::Epsilon]));
                vec![NT::Non(new_nt)]
            }
            EBNF::Iteration(a) => {
                let mut na = a.to_chomsky(rules, nonterm_counter);
                let new_nt = num_to_alphastr(*nonterm_counter);
                *nonterm_counter += 1;
                rules.push((new_nt.clone(), vec![NT::Epsilon]));
                na.push(NT::Non(new_nt.clone()));
                rules.push((new_nt.clone(), na));
                vec![NT::Non(new_nt)]
            }
            EBNF::OneOrMore(a) => {
                let mut na = a.to_chomsky(rules, nonterm_counter);
                let new_nt = num_to_alphastr(*nonterm_counter);
                *nonterm_counter += 1;
                let new_new_nt = num_to_alphastr(*nonterm_counter);
                *nonterm_counter += 1;
                na.push(NT::Non(new_new_nt.clone()));
                rules.push((new_nt.clone(), na.clone()));
                rules.push((new_new_nt.clone(), na));
                rules.push((new_new_nt, vec![NT::Epsilon]));
                vec![NT::Non(new_nt)]
            }
        }
    }
}
