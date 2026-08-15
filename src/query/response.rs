use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::cache::CachedResult;
use crate::execution;
use crate::value::Value;

#[derive(Debug, Serialize)]
pub(crate) struct QueryResponse {
    pub(crate) meta: Meta,
    pub(crate) rows: Vec<JsonValue>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Meta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) columns: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) row_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rows_affected: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last_insert_id: Option<i64>,
}

pub(crate) fn build_response(result: &execution::ExecutionResult) -> QueryResponse {
    let is_row_statement = !result.columns.is_empty();
    let rows = result
        .rows
        .iter()
        .map(|row| row_to_json(&result.columns, row))
        .collect();
    let meta = Meta {
        columns: is_row_statement.then_some(result.columns.clone()),
        row_count: is_row_statement.then_some(result.rows.len() as u64),
        rows_affected: (!is_row_statement).then_some(result.rows_affected),
        last_insert_id: (!is_row_statement).then_some(result.last_insert_id),
    };
    QueryResponse { meta, rows }
}

pub(crate) fn build_cached_response(result: &CachedResult) -> QueryResponse {
    let meta = Meta {
        columns: Some(result.columns.clone()),
        row_count: Some(result.rows.len() as u64),
        rows_affected: None,
        last_insert_id: None,
    };
    QueryResponse {
        meta,
        rows: result.rows.clone(),
    }
}

pub(crate) fn row_to_json(columns: &[String], row: &[Value]) -> JsonValue {
    let mut object = serde_json::Map::new();
    for (column, value) in columns.iter().zip(row) {
        object.insert(column.clone(), JsonValue::from(value.clone()));
    }
    JsonValue::Object(object)
}
