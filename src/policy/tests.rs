use super::{StatementClass, classify};

fn classes(sql: &str) -> Vec<StatementClass> {
    classify(sql).expect("classification")
}

#[test]
fn classifies_select_as_read() {
    assert_eq!(classes("SELECT 1"), vec![StatementClass::Read]);
}

#[test]
fn classifies_parameterized_select_as_read() {
    assert_eq!(
        classes("SELECT id FROM items WHERE price > ? AND name = 'widget'"),
        vec![StatementClass::Read]
    );
}

#[test]
fn classifies_data_changes_as_write() {
    assert_eq!(
        classes("INSERT INTO items (name) VALUES ('widget')"),
        vec![StatementClass::Write]
    );
    assert_eq!(
        classes("UPDATE items SET price = 1.0 WHERE id = 1"),
        vec![StatementClass::Write]
    );
    assert_eq!(
        classes("DELETE FROM items WHERE id = 1"),
        vec![StatementClass::Write]
    );
    assert_eq!(
        classes("REPLACE INTO items (id, name) VALUES (1, 'widget')"),
        vec![StatementClass::Write]
    );
}

#[test]
fn classifies_with_select_as_read() {
    assert_eq!(
        classes("WITH top AS (SELECT * FROM items ORDER BY price DESC LIMIT 1) SELECT * FROM top"),
        vec![StatementClass::Read]
    );
}

#[test]
fn classifies_with_write_as_write() {
    assert_eq!(
        classes("WITH top AS (SELECT * FROM items LIMIT 1) INSERT INTO archive SELECT * FROM top"),
        vec![StatementClass::Write]
    );
}

#[test]
fn splits_multiple_statements() {
    assert_eq!(
        classes("SELECT 1; INSERT INTO items (name) VALUES ('x'); SELECT 2"),
        vec![
            StatementClass::Read,
            StatementClass::Write,
            StatementClass::Read
        ]
    );
}

#[test]
fn ignores_semicolons_inside_strings_and_comments() {
    assert_eq!(
        classes("SELECT 'a;b' AS label -- ; not a boundary\n/* ; still not */ ; SELECT 2"),
        vec![StatementClass::Read, StatementClass::Read]
    );
}

#[test]
fn ignores_keywords_inside_strings() {
    assert_eq!(
        classes("SELECT 'drop table' AS message"),
        vec![StatementClass::Read]
    );
}

#[test]
fn is_case_insensitive() {
    assert_eq!(classes("select 1"), vec![StatementClass::Read]);
    assert_eq!(classes("Insert"), vec![StatementClass::Write]);
}

#[test]
fn ignores_keywords_inside_subqueries() {
    assert_eq!(
        classes("SELECT * FROM items WHERE id IN (SELECT id FROM archive)"),
        vec![StatementClass::Read]
    );
}

#[test]
fn rejects_prohibited_statements() {
    for sql in [
        "PRAGMA journal_mode=WAL",
        "DROP TABLE items",
        "ALTER TABLE items ADD COLUMN note TEXT",
        "CREATE TABLE notes (id INTEGER PRIMARY KEY)",
        "BEGIN",
        "COMMIT",
        "ROLLBACK",
        "ATTACH DATABASE 'other.db' AS other",
        "DETACH DATABASE other",
        "VACUUM",
        "EXPLAIN SELECT 1",
        "GRANT ALL ON items TO analyst",
        "INSERT INTO items (name) VALUES ('x'); DROP TABLE items",
    ] {
        assert!(classify(sql).is_err(), "expected rejection for {sql:?}");
    }
}

#[test]
fn accepts_empty_and_comment_only_input() {
    assert_eq!(classify("").unwrap(), vec![]);
    assert_eq!(classify("   ").unwrap(), vec![]);
    assert_eq!(classify("-- just a comment").unwrap(), vec![]);
}
