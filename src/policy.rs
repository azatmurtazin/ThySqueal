mod error;
#[cfg(test)]
mod tests;

use logos::Logos;

use crate::policy::error::Error;

pub(crate) use self::error::Error as PolicyError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StatementClass {
    Read,
    Write,
}

#[derive(Logos, Debug, PartialEq)]
enum Token {
    #[token(";")]
    Semicolon,
    #[token("(")]
    OpenParen,
    #[token(")")]
    CloseParen,
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", |lex| lex.slice().to_ascii_lowercase())]
    Word(String),
    #[regex(r"[ \t\r\n\f\v]+", logos::skip)]
    #[regex(r"--[^\n]*?", logos::skip)]
    #[regex(r"/\*[^\*/]*(\*[^\*/]*)*\*/", logos::skip)]
    #[regex(r"'([^']|'')*'", logos::skip)]
    #[regex(r#""([^"]|"")*""#, logos::skip)]
    #[regex(r"`([^`]|``)*`", logos::skip)]
    #[regex(r"\[[^\]]*\]", logos::skip)]
    #[regex(r"[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?", logos::skip)]
    Noise,
}

pub(crate) fn classify(sql: &str) -> Result<Vec<StatementClass>, Error> {
    let mut classes = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut depth = 0usize;

    for token in Token::lexer(sql) {
        match token {
            Ok(Token::Semicolon) => {
                if !words.is_empty() {
                    classes.push(classify_words(&words)?);
                    words.clear();
                }
            }
            Ok(Token::OpenParen) => depth += 1,
            Ok(Token::CloseParen) => depth = depth.saturating_sub(1),
            Ok(Token::Word(word)) if depth == 0 => words.push(word),
            Ok(_) => {}
            Err(_) => {}
        }
    }

    if !words.is_empty() {
        classes.push(classify_words(&words)?);
    }

    Ok(classes)
}

fn classify_words(words: &[String]) -> Result<StatementClass, Error> {
    match words.first().map(String::as_str) {
        Some("select") => Ok(StatementClass::Read),
        Some("insert" | "update" | "delete" | "replace") => Ok(StatementClass::Write),
        Some("with") => classify_with(words),
        Some(operation) => Err(Error::rejected(format!(
            "'{operation}' statements are not permitted by the access policy"
        ))),
        None => Err(Error::rejected("statement has no operation")),
    }
}

fn classify_with(words: &[String]) -> Result<StatementClass, Error> {
    words
        .iter()
        .skip(1)
        .find(|word| {
            matches!(
                word.as_str(),
                "select" | "insert" | "update" | "delete" | "replace"
            )
        })
        .map(|word| {
            if word == "select" {
                StatementClass::Read
            } else {
                StatementClass::Write
            }
        })
        .ok_or_else(|| Error::rejected("'with' statement has no recognized data operation"))
}
