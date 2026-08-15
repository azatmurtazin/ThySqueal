#![allow(dead_code)]

use serde_json::Value as JsonValue;
use sqlx::encode::IsNull;
use sqlx::sqlite::{SqliteArgumentsBuffer, SqliteTypeInfo};
use sqlx::{Encode, Sqlite, Type};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Text(String),
}

#[derive(Debug, Error)]
#[error("invalid parameter value: {0}")]
pub(crate) struct InvalidValue(String);

impl TryFrom<JsonValue> for Value {
    type Error = InvalidValue;

    fn try_from(json: JsonValue) -> Result<Self, Self::Error> {
        match json {
            JsonValue::Null => Ok(Self::Null),
            JsonValue::Bool(boolean) => Ok(Self::Boolean(boolean)),
            JsonValue::Number(number) => {
                if let Some(integer) = number.as_i64() {
                    Ok(Self::Integer(integer))
                } else {
                    number
                        .as_f64()
                        .map(Self::Float)
                        .ok_or_else(|| InvalidValue("number out of range".to_owned()))
                }
            }
            JsonValue::String(text) => Ok(Self::Text(text)),
            JsonValue::Array(_) | JsonValue::Object(_) => Err(InvalidValue(
                "arrays and objects cannot be bound".to_owned(),
            )),
        }
    }
}

impl From<Value> for JsonValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Boolean(boolean) => Self::Bool(boolean),
            Value::Integer(integer) => Self::Number(integer.into()),
            Value::Float(float) => serde_json::Number::from_f64(float)
                .map(Self::Number)
                .unwrap_or(Self::Null),
            Value::Text(text) => Self::String(text),
        }
    }
}

impl<'q> Encode<'q, Sqlite> for Value {
    fn encode_by_ref(
        &self,
        buf: &mut SqliteArgumentsBuffer,
    ) -> Result<IsNull, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            Self::Null => Ok(IsNull::Yes),
            Self::Boolean(boolean) => boolean.encode(buf),
            Self::Integer(integer) => integer.encode(buf),
            Self::Float(float) => float.encode(buf),
            Self::Text(text) => text.encode(buf),
        }
    }
}

impl Type<Sqlite> for Value {
    fn type_info() -> SqliteTypeInfo {
        <i64 as Type<Sqlite>>::type_info()
    }
}
