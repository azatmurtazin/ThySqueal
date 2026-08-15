mod error;
#[cfg(test)]
mod tests;

use std::borrow::Cow;
use std::ops::ControlFlow;

use sqlparser::ast::{Expr, ObjectNamePart, Query, SetExpr, Statement, Visit, Visitor};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;

use crate::policy::error::Error;

pub(crate) use self::error::Error as PolicyError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StatementClass {
    Read,
    Write,
}

pub(crate) struct Classification {
    pub(crate) classes: Vec<StatementClass>,
    pub(crate) cacheable: bool,
}

pub(crate) fn classify(sql: &str) -> Result<Classification, Error> {
    if sql.trim().is_empty() {
        return Ok(Classification {
            classes: Vec::new(),
            cacheable: false,
        });
    }

    let dialect = SQLiteDialect {};
    let statements = Parser::parse_sql(&dialect, &normalized(sql))
        .map_err(|error| Error::invalid_syntax(error.to_string()))?;

    let classes = statements
        .iter()
        .map(classify_statement)
        .collect::<Result<Vec<_>, _>>()?;
    let cacheable = matches!(classes.as_slice(), [StatementClass::Read])
        && is_deterministic_query(&statements[0]);

    Ok(Classification { classes, cacheable })
}

fn is_deterministic_query(statement: &Statement) -> bool {
    match statement {
        Statement::Query(query) => {
            let mut visitor = FunctionVisitor {
                nondeterministic: false,
            };
            let _ = query.visit(&mut visitor);
            !visitor.nondeterministic
        }
        _ => false,
    }
}

const NON_DETERMINISTIC_FUNCTIONS: &[&str] = &[
    "changes",
    "current_date",
    "current_time",
    "current_timestamp",
    "date",
    "datetime",
    "julianday",
    "last_insert_rowid",
    "localtime",
    "localtimestamp",
    "random",
    "randomblob",
    "strftime",
    "time",
    "total_changes",
    "unixepoch",
];

struct FunctionVisitor {
    nondeterministic: bool,
}

impl Visitor for FunctionVisitor {
    type Break = ();

    fn post_visit_expr(&mut self, expr: &Expr) -> ControlFlow<Self::Break> {
        if let Expr::Function(function) = expr {
            let name = function
                .name
                .0
                .first()
                .and_then(ObjectNamePart::as_ident)
                .map(|ident| ident.value.to_ascii_lowercase());
            if name.is_some_and(|name| NON_DETERMINISTIC_FUNCTIONS.contains(&name.as_str())) {
                self.nondeterministic = true;
            }
        }
        ControlFlow::Continue(())
    }
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
