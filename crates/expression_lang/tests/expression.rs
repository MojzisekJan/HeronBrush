#![allow(
    clippy::identity_op,
    clippy::erasing_op,
    clippy::precedence,
    clippy::eq_op,
    clippy::double_parens,
    clippy::type_complexity
)]
//! Expression parsing and evaluation tests.

mod common;

use common::*;
use expression_lang::types::{BoundedRange, Range};
use expression_lang::{parse_expression, EvaluationContext, ParseMode, Value};

fn range_var() -> Value {
    Value::range(Range::Bounded(BoundedRange { first: 1, last: 3 }))
}

#[test]
fn value_literals() {
    let cases: Vec<(&str, Value)> = vec![
        ("true", Value::boolean(true)),
        ("false", Value::boolean(false)),
        ("'asdf'", Value::string("asdf")),
        ("2", Value::integer(2)),
        ("-2", Value::integer(-2)),
        (
            "[2, 3]",
            Value::array(vec![Value::integer(2), Value::integer(3)]),
        ),
        (
            "{k1:2, k2:3}",
            v_map(vec![("k1", Value::integer(2)), ("k2", Value::integer(3))]),
        ),
    ];
    for (expr, expected) in cases {
        assert_eq!(evaluate(expr), Ok(expected), "{expr}");
    }
}

#[test]
fn variables() {
    assert_eq!(evaluate_with("x", &[("x", Value::boolean(true))]), ok(true));
    assert_eq!(
        evaluate_with("x", &[("y", Value::boolean(true))]),
        Ok(Value::undefined())
    );
    assert_eq!(evaluate_with("x", &[("x", Value::integer(7))]), ok(7_i64));
    assert_eq!(evaluate_with("x", &[]), Ok(Value::undefined()));
}

#[test]
fn arrays() {
    assert_eq!(evaluate("[]"), ok(Value::array(vec![])));
    assert_eq!(
        evaluate("[1, 2, 3]"),
        ok(Value::array(vec![
            Value::integer(1),
            Value::integer(2),
            Value::integer(3)
        ]))
    );
    assert_eq!(
        evaluate_with("[1, 2, x]", &[("x", Value::string("test"))]),
        ok(Value::array(vec![
            Value::integer(1),
            Value::integer(2),
            Value::string("test")
        ]))
    );
}

#[test]
fn maps() {
    assert_eq!(
        evaluate("{}"),
        ok(Value::map(std::collections::BTreeMap::new()))
    );
    assert_eq!(
        evaluate("{k: true}"),
        ok(v_map(vec![("k", Value::boolean(true))]))
    );
    assert_eq!(
        evaluate("{k1: true, k2: 3, k3: 3 + 7}"),
        ok(v_map(vec![
            ("k1", Value::boolean(true)),
            ("k2", Value::integer(3)),
            ("k3", Value::integer(10)),
        ]))
    );
    assert_eq!(
        evaluate_with("{k1: 'asdf', k2: x}", &[("x", Value::integer(55))]),
        ok(v_map(vec![
            ("k1", Value::string("asdf")),
            ("k2", Value::integer(55))
        ]))
    );
}

