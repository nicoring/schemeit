use std::collections::VecDeque;

#[derive(Debug)]
pub enum Token {
    LeftParanthesis,
    RightParanthesis,
    Int(i128),
    Float(f64),
    String(String),
    Symbol(String),
}

pub fn tokenize(code: &str) -> VecDeque<Token> {
    code.replace("(", " ( ")
        .replace(")", " ) ")
        .split_whitespace()
        .map(|token| match token {
            "(" => Token::LeftParanthesis,
            ")" => Token::RightParanthesis,
            _ => {
                if let Ok(int) = token.parse::<i128>() {
                    Token::Int(int)
                } else if let Ok(float) = token.parse::<f64>() {
                    Token::Float(float)
                } else if token.starts_with("\"") & token.ends_with("\"") {
                    Token::String(token.to_string())
                } else {
                    Token::Symbol(token.to_string())
                }
            }
        })
        .collect()
}
