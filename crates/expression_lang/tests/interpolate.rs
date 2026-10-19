//! Tests for interpolate behavior.

use expression_lang::{interpolate, Value, VariableTable};

fn interp(vars: &VariableTable, s: &str) -> Result<String, String> {
    interpolate(vars, s)
}

#[test]
fn empty_string() {
    let vars = VariableTable::new();
    assert_eq!(interp(&vars, ""), Ok(String::new()));
    assert_eq!(interp(&vars, "   "), Ok("   ".to_string()));
}

#[test]
fn string_without_expression() {
    let vars = VariableTable::new();
    assert_eq!(interp(&vars, " asdfasdf  sdf "), Ok(" asdfasdf  sdf ".to_string()));
}

#[test]
fn string_with_simple_expression() {
    let vars = VariableTable::new();
    assert_eq!(
        interp(&vars, " asdfasdf ${'asdf'}  sdf "),
        Ok(" asdfasdf asdf  sdf ".to_string())
    );
    assert_eq!(
        interp(&vars, " asdfasdf ${'asdf'} ${'AND'}  sdf "),
        Ok(" asdfasdf asdf AND  sdf ".to_string())
    );
    assert_eq!(
        interp(&vars, " asdfasdf ${'asdf'}${' AND'}  sdf "),
        Ok(" asdfasdf asdf AND  sdf ".to_string())
    );
    assert_eq!(interp(&vars, " ${ true } "), Ok(" true ".to_string()));
    assert_eq!(
        interp(&vars, " ${ 'this'+' and ' }${'that'} "),
        Ok(" this and that ".to_string())
    );
}

#[test]
fn string_with_nested_expression() {
    let vars = VariableTable::new();
    assert_eq!(
        interp(&vars, " asdfasdf ${ 'nested ${TEST} expression' }  sdf "),
        Ok(" asdfasdf nested ${TEST} expression  sdf ".to_string())
    );
}

#[test]
fn string_with_variable() {
    let vars: VariableTable = [("TEST", Value::string("interesting"))].into_iter().collect();
    assert_eq!(
        interp(&vars, " an ${TEST} expression"),
        Ok(" an interesting expression".to_string())
    );
}

#[test]
fn string_with_backslash_and_variable() {
    let vars: VariableTable = [("TEST", Value::string("interesting"))].into_iter().collect();
    assert_eq!(
        interp(&vars, " an \\${TEST} expression"),
        Ok(" an \\interesting expression".to_string())
    );
}

#[test]
fn string_with_unknown_variable() {
    let vars = VariableTable::new();
    assert!(interp(&vars, " an ${TEST} expression").is_err());
}

#[test]
fn string_with_unterminated_el() {
    let vars = VariableTable::new();
    assert!(interp(&vars, " an ${TEST").is_err());
    assert!(interp(&vars, " an ${TEST expression").is_err());
}