#[test]
fn operators() {
    let cases: Vec<(&str, Result<Value, String>)> = vec![
        ("+true", ok(1_i64)),
        ("+false", ok(0_i64)),
        ("+1", ok(1_i64)),
        (
            "+'test'",
            err(
                r##"At line 1, column 1: Cannot evaluate expression '+"test"': Invalid type String"##,
            ),
        ),
        ("+'2'", ok(2_i64)),
        (
            "+null",
            err(r##"At line 1, column 1: Cannot evaluate expression '+null': Invalid type Null"##),
        ),
        (
            "+[]",
            err(r##"At line 1, column 1: Cannot evaluate expression '+[]': Invalid type Array"##),
        ),
        (
            "+{}",
            err(r##"At line 1, column 1: Cannot evaluate expression '+{}': Invalid type Map"##),
        ),
        ("-true", ok(-1_i64)),
        ("-false", ok(0_i64)),
        ("-1", ok(-1_i64)),
        ("-'2'", ok(-2_i64)),
        (
            "-'test'",
            err(
                r##"At line 1, column 1: Cannot evaluate expression '-"test"': Invalid type String"##,
            ),
        ),
        (
            "-null",
            err(r##"At line 1, column 1: Cannot evaluate expression '-null': Invalid type Null"##),
        ),
        (
            "-[]",
            err(r##"At line 1, column 1: Cannot evaluate expression '-[]': Invalid type Array"##),
        ),
        (
            "-{}",
            err(r##"At line 1, column 1: Cannot evaluate expression '-{}': Invalid type Map"##),
        ),
        ("true + true", ok(2_i64)),
        ("false + 3", ok(3_i64)),
        (
            "true + 'test'",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true + "test"': Invalid operand types Boolean and String"##,
            ),
        ),
        ("true + '1.23'", ok(Value::number(2.23))),
        (
            "true + null",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true + null': Invalid operand types Boolean and Null"##,
            ),
        ),
        (
            "true + []",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true + []': Invalid operand types Boolean and Array"##,
            ),
        ),
        (
            "true + {}",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true + {}': Invalid operand types Boolean and Map"##,
            ),
        ),
        ("1 + true", ok(2_i64)),
        ("3 + -1", ok(2_i64)),
        ("1 + '1.23'", ok(Value::number(2.23))),
        (
            "1 + 'test'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 + "test"': Invalid operand types Number and String"##,
            ),
        ),
        (
            "1 + null",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 + null': Invalid operand types Number and Null"##,
            ),
        ),
        (
            "1 + []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 + []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "1 + {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 + {}': Invalid operand types Number and Map"##,
            ),
        ),
        (
            "'test' + true",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" + true': Invalid operand types String and Boolean"##,
            ),
        ),
        (
            "'test' + 2",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" + 2': Invalid operand types String and Number"##,
            ),
        ),
        ("'1.23' + 2", ok(Value::number(3.23))),
        ("'this' + 'test'", ok(Value::string("thistest"))),
        ("'this' + '1.23'", ok(Value::string("this1.23"))),
        (
            "'test' + null",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" + null': Invalid operand types String and Null"##,
            ),
        ),
        (
            "'test' + []",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" + []': Invalid operand types String and Array"##,
            ),
        ),
        (
            "'test' + {}",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" + {}': Invalid operand types String and Map"##,
            ),
        ),
        (
            "null + true",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null + true': Invalid operand types Null and Boolean"##,
            ),
        ),
        (
            "null + 2",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null + 2': Invalid operand types Null and Number"##,
            ),
        ),
        (
            "null + 'test'",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null + "test"': Invalid operand types Null and String"##,
            ),
        ),
        (
            "null + null",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null + null': Invalid operand types Null and Null"##,
            ),
        ),
        (
            "null + []",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null + []': Invalid operand types Null and Array"##,
            ),
        ),
        (
            "null + {}",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null + {}': Invalid operand types Null and Map"##,
            ),
        ),
        (
            "[] + true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] + true': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "[] + 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] + 2': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "[] + 'test'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] + "test"': Invalid operand types Array and String"##,
            ),
        ),
        (
            "[] + null",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] + null': Invalid operand types Array and Null"##,
            ),
        ),
        (
            "[1, 2] + [2, 3]",
            ok(Value::array(vec![
                Value::integer(1),
                Value::integer(2),
                Value::integer(2),
                Value::integer(3),
            ])),
        ),
        (
            "[] + {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] + {}': Invalid operand types Array and Map"##,
            ),
        ),
        (
            "{} + true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} + true': Invalid operand types Map and Boolean"##,
            ),
        ),
        (
            "{} + 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} + 2': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "{} + 'test'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} + "test"': Invalid operand types Map and String"##,
            ),
        ),
        (
            "{} + null",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} + null': Invalid operand types Map and Null"##,
            ),
        ),
        (
            "{} + []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} + []': Invalid operand types Map and Array"##,
            ),
        ),
        (
            "{k1: 1, k2: 2, k3: 3} + {k3: 4, k4: 5}",
            ok(v_map(vec![
                ("k1", Value::integer(1)),
                ("k2", Value::integer(2)),
                ("k3", Value::integer(4)),
                ("k4", Value::integer(5)),
            ])),
        ),
        ("true - true", ok(0_i64)),
        ("false - 3", ok(-3_i64)),
        (
            "true - 'test'",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true - "test"': Invalid operand types Boolean and String"##,
            ),
        ),
        ("true - '2.0'", ok(Value::number(-1.0))),
        (
            "true - null",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true - null': Invalid operand types Boolean and Null"##,
            ),
        ),
        (
            "true - []",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true - []': Invalid operand types Boolean and Array"##,
            ),
        ),
        (
            "true - {}",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true - {}': Invalid operand types Boolean and Map"##,
            ),
        ),
        ("1 - true", ok(0_i64)),
        ("3 - 1", ok(2_i64)),
        (
            "1 - 'test'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 - "test"': Invalid operand types Number and String"##,
            ),
        ),
        ("1 - '2.23'", ok(Value::number(-1.23))),
        (
            "1 - null",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 - null': Invalid operand types Number and Null"##,
            ),
        ),
        (
            "1 - []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 - []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "1 - {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 - {}': Invalid operand types Number and Map"##,
            ),
        ),
        (
            "'test' - true",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" - true': Invalid operand types String and Boolean"##,
            ),
        ),
        (
            "'test' - 2",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" - 2': Invalid operand types String and Number"##,
            ),
        ),
        ("'3.23' - 2", ok(Value::number(1.23))),
        (
            "'this' - 'test'",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"this" - "test"': Invalid operand types String and String"##,
            ),
        ),
        (
            "'test' - null",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" - null': Invalid operand types String and Null"##,
            ),
        ),
        (
            "'test' - []",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" - []': Invalid operand types String and Array"##,
            ),
        ),
        (
            "'test' - {}",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" - {}': Invalid operand types String and Map"##,
            ),
        ),
        (
            "null - true",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null - true': Invalid operand types Null and Boolean"##,
            ),
        ),
        (
            "null - 2",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null - 2': Invalid operand types Null and Number"##,
            ),
        ),
        (
            "null - 'test'",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null - "test"': Invalid operand types Null and String"##,
            ),
        ),
        (
            "null - null",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null - null': Invalid operand types Null and Null"##,
            ),
        ),
        (
            "null - []",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null - []': Invalid operand types Null and Array"##,
            ),
        ),
        (
            "null - {}",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null - {}': Invalid operand types Null and Map"##,
            ),
        ),
        (
            "[] - true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] - true': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "[] - 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] - 2': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "[] - 'test'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] - "test"': Invalid operand types Array and String"##,
            ),
        ),
        (
            "[] - null",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] - null': Invalid operand types Array and Null"##,
            ),
        ),
        (
            "[] - []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] - []': Invalid operand types Array and Array"##,
            ),
        ),
        (
            "[] - {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] - {}': Invalid operand types Array and Map"##,
            ),
        ),
        (
            "{} - true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} - true': Invalid operand types Map and Boolean"##,
            ),
        ),
        (
            "{} - 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} - 2': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "{} - 'test'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} - "test"': Invalid operand types Map and String"##,
            ),
        ),
        (
            "{} - null",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} - null': Invalid operand types Map and Null"##,
            ),
        ),
        (
            "{} - []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} - []': Invalid operand types Map and Array"##,
            ),
        ),
        (
            "{} - {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} - {}': Invalid operand types Map and Map"##,
            ),
        ),
        ("true * true", ok(1_i64)),
        ("true * false", ok(0_i64)),
        ("true * 3", ok(3_i64)),
        ("true * '3'", ok(3_i64)),
        (
            "true * 'test'",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true * "test"': Invalid operand types Boolean and String"##,
            ),
        ),
        (
            "true * null",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true * null': Invalid operand types Boolean and Null"##,
            ),
        ),
        (
            "true * []",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true * []': Invalid operand types Boolean and Array"##,
            ),
        ),
        (
            "true * {}",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true * {}': Invalid operand types Boolean and Map"##,
            ),
        ),
        ("1 * true", ok(1_i64)),
        ("3 * 2", ok(6_i64)),
        (
            "1 * 'test'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 * "test"': Invalid operand types Number and String"##,
            ),
        ),
        ("2 * '2.23'", ok(Value::number(4.46))),
        (
            "1 * null",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 * null': Invalid operand types Number and Null"##,
            ),
        ),
        (
            "1 * []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 * []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "1 * {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 * {}': Invalid operand types Number and Map"##,
            ),
        ),
        (
            "'test' * true",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" * true': Invalid operand types String and Boolean"##,
            ),
        ),
        (
            "'test' * 2",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" * 2': Invalid operand types String and Number"##,
            ),
        ),
        ("'1.23' * 2", ok(Value::number(2.46))),
        (
            "'this' * 'test'",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"this" * "test"': Invalid operand types String and String"##,
            ),
        ),
        (
            "'test' * null",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" * null': Invalid operand types String and Null"##,
            ),
        ),
        (
            "'test' * []",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" * []': Invalid operand types String and Array"##,
            ),
        ),
        (
            "'test' * {}",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" * {}': Invalid operand types String and Map"##,
            ),
        ),
        (
            "null * true",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null * true': Invalid operand types Null and Boolean"##,
            ),
        ),
        (
            "null * 2",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null * 2': Invalid operand types Null and Number"##,
            ),
        ),
        (
            "null * 'test'",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null * "test"': Invalid operand types Null and String"##,
            ),
        ),
        (
            "null * null",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null * null': Invalid operand types Null and Null"##,
            ),
        ),
        (
            "null * []",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null * []': Invalid operand types Null and Array"##,
            ),
        ),
        (
            "null * {}",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null * {}': Invalid operand types Null and Map"##,
            ),
        ),
        (
            "[] * true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] * true': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "[] * 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] * 2': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "[] * 'test'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] * "test"': Invalid operand types Array and String"##,
            ),
        ),
        (
            "[] * null",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] * null': Invalid operand types Array and Null"##,
            ),
        ),
        (
            "[] * []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] * []': Invalid operand types Array and Array"##,
            ),
        ),
        (
            "[] * {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] * {}': Invalid operand types Array and Map"##,
            ),
        ),
        (
            "{} * true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} * true': Invalid operand types Map and Boolean"##,
            ),
        ),
        (
            "{} * 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} * 2': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "{} * 'test'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} * "test"': Invalid operand types Map and String"##,
            ),
        ),
        (
            "{} * null",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} * null': Invalid operand types Map and Null"##,
            ),
        ),
        (
            "{} * []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} * []': Invalid operand types Map and Array"##,
            ),
        ),
        (
            "{} * {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} * {}': Invalid operand types Map and Map"##,
            ),
        ),
        ("true / true", ok(1_i64)),
        ("true / false", ok(Value::number(f64::INFINITY))),
        ("true / 3", ok(Value::number(1.0 / 3.0))),
        ("true / '2'", ok(Value::number(1.0 / 2.0))),
        (
            "true / 'test'",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true / "test"': Invalid operand types Boolean and String"##,
            ),
        ),
        (
            "true / null",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true / null': Invalid operand types Boolean and Null"##,
            ),
        ),
        (
            "true / []",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true / []': Invalid operand types Boolean and Array"##,
            ),
        ),
        (
            "true / {}",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true / {}': Invalid operand types Boolean and Map"##,
            ),
        ),
        ("1 / true", ok(1_i64)),
        ("3 / 2", ok(Value::number(1.5))),
        (
            "1 / 'test'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 / "test"': Invalid operand types Number and String"##,
            ),
        ),
        ("1 / '3.0'", ok(Value::number(1.0 / 3.0))),
        (
            "1 / null",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 / null': Invalid operand types Number and Null"##,
            ),
        ),
        (
            "1 / []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 / []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "1 / {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 / {}': Invalid operand types Number and Map"##,
            ),
        ),
        (
            "'test' / true",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" / true': Invalid operand types String and Boolean"##,
            ),
        ),
        (
            "'test' / 2",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" / 2': Invalid operand types String and Number"##,
            ),
        ),
        ("'3' / 2", ok(Value::number(3.0 / 2.0))),
        (
            "'this' / 'test'",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"this" / "test"': Invalid operand types String and String"##,
            ),
        ),
        (
            "'test' / null",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" / null': Invalid operand types String and Null"##,
            ),
        ),
        (
            "'test' / []",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" / []': Invalid operand types String and Array"##,
            ),
        ),
        (
            "'test' / {}",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" / {}': Invalid operand types String and Map"##,
            ),
        ),
        (
            "null / true",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null / true': Invalid operand types Null and Boolean"##,
            ),
        ),
        (
            "null / 2",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null / 2': Invalid operand types Null and Number"##,
            ),
        ),
        (
            "null / 'test'",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null / "test"': Invalid operand types Null and String"##,
            ),
        ),
        (
            "null / null",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null / null': Invalid operand types Null and Null"##,
            ),
        ),
        (
            "null / []",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null / []': Invalid operand types Null and Array"##,
            ),
        ),
        (
            "null / {}",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null / {}': Invalid operand types Null and Map"##,
            ),
        ),
        (
            "[] / true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] / true': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "[] / 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] / 2': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "[] / 'test'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] / "test"': Invalid operand types Array and String"##,
            ),
        ),
        (
            "[] / null",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] / null': Invalid operand types Array and Null"##,
            ),
        ),
        (
            "[] / []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] / []': Invalid operand types Array and Array"##,
            ),
        ),
        (
            "[] / {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] / {}': Invalid operand types Array and Map"##,
            ),
        ),
        (
            "{} / true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} / true': Invalid operand types Map and Boolean"##,
            ),
        ),
        (
            "{} / 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} / 2': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "{} / 'test'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} / "test"': Invalid operand types Map and String"##,
            ),
        ),
        (
            "{} / null",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} / null': Invalid operand types Map and Null"##,
            ),
        ),
        (
            "{} / []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} / []': Invalid operand types Map and Array"##,
            ),
        ),
        (
            "{} / {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} / {}': Invalid operand types Map and Map"##,
            ),
        ),
        ("true % true", ok(0_i64)),
        ("true % -2", ok(1_i64)),
        (
            "true % 'test'",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true % "test"': Invalid operand types Boolean and String"##,
            ),
        ),
        ("true % '1'", ok(0_i64)),
        (
            "true % null",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true % null': Invalid operand types Boolean and Null"##,
            ),
        ),
        (
            "true % []",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true % []': Invalid operand types Boolean and Array"##,
            ),
        ),
        (
            "true % {}",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true % {}': Invalid operand types Boolean and Map"##,
            ),
        ),
        ("3 % -2", ok(1_i64)),
        ("3 % '-2'", ok(1_i64)),
        (
            "1 % 'test'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 % "test"': Invalid operand types Number and String"##,
            ),
        ),
        (
            "1 % null",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 % null': Invalid operand types Number and Null"##,
            ),
        ),
        (
            "1 % []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 % []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "1 % {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 % {}': Invalid operand types Number and Map"##,
            ),
        ),
        (
            "'test' % true",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" % true': Invalid operand types String and Boolean"##,
            ),
        ),
        (
            "'test' % 2",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" % 2': Invalid operand types String and Number"##,
            ),
        ),
        ("'3' % 2", ok(1_i64)),
        (
            "'this' % 'test'",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"this" % "test"': Invalid operand types String and String"##,
            ),
        ),
        (
            "'test' % null",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" % null': Invalid operand types String and Null"##,
            ),
        ),
        (
            "'test' % []",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" % []': Invalid operand types String and Array"##,
            ),
        ),
        (
            "'test' % {}",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"test" % {}': Invalid operand types String and Map"##,
            ),
        ),
        (
            "null % true",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null % true': Invalid operand types Null and Boolean"##,
            ),
        ),
        (
            "null % 2",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null % 2': Invalid operand types Null and Number"##,
            ),
        ),
        (
            "null % 'test'",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null % "test"': Invalid operand types Null and String"##,
            ),
        ),
        (
            "null % null",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null % null': Invalid operand types Null and Null"##,
            ),
        ),
        (
            "null % []",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null % []': Invalid operand types Null and Array"##,
            ),
        ),
        (
            "null % {}",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'null % {}': Invalid operand types Null and Map"##,
            ),
        ),
        (
            "[] % true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] % true': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "[] % 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] % 2': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "[] % 'test'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] % "test"': Invalid operand types Array and String"##,
            ),
        ),
        (
            "[] % null",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] % null': Invalid operand types Array and Null"##,
            ),
        ),
        (
            "[] % []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] % []': Invalid operand types Array and Array"##,
            ),
        ),
        (
            "[] % {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] % {}': Invalid operand types Array and Map"##,
            ),
        ),
        (
            "{} % true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} % true': Invalid operand types Map and Boolean"##,
            ),
        ),
        (
            "{} % 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} % 2': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "{} % 'test'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} % "test"': Invalid operand types Map and String"##,
            ),
        ),
        (
            "{} % null",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} % null': Invalid operand types Map and Null"##,
            ),
        ),
        (
            "{} % []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} % []': Invalid operand types Map and Array"##,
            ),
        ),
        (
            "{} % {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} % {}': Invalid operand types Map and Map"##,
            ),
        ),
        ("!true", ok(false)),
        ("!false", ok(true)),
        (
            "!1",
            err(r##"At line 1, column 1: Cannot evaluate expression '!1': Invalid type Number"##),
        ),
        (
            "!'test'",
            err(
                r##"At line 1, column 1: Cannot evaluate expression '!"test"': Invalid type String"##,
            ),
        ),
        (
            "!null",
            err(r##"At line 1, column 1: Cannot evaluate expression '!null': Invalid type Null"##),
        ),
        (
            "![]",
            err(r##"At line 1, column 1: Cannot evaluate expression '![]': Invalid type Array"##),
        ),
        (
            "!{}",
            err(r##"At line 1, column 1: Cannot evaluate expression '!{}': Invalid type Map"##),
        ),
        ("false && false", ok(false)),
        ("false && true", ok(false)),
        ("true && false", ok(false)),
        ("true && true", ok(true)),
        ("null && true", ok(false)),
        ("true && null", ok(false)),
        ("null && null", ok(false)),
        ("false || false", ok(false)),
        ("false || true", ok(true)),
        ("true || false", ok(true)),
        ("true || true", ok(true)),
        ("null || true", ok(true)),
        ("false || null", ok(false)),
        ("null || null", ok(false)),
        ("undefined && true", Ok(Value::undefined())),
        ("true && undefined", Ok(Value::undefined())),
        ("undefined || true", Ok(Value::undefined())),
        ("false || undefined", Ok(Value::undefined())),
        (
            "true && 1",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true && 1': Invalid operand types Boolean and Number"##,
            ),
        ),
        (
            "true && 'a'",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true && "a"': Invalid operand types Boolean and String"##,
            ),
        ),
        (
            "false || []",
            err(
                r##"At line 1, column 7: Cannot evaluate expression 'false || []': Invalid operand types Boolean and Array"##,
            ),
        ),
        (
            "1 && true",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 && true': Invalid operand types Number and Boolean"##,
            ),
        ),
        (
            "'a' || false",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" || false': Invalid operand types String and Boolean"##,
            ),
        ),
        (
            "[1] && true",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '[1] && true': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "{} || false",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} || false': Invalid operand types Map and Boolean"##,
            ),
        ),
        (
            "1 && 'a'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 && "a"': Invalid operand types Number and String"##,
            ),
        ),
        ("1 && undefined", Ok(Value::undefined())),
        ("'a' || undefined", Ok(Value::undefined())),
        ("false && x[-1]", ok(false)),
        ("true || x[-1]", ok(true)),
        ("+undefined", Ok(Value::undefined())),
        ("-undefined", Ok(Value::undefined())),
        ("!undefined", Ok(Value::undefined())),
        ("~undefined", Ok(Value::undefined())),
        ("1 + undefined", Ok(Value::undefined())),
        ("undefined + 1", Ok(Value::undefined())),
        ("1 - undefined", Ok(Value::undefined())),
        ("1 * undefined", Ok(Value::undefined())),
        ("1 / undefined", Ok(Value::undefined())),
        ("1 % undefined", Ok(Value::undefined())),
        ("1 & undefined", Ok(Value::undefined())),
        ("undefined & 1", Ok(Value::undefined())),
        ("1 ^ undefined", Ok(Value::undefined())),
        ("1 | undefined", Ok(Value::undefined())),
        ("1 << undefined", Ok(Value::undefined())),
        ("1 >> undefined", Ok(Value::undefined())),
        ("~23423", ok(Value::integer(!23423_i64))),
        ("~23423.1", ok(Value::integer(!23423_i64))),
        ("~23423.8", ok(Value::integer(!23423_i64))),
        (
            "~true",
            err(
                r##"At line 1, column 1: Cannot evaluate expression '~true': Invalid type Boolean"##,
            ),
        ),
        ("~'23423'", ok(Value::integer(!23423_i64))),
        (
            "~'asdf'",
            err(
                r##"At line 1, column 1: Cannot evaluate expression '~"asdf"': Invalid type String"##,
            ),
        ),
        (
            "~null",
            err(r##"At line 1, column 1: Cannot evaluate expression '~null': Invalid type Null"##),
        ),
        (
            "~[]",
            err(r##"At line 1, column 1: Cannot evaluate expression '~[]': Invalid type Array"##),
        ),
        (
            "~{}",
            err(r##"At line 1, column 1: Cannot evaluate expression '~{}': Invalid type Map"##),
        ),
        ("0 & 0", ok(Value::integer(0 & 0))),
        ("123 & 456", ok(Value::integer(123 & 456))),
        ("true & 123", ok(Value::integer(1 & 123))),
        ("123 & true", ok(Value::integer(123 & 1))),
        (
            "'asdf' & 123",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"asdf" & 123': Invalid operand types String and Number"##,
            ),
        ),
        ("'456' & 123", ok(Value::integer(456 & 123))),
        (
            "123 & 'asdf'",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '123 & "asdf"': Invalid operand types Number and String"##,
            ),
        ),
        ("123 & '456'", ok(Value::integer(123 & 456))),
        ("null & 123", ok(Value::integer(0 & 123))),
        ("123 & null", ok(Value::integer(123 & 0))),
        (
            "[] & 123",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] & 123': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "123 & []",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '123 & []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "{} & 123",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} & 123': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "123 & {}",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '123 & {}': Invalid operand types Number and Map"##,
            ),
        ),
        ("0 | 0", ok(Value::integer(0 | 0))),
        ("123 | 456", ok(Value::integer(123 | 456))),
        ("true | 123", ok(Value::integer(1 | 123))),
        ("123 | true", ok(Value::integer(123 | 1))),
        (
            "'asdf' | 123",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"asdf" | 123': Invalid operand types String and Number"##,
            ),
        ),
        ("'456' | 123", ok(Value::integer(456 | 123))),
        (
            "123 | 'asdf'",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '123 | "asdf"': Invalid operand types Number and String"##,
            ),
        ),
        ("123 | '456'", ok(Value::integer(123 | 456))),
        ("null | 123", ok(Value::integer(0 | 123))),
        ("123 | null", ok(Value::integer(123 | 0))),
        (
            "[] | 123",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] | 123': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "123 | []",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '123 | []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "{} | 123",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} | 123': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "123 | {}",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '123 | {}': Invalid operand types Number and Map"##,
            ),
        ),
        ("0 ^ 0", ok(Value::integer(0 ^ 0))),
        ("123 ^ 456", ok(Value::integer(123 ^ 456))),
        ("true ^ 123", ok(Value::integer(1 ^ 123))),
        ("123 ^ true", ok(Value::integer(123 ^ 1))),
        (
            "'asdf' ^ 123",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"asdf" ^ 123': Invalid operand types String and Number"##,
            ),
        ),
        ("'456' ^ 123", ok(Value::integer(456 ^ 123))),
        (
            "123 ^ 'asdf'",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '123 ^ "asdf"': Invalid operand types Number and String"##,
            ),
        ),
        ("123 ^ '456'", ok(Value::integer(123 ^ 456))),
        ("null ^ 123", ok(Value::integer(0 ^ 123))),
        ("123 ^ null", ok(Value::integer(123 ^ 0))),
        (
            "[] ^ 123",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] ^ 123': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "123 ^ []",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '123 ^ []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "{} ^ 123",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} ^ 123': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "123 ^ {}",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '123 ^ {}': Invalid operand types Number and Map"##,
            ),
        ),
        ("1 << 2", ok(Value::integer(1 << 2))),
        ("true << 2", ok(Value::integer(1 << 2))),
        ("1 << false", ok(Value::integer(1 << 0))),
        (
            "'asdf' << 2",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"asdf" << 2': Invalid operand types String and Number"##,
            ),
        ),
        ("'1' << 2", ok(Value::integer(1 << 2))),
        (
            "1 << 'asdf'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 << "asdf"': Invalid operand types Number and String"##,
            ),
        ),
        ("1 << '2'", ok(Value::integer(1 << 2))),
        ("null << 2", ok(Value::integer(0 << 2))),
        ("1 << null", ok(Value::integer(1 << 0))),
        (
            "[] << 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] << 2': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "1 << []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 << []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "{} << 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} << 2': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "1 << {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 << {}': Invalid operand types Number and Map"##,
            ),
        ),
        ("1 >> 2", ok(Value::integer(1 >> 2))),
        ("true >> 2", ok(Value::integer(1 >> 2))),
        ("1 >> false", ok(Value::integer(1 >> 0))),
        (
            "'asdf' >> 2",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"asdf" >> 2': Invalid operand types String and Number"##,
            ),
        ),
        ("'1' >> 2", ok(Value::integer(1 >> 2))),
        (
            "1 >> 'asdf'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 >> "asdf"': Invalid operand types Number and String"##,
            ),
        ),
        ("1 >> '2'", ok(Value::integer(1 >> 2))),
        ("null >> 2", ok(Value::integer(0 >> 2))),
        ("1 >> null", ok(Value::integer(1 >> 0))),
        (
            "[] >> 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] >> 2': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "1 >> []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 >> []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "{} >> 2",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} >> 2': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "1 >> {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 >> {}': Invalid operand types Number and Map"##,
            ),
        ),
        ("false < false", ok(false)),
        ("false < true", ok(true)),
        ("true < false", ok(false)),
        ("true < true", ok(false)),
        ("false < 0", ok(false)),
        ("false < 1", ok(true)),
        ("false < 'true'", ok(true)),
        ("false < 'false'", ok(false)),
        ("false < ''", ok(false)),
        ("false < null", ok(false)),
        (
            "false < []",
            err(
                r##"At line 1, column 7: Cannot evaluate expression 'false < []': Invalid operand types Boolean and Array"##,
            ),
        ),
        (
            "false < {}",
            err(
                r##"At line 1, column 7: Cannot evaluate expression 'false < {}': Invalid operand types Boolean and Map"##,
            ),
        ),
        ("0 < 0", ok(false)),
        ("0 < 1", ok(true)),
        (
            "0 < 'true'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 < "true"': At line 1, column 5: Cannot convert value '"true"' of type 'String' to type 'Number'"##,
            ),
        ),
        (
            "0 < 'false'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 < "false"': At line 1, column 5: Cannot convert value '"false"' of type 'String' to type 'Number'"##,
            ),
        ),
        ("0 < ''", ok(false)),
        ("0 < '0'", ok(false)),
        ("0 < '1'", ok(true)),
        ("0 < null", ok(false)),
        (
            "0 < []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 < []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "0 < {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 < {}': Invalid operand types Number and Map"##,
            ),
        ),
        (
            "'a' < 0",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" < 0': At line 1, column 1: Cannot convert value '"a"' of type 'String' to type 'Number'"##,
            ),
        ),
        (
            "'a' < 1",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" < 1': At line 1, column 1: Cannot convert value '"a"' of type 'String' to type 'Number'"##,
            ),
        ),
        ("'a' < 'true'", ok(true)),
        ("'a' < 'false'", ok(true)),
        ("'a' < ''", ok(false)),
        ("'a' < 'b'", ok(true)),
        ("'a' < 'a'", ok(false)),
        ("'aa' < 'ab'", ok(true)),
        ("'a' < null", ok(false)),
        (
            "'a' < []",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" < []': Invalid operand types String and Array"##,
            ),
        ),
        (
            "'a' < {}",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" < {}': Invalid operand types String and Map"##,
            ),
        ),
        ("'0' < 1", ok(true)),
        ("'1' < 0", ok(false)),
        ("null < true", ok(true)),
        ("null < false", ok(true)),
        ("null < 0", ok(true)),
        ("null < 1", ok(true)),
        ("null < ''", ok(true)),
        ("null < 'a'", ok(true)),
        ("null < null", ok(false)),
        ("null < []", ok(true)),
        ("null < {}", ok(true)),
        (
            "[] < true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] < true': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "[] < false",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] < false': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "[] < 0",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] < 0': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "[] < 1",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] < 1': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "[] < ''",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] < ""': Invalid operand types Array and String"##,
            ),
        ),
        (
            "[] < 'a'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] < "a"': Invalid operand types Array and String"##,
            ),
        ),
        ("[] < null", ok(false)),
        ("[] < []", ok(false)),
        ("[1] < [1]", ok(false)),
        ("[1] < [2]", ok(true)),
        ("[1] < [1,2]", ok(true)),
        ("[1,2] < [1,2]", ok(false)),
        ("[1,2] < [1,2,3]", ok(true)),
        ("[1,2,3] < [1,2]", ok(false)),
        (
            "[] < {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] < {}': Invalid operand types Array and Map"##,
            ),
        ),
        (
            "{} < true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} < true': Invalid operand types Map and Boolean"##,
            ),
        ),
        (
            "{} < false",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} < false': Invalid operand types Map and Boolean"##,
            ),
        ),
        (
            "{} < 0",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} < 0': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "{} < 1",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} < 1': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "{} < ''",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} < ""': Invalid operand types Map and String"##,
            ),
        ),
        (
            "{} < 'a'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} < "a"': Invalid operand types Map and String"##,
            ),
        ),
        ("{} < null", ok(false)),
        (
            "{} < []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} < []': Invalid operand types Map and Array"##,
            ),
        ),
        ("{} < {}", ok(false)),
        ("{k1:1} < {k1:1}", ok(false)),
        ("{k1:1} < {k2:1}", ok(true)),
        ("{k2:1} < {k1:1}", ok(false)),
        ("{k1:1} < {k1:2}", ok(true)),
        ("{k1:1} < {k1:1, k2:2}", ok(true)),
        ("{k1:1} < {k1:2, k2:2}", ok(true)),
        ("false <= false", ok(true)),
        ("false <= true", ok(true)),
        ("true <= false", ok(false)),
        ("true <= true", ok(true)),
        ("false <= 0", ok(true)),
        ("false <= 1", ok(true)),
        ("false <= 'true'", ok(true)),
        ("false <= 'false'", ok(true)),
        ("false <= ''", ok(true)),
        ("false <= null", ok(false)),
        (
            "false <= []",
            err(
                r##"At line 1, column 7: Cannot evaluate expression 'false <= []': Invalid operand types Boolean and Array"##,
            ),
        ),
        (
            "false <= {}",
            err(
                r##"At line 1, column 7: Cannot evaluate expression 'false <= {}': Invalid operand types Boolean and Map"##,
            ),
        ),
        ("0 <= 0", ok(true)),
        ("0 <= 1", ok(true)),
        (
            "0 <= 'true'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 <= "true"': At line 1, column 6: Cannot convert value '"true"' of type 'String' to type 'Number'"##,
            ),
        ),
        (
            "0 <= 'false'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 <= "false"': At line 1, column 6: Cannot convert value '"false"' of type 'String' to type 'Number'"##,
            ),
        ),
        ("0 <= ''", ok(true)),
        ("0 <= '0'", ok(true)),
        ("0 <= '1'", ok(true)),
        ("0 <= null", ok(false)),
        (
            "0 <= []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 <= []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "0 <= {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 <= {}': Invalid operand types Number and Map"##,
            ),
        ),
        (
            "'a' <= 0",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" <= 0': At line 1, column 1: Cannot convert value '"a"' of type 'String' to type 'Number'"##,
            ),
        ),
        (
            "'a' <= 1",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" <= 1': At line 1, column 1: Cannot convert value '"a"' of type 'String' to type 'Number'"##,
            ),
        ),
        ("'a' <= 'true'", ok(true)),
        ("'a' <= 'false'", ok(true)),
        ("'a' <= ''", ok(false)),
        ("'a' <= 'b'", ok(true)),
        ("'a' <= 'a'", ok(true)),
        ("'aa' <= 'ab'", ok(true)),
        ("'a' <= null", ok(false)),
        (
            "'a' <= []",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" <= []': Invalid operand types String and Array"##,
            ),
        ),
        (
            "'a' <= {}",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" <= {}': Invalid operand types String and Map"##,
            ),
        ),
        ("'0' <= 1", ok(true)),
        ("'1' <= 0", ok(false)),
        ("null <= true", ok(true)),
        ("null <= false", ok(true)),
        ("null <= 0", ok(true)),
        ("null <= 1", ok(true)),
        ("null <= ''", ok(true)),
        ("null <= 'a'", ok(true)),
        ("null <= null", ok(true)),
        ("null <= []", ok(true)),
        ("null <= {}", ok(true)),
        (
            "[] <= true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] <= true': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "[] <= false",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] <= false': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "[] <= 0",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] <= 0': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "[] <= 1",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] <= 1': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "[] <= ''",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] <= ""': Invalid operand types Array and String"##,
            ),
        ),
        (
            "[] <= 'a'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] <= "a"': Invalid operand types Array and String"##,
            ),
        ),
        ("[] <= null", ok(false)),
        ("[] <= []", ok(true)),
        ("[1] <= [1]", ok(true)),
        ("[1] <= [2]", ok(true)),
        ("[1] <= [1,2]", ok(true)),
        ("[1,2] <= [1,2]", ok(true)),
        ("[1,2] <= [1,2,3]", ok(true)),
        ("[1,2,3] <= [1,2]", ok(false)),
        (
            "[] <= {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] <= {}': Invalid operand types Array and Map"##,
            ),
        ),
        (
            "{} <= true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} <= true': Invalid operand types Map and Boolean"##,
            ),
        ),
        (
            "{} <= false",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} <= false': Invalid operand types Map and Boolean"##,
            ),
        ),
        (
            "{} <= 0",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} <= 0': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "{} <= 1",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} <= 1': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "{} <= ''",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} <= ""': Invalid operand types Map and String"##,
            ),
        ),
        (
            "{} <= 'a'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} <= "a"': Invalid operand types Map and String"##,
            ),
        ),
        ("{} <= null", ok(false)),
        (
            "{} <= []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} <= []': Invalid operand types Map and Array"##,
            ),
        ),
        ("{} <= {}", ok(true)),
        ("{k1:1} <= {k1:1}", ok(true)),
        ("{k1:1} <= {k2:1}", ok(true)),
        ("{k2:1} <= {k1:1}", ok(false)),
        ("{k1:1} <= {k1:2}", ok(true)),
        ("{k1:1} <= {k1:1, k2:2}", ok(true)),
        ("{k1:1} <= {k1:2, k2:2}", ok(true)),
        ("false > false", ok(false)),
        ("true > false", ok(true)),
        ("false > true", ok(false)),
        ("true > true", ok(false)),
        ("0 > false", ok(false)),
        ("1 > false", ok(true)),
        ("'true' > false", ok(true)),
        ("'false' > false", ok(false)),
        ("'' > false", ok(false)),
        ("null > false", ok(false)),
        (
            "[] > false",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] > false': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "{} > false",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} > false': Invalid operand types Map and Boolean"##,
            ),
        ),
        ("0 > 0", ok(false)),
        ("1 > 0", ok(true)),
        (
            "'true' > 0",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"true" > 0': At line 1, column 1: Cannot convert value '"true"' of type 'String' to type 'Number'"##,
            ),
        ),
        (
            "'false' > 0",
            err(
                r##"At line 1, column 9: Cannot evaluate expression '"false" > 0': At line 1, column 1: Cannot convert value '"false"' of type 'String' to type 'Number'"##,
            ),
        ),
        ("'' > 0", ok(false)),
        ("'0' > 0", ok(false)),
        ("'1' > 0", ok(true)),
        ("null > 0", ok(false)),
        (
            "[] > 0",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] > 0': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "{} > 0",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} > 0': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "0 > 'a'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 > "a"': At line 1, column 5: Cannot convert value '"a"' of type 'String' to type 'Number'"##,
            ),
        ),
        (
            "1 > 'a'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 > "a"': At line 1, column 5: Cannot convert value '"a"' of type 'String' to type 'Number'"##,
            ),
        ),
        ("'true' > 'a'", ok(true)),
        ("'false' > 'a'", ok(true)),
        ("'' > 'a'", ok(false)),
        ("'b' > 'a'", ok(true)),
        ("'a' > 'a'", ok(false)),
        ("'ab' > 'aa'", ok(true)),
        ("null > 'a'", ok(false)),
        (
            "[] > 'a'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] > "a"': Invalid operand types Array and String"##,
            ),
        ),
        (
            "{} > 'a'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} > "a"': Invalid operand types Map and String"##,
            ),
        ),
        ("1 > '0'", ok(true)),
        ("0 > '1'", ok(false)),
        ("true > null", ok(true)),
        ("false > null", ok(true)),
        ("0 > null", ok(true)),
        ("1 > null", ok(true)),
        ("'' > null", ok(true)),
        ("'a' > null", ok(true)),
        ("null > null", ok(false)),
        ("[] > null", ok(true)),
        ("{} > null", ok(true)),
        (
            "true > []",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true > []': Invalid operand types Boolean and Array"##,
            ),
        ),
        (
            "false > []",
            err(
                r##"At line 1, column 7: Cannot evaluate expression 'false > []': Invalid operand types Boolean and Array"##,
            ),
        ),
        (
            "0 > []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 > []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "1 > []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 > []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "'' > []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '"" > []': Invalid operand types String and Array"##,
            ),
        ),
        (
            "'a' > []",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" > []': Invalid operand types String and Array"##,
            ),
        ),
        ("null > []", ok(false)),
        ("[] > []", ok(false)),
        ("[1] > [1]", ok(false)),
        ("[2] > [1]", ok(true)),
        ("[1,2] > [1]", ok(true)),
        ("[1,2] > [1,2]", ok(false)),
        ("[1,2,3] > [1,2]", ok(true)),
        ("[1,2] > [1,2,3]", ok(false)),
        (
            "{} > []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} > []': Invalid operand types Map and Array"##,
            ),
        ),
        (
            "true > {}",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true > {}': Invalid operand types Boolean and Map"##,
            ),
        ),
        (
            "false > {}",
            err(
                r##"At line 1, column 7: Cannot evaluate expression 'false > {}': Invalid operand types Boolean and Map"##,
            ),
        ),
        (
            "0 > {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 > {}': Invalid operand types Number and Map"##,
            ),
        ),
        (
            "1 > {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 > {}': Invalid operand types Number and Map"##,
            ),
        ),
        (
            "'' > {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '"" > {}': Invalid operand types String and Map"##,
            ),
        ),
        (
            "'a' > {}",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" > {}': Invalid operand types String and Map"##,
            ),
        ),
        ("null > {}", ok(false)),
        (
            "[] > {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] > {}': Invalid operand types Array and Map"##,
            ),
        ),
        ("{} > {}", ok(false)),
        ("{k1:1} > {k1:1}", ok(false)),
        ("{k2:1} > {k1:1}", ok(true)),
        ("{k1:1} > {k2:1}", ok(false)),
        ("{k1:2} > {k1:1}", ok(true)),
        ("{k1:1, k2:2} > {k1:1}", ok(true)),
        ("{k1:2, k2:2} > {k1:1}", ok(true)),
        ("false >= false", ok(true)),
        ("true >= false", ok(true)),
        ("false >= true", ok(false)),
        ("true >= true", ok(true)),
        ("0 >= false", ok(true)),
        ("1 >= false", ok(true)),
        ("'true' >= false", ok(true)),
        ("'false' >= false", ok(true)),
        ("'' >= false", ok(true)),
        ("null >= false", ok(false)),
        (
            "[] >= false",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] >= false': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "{} >= false",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} >= false': Invalid operand types Map and Boolean"##,
            ),
        ),
        ("0 >= 0", ok(true)),
        ("1 >= 0", ok(true)),
        (
            "'true' >= 0",
            err(
                r##"At line 1, column 8: Cannot evaluate expression '"true" >= 0': At line 1, column 1: Cannot convert value '"true"' of type 'String' to type 'Number'"##,
            ),
        ),
        (
            "'false' >= 0",
            err(
                r##"At line 1, column 9: Cannot evaluate expression '"false" >= 0': At line 1, column 1: Cannot convert value '"false"' of type 'String' to type 'Number'"##,
            ),
        ),
        ("'' >= 0", ok(true)),
        ("'0' >= 0", ok(true)),
        ("'1' >= 0", ok(true)),
        ("null >= 0", ok(false)),
        (
            "[] >= 0",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] >= 0': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "{} >= 0",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} >= 0': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "0 >= 'a'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 >= "a"': At line 1, column 6: Cannot convert value '"a"' of type 'String' to type 'Number'"##,
            ),
        ),
        (
            "1 >= 'a'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 >= "a"': At line 1, column 6: Cannot convert value '"a"' of type 'String' to type 'Number'"##,
            ),
        ),
        ("'true' >= 'a'", ok(true)),
        ("'false' >= 'a'", ok(true)),
        ("'' >= 'a'", ok(false)),
        ("'b' >= 'a'", ok(true)),
        ("'a' >= 'a'", ok(true)),
        ("'ab' >= 'aa'", ok(true)),
        ("null >= 'a'", ok(false)),
        (
            "[] >= 'a'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] >= "a"': Invalid operand types Array and String"##,
            ),
        ),
        (
            "{} >= 'a'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} >= "a"': Invalid operand types Map and String"##,
            ),
        ),
        ("1 >= '0'", ok(true)),
        ("0 >= '1'", ok(false)),
        ("true >= null", ok(true)),
        ("false >= null", ok(true)),
        ("0 >= null", ok(true)),
        ("1 >= null", ok(true)),
        ("'' >= null", ok(true)),
        ("'a' >= null", ok(true)),
        ("null >= null", ok(true)),
        ("[] >= null", ok(true)),
        ("{} >= null", ok(true)),
        (
            "true >= []",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true >= []': Invalid operand types Boolean and Array"##,
            ),
        ),
        (
            "false >= []",
            err(
                r##"At line 1, column 7: Cannot evaluate expression 'false >= []': Invalid operand types Boolean and Array"##,
            ),
        ),
        (
            "0 >= []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 >= []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "1 >= []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 >= []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "'' >= []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '"" >= []': Invalid operand types String and Array"##,
            ),
        ),
        (
            "'a' >= []",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" >= []': Invalid operand types String and Array"##,
            ),
        ),
        ("null >= []", ok(false)),
        ("[] >= []", ok(true)),
        ("[1] >= [1]", ok(true)),
        ("[2] >= [1]", ok(true)),
        ("[1,2] >= [1]", ok(true)),
        ("[1,2] >= [1,2]", ok(true)),
        ("[1,2,3] >= [1,2]", ok(true)),
        ("[1,2] >= [1,2,3]", ok(false)),
        (
            "{} >= []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} >= []': Invalid operand types Map and Array"##,
            ),
        ),
        (
            "true >= {}",
            err(
                r##"At line 1, column 6: Cannot evaluate expression 'true >= {}': Invalid operand types Boolean and Map"##,
            ),
        ),
        (
            "false >= {}",
            err(
                r##"At line 1, column 7: Cannot evaluate expression 'false >= {}': Invalid operand types Boolean and Map"##,
            ),
        ),
        (
            "0 >= {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 >= {}': Invalid operand types Number and Map"##,
            ),
        ),
        (
            "1 >= {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '1 >= {}': Invalid operand types Number and Map"##,
            ),
        ),
        (
            "'' >= {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '"" >= {}': Invalid operand types String and Map"##,
            ),
        ),
        (
            "'a' >= {}",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" >= {}': Invalid operand types String and Map"##,
            ),
        ),
        ("null >= {}", ok(false)),
        (
            "[] >= {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] >= {}': Invalid operand types Array and Map"##,
            ),
        ),
        ("{} >= {}", ok(true)),
        ("{k1:1} >= {k1:1}", ok(true)),
        ("{k2:1} >= {k1:1}", ok(true)),
        ("{k1:1} >= {k2:1}", ok(false)),
        ("{k1:2} >= {k1:1}", ok(true)),
        ("{k1:1, k2:2} >= {k1:1}", ok(true)),
        ("{k1:2, k2:2} >= {k1:1}", ok(true)),
        ("false == false", ok(true)),
        ("false == true", ok(false)),
        ("true == false", ok(false)),
        ("true == true", ok(true)),
        ("false == 0", ok(true)),
        ("false == 1", ok(false)),
        ("false == 'true'", ok(false)),
        ("false == 'false'", ok(true)),
        ("false == ''", ok(true)),
        ("false == null", ok(false)),
        (
            "false == []",
            err(
                r##"At line 1, column 7: Cannot evaluate expression 'false == []': Invalid operand types Boolean and Array"##,
            ),
        ),
        (
            "false == {}",
            err(
                r##"At line 1, column 7: Cannot evaluate expression 'false == {}': Invalid operand types Boolean and Map"##,
            ),
        ),
        ("0 == 0", ok(true)),
        ("0 == 1", ok(false)),
        (
            "0 == 'true'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 == "true"': At line 1, column 6: Cannot convert value '"true"' of type 'String' to type 'Number'"##,
            ),
        ),
        (
            "0 == 'false'",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 == "false"': At line 1, column 6: Cannot convert value '"false"' of type 'String' to type 'Number'"##,
            ),
        ),
        ("0 == ''", ok(true)),
        ("0 == '0'", ok(true)),
        ("0 == '1'", ok(false)),
        ("0 == null", ok(false)),
        (
            "0 == []",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 == []': Invalid operand types Number and Array"##,
            ),
        ),
        (
            "0 == {}",
            err(
                r##"At line 1, column 3: Cannot evaluate expression '0 == {}': Invalid operand types Number and Map"##,
            ),
        ),
        (
            "'a' == 0",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" == 0': At line 1, column 1: Cannot convert value '"a"' of type 'String' to type 'Number'"##,
            ),
        ),
        (
            "'a' == 1",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" == 1': At line 1, column 1: Cannot convert value '"a"' of type 'String' to type 'Number'"##,
            ),
        ),
        ("'a' == 'b'", ok(false)),
        ("'a' == 'a'", ok(true)),
        ("'aa' == 'ab'", ok(false)),
        ("'a' == null", ok(false)),
        (
            "'a' == []",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" == []': Invalid operand types String and Array"##,
            ),
        ),
        (
            "'a' == {}",
            err(
                r##"At line 1, column 5: Cannot evaluate expression '"a" == {}': Invalid operand types String and Map"##,
            ),
        ),
        ("'0' == 0", ok(true)),
        ("'0' == 1", ok(false)),
        ("null == true", ok(false)),
        ("null == false", ok(false)),
        ("null == 0", ok(false)),
        ("null == 1", ok(false)),
        ("null == ''", ok(false)),
        ("null == 'a'", ok(false)),
        ("null == null", ok(true)),
        ("null == []", ok(false)),
        ("null == {}", ok(false)),
        (
            "[] == true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] == true': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "[] == false",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] == false': Invalid operand types Array and Boolean"##,
            ),
        ),
        (
            "[] == 0",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] == 0': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "[] == 1",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] == 1': Invalid operand types Array and Number"##,
            ),
        ),
        (
            "[] == ''",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] == ""': Invalid operand types Array and String"##,
            ),
        ),
        (
            "[] == 'a'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] == "a"': Invalid operand types Array and String"##,
            ),
        ),
        ("[] == null", ok(false)),
        ("[] == []", ok(true)),
        ("[1] == [1]", ok(true)),
        ("[1] == [2]", ok(false)),
        ("[1] == [1,2]", ok(false)),
        ("[1,2] == [1,2]", ok(true)),
        ("[1,2] == [1,2,3]", ok(false)),
        ("[1,2,3] == [1,2]", ok(false)),
        (
            "[] == {}",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '[] == {}': Invalid operand types Array and Map"##,
            ),
        ),
        (
            "{} == true",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} == true': Invalid operand types Map and Boolean"##,
            ),
        ),
        (
            "{} == false",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} == false': Invalid operand types Map and Boolean"##,
            ),
        ),
        (
            "{} == 0",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} == 0': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "{} == 1",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} == 1': Invalid operand types Map and Number"##,
            ),
        ),
        (
            "{} == ''",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} == ""': Invalid operand types Map and String"##,
            ),
        ),
        (
            "{} == 'a'",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} == "a"': Invalid operand types Map and String"##,
            ),
        ),
        ("{} == null", ok(false)),
        (
            "{} == []",
            err(
                r##"At line 1, column 4: Cannot evaluate expression '{} == []': Invalid operand types Map and Array"##,
            ),
        ),
        ("{} == {}", ok(true)),
        ("{k1:1} == {k1:1}", ok(true)),
        ("{k1:1} == {k2:1}", ok(false)),
        ("{k2:1} == {k1:1}", ok(false)),
        ("{k1:1} == {k1:2}", ok(false)),
        ("{k1:1} == {k1:1, k2:2}", ok(false)),
        ("{k1:1} == {k1:2, k2:2}", ok(false)),
        ("undefined == undefined", ok(true)),
        ("undefined == null", ok(false)),
        ("null == undefined", ok(false)),
        ("undefined == false", ok(false)),
        ("undefined == 0", ok(false)),
        ("undefined == ''", ok(false)),
        ("undefined == []", ok(false)),
        ("undefined == {}", ok(false)),
        ("undefined < null", ok(true)),
        ("undefined < 0", ok(true)),
        ("0 > undefined", ok(true)),
        ("[] > undefined", ok(true)),
        ("{} > undefined", ok(true)),
        ("true -> 'asdf'", ok(Value::string("asdf"))),
        ("false -> 'asdf'", Ok(Value::undefined())),
        ("false -> x[-1]", Ok(Value::undefined())),
    ];
    for (expr, expected) in cases {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}

