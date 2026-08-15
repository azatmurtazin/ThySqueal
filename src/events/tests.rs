use super::table_of;

#[test]
fn insert_extracts_target_table() {
    assert_eq!(
        table_of("INSERT INTO items (name) VALUES ('x')"),
        Some("items".to_owned())
    );
}

#[test]
fn update_extracts_target_table() {
    assert_eq!(
        table_of("UPDATE items SET name = 'y' WHERE id = 1"),
        Some("items".to_owned())
    );
}

#[test]
fn delete_extracts_target_table() {
    assert_eq!(
        table_of("DELETE FROM items WHERE id = 1"),
        Some("items".to_owned())
    );
}

#[test]
fn replace_extracts_target_table() {
    assert_eq!(
        table_of("REPLACE INTO items (id) VALUES (1)"),
        Some("items".to_owned())
    );
}

#[test]
fn insert_from_select_extracts_target_table() {
    assert_eq!(
        table_of("INSERT INTO archive SELECT * FROM items"),
        Some("archive".to_owned())
    );
}

#[test]
fn qualified_table_keeps_last_part() {
    assert_eq!(
        table_of("UPDATE main.items SET name = 'y'"),
        Some("items".to_owned())
    );
}

#[test]
fn reads_and_unknown_statements_have_no_table() {
    assert_eq!(table_of("SELECT 1"), None);
    assert_eq!(table_of("PRAGMA integrity_check"), None);
    assert_eq!(table_of(""), None);
}
