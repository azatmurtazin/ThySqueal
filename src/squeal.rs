mod error;
#[cfg(test)]
mod tests;

use serde_json::{Map, Value as JsonValue};

use crate::squeal::error::Error;
use crate::value::Value;

pub(crate) use self::error::Error as SquealError;

pub(crate) struct CompiledQuery {
    pub(crate) sql: String,
    pub(crate) params: Vec<Value>,
}

pub(crate) fn compile(squeal: &JsonValue) -> Result<CompiledQuery, Error> {
    let object = squeal
        .as_object()
        .ok_or_else(|| Error::invalid("squeal must be a JSON object"))?;
    let operation = object
        .get("_")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| Error::invalid("squeal field '_' must be a string"))?;

    match operation {
        "select" => compile_select(object),
        other => Err(Error::invalid(format!(
            "unsupported squeal operation '{other}'"
        ))),
    }
}

fn compile_select(squeal: &Map<String, JsonValue>) -> Result<CompiledQuery, Error> {
    reject_unknown_fields(squeal, &["_", "from", "cols"])?;

    let table = squeal
        .get("from")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| Error::invalid("squeal field 'from' must be a table identifier"))?;
    validate_identifier(table)?;

    let columns = squeal
        .get("cols")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| Error::invalid("squeal field 'cols' must be an array"))?;
    if columns.is_empty() {
        return Err(Error::invalid("squeal field 'cols' must not be empty"));
    }

    let mut compiled_columns = Vec::with_capacity(columns.len());
    for column in columns {
        let name = column
            .as_str()
            .ok_or_else(|| Error::invalid("each squeal column must be an identifier or '*'"))?;
        if name == "*" {
            if columns.len() != 1 {
                return Err(Error::invalid("'*' cannot be combined with other columns"));
            }
            compiled_columns.push("*".to_owned());
        } else {
            validate_identifier(name)?;
            compiled_columns.push(name.to_owned());
        }
    }

    let sql = format!("SELECT {} FROM {}", compiled_columns.join(", "), table);
    Ok(CompiledQuery {
        sql,
        params: Vec::new(),
    })
}

fn reject_unknown_fields(squeal: &Map<String, JsonValue>, allowed: &[&str]) -> Result<(), Error> {
    for field in squeal.keys() {
        if !allowed.contains(&field.as_str()) {
            return Err(Error::invalid(format!(
                "unsupported squeal field '{field}'"
            )));
        }
    }
    Ok(())
}

fn validate_identifier(identifier: &str) -> Result<(), Error> {
    let mut chars = identifier.chars();
    let valid_start = chars
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_');
    if !valid_start || !chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(Error::invalid(format!("invalid identifier '{identifier}'")));
    }
    Ok(())
}
