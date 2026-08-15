mod error;
mod response;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::app::AppState;
use crate::cache;
use crate::execution;
use crate::policy;
use crate::policy::StatementClass;
use crate::squeal;
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

    let classification = match policy::classify(&parsed.sql) {
        Ok(classification) => classification,
        Err(error) => return QueryError::Policy(error).into_response(),
    };

    let pool = match state.databases.get(&parsed.db) {
        Some(pool) => pool,
        None => return QueryError::UnknownDatabase(parsed.db).into_response(),
    };

    let key = cache::build_key(&parsed.db, &parsed.sql, &parsed.params);

    if classification.cacheable {
        if let Some(cached) = state.cache.lookup(&key) {
            return (
                StatusCode::OK,
                Json(response::build_cached_response(&cached)),
            )
                .into_response();
        }
        state.cache.record_miss();
    }

    match execution::execute(pool, &parsed.sql, &parsed.params).await {
        Ok(result) => {
            if is_write(&classification.classes) {
                state.cache.invalidate_all();
            }
            if classification.cacheable
                && let Some(cached) = store_cached_result(&state, key, &result)
            {
                return (
                    StatusCode::OK,
                    Json(response::build_cached_response(&cached)),
                )
                    .into_response();
            }
            (StatusCode::OK, Json(response::build_response(&result))).into_response()
        }
        Err(error) => QueryError::Execution(error).into_response(),
    }
}

fn is_write(classes: &[StatementClass]) -> bool {
    classes
        .iter()
        .any(|class| matches!(class, StatementClass::Write))
}

fn store_cached_result(
    state: &AppState,
    key: cache::CacheKey,
    result: &execution::ExecutionResult,
) -> Option<Arc<cache::CachedResult>> {
    if result.columns.is_empty() {
        return None;
    }
    let rows = result
        .rows
        .iter()
        .map(|row| response::row_to_json(&result.columns, row))
        .collect();
    let cached = cache::CachedResult {
        columns: result.columns.clone(),
        rows,
    };
    state.cache.store(key, cached)
}

fn parse_request(request: QueryRequest) -> Result<ParsedQuery, QueryError> {
    if let Some(squeal) = request.squeal {
        if request.sql.is_some() {
            return Err(QueryError::invalid_request(
                "provide exactly one of sql or squeal",
            ));
        }
        if request.params.is_some() {
            return Err(QueryError::invalid_request(
                "params is accepted only with raw sql",
            ));
        }
        let compiled = squeal::compile(&squeal).map_err(QueryError::Squeal)?;
        return Ok(ParsedQuery {
            db: request.db,
            sql: compiled.sql,
            params: compiled.params,
        });
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
