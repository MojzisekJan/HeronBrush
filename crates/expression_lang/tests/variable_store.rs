//! Tests for variable store behavior.

use expression_lang::{stores_equal, NullVariableStore, Value, VariableStore, VariableTable};

fn to_string(store: &dyn VariableStore) -> String {
    store.to_string()
}

#[test]
fn variable_table_constructor() {
    assert_eq!(VariableTable::new().len(), 0);
    let t: VariableTable = [("a", Value::number(1.0))].into_iter().collect();
    assert_eq!(t.len(), 1);
}

#[test]
fn variable_table_clone() {
    let mut table: VariableTable = [("a", Value::number(1.0))].into_iter().collect();
    let clone = table.box_clone();

    assert_eq!(clone.len(), 1);
    assert_eq!(clone.value("a"), Value::number(1.0));

    table.set("b".into(), Value::number(2.0));
    assert_eq!(clone.len(), 1);
    assert_eq!(clone.value("b"), Value::undefined());
}

#[test]
fn variable_table_len() {
    let mut table = VariableTable::new();
    assert_eq!(table.len(), 0);
    table.set("a".into(), Value::number(1.0));
    assert_eq!(table.len(), 1);
    table.set("a".into(), Value::number(2.0));
    assert_eq!(table.len(), 1);
    table.set("b".into(), Value::number(3.0));
    assert_eq!(table.len(), 2);
}

#[test]
fn variable_table_value() {
    let table: VariableTable = [("a", Value::number(1.0))].into_iter().collect();
    assert_eq!(table.value("a"), Value::number(1.0));
    // an unknown name is undefined rather than null
    assert_eq!(table.value("b"), Value::undefined());
    assert_eq!(VariableTable::new().value("a"), Value::undefined());
}

#[test]
fn variable_table_names() {
    assert_eq!(VariableTable::new().names(), Vec::<String>::new());
    let t: VariableTable = [("b", Value::number(1.0)), ("a", Value::number(2.0))]
        .into_iter()
        .collect();
    assert_eq!(t.names(), vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn variable_table_set() {
    let mut table = VariableTable::new();
    table.set("a".into(), Value::number(1.0));
    assert_eq!(table.value("a"), Value::number(1.0));
    table.set("a".into(), Value::number(2.0));
    assert_eq!(table.value("a"), Value::number(2.0));
}

#[test]
fn null_variable_store_clone() {
    let store = NullVariableStore;
    let clone = store.box_clone();
    assert_eq!(clone.len(), 0);
    assert_eq!(clone.value("a"), Value::null());
}

#[test]
fn null_variable_store_len() {
    assert_eq!(NullVariableStore.len(), 0);
}

#[test]
fn null_variable_store_value() {
    // unlike VariableTable, an unknown name is null rather than undefined
    assert_eq!(NullVariableStore.value("a"), Value::null());
    assert_eq!(NullVariableStore.value(""), Value::null());
}

#[test]
fn null_variable_store_names() {
    assert_eq!(NullVariableStore.names(), Vec::<String>::new());
}

#[test]
fn null_variable_store_set() {
    let mut store = NullVariableStore;
    store.set("a".into(), Value::number(1.0));
    assert_eq!(store.len(), 0);
    assert_eq!(store.value("a"), Value::null());
}

#[test]
fn variable_store_formatting() {
    assert_eq!(to_string(&VariableTable::new()), "{\n}");
    assert_eq!(to_string(&NullVariableStore), "{\n}");

    let one: VariableTable = [("a", Value::number(1.0))].into_iter().collect();
    assert_eq!(to_string(&one), "{\n  a: 1}");

    let two: VariableTable = [("a", Value::number(1.0)), ("b", Value::string("x"))]
        .into_iter()
        .collect();
    assert_eq!(to_string(&two), "{\n  a: 1,   b: \"x\"}");
}

#[test]
fn variable_store_display() {
    let t: VariableTable = [("a", Value::number(1.0))].into_iter().collect();
    let s: &dyn VariableStore = &t;
    assert_eq!(format!("{s}"), "{\n  a: 1}");
}

#[test]
fn variable_store_equality() {
    let empty = VariableTable::new();
    let a1: VariableTable = [("a", Value::number(1.0))].into_iter().collect();
    let a1b: VariableTable = [("a", Value::number(1.0))].into_iter().collect();
    let a2: VariableTable = [("a", Value::number(2.0))].into_iter().collect();
    let b1: VariableTable = [("b", Value::number(1.0))].into_iter().collect();

    assert!(stores_equal(&empty, &VariableTable::new()));
    assert!(stores_equal(&a1, &a1b));
    assert!(!stores_equal(&a1, &a2));
    assert!(!stores_equal(&a1, &b1));
    assert!(!stores_equal(&a1, &empty));

    // stores are compared by their contents, not by their type
    assert!(stores_equal(&empty, &NullVariableStore));
    assert!(!stores_equal(&a1, &NullVariableStore));
}

#[test]
fn emptiness_tracks_bindings() {
    let mut table = VariableTable::new();
    assert!(table.is_empty());
    table.set("key".into(), Value::null());
    assert!(!table.is_empty());
    assert!(NullVariableStore.is_empty());
}
