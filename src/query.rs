mod error;
#[cfg(test)]
mod tests;

use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::app::AppState;
use crate::execution;
use crate::value::Value;

pub(crate) use self::error::QueryError;

#[derive(Debug, Deserialize)]
pub(crate) struct QueryRequest {
    sql: Option<String>,
    params: Option<Vec<JsonValue>>,
    squeal: Option<JsonValue>,
    #[serde(default = "default_database")]
    db: String,
}

fn default_database() -> String {
    "main".to_owned()
}

struct ParsedQuery {
    db: String,
    sql: String,
    params: Vec<Value>,
}

pub(crate) async fn execute_query(
    State(state): State<AppState>,
    body: Result<Json<QueryRequest>, JsonRejection>,
) -> Response {
    let request = match body {
        Ok(Json(request)) => request,
        Err(rejection) => {
            return QueryError::invalid_request(rejection.body_text()).into_response();
        }
    };

    let parsed = match parse_request(request) {
        Ok(parsed) => parsed,
        Err(error) => return error.into_response(),
    };

    let pool = match state.databases.get(&parsed.db) {
        Some(pool) => pool,
        None => return QueryError::UnknownDatabase(parsed.db).into_response(),
    };

    match execution::execute(pool, &parsed.sql, &parsed.params).await {
        Ok(result) => (StatusCode::OK, Json(build_response(result))).into_response(),
        Err(error) => QueryError::Execution(error).into_response(),
    }
}

fn parse_request(request: QueryRequest) -> Result<ParsedQuery, QueryError> {
    if request.squeal.is_some() {
        return Err(QueryError::SquealUnsupported);
    }
    let sql = match request.sql {
        Some(sql) if !sql.trim().is_empty() => sql,
        Some(_) => {
            return Err(QueryError::invalid_request(
                "sql must be a non-empty string",
            ));
        }
        None => {
            return Err(QueryError::invalid_request(
                "provide exactly one of sql or squeal",
            ));
        }
    };
    let params = request
        .params
        .unwrap_or_default()
        .into_iter()
        .map(Value::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| QueryError::invalid_request(error.to_string()))?;

    Ok(ParsedQuery {
        db: request.db,
        sql,
        params,
    })
}

fn build_response(result: execution::ExecutionResult) -> QueryResponse {
    let is_row_statement = !result.columns.is_empty();
    let rows = result
        .rows
        .iter()
        .map(|row| {
            let mut object = serde_json::Map::new();
            for (column, value) in result.columns.iter().zip(row) {
                object.insert(column.clone(), JsonValue::from(value.clone()));
            }
            JsonValue::Object(object)
        })
        .collect();

    let meta = Meta {
        columns: is_row_statement.then_some(result.columns),
        row_count: is_row_statement.then_some(result.rows.len() as u64),
        rows_affected: (!is_row_statement).then_some(result.rows_affected),
        last_insert_id: (!is_row_statement).then_some(result.last_insert_id),
    };

    QueryResponse { meta, rows }
}

#[derive(Debug, Serialize)]
struct Meta {
    #[serde(skip_serializing_if = "Option::is_none")]
    columns: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    row_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rows_affected: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_insert_id: Option<i64>,
}

#[derive(Debug, Serialize)]
struct QueryResponse {
    meta: Meta,
    rows: Vec<JsonValue>,
}
