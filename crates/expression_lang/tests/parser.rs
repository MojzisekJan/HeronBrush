//! Tests for parser behavior.

mod common;

use common::*;
use expression_lang::{parse_expression, ExpressionNode, ParseMode, Parser};

fn parse(s: &str) -> Result<ExpressionNode, expression_lang::ElError> {
    parse_expression(ParseMode::Strict, s)
}

fn p(s: &str) -> ExpressionNode {
    parse(s).unwrap_or_else(|e| panic!("parse {s:?} failed: {e}"))
}

#[test]
fn empty_expression() {
    assert!(parse("").is_err());
    assert!(parse("    ").is_err());
    assert!(parse("\n").is_err());
}

#[test]
fn literals_strings() {
    assert!(parse(r#""asdf"#).is_err());
    assert_eq!(p(r#""asdf""#), lit("asdf"));
    assert_eq!(p("\"asdf\\\" \\\"asdf\""), lit(r#"asdf" "asdf"#));
}

#[test]
fn literals_numbers() {
    assert!(parse("1.123.34").is_err());
    assert_eq!(p("1"), lit(1.0));
    assert_eq!(p("1.0"), lit(1.0));
    assert_eq!(p("01.00"), lit(1.0));
    assert_eq!(p(".0"), lit(0.0));
    assert_eq!(p("0"), lit(0.0));
}

#[test]
fn literals_booleans() {
    assert_eq!(p("true"), lit(true));
    assert_eq!(p("false"), lit(false));
}

#[test]
fn literals_arrays() {
    assert_eq!(p("[]"), arr(vec![]));
    assert_eq!(
        p(r#"[ 1.0 , "test",[ true] ]"#),
        arr(vec![lit(1.0), lit("test"), arr(vec![lit(true)])])
    );
}

#[test]
fn literals_ranges() {
    assert_eq!(p("[1..3]"), arr(vec![b_rng(lit(1.0), lit(3.0))]));
    assert_eq!(p("[][1..3]"), scr(arr(vec![]), b_rng(lit(1.0), lit(3.0))));
    assert_eq!(p("[][..3]"), scr(arr(vec![]), rb_rng(lit(3.0))));
    assert_eq!(p("[][1..]"), scr(arr(vec![]), lb_rng(lit(1.0))));
}

#[test]
fn literals_maps() {
    assert_eq!(p("{}"), map_(vec![]));
    assert_eq!(
        p(r#" { "testkey1": 1, "testkey2"   :"asdf", "testkey3":{"nestedKey":true} }"#),
        map_(vec![
            ("testkey1", lit(1.0)),
            ("testkey2", lit("asdf")),
            ("testkey3", map_(vec![("nestedKey", lit(true))])),
        ])
    );
    assert_eq!(
        p(r#"[ { "key": "value" } ]"#),
        arr(vec![map_(vec![("key", lit("value"))])])
    );
    assert_eq!(
        p(r#"{ "outerkey1": [ { "key": "value" } ], "outerkey2": "asdf" }"#),
        map_(vec![
            ("outerkey1", arr(vec![map_(vec![("key", lit("value"))])])),
            ("outerkey2", lit("asdf")),
        ])
    );
    assert!(parse("{\n  \"profiles\": [],\n  \"version\": 1\n}\nasdf").is_err());
}

#[test]
fn variables() {
    assert_eq!(p("test"), var("test"));
}

#[test]
fn unary_operators() {
    assert_eq!(p("+1.0"), plus(lit(1)));
    assert_eq!(p("-1.0"), minus(lit(1)));
    assert_eq!(p("!true"), log_neg(lit(true)));
    assert_eq!(p("!false"), log_neg(lit(false)));
    assert_eq!(p("!0"), log_neg(lit(0)));
    assert_eq!(p("~393"), bit_neg(lit(393)));
    assert!(parse("~").is_err());
    assert!(parse("~~").is_err());
}

#[test]
fn binary_operators() {
    assert_eq!(p("2 + 3"), add(lit(2.0), lit(3.0)));
    assert_eq!(p("\"as\"+\"df\""), add(lit("as"), lit("df")));
    assert_eq!(p("2 + 3 + 4"), add(add(lit(2.0), lit(3.0)), lit(4.0)));

    assert_eq!(p("2-3.0"), sub(lit(2.0), lit(3.0)));
    assert_eq!(p("2-3 -  4"), sub(sub(lit(2.0), lit(3.0)), lit(4.0)));
    assert_eq!(
        p("2-3-4-2"),
        sub(sub(sub(lit(2.0), lit(3.0)), lit(4.0)), lit(2.0))
    );

    assert_eq!(p("2 * 3.0"), mul(lit(2.0), lit(3.0)));
    assert_eq!(p("2 * 3 * 4"), mul(mul(lit(2.0), lit(3.0)), lit(4.0)));

    assert_eq!(p("12 / 2.0"), div(lit(12.0), lit(2.0)));
    assert_eq!(p("12 / 2 / 2"), div(div(lit(12.0), lit(2.0)), lit(2.0)));

    assert_eq!(p("12 % 2.0"), r#mod(lit(12.0), lit(2.0)));
    assert_eq!(p("12 % 5 % 3"), r#mod(r#mod(lit(12.0), lit(5.0)), lit(3.0)));

    assert_eq!(p("true && true"), log_and(lit(true), lit(true)));
    assert_eq!(p("true || true"), log_or(lit(true), lit(true)));
    assert_eq!(p("23 & 24"), bit_and(lit(23), lit(24)));
    assert_eq!(p("23 | 24"), bit_or(lit(23), lit(24)));
    assert_eq!(p("23 ^ 24"), bit_xor(lit(23), lit(24)));
    assert!(parse("23 ^^ 23").is_err());
    assert_eq!(p("1 << 7"), bit_shl(lit(1), lit(7)));
    assert_eq!(p("8 >> 2"), bit_shr(lit(8), lit(2)));
    assert_eq!(p("true -> 1"), cs(lit(true), lit(1)));

    assert_eq!(p("1 < 2"), ls(lit(1), lit(2)));
    assert_eq!(p("1 <= 2"), ls_eq(lit(1), lit(2)));
    assert_eq!(p("1 > 2"), gr(lit(1), lit(2)));
    assert_eq!(p("1 >= 2"), gr_eq(lit(1), lit(2)));
    assert_eq!(p("1 == 2"), eq(lit(1), lit(2)));
    assert_eq!(p("1 != 2"), neq(lit(1), lit(2)));
}

#[test]
fn operator_combinations() {
    assert_eq!(p("1 + 2 * 3"), add(lit(1), mul(lit(2), lit(3))));
    assert_eq!(p("1 * 2 + 3"), add(mul(lit(1), lit(2)), lit(3)));
    assert_eq!(p("1 + 2 * 3 + 2"), add(add(lit(1), mul(lit(2), lit(3))), lit(2)));
    assert_eq!(
        p("1 + 2 * 3 + 2 * 2"),
        add(add(lit(1), mul(lit(2), lit(3))), mul(lit(2), lit(2)))
    );
    assert_eq!(p("3 + 2 < 3 + 3"), ls(add(lit(3), lit(2)), add(lit(3), lit(3))));
    assert_eq!(
        p("3 + 2 < 3 + 3 + 0 && true"),
        log_and(
            ls(add(lit(3), lit(2)), add(add(lit(3), lit(3)), lit(0))),
            lit(true)
        )
    );
    assert_eq!(
        p("false && false || true"),
        log_or(log_and(lit(false), lit(false)), lit(true))
    );
    assert_eq!(
        p("false && (false || true)"),
        log_and(lit(false), grp(log_or(lit(false), lit(true))))
    );
}

#[test]
fn subscript() {
    assert_eq!(
        p(r#"[ 1.0, 2.0, "test" ][0]"#),
        scr(arr(vec![lit(1.0), lit(2.0), lit("test")]), lit(0))
    );
    assert_eq!(
        p(r#"[ 1.0, 2.0, "test" ][1+1]"#),
        scr(arr(vec![lit(1.0), lit(2.0), lit("test")]), add(lit(1), lit(1)))
    );
    assert_eq!(
        p(r#"{ "key1":1, "key2":2, "key3":"test"}["key1"]"#),
        scr(
            map_(vec![
                ("key1", lit(1.0)),
                ("key2", lit(2.0)),
                ("key3", lit("test")),
            ]),
            lit("key1")
        )
    );
    assert_eq!(
        p(r#"[ 1.0, [ 2.0, "test"] ][1][0]"#),
        scr(
            scr(arr(vec![lit(1.0), arr(vec![lit(2.0), lit("test")])]), lit(1)),
            lit(0)
        )
    );
    assert_eq!(
        p(r#"[ 1.0, 2.0, "test" ][0,1,2]"#),
        scr(
            arr(vec![lit(1.0), lit(2.0), lit("test")]),
            arr(vec![lit(0), lit(1), lit(2)])
        )
    );
    assert_eq!(
        p(r#"[ 1.0, 2.0, "test" ][0..2]"#),
        scr(arr(vec![lit(1.0), lit(2.0), lit("test")]), b_rng(lit(0), lit(2)))
    );
    assert_eq!(
        p(r#"[ 1.0, 2.0, "test" ][0..2,3]"#),
        scr(
            arr(vec![lit(1.0), lit(2.0), lit("test")]),
            arr(vec![b_rng(lit(0), lit(2)), lit(3)])
        )
    );
    assert_eq!(p(r#"[ 1.0 ][]"#), scr(arr(vec![lit(1.0)]), arr(vec![])));
}

#[test]
fn switch() {
    assert_eq!(p("{{}}"), swt(vec![]));
    assert_eq!(p("{{'asdf'}}"), swt(vec![lit("asdf")]));
    assert_eq!(p("{{'fdsa', 'asdf'}}"), swt(vec![lit("fdsa"), lit("asdf")]));
    assert_eq!(
        p("{{false -> 'fdsa', 'asdf'}}"),
        swt(vec![cs(lit(false), lit("fdsa")), lit("asdf")])
    );
}

#[test]
fn comments() {
    assert_eq!(p("1 // comment"), lit(1));
    assert_eq!(p("1 // comment\n"), lit(1));
    assert_eq!(p("// comment\n1"), lit(1));
    assert_eq!(p("1 + // comment\n2"), add(lit(1), lit(2)));
    assert!(parse("// comment").is_err());
    assert_eq!(p("4 / 2"), div(lit(4), lit(2)));
}

#[test]
fn whitespace() {
    assert_eq!(p("\t1"), lit(1));
    assert_eq!(p("\r1"), lit(1));
    assert_eq!(p("1\t+\r2"), add(lit(1), lit(2)));
    assert!(parse("\t\r\n ").is_err());
}

#[test]
fn names() {
    assert_eq!(p("_a"), var("_a"));
    assert_eq!(p("a_b"), var("a_b"));
    assert_eq!(p("a1"), var("a1"));
    assert_eq!(p("_1"), var("_1"));
    assert_eq!(p("a_1b"), var("a_1b"));
}

#[test]
fn malformed_input() {
    assert!(parse("1.2.3").is_err());
    assert!(parse("$").is_err());
    assert!(parse("1 = 2").is_err());
    assert!(parse("1 =2").is_err());
    assert!(parse("1 =+ 2").is_err());
    assert!(parse("1 =* 2").is_err());
    assert!(parse("=").is_err());

    assert_eq!(p("1 == 2"), eq(lit(1), lit(2)));
    assert_eq!(p("1 ==2"), eq(lit(1), lit(2)));
    assert_eq!(p("a==b"), eq(var("a"), var("b")));
}

#[test]
fn numbers_need_not_be_separated_from_the_operator() {
    assert_eq!(p("1+2"), add(lit(1), lit(2)));
    assert_eq!(p("1-2"), sub(lit(1), lit(2)));
    assert_eq!(p("1*2"), mul(lit(1), lit(2)));
    assert_eq!(p("1/2"), div(lit(1), lit(2)));
    assert_eq!(p("1%2"), r#mod(lit(1), lit(2)));
    assert_eq!(p("1==2"), eq(lit(1), lit(2)));
    assert_eq!(p("1!=2"), neq(lit(1), lit(2)));
    assert_eq!(p("1<2"), ls(lit(1), lit(2)));
    assert_eq!(p("1<=2"), ls_eq(lit(1), lit(2)));
    assert_eq!(p("1>2"), gr(lit(1), lit(2)));
    assert_eq!(p("1>=2"), gr_eq(lit(1), lit(2)));
    assert_eq!(p("1&2"), bit_and(lit(1), lit(2)));
    assert_eq!(p("1|2"), bit_or(lit(1), lit(2)));
    assert_eq!(p("1^2"), bit_xor(lit(1), lit(2)));
    assert_eq!(p("1<<2"), bit_shl(lit(1), lit(2)));
    assert_eq!(p("1>>2"), bit_shr(lit(1), lit(2)));
    assert_eq!(p("1&&2"), log_and(lit(1), lit(2)));
    assert_eq!(p("1||2"), log_or(lit(1), lit(2)));
    assert_eq!(p("1->2"), cs(lit(1), lit(2)));

    assert_eq!(p("1.5<2"), ls(lit(1.5), lit(2)));
    assert_eq!(p("1e2<2"), ls(lit(100.0), lit(2)));
    assert_eq!(p("1e-2<2"), ls(lit(0.01), lit(2)));

    assert_eq!(p("[1,2][0..1]"), scr(arr(vec![lit(1), lit(2)]), b_rng(lit(0), lit(1))));
}

#[test]
fn tokenizer_state() {
    let mut parser = Parser::new(ParseMode::Lenient, "1 + 2 ) rest");
    assert_eq!(parser.parse().unwrap(), add(lit(1), lit(2)));

    let state = parser.tokenizer_state();
    assert_eq!(state.line, 1);
    assert_eq!(state.column, 6);
    assert_eq!(state.cur, " ) rest");
}

#[test]
fn groups() {
    assert!(parse("()").is_err());
    assert_eq!(p("(1)"), grp(lit(1)));
    assert_eq!(p("(2+1)*3"), mul(grp(add(lit(2), lit(1))), lit(3)));
    assert_eq!(
        p("(2+1)*(2+1)"),
        mul(grp(add(lit(2), lit(1))), grp(add(lit(2), lit(1))))
    );
    assert_eq!(
        p("(2+1)*((1+1)*2)"),
        mul(
            grp(add(lit(2), lit(1))),
            grp(mul(grp(add(lit(1), lit(1))), lit(2)))
        )
    );
}
