#[derive(Debug)]
pub enum Token {
    Terminal(String),
    Nonterminal(String),
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Or,
    Equal,
    Plus,
    Semicolon,
    Asterix,
    Epsilon,
}

pub fn tokenize(mut string: &str) -> Vec<Token> {
    string = string.trim();
    let mut ts = Vec::<Token>::new();
    use Token::*;
    while !string.is_empty() {
        if let Some(r) = string.strip_prefix(';') {
            string = r.trim_start();
            ts.push(Semicolon);
        } else if let Some(r) = string.strip_prefix('ε') {
            string = r.trim_start();
            ts.push(Epsilon);
        } else if let Some(r) = string.strip_prefix('+') {
            string = r.trim_start();
            ts.push(Plus);
        } else if let Some(r) = string.strip_prefix('*') {
            string = r.trim_start();
            ts.push(Asterix);
        } else if let Some(r) = string.strip_prefix('=') {
            string = r.trim_start();
            ts.push(Equal);
        } else if let Some(r) = string.strip_prefix('|') {
            string = r.trim_start();
            ts.push(Or);
        } else if let Some(r) = string.strip_prefix('(') {
            string = r.trim_start();
            ts.push(LParen);
        } else if let Some(r) = string.strip_prefix(')') {
            string = r.trim_start();
            ts.push(RParen);
        } else if let Some(r) = string.strip_prefix('{') {
            string = r.trim_start();
            ts.push(LBrace);
        } else if let Some(r) = string.strip_prefix('}') {
            string = r.trim_start();
            ts.push(RBrace);
        } else if let Some(r) = string.strip_prefix('[') {
            string = r.trim_start();
            ts.push(LBracket);
        } else if let Some(r) = string.strip_prefix(']') {
            string = r.trim_start();
            ts.push(RBracket);
        } else if let Some(r) = string.strip_prefix('"') {
            let n = r.find('"').expect("Syntax error");
            let (t, mut r) = r.split_at(n);
            r = r.strip_prefix('"').expect("impossible");
            string = r.trim_start();
            ts.push(Terminal(t.into()));
        } else {
            let n = string
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .expect("Syntax error");
            if n == 0 {
                ts.push(Epsilon);
                string = string.trim_start();
                continue;
            }
            let (t, r) = string.split_at(n);
            string = r.trim_start();
            ts.push(Nonterminal(t.into()));
        }
    }
    ts
}
