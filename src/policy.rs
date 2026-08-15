mod error;
#[cfg(test)]
mod tests;

use std::borrow::Cow;

use sqlparser::ast::{Query, SetExpr, Statement};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;

use crate::policy::error::Error;

pub(crate) use self::error::Error as PolicyError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StatementClass {
    Read,
    Write,
}

pub(crate) fn classify(sql: &str) -> Result<Vec<StatementClass>, Error> {
    if sql.trim().is_empty() {
        return Ok(Vec::new());
    }

    let dialect = SQLiteDialect {};
    let statements = Parser::parse_sql(&dialect, &normalized(sql))
        .map_err(|error| Error::invalid_syntax(error.to_string()))?;

    statements.iter().map(classify_statement).collect()
}

fn classify_statement(statement: &Statement) -> Result<StatementClass, Error> {
    match statement {
        Statement::Insert(_) | Statement::Update { .. } | Statement::Delete { .. } => {
            Ok(StatementClass::Write)
        }
        Statement::Query(query) => classify_query(query),
        _ => Err(Error::rejected(
            "statement is not permitted by the access policy",
        )),
    }
}

fn classify_query(query: &Query) -> Result<StatementClass, Error> {
    match &*query.body {
        SetExpr::Insert(_) | SetExpr::Update(_) | SetExpr::Delete(_) | SetExpr::Merge(_) => {
            Ok(StatementClass::Write)
        }
        SetExpr::Select(_)
        | SetExpr::Query(_)
        | SetExpr::SetOperation { .. }
        | SetExpr::Values(_)
        | SetExpr::Table(_) => Ok(StatementClass::Read),
    }
}

fn normalized(sql: &str) -> Cow<'_, str> {
    let leading = sql.trim_start();
    let prefix = leading.get(.."REPLACE".len()).unwrap_or_default();
    if !prefix.eq_ignore_ascii_case("REPLACE") {
        return Cow::Borrowed(sql);
    }
    let rest = &leading["REPLACE".len()..];
    let is_boundary = rest
        .chars()
        .next()
        .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
    if !is_boundary {
        return Cow::Borrowed(sql);
    }

    let start = sql.len() - leading.len();
    let mut out = String::with_capacity(sql.len() + 8);
    out.push_str(&sql[..start]);
    out.push_str("INSERT OR REPLACE");
    out.push_str(&sql[start + "REPLACE".len()..]);
    Cow::Owned(out)
}
