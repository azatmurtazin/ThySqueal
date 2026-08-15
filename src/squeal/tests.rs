use serde_json::json;

use super::{CompiledQuery, compile};

fn compiled(input: serde_json::Value) -> CompiledQuery {
    compile(&input).expect("compile squeal")
}

#[test]
fn compiles_star_select() {
    let compiled = compiled(json!({ "_": "select", "from": "posts", "cols": ["*"] }));
    assert_eq!(compiled.sql, "SELECT * FROM posts");
    assert!(compiled.params.is_empty());
}

#[test]
fn compiles_column_select() {
    let compiled = compiled(json!({ "_": "select", "from": "posts", "cols": ["id", "title"] }));
    assert_eq!(compiled.sql, "SELECT id, title FROM posts");
}

#[test]
fn rejects_unknown_operation() {
    let result = compile(&json!({ "_": "insert", "from": "posts" }));
    assert!(result.is_err());
}

#[test]
fn rejects_missing_operation() {
    let result = compile(&json!({ "from": "posts", "cols": ["*"] }));
    assert!(result.is_err());
}

#[test]
fn rejects_non_object_squeal() {
    let result = compile(&json!("select"));
    assert!(result.is_err());
}

#[test]
fn rejects_missing_from() {
    let result = compile(&json!({ "_": "select", "cols": ["*"] }));
    assert!(result.is_err());
}

#[test]
fn rejects_missing_cols() {
    let result = compile(&json!({ "_": "select", "from": "posts" }));
    assert!(result.is_err());
}

#[test]
fn rejects_empty_cols() {
    let result = compile(&json!({ "_": "select", "from": "posts", "cols": [] }));
    assert!(result.is_err());
}

#[test]
fn rejects_mixed_star_and_columns() {
    let result = compile(&json!({ "_": "select", "from": "posts", "cols": ["*", "id"] }));
    assert!(result.is_err());
}

#[test]
fn rejects_non_string_column() {
    let result = compile(&json!({ "_": "select", "from": "posts", "cols": [1] }));
    assert!(result.is_err());
}

#[test]
fn rejects_unsupported_field() {
    let result = compile(&json!({
        "_": "select",
        "from": "posts",
        "cols": ["*"],
        "where": "id = 1"
    }));
    assert!(result.is_err());
}

#[test]
fn rejects_invalid_identifiers() {
    for invalid in ["posts; DROP TABLE items", "1abc", "a b", "a-b", ""] {
        let table = compile(&json!({ "_": "select", "from": invalid, "cols": ["*"] }));
        assert!(table.is_err(), "table identifier '{invalid}'");
        let column = compile(&json!({ "_": "select", "from": "posts", "cols": [invalid] }));
        assert!(column.is_err(), "column identifier '{invalid}'");
    }
}

#[test]
fn accepts_underscore_identifiers() {
    let compiled = compiled(json!({ "_": "select", "from": "_posts", "cols": ["_id"] }));
    assert_eq!(compiled.sql, "SELECT _id FROM _posts");
}