#[test]
fn comparison_with_ranges() {
    let vars = &[("r", range_var())];
    assert_eq!(evaluate_with("r == null", vars), ok(false));
    assert_eq!(evaluate_with("r > null", vars), ok(true));
    assert_eq!(evaluate_with("r == undefined", vars), ok(false));
    assert_eq!(evaluate_with("r > undefined", vars), ok(true));
    assert_eq!(evaluate_with("null == r", vars), ok(false));
    assert_eq!(evaluate_with("undefined == r", vars), ok(false));
    assert_eq!(
        evaluate_with("r == r", vars),
        err("At line 1, column 3: Cannot evaluate expression 'r == r': Invalid operand types Range and Range")
    );
    assert_eq!(
        evaluate_with("r == 0", vars),
        err("At line 1, column 3: Cannot evaluate expression 'r == 0': Invalid operand types Range and Number")
    );
}

#[test]
fn subscript() {
    let cases: Vec<(&str, Result<Value, String>)> = vec![
        ("'asdf'[0]", ok(Value::string("a"))),
        ("'asdf'[1]", ok(Value::string("s"))),
        ("'asdf'[3]", ok(Value::string("f"))),
        ("'asdf'[-1]", ok(Value::string("f"))),
        ("'asdf'[-4]", ok(Value::string("a"))),
        ("'asdf'[true]", ok(Value::string("s"))),
        ("'asdf'[false]", ok(Value::string("a"))),
        ("'asdf'[0, 1]", ok(Value::string("as"))),
        ("'asdf'[0, 1, 2]", ok(Value::string("asd"))),
        ("'asdf'[1, 2]", ok(Value::string("sd"))),
        ("'asdf'[1, 2, 7]", ok(Value::string("sd"))),
        ("'asdf'[3, 2, 1, 0]", ok(Value::string("fdsa"))),
        ("'asdf'[0, -1]", ok(Value::string("af"))),
        ("'asdf'[-4, -3, -2, -1]", ok(Value::string("asdf"))),
        ("'asdf'[0..1]", ok(Value::string("as"))),
        ("'asdf'[1..2]", ok(Value::string("sd"))),
        ("'asdf'[0..5]", ok(Value::string("asdf"))),
        ("'asdf'[3..0]", ok(Value::string("fdsa"))),
        ("'asdf'[3..1]", ok(Value::string("fds"))),
        ("'asdf'[3..2]", ok(Value::string("fd"))),
        ("'asdf'[3..3]", ok(Value::string("f"))),
        ("'asdf'[3..4]", ok(Value::string("f"))),
        ("'asdf'[0..]", ok(Value::string("asdf"))),
        ("'asdf'[1..]", ok(Value::string("sdf"))),
        ("'asdf'[..0]", ok(Value::string("fdsa"))),
        ("'asdf'[..1]", ok(Value::string("fds"))),
        ("'asdf'[..2]", ok(Value::string("fd"))),
        ("'asdf'[..3]", ok(Value::string("f"))),
        ("'asdf'[..4]", ok(Value::string("f"))),
        ("'asdf'[..5]", ok(Value::string("f"))),
        ("'asdf'[-4..-1]", ok(Value::string("asdf"))),
        ("'asdf'[-4..0]", ok(Value::string("asdfa"))),
        ("'asdf'[-4..1]", ok(Value::string("asdfas"))),
        ("'asdf'[-4..4]", ok(Value::string("asdfasdf"))),
        ("'asdf'[-4..]", ok(Value::string("asdfasdf"))),
        ("'asdf'[..-4]", ok(Value::string("fdsafdsa"))),
        (
            "'asdfxyz'[0, 1..3, 3..1, -1..-3]",
            ok(Value::string("asdffdszyx")),
        ),
        ("[1, 2, [4, 5]][2][1]", ok(5_i64)),
        ("[1, 2, 3][0]", ok(1_i64)),
        ("[1, 2, 3][-1]", ok(3_i64)),
        (
            "[1, 2, 3][0, 2]",
            ok(Value::array(vec![Value::integer(1), Value::integer(3)])),
        ),
        (
            "[1, 2, 3][2, 1, 0]",
            ok(Value::array(vec![
                Value::integer(3),
                Value::integer(2),
                Value::integer(1),
            ])),
        ),
        (
            "[1, 2, 3][0, -1]",
            ok(Value::array(vec![Value::integer(1), Value::integer(3)])),
        ),
        (
            "[1, 2, 3][0..1]",
            ok(Value::array(vec![Value::integer(1), Value::integer(2)])),
        ),
        (
            "[1, 2, 3][2..0]",
            ok(Value::array(vec![
                Value::integer(3),
                Value::integer(2),
                Value::integer(1),
            ])),
        ),
        (
            "[1, 2, 3][1..]",
            ok(Value::array(vec![Value::integer(2), Value::integer(3)])),
        ),
        (
            "[1, 2, 3][..1]",
            ok(Value::array(vec![Value::integer(3), Value::integer(2)])),
        ),
        (
            "[1, 2, 3][0, 1..2]",
            ok(Value::array(vec![
                Value::integer(1),
                Value::integer(2),
                Value::integer(3),
            ])),
        ),
        ("{a: 1, b: 2, c: 3}['a']", ok(1_i64)),
        ("{a: 1, b: 2, c: 3}['b']", ok(2_i64)),
        ("{a: 1, b: 2, c: 3}['c']", ok(3_i64)),
        (
            "{a: 1, b: 2, c: 3}['a', 'c']",
            ok(v_map(vec![
                ("a", Value::integer(1)),
                ("c", Value::integer(3)),
            ])),
        ),
        (
            "{a: 1, b: 2, c: 3}['c', 'a']",
            ok(v_map(vec![
                ("a", Value::integer(1)),
                ("c", Value::integer(3)),
            ])),
        ),
        (
            "{a: 1, b: 2, c: 3}['a', 'a']",
            ok(v_map(vec![("a", Value::integer(1))])),
        ),
        (
            "{a: 1, b: 2, c: 3}['a', 'd']",
            ok(v_map(vec![("a", Value::integer(1))])),
        ),
        ("{a: 1, b: 2, c: 3}['d', 'e']", ok(v_map(vec![]))),
        ("'asdf'[5]", ok(Value::string(""))),
        ("'asdf'[-5]", ok(Value::string(""))),
        ("'asdf'[4, 5]", ok(Value::string(""))),
        (
            "[1, 2, 3][-4]",
            err(
                r##"At line 1, column 10: Cannot evaluate expression '[1, 2, 3][-4]': 3 is out of bounds for '[1, 2, 3]'"##,
            ),
        ),
        (
            "[0, 1, 2, 3][5]",
            err(
                r##"At line 1, column 13: Cannot evaluate expression '[0, 1, 2, 3][5]': 4 is out of bounds for '[0, 1, 2, 3]'"##,
            ),
        ),
        (
            "[1, 2, 3][0, 5]",
            err(
                r##"At line 1, column 10: Cannot evaluate expression '[1, 2, 3][[0, 5]]': 3 is out of bounds for '[1, 2, 3]'"##,
            ),
        ),
        (
            "[1, 2, 3][0..5]",
            err(
                r##"At line 1, column 10: Cannot evaluate expression '[1, 2, 3][0..5]': 3 is out of bounds for '[1, 2, 3]'"##,
            ),
        ),
        ("{a: 1, b: 2, c: 3}['d']", Ok(Value::undefined())),
        (
            "'asdf'[undefined..1]",
            err(
                r##"At line 1, column 7: Cannot evaluate expression '"asdf"[undefined..1]': 'undefined' is not a compatible index for '"asdf"'"##,
            ),
        ),
        (
            "'asdf'[1..undefined]",
            err(
                r##"At line 1, column 7: Cannot evaluate expression '"asdf"[1..undefined]': 'undefined' is not a compatible index for '"asdf"'"##,
            ),
        ),
        (
            "{a: 1}['a', 1]",
            err(
                r##"At line 1, column 13: Cannot convert value '1' of type 'Number' to type 'String'"##,
            ),
        ),
        (
            "1[0]",
            err(
                r##"At line 1, column 2: Cannot evaluate expression '1[0]': '0' is not a compatible index for '1'"##,
            ),
        ),
        (
            "true[0]",
            err(
                r##"At line 1, column 5: Cannot evaluate expression 'true[0]': '0' is not a compatible index for 'true'"##,
            ),
        ),
        (
            "null[0]",
            err(
                r##"At line 1, column 5: Cannot evaluate expression 'null[0]': '0' is not a compatible index for 'null'"##,
            ),
        ),
        (
            "'asdf'['a']",
            err(
                r##"At line 1, column 7: Cannot evaluate expression '"asdf"["a"]': '"a"' is not a compatible index for '"asdf"'"##,
            ),
        ),
        (
            "{a: 1}[0]",
            err(
                r##"At line 1, column 7: Cannot evaluate expression '{ "a": 1 }[0]': '0' is not a compatible index for '{ "a": 1 }'"##,
            ),
        ),
    ];
    for (expr, expected) in cases {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}

#[test]
fn switch_() {
    let cases: Vec<(&str, Result<Value, String>)> = vec![
        (
            "\n    {{\n    true -> 1,\n    x -> 2\n    }}\n    ",
            ok(1_i64),
        ),
        (
            "\n    {{\n    x -> 2,\n    true -> 1\n    }}\n    ",
            ok(1_i64),
        ),
        ("\n    {{}}\n    ", Ok(Value::undefined())),
        (
            "\n    {{\n    x -> 2,\n    y -> 1\n    }}\n    ",
            Ok(Value::undefined()),
        ),
        ("\n    {{\n    false -> 1,\n    2\n    }}\n    ", ok(2_i64)),
    ];
    for (expr, expected) in cases {
        assert_eq!(evaluate(expr), expected, "{expr:?}");
    }
}

#[test]
fn operator_precedence() {
    let cases: Vec<(&str, Result<Value, String>)> = vec![
        ("1 + 2 - 3", ok(Value::number(1.0 + 2.0 - 3.0))),
        ("1 - 2 + 3", ok(Value::number(1.0 - 2.0 + 3.0))),
        ("2 * 3 + 4", ok(Value::number(2.0 * 3.0 + 4.0))),
        ("2 + 3 * 4", ok(Value::number(2.0 + 3.0 * 4.0))),
        ("2 * 3 - 4", ok(Value::number(2.0 * 3.0 - 4.0))),
        ("2 - 3 * 4", ok(Value::number(2.0 - 3.0 * 4.0))),
        ("6 / 2 + 4", ok(Value::number(6.0 / 2.0 + 4.0))),
        ("6 + 2 / 4", ok(Value::number(6.0 + 2.0 / 4.0))),
        ("6 / 2 - 4", ok(Value::number(6.0 / 2.0 - 4.0))),
        ("6 - 2 / 4", ok(Value::number(6.0 - 2.0 / 4.0))),
        ("2 * 6 / 4", ok(Value::number(2.0 * 6.0 / 4.0))),
        ("2 / 6 * 4", ok(Value::number(2.0 / 6.0 * 4.0))),
        ("2 + 3 * 4 + 5", ok(Value::integer(2 + 3 * 4 + 5))),
        ("2 * 3 + 4 + 5", ok(Value::integer(2 * 3 + 4 + 5))),
        ("2 * 3 + 4 & 5", ok(Value::integer(2 * 3 + 4 & 5))),
        ("7 + 2 * 3 + 2 * 2", ok(Value::integer(7 + 2 * 3 + 2 * 2))),
        (
            "7 + 2 / 3 + 2 * 2",
            ok(Value::number(7.0 + 2.0 / 3.0 + 2.0 * 2.0)),
        ),
        ("false && false || true", ok(true)),
        ("!true && !true || !false", ok(true)),
        ("3 < 10 || 10 > 2", ok(true)),
        (
            "3 + 2 < 3 + 3 + 0 && true",
            ok(((3 + 2) < (3 + 3 + 0)) && true),
        ),
        ("2 + 3 < 2 + 4", ok(true)),
        ("(2+1)*3", ok(Value::integer((2 + 1) * 3))),
        (
            "(2+1)*((1+1)*2)",
            ok(Value::integer((2 + 1) * ((1 + 1) * 2))),
        ),
        ("true && false -> true", Ok(Value::undefined())),
        ("true && true -> false", ok(false)),
        ("2 + 3 < 2 + 4 -> 6 % 5", ok(1_i64)),
    ];
    for (expr, expected) in cases {
        assert_eq!(evaluate(expr), expected, "{expr}");
    }
}

#[test]
fn try_evaluate_cases() {
    let cases: Vec<(&str, Vec<(&str, Value)>, Value)> = vec![
        ("1", vec![], Value::number(1.0)),
        ("a", vec![("a", Value::number(2.0))], Value::number(2.0)),
        ("1 + a", vec![("a", Value::number(2.0))], Value::number(3.0)),
        ("a", vec![], Value::undefined()),
        ("1 + a", vec![], Value::undefined()),
        (
            "[a, 1, 2]",
            vec![],
            Value::array(vec![
                Value::undefined(),
                Value::number(1.0),
                Value::number(2.0),
            ]),
        ),
        (
            "{a: 1, b: x, c: 3}",
            vec![],
            v_map(vec![
                ("a", Value::number(1.0)),
                ("b", Value::undefined()),
                ("c", Value::number(3.0)),
            ]),
        ),
        ("1 + 'a'", vec![], Value::undefined()),
        (
            "[1 + 'a', 2]",
            vec![],
            Value::array(vec![Value::undefined(), Value::number(2.0)]),
        ),
        (
            "{a: 1 + 'a'}",
            vec![],
            v_map(vec![("a", Value::undefined())]),
        ),
        ("1[0]", vec![], Value::undefined()),
    ];
    for (expr, vars, expected) in cases {
        assert_eq!(try_evaluate_with(expr, &vars), Ok(expected), "{expr}");
    }
}

#[test]
fn optimize() {
    let cases: Vec<(&str, expression_lang::ExpressionNode)> = vec![
        ("3 + 7", lit(10)),
        ("x == 1", eq(var("x"), lit(1))),
        (
            "[1, 2, 3]",
            lit(Value::array(vec![
                Value::integer(1),
                Value::integer(2),
                Value::integer(3),
            ])),
        ),
        ("[1 + 2, 2, a]", arr(vec![lit(3), lit(2), var("a")])),
        (
            "{a:1, b:2, c:3}",
            lit(v_map(vec![
                ("a", Value::integer(1)),
                ("b", Value::integer(2)),
                ("c", Value::integer(3)),
            ])),
        ),
        ("{{ true -> 1, x -> 2 }}", lit(1)),
        (
            "{{ x -> 2, true -> 1 }}",
            swt(vec![cs(var("x"), lit(2)), lit(1)]),
        ),
        (
            "{{ x == 1 -> 2, 1 }}",
            swt(vec![cs(eq(var("x"), lit(1)), lit(2)), lit(1)]),
        ),
        ("false && x", lit(false)),
        ("true || x", lit(true)),
        ("true && x", log_and(lit(true), var("x"))),
        ("false || x", log_or(lit(false), var("x"))),
        ("-1", lit(-1)),
        ("!true", lit(false)),
        ("-x", minus(var("x"))),
        ("'asdf'[1]", lit("s")),
        ("'asdf'[1..2]", lit("sd")),
        ("[1, 2, 3][1]", lit(2)),
        ("{a:1, b:2}['a']", lit(1)),
        ("x[1]", scr(var("x"), lit(1))),
        ("'asdf'[x]", scr(lit("asdf"), var("x"))),
        ("{a:1, b:x}", map_(vec![("a", lit(1)), ("b", var("x"))])),
        ("{{}}", lit(Value::undefined())),
    ];
    for (expr, expected) in cases {
        let mut ctx = EvaluationContext::new();
        let node = parse_expression(ParseMode::Strict, expr).unwrap();
        assert_eq!(node.optimize(&mut ctx).unwrap(), expected, "{expr}");
    }
}

#[test]
fn as_string() {
    let cases: Vec<(&str, &str)> = vec![
        ("+1", "+1"),
        ("-1", "-1"),
        ("!true", "!true"),
        ("~1", "~1"),
        ("(1)", "( 1 )"),
        ("'asdf'[1..]", "\"asdf\"[1..]"),
        ("'asdf'[..1]", "\"asdf\"[..1]"),
        ("1 + 2", "1 + 2"),
        ("1 - 2", "1 - 2"),
        ("1 * 2", "1 * 2"),
        ("1 / 2", "1 / 2"),
        ("1 % 2", "1 % 2"),
        ("true && false", "true && false"),
        ("true || false", "true || false"),
        ("1 & 2", "1 & 2"),
        ("1 ^ 2", "1 ^ 2"),
        ("1 | 2", "1 | 2"),
        ("1 << 2", "1 << 2"),
        ("1 >> 2", "1 >> 2"),
        ("1 < 2", "1 < 2"),
        ("1 <= 2", "1 <= 2"),
        ("1 > 2", "1 > 2"),
        ("1 >= 2", "1 >= 2"),
        ("1 == 2", "1 == 2"),
        ("1 != 2", "1 != 2"),
        ("'asdf'[1..2]", "\"asdf\"[1..2]"),
        ("true -> 1", "true -> 1"),
        ("x", "x"),
        ("[1, 2]", "[1, 2]"),
        ("{a: 1}", "{ \"a\": 1 }"),
        ("{{ x -> 1, 2 }}", "{{ x -> 1, 2 }}"),
    ];
    for (expr, expected) in cases {
        let node = parse_expression(ParseMode::Strict, expr).unwrap();
        assert_eq!(node.as_string(), expected, "{expr}");
    }
}

#[test]
fn operator_ne() {
    assert_ne!(lit(1), lit(2));
    assert_eq!(lit(1), lit(1));
    assert_ne!(var("a"), var("b"));
    assert_ne!(arr(vec![lit(1)]), arr(vec![lit(2)]));
    assert_ne!(map_(vec![("a", lit(1))]), map_(vec![("b", lit(1))]));
    assert_ne!(minus(lit(1)), plus(lit(1)));
    assert_ne!(add(lit(1), lit(2)), sub(lit(1), lit(2)));
    assert_ne!(scr(lit("ab"), lit(0)), scr(lit("ab"), lit(1)));
    assert_ne!(swt(vec![lit(1)]), swt(vec![lit(2)]));
    assert_ne!(minus(lit(1)), minus(lit(2)));
    assert_ne!(add(lit(1), lit(2)), add(lit(3), lit(2)));
    assert_ne!(add(lit(1), lit(2)), add(lit(1), lit(3)));
    assert_ne!(scr(lit("ab"), lit(0)), scr(lit("cd"), lit(0)));
}

#[test]
fn optimize_agrees_with_evaluate_for_every_binary_operator() {
    let exprs = [
        "1 + 2",
        "3 - 1",
        "2 * 3",
        "6 / 2",
        "7 % 3",
        "true && false",
        "true || false",
        "12 & 10",
        "12 ^ 10",
        "12 | 10",
        "1 << 3",
        "8 >> 2",
        "1 < 2",
        "1 <= 2",
        "1 > 2",
        "1 >= 2",
        "1 == 2",
        "1 != 2",
        "'asdf'[1..2]",
        "true -> 1",
    ];
    for expr in exprs {
        let mut ctx = EvaluationContext::new();
        let node = parse_expression(ParseMode::Strict, expr).unwrap();
        let optimized = node.optimize(&mut ctx).unwrap();
        let evaluated = node.evaluate(&mut ctx).unwrap();
        assert_eq!(optimized, lit(evaluated), "{expr}");
    }
}

#[test]
fn accept_preorder() {
    let pv = |s: &str| -> Vec<String> {
        let node = parse_expression(ParseMode::Strict, s).unwrap();
        let mut out = Vec::new();
        node.visit_preorder(&mut out);
        out
    };
    assert_eq!(pv("1"), vec!["1"]);
    assert_eq!(pv("a"), vec!["a"]);
    assert_eq!(pv("[1, 2]"), vec!["[1, 2]", "1", "2"]);
    assert_eq!(pv("{x:1, y:2}"), vec![r#"{ "x": 1, "y": 2 }"#, "1", "2"]);
    assert_eq!(pv("+1"), vec!["+1", "1"]);
    assert_eq!(pv("1 + 2"), vec!["1 + 2", "1", "2"]);
    assert_eq!(pv("x[1]"), vec!["x[1]", "x", "1"]);
    assert_eq!(pv("{{ x -> 1 }}"), vec!["{{ x -> 1 }}", "x -> 1", "x", "1"]);
}
