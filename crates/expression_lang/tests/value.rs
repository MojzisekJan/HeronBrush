//! Tests for value behavior.

use std::collections::BTreeMap;

use expression_lang::{
    BoundedRange, ElError, EvaluationContext, LeftBoundedRange, Range, RightBoundedRange, Value,
    ValueType,
};

fn ctx() -> EvaluationContext {
    EvaluationContext::new()
}

fn v_arr(items: Vec<Value>) -> Value {
    Value::array(items)
}

fn v_map(pairs: Vec<(&str, Value)>) -> Value {
    let m: BTreeMap<String, Value> = pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    Value::map(m)
}

fn bounded_range() -> Value {
    Value::range(Range::Bounded(BoundedRange { first: 1, last: 3 }))
}
fn left_bounded_range() -> Value {
    Value::range(Range::Left(LeftBoundedRange { first: 2 }))
}
fn right_bounded_range() -> Value {
    Value::range(Range::Right(RightBoundedRange { last: 5 }))
}

const ALL_TYPES: [ValueType; 8] = [
    ValueType::Boolean,
    ValueType::String,
    ValueType::Number,
    ValueType::Array,
    ValueType::Map,
    ValueType::Range,
    ValueType::Null,
    ValueType::Undefined,
];

fn convertible_types(value: &Value) -> Vec<ValueType> {
    ALL_TYPES
        .into_iter()
        .filter(|t| value.convertible_to(*t))
        .collect()
}

fn is_deref(r: &Result<impl std::fmt::Debug, ElError>) -> bool {
    matches!(r, Err(ElError::Dereference(_)))
}
fn is_conv(r: &Result<impl std::fmt::Debug, ElError>) -> bool {
    matches!(r, Err(ElError::Conversion(_)))
}

#[test]
fn constructor() {
    assert_eq!(Value::null().value_type(), ValueType::Null);
    assert_eq!(Value::boolean(true).value_type(), ValueType::Boolean);
    assert_eq!(Value::boolean(false).value_type(), ValueType::Boolean);
    assert_eq!(Value::string("test").value_type(), ValueType::String);
    assert_eq!(Value::number(1.0).value_type(), ValueType::Number);
    assert_eq!(Value::integer(1).value_type(), ValueType::Number);
    assert_eq!(Value::array(vec![]).value_type(), ValueType::Array);
    assert_eq!(Value::map(BTreeMap::new()).value_type(), ValueType::Map);
    assert_eq!(bounded_range().value_type(), ValueType::Range);
    assert_eq!(Value::undefined().value_type(), ValueType::Undefined);

    assert_eq!(Value::integer(1), Value::number(1.0));
}

#[test]
fn has_type() {
    assert!(Value::number(1.0).has_type(&[ValueType::Number]));
    assert!(!Value::number(1.0).has_type(&[ValueType::String]));
    assert!(Value::number(1.0).has_type(&[ValueType::String, ValueType::Number]));
    assert!(!Value::number(1.0).has_type(&[ValueType::String, ValueType::Boolean]));
}

#[test]
fn type_name() {
    assert_eq!(Value::boolean(true).type_name(), "Boolean");
    assert_eq!(Value::string("test").type_name(), "String");
    assert_eq!(Value::number(1.0).type_name(), "Number");
    assert_eq!(Value::array(vec![]).type_name(), "Array");
    assert_eq!(Value::map(BTreeMap::new()).type_name(), "Map");
    assert_eq!(bounded_range().type_name(), "Range");
    assert_eq!(Value::null().type_name(), "Null");
    assert_eq!(Value::undefined().type_name(), "Undefined");
}

