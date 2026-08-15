mod handler;
mod limits;
#[cfg(test)]
mod tests;

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use sqlparser::ast::{
    FromTable, ObjectName, ObjectNamePart, SetExpr, Statement, TableFactor, TableObject,
    TableWithJoins,
};
use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;

use crate::policy;

pub(crate) use self::handler::wait_for_event;
pub(crate) use self::limits::WaiterLimits;

pub(crate) const EVENT_CHANNEL_CAPACITY: usize = 64;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ChangeEvent {
    pub(crate) database: String,
    pub(crate) table: Option<String>,
    pub(crate) at: u64,
}

pub(crate) fn table_of(sql: &str) -> Option<String> {
    let dialect = SQLiteDialect {};
    let statements = Parser::parse_sql(&dialect, &policy::normalized(sql)).ok()?;
    statement_table(statements.first()?)
}

fn statement_table(statement: &Statement) -> Option<String> {
    match statement {
        Statement::Insert(insert) => match &insert.table {
            TableObject::TableName(name) => object_name(name),
            _ => None,
        },
        Statement::Update(update) => relation_name(&update.table),
        Statement::Delete(delete) => {
            let from_mysql_tables = delete.tables.first().and_then(object_name);
            if from_mysql_tables.is_some() {
                return from_mysql_tables;
            }
            match &delete.from {
                FromTable::WithFromKeyword(tables) | FromTable::WithoutKeyword(tables) => {
                    tables.first().and_then(relation_name)
                }
            }
        }
        Statement::Query(query) => set_expr_table(&query.body),
        _ => None,
    }
}

fn set_expr_table(set_expr: &SetExpr) -> Option<String> {
    match set_expr {
        SetExpr::Insert(statement)
        | SetExpr::Update(statement)
        | SetExpr::Delete(statement)
        | SetExpr::Merge(statement) => statement_table(statement),
        _ => None,
    }
}

fn relation_name(table: &TableWithJoins) -> Option<String> {
    match &table.relation {
        TableFactor::Table { name, .. } => object_name(name),
        _ => None,
    }
}

fn object_name(name: &ObjectName) -> Option<String> {
    name.0
        .last()
        .and_then(ObjectNamePart::as_ident)
        .map(|ident| ident.value.to_owned())
}

pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