#[test]
fn describe() {
    assert_eq!(Value::boolean(true).describe(), "true");
    assert_eq!(Value::string("test").describe(), r#""test""#);
    assert_eq!(Value::number(1.0).describe(), "1");
    assert_eq!(
        v_arr(vec![Value::number(1.0), Value::number(2.0)]).describe(),
        "[1, 2]"
    );
    assert_eq!(
        v_map(vec![("key", Value::number(1.0))]).describe(),
        r#"{ "key": 1 }"#
    );
}

#[test]
fn boolean_value() {
    let c = ctx();
    assert_eq!(Value::boolean(true).boolean_value(&c), Ok(true));
    assert_eq!(Value::boolean(false).boolean_value(&c), Ok(false));
    assert_eq!(Value::null().boolean_value(&c), Ok(false));

    assert!(is_deref(&Value::string("test").boolean_value(&c)));
    assert!(is_deref(&Value::number(1.0).boolean_value(&c)));
    assert!(is_deref(&Value::array(vec![]).boolean_value(&c)));
    assert!(is_deref(&Value::map(BTreeMap::new()).boolean_value(&c)));
    assert!(is_deref(&bounded_range().boolean_value(&c)));
    assert!(is_deref(&Value::undefined().boolean_value(&c)));

    assert_eq!(
        Value::number(1.0)
            .boolean_value(&c)
            .map_err(|e| e.into_message()),
        Err(
            "At unknown location: Cannot dereference value '1' of type 'Number' as type 'Boolean'"
                .to_string()
        )
    );
}

#[test]
fn string_value() {
    let c = ctx();
    assert_eq!(
        Value::string("test").string_value(&c),
        Ok("test".to_string())
    );
    assert_eq!(Value::null().string_value(&c), Ok(String::new()));

    assert!(is_deref(&Value::boolean(true).string_value(&c)));
    assert!(is_deref(&Value::number(1.0).string_value(&c)));
    assert!(is_deref(&Value::array(vec![]).string_value(&c)));
    assert!(is_deref(&bounded_range().string_value(&c)));
    assert!(is_deref(&Value::undefined().string_value(&c)));

    assert_eq!(
        Value::number(1.0)
            .string_value(&c)
            .map_err(|e| e.into_message()),
        Err(
            "At unknown location: Cannot dereference value '1' of type 'Number' as type 'String'"
                .to_string()
        )
    );
}

#[test]
fn number_value() {
    let c = ctx();
    assert_eq!(Value::number(1.5).number_value(&c), Ok(1.5));
    assert_eq!(Value::null().number_value(&c), Ok(0.0));

    assert!(is_deref(&Value::boolean(true).number_value(&c)));
    assert!(is_deref(&Value::string("test").number_value(&c)));
    assert!(is_deref(&bounded_range().number_value(&c)));
    assert!(is_deref(&Value::undefined().number_value(&c)));

    assert_eq!(
        Value::string("test").number_value(&c).map_err(|e| e.into_message()),
        Err(r#"At unknown location: Cannot dereference value '"test"' of type 'String' as type 'Number'"#.to_string())
    );
}

#[test]
fn integer_value() {
    let c = ctx();
    assert_eq!(Value::number(1.0).integer_value(&c), Ok(1));
    assert_eq!(Value::number(1.7).integer_value(&c), Ok(1));
    assert_eq!(Value::number(-1.7).integer_value(&c), Ok(-1));
    assert_eq!(Value::null().integer_value(&c), Ok(0));
    assert!(is_deref(&Value::string("test").integer_value(&c)));
}

#[test]
fn array_value() {
    let c = ctx();
    assert_eq!(
        v_arr(vec![Value::number(1.0)]).array_value(&c),
        Ok(vec![Value::number(1.0)])
    );
    assert_eq!(Value::null().array_value(&c), Ok(vec![]));
    assert!(is_deref(&Value::boolean(true).array_value(&c)));
    assert!(is_deref(&Value::string("test").array_value(&c)));
    assert!(is_deref(&Value::map(BTreeMap::new()).array_value(&c)));
    assert!(is_deref(&Value::undefined().array_value(&c)));
}

#[test]
fn map_value() {
    let c = ctx();
    assert_eq!(
        v_map(vec![("key", Value::number(1.0))]).map_value(&c),
        Ok([("key".to_string(), Value::number(1.0))]
            .into_iter()
            .collect())
    );
    assert_eq!(Value::null().map_value(&c), Ok(BTreeMap::new()));
    assert!(is_deref(&Value::boolean(true).map_value(&c)));
    assert!(is_deref(&Value::array(vec![]).map_value(&c)));
    assert!(is_deref(&Value::undefined().map_value(&c)));
}

#[test]
fn range_value() {
    let c = ctx();
    assert_eq!(
        bounded_range().range_value(&c),
        Ok(Range::Bounded(BoundedRange { first: 1, last: 3 }))
    );
    assert_eq!(
        left_bounded_range().range_value(&c),
        Ok(Range::Left(LeftBoundedRange { first: 2 }))
    );
    assert_eq!(
        right_bounded_range().range_value(&c),
        Ok(Range::Right(RightBoundedRange { last: 5 }))
    );
    assert!(is_deref(&Value::boolean(true).range_value(&c)));
    assert!(is_deref(&Value::string("test").range_value(&c)));
    // unlike the other accessors, a range cannot be dereferenced from null
    assert!(is_deref(&Value::null().range_value(&c)));
    assert!(is_deref(&Value::undefined().range_value(&c)));
}

#[test]
fn as_string_list_and_set() {
    let c = ctx();
    assert_eq!(Value::array(vec![]).as_string_list(&c), Ok(vec![]));
    assert_eq!(Value::null().as_string_list(&c), Ok(vec![]));
    assert_eq!(
        v_arr(vec![
            Value::string("b"),
            Value::string("a"),
            Value::string("b"),
            Value::null()
        ])
        .as_string_list(&c),
        Ok(vec!["b".to_string(), "a".into(), "b".into(), "".into()])
    );
    assert!(is_deref(
        &v_arr(vec![Value::number(1.0)]).as_string_list(&c)
    ));
    assert!(is_deref(&Value::string("test").as_string_list(&c)));

    assert_eq!(
        v_arr(vec![
            Value::string("b"),
            Value::string("a"),
            Value::string("b")
        ])
        .as_string_set(&c),
        Ok(vec!["a".to_string(), "b".into()])
    );
}

#[test]
fn length() {
    assert_eq!(Value::boolean(true).length(), 1);
    assert_eq!(Value::string("").length(), 0);
    assert_eq!(Value::string("test").length(), 4);
    assert_eq!(Value::number(1.0).length(), 1);
    assert_eq!(Value::array(vec![]).length(), 0);
    assert_eq!(
        v_arr(vec![Value::number(1.0), Value::number(2.0)]).length(),
        2
    );
    assert_eq!(Value::map(BTreeMap::new()).length(), 0);
    assert_eq!(v_map(vec![("key", Value::number(1.0))]).length(), 1);
    assert_eq!(bounded_range().length(), 2);
    assert_eq!(Value::null().length(), 0);
    assert_eq!(Value::undefined().length(), 0);
}

#[test]
fn convertible_to() {
    use ValueType::*;
    assert_eq!(
        convertible_types(&Value::boolean(true)),
        vec![Boolean, String, Number]
    );
    assert_eq!(
        convertible_types(&Value::string("test")),
        vec![Boolean, String]
    );
    assert_eq!(
        convertible_types(&Value::string("2.0")),
        vec![Boolean, String, Number]
    );
    assert_eq!(
        convertible_types(&Value::string(" ")),
        vec![Boolean, String, Number]
    );
    assert_eq!(
        convertible_types(&Value::number(1.0)),
        vec![Boolean, String, Number]
    );
    assert_eq!(convertible_types(&Value::array(vec![])), vec![Array]);
    assert_eq!(convertible_types(&Value::map(BTreeMap::new())), vec![Map]);
    assert_eq!(convertible_types(&bounded_range()), vec![Range]);
    assert_eq!(
        convertible_types(&Value::null()),
        vec![Boolean, String, Number, Array, Map, Null]
    );
    assert_eq!(convertible_types(&Value::undefined()), vec![Undefined]);
}

#[test]
fn convert_to() {
    use ValueType::*;
    let mut c = ctx();
    let mut conv = |v: Value, t: ValueType| v.convert_to(&mut c, t);

    assert_eq!(
        conv(Value::boolean(true), Boolean),
        Ok(Value::boolean(true))
    );
    assert_eq!(
        conv(Value::boolean(true), String),
        Ok(Value::string("true"))
    );
    assert_eq!(
        conv(Value::boolean(false), String),
        Ok(Value::string("false"))
    );
    assert_eq!(conv(Value::boolean(true), Number), Ok(Value::integer(1)));
    assert_eq!(conv(Value::boolean(false), Number), Ok(Value::integer(0)));
    assert!(is_conv(&conv(Value::boolean(true), Array)));
    assert!(is_conv(&conv(Value::boolean(true), Map)));
    assert!(is_conv(&conv(Value::boolean(true), Range)));
    assert!(is_conv(&conv(Value::boolean(true), Null)));
    assert!(is_conv(&conv(Value::boolean(true), Undefined)));

    assert_eq!(
        conv(Value::string("asdf"), Boolean),
        Ok(Value::boolean(true))
    );
    assert_eq!(
        conv(Value::string("false"), Boolean),
        Ok(Value::boolean(false))
    );
    assert_eq!(conv(Value::string(""), Boolean), Ok(Value::boolean(false)));
    assert_eq!(
        conv(Value::string("asdf"), String),
        Ok(Value::string("asdf"))
    );
    assert_eq!(conv(Value::string("2"), Number), Ok(Value::integer(2)));
    assert_eq!(conv(Value::string("-2.0"), Number), Ok(Value::integer(-2)));
    assert_eq!(conv(Value::string(" "), Number), Ok(Value::integer(0)));
    assert!(is_conv(&conv(Value::string("asdf"), Number)));
    assert!(is_conv(&conv(Value::string("asdf"), Array)));
    assert!(is_conv(&conv(Value::string("asdf"), Map)));
    assert!(is_conv(&conv(Value::string("asdf"), Range)));
    assert!(is_conv(&conv(Value::string("asdf"), Null)));
    assert!(is_conv(&conv(Value::string("asdf"), Undefined)));

    assert_eq!(conv(Value::integer(1), Boolean), Ok(Value::boolean(true)));
    assert_eq!(conv(Value::integer(-2), Boolean), Ok(Value::boolean(true)));
    assert_eq!(conv(Value::integer(0), Boolean), Ok(Value::boolean(false)));
    assert_eq!(conv(Value::number(1.0), String), Ok(Value::string("1")));
    assert_eq!(conv(Value::number(-1.0), String), Ok(Value::string("-1")));
    assert_eq!(
        conv(Value::number(1.1), String),
        Ok(Value::string("1.1000000000000001"))
    );
    assert_eq!(
        conv(Value::number(-1.1), String),
        Ok(Value::string("-1.1000000000000001"))
    );
    assert_eq!(conv(Value::number(1.0), Number), Ok(Value::integer(1)));
    assert!(is_conv(&conv(Value::integer(1), Array)));
    assert!(is_conv(&conv(Value::integer(2), Map)));
    assert!(is_conv(&conv(Value::integer(3), Range)));
    assert!(is_conv(&conv(Value::integer(4), Null)));
    assert!(is_conv(&conv(Value::integer(5), Undefined)));

    assert!(is_conv(&conv(Value::array(vec![]), Boolean)));
    assert!(is_conv(&conv(Value::array(vec![]), String)));
    assert!(is_conv(&conv(Value::array(vec![]), Number)));
    assert_eq!(conv(Value::array(vec![]), Array), Ok(Value::array(vec![])));
    assert!(is_conv(&conv(Value::array(vec![]), Map)));

    assert!(is_conv(&conv(Value::map(BTreeMap::new()), Boolean)));
    assert_eq!(
        conv(Value::map(BTreeMap::new()), Map),
        Ok(Value::map(BTreeMap::new()))
    );

    assert!(is_conv(&conv(bounded_range(), Boolean)));
    assert_eq!(conv(bounded_range(), Range), Ok(bounded_range()));

    assert_eq!(conv(Value::null(), Boolean), Ok(Value::boolean(false)));
    assert_eq!(conv(Value::null(), String), Ok(Value::string("")));
    assert_eq!(conv(Value::null(), Number), Ok(Value::integer(0)));
    assert_eq!(conv(Value::null(), Array), Ok(Value::array(vec![])));
    assert_eq!(conv(Value::null(), Map), Ok(Value::map(BTreeMap::new())));
    assert!(is_conv(&conv(Value::null(), Range)));
    assert_eq!(conv(Value::null(), Null), Ok(Value::null()));
    assert!(is_conv(&conv(Value::null(), Undefined)));

    assert!(is_conv(&conv(Value::undefined(), Boolean)));
    assert_eq!(conv(Value::undefined(), Undefined), Ok(Value::undefined()));
}

#[test]
fn try_convert_to() {
    let mut c = ctx();
    assert_eq!(
        Value::string("2").try_convert_to(&mut c, ValueType::Number),
        Ok(Some(Value::integer(2)))
    );
    assert_eq!(
        Value::string("asdf").try_convert_to(&mut c, ValueType::Number),
        Ok(None)
    );
    assert_eq!(
        Value::array(vec![]).try_convert_to(&mut c, ValueType::Map),
        Ok(None)
    );
}

#[test]
fn as_string_single_line() {
    assert_eq!(Value::boolean(true).as_string(false), "true");
    assert_eq!(Value::boolean(false).as_string(false), "false");
    assert_eq!(Value::string("test").as_string(false), r#""test""#);
    assert_eq!(Value::string("a\"b\\c").as_string(false), "\"a\\\"b\\\\c\"");
    assert_eq!(Value::number(16.0).as_string(false), "16");
    assert_eq!(Value::number(-16.0).as_string(false), "-16");
    assert_eq!(Value::number(1.1).as_string(false), "1.1000000000000001");
    assert_eq!(Value::array(vec![]).as_string(false), "[]");
    assert_eq!(v_arr(vec![Value::number(1.0)]).as_string(false), "[1]");
    assert_eq!(
        v_arr(vec![Value::number(1.0), Value::string("a")]).as_string(false),
        r#"[1, "a"]"#
    );
    assert_eq!(
        v_arr(vec![v_arr(vec![Value::number(1.0)])]).as_string(false),
        "[[1]]"
    );
    assert_eq!(Value::map(BTreeMap::new()).as_string(false), "{}");
    assert_eq!(
        v_map(vec![("a", Value::number(1.0))]).as_string(false),
        r#"{ "a": 1 }"#
    );
    assert_eq!(
        v_map(vec![("a", Value::number(1.0)), ("b", Value::number(2.0))]).as_string(false),
        r#"{ "a": 1, "b": 2 }"#
    );
    assert_eq!(
        v_map(vec![("a", v_map(vec![("b", Value::number(1.0))]))]).as_string(false),
        r#"{ "a": { "b": 1 } }"#
    );
    assert_eq!(bounded_range().as_string(false), "[1..3]");
    assert_eq!(left_bounded_range().as_string(false), "[2..]");
    assert_eq!(right_bounded_range().as_string(false), "[..5]");
    assert_eq!(Value::null().as_string(false), "null");
    assert_eq!(Value::undefined().as_string(false), "undefined");
}

#[test]
fn as_string_multiline() {
    assert_eq!(Value::boolean(true).as_string(true), "true");
    assert_eq!(Value::array(vec![]).as_string(true), "[]");
    assert_eq!(
        v_arr(vec![Value::number(1.0), Value::number(2.0)]).as_string(true),
        "[\n\t1,\n\t2\n]"
    );
    assert_eq!(
        v_arr(vec![v_arr(vec![Value::number(1.0)])]).as_string(true),
        "[\n\t[\n\t\t1\n\t]\n]"
    );
    assert_eq!(Value::map(BTreeMap::new()).as_string(true), "{}");
    assert_eq!(
        v_map(vec![("a", Value::number(1.0)), ("b", Value::number(2.0))]).as_string(true),
        "{\n\t\"a\": 1,\n\t\"b\": 2\n}"
    );
    assert_eq!(
        v_map(vec![("a", v_map(vec![("b", Value::number(1.0))]))]).as_string(true),
        "{\n\t\"a\": {\n\t\t\"b\": 1\n\t}\n}"
    );
}

#[test]
fn append_to_stream() {
    let mut s = String::from("prefix ");
    v_arr(vec![Value::number(1.0), Value::number(2.0)]).append_to_stream(&mut s, true, "");
    assert_eq!(s, "prefix [\n\t1,\n\t2\n]");
}

#[test]
fn contains_by_index() {
    assert!(Value::string("ab").contains_index(0));
    assert!(Value::string("ab").contains_index(1));
    assert!(!Value::string("ab").contains_index(2));
    assert!(!Value::string("").contains_index(0));

    assert!(v_arr(vec![Value::number(1.0)]).contains_index(0));
    assert!(!v_arr(vec![Value::number(1.0)]).contains_index(1));
    assert!(!Value::array(vec![]).contains_index(0));

    assert!(!v_map(vec![("0", Value::number(1.0))]).contains_index(0));
    assert!(!Value::boolean(true).contains_index(0));
    assert!(!Value::number(1.0).contains_index(0));
    assert!(!bounded_range().contains_index(0));
    assert!(!Value::null().contains_index(0));
    assert!(!Value::undefined().contains_index(0));
}

#[test]
fn contains_by_key() {
    let c = ctx();
    assert_eq!(
        v_map(vec![("a", Value::number(1.0))]).contains_key(&c, "a"),
        Ok(true)
    );
    assert_eq!(
        v_map(vec![("a", Value::number(1.0))]).contains_key(&c, "b"),
        Ok(false)
    );
    assert_eq!(Value::map(BTreeMap::new()).contains_key(&c, "a"), Ok(false));
    assert_eq!(Value::null().contains_key(&c, "a"), Ok(false));

    assert!(is_deref(&Value::string("ab").contains_key(&c, "a")));
    assert!(is_deref(&Value::array(vec![]).contains_key(&c, "a")));
    assert!(is_deref(&Value::undefined().contains_key(&c, "a")));
}

#[test]
fn keys() {
    let c = ctx();
    assert_eq!(
        v_map(vec![("b", Value::number(1.0)), ("a", Value::number(2.0))]).keys(&c),
        Ok(vec!["a".to_string(), "b".into()])
    );
    assert_eq!(Value::map(BTreeMap::new()).keys(&c), Ok(vec![]));
    assert_eq!(Value::null().keys(&c), Ok(vec![]));
    assert!(is_deref(&Value::array(vec![]).keys(&c)));
}

#[test]
fn at_by_index() {
    let c = ctx();
    assert_eq!(Value::string("abc").at_index(&c, 0), Ok(Value::string("a")));
    assert_eq!(Value::string("abc").at_index(&c, 2), Ok(Value::string("c")));
    assert!(Value::string("abc")
        .at_index(&c, 3)
        .unwrap_err()
        .message()
        .contains("out of bounds"));

    let array = v_arr(vec![Value::number(1.0), Value::string("a")]);
    assert_eq!(array.at_index(&c, 0), Ok(Value::number(1.0)));
    assert_eq!(array.at_index(&c, 1), Ok(Value::string("a")));
    assert!(array
        .at_index(&c, 2)
        .unwrap_err()
        .message()
        .contains("out of bounds"));

    for v in [
        Value::map(BTreeMap::new()),
        Value::boolean(true),
        Value::number(1.0),
        bounded_range(),
        Value::null(),
        Value::undefined(),
    ] {
        assert!(v
            .at_index(&c, 0)
            .unwrap_err()
            .message()
            .contains("not a compatible index"));
    }
}

#[test]
fn at_by_key() {
    let c = ctx();
    let map = v_map(vec![("a", Value::number(1.0))]);
    assert_eq!(map.at_key(&c, "a"), Ok(Value::number(1.0)));
    assert!(map
        .at_key(&c, "b")
        .unwrap_err()
        .message()
        .contains("not found in"));

    for v in [
        Value::string("abc"),
        Value::array(vec![]),
        Value::boolean(true),
        Value::number(1.0),
        bounded_range(),
        Value::null(),
        Value::undefined(),
    ] {
        assert!(v
            .at_key(&c, "a")
            .unwrap_err()
            .message()
            .contains("not a compatible index"));
    }
}

#[test]
fn at_or_default_by_index() {
    let c = ctx();
    assert_eq!(
        Value::string("abc").at_or_default_index(&c, 0, Value::null()),
        Ok(Value::string("a"))
    );
    assert_eq!(
        Value::string("abc").at_or_default_index(&c, 3, Value::null()),
        Ok(Value::null())
    );
    assert_eq!(
        Value::string("abc").at_or_default_index(&c, 3, Value::string("x")),
        Ok(Value::string("x"))
    );

    let array = v_arr(vec![Value::number(1.0)]);
    assert_eq!(
        array.at_or_default_index(&c, 0, Value::null()),
        Ok(Value::number(1.0))
    );
    assert_eq!(
        array.at_or_default_index(&c, 1, Value::null()),
        Ok(Value::null())
    );
    assert_eq!(
        array.at_or_default_index(&c, 1, Value::string("x")),
        Ok(Value::string("x"))
    );

    for v in [
        Value::map(BTreeMap::new()),
        Value::boolean(true),
        Value::number(1.0),
        bounded_range(),
        Value::null(),
        Value::undefined(),
    ] {
        assert!(v.at_or_default_index(&c, 0, Value::null()).is_err());
    }
}

#[test]
fn at_or_default_by_key() {
    let c = ctx();
    let map = v_map(vec![("a", Value::number(1.0))]);
    assert_eq!(
        map.at_or_default_key(&c, "a", Value::null()),
        Ok(Value::number(1.0))
    );
    assert_eq!(
        map.at_or_default_key(&c, "b", Value::null()),
        Ok(Value::null())
    );
    assert_eq!(
        map.at_or_default_key(&c, "b", Value::string("x")),
        Ok(Value::string("x"))
    );

    for v in [
        Value::string("abc"),
        Value::array(vec![]),
        Value::boolean(true),
        Value::number(1.0),
        bounded_range(),
        Value::null(),
        Value::undefined(),
    ] {
        assert!(v.at_or_default_key(&c, "a", Value::null()).is_err());
    }
}

#[test]
fn equality() {
    assert_eq!(Value::boolean(true), Value::boolean(true));
    assert_ne!(Value::boolean(true), Value::boolean(false));
    assert_eq!(Value::string("a"), Value::string("a"));
    assert_ne!(Value::string("a"), Value::string("b"));
    assert_eq!(Value::number(1.0), Value::number(1.0));
    assert_ne!(Value::number(1.0), Value::number(2.0));
    assert_eq!(
        v_arr(vec![Value::number(1.0)]),
        v_arr(vec![Value::number(1.0)])
    );
    assert_ne!(v_arr(vec![Value::number(1.0)]), Value::array(vec![]));
    assert_eq!(
        v_map(vec![("a", Value::number(1.0))]),
        v_map(vec![("a", Value::number(1.0))])
    );
    assert_ne!(
        v_map(vec![("a", Value::number(1.0))]),
        Value::map(BTreeMap::new())
    );
    assert_eq!(
        bounded_range(),
        Value::range(Range::Bounded(BoundedRange { first: 1, last: 3 }))
    );
    assert_ne!(bounded_range(), left_bounded_range());
    assert_eq!(Value::null(), Value::null());
    assert_eq!(Value::undefined(), Value::undefined());

    assert_ne!(Value::null(), Value::undefined());
    assert_ne!(Value::number(1.0), Value::string("1"));
    assert_ne!(Value::boolean(true), Value::number(1.0));

    let value = v_arr(vec![Value::number(1.0)]);
    let copy = value.clone();
    assert_eq!(value, copy);
}

#[test]
fn display_is_multiline() {
    assert_eq!(
        format!("{}", v_arr(vec![Value::number(1.0), Value::number(2.0)])),
        "[\n\t1,\n\t2\n]"
    );
}
