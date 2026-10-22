//! Tests for types behavior.

use expression_lang::{
    type_name, BoundedRange, LeftBoundedRange, Range, RightBoundedRange, ValueType,
};

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

fn visited_left(r: LeftBoundedRange, size: usize) -> Vec<i64> {
    let mut out = Vec::new();
    r.for_each(|i| out.push(i), size);
    out
}
fn visited_right(r: RightBoundedRange, size: usize) -> Vec<i64> {
    let mut out = Vec::new();
    r.for_each(|i| out.push(i), size);
    out
}
fn visited_bounded(r: BoundedRange) -> Vec<i64> {
    let mut out = Vec::new();
    r.for_each(|i| out.push(i));
    out
}

#[test]
fn left_bounded_range_length() {
    assert_eq!(LeftBoundedRange { first: 0 }.length(3), 3);
    assert_eq!(LeftBoundedRange { first: 1 }.length(3), 2);
    assert_eq!(LeftBoundedRange { first: 2 }.length(3), 1);
    // extends backwards if it starts past the end
    assert_eq!(LeftBoundedRange { first: 5 }.length(3), 4);
    // an empty indexable clamps to index 0
    assert_eq!(LeftBoundedRange { first: 0 }.length(0), 1);
}

#[test]
fn left_bounded_range_for_each() {
    assert_eq!(
        visited_left(LeftBoundedRange { first: 0 }, 3),
        vec![0, 1, 2]
    );
    assert_eq!(visited_left(LeftBoundedRange { first: 2 }, 3), vec![2]);
    assert_eq!(
        visited_left(LeftBoundedRange { first: 3 }, 2),
        vec![3, 2, 1]
    );
}

#[test]
fn right_bounded_range_length() {
    assert_eq!(RightBoundedRange { last: 0 }.length(3), 3);
    assert_eq!(RightBoundedRange { last: 1 }.length(3), 2);
    assert_eq!(RightBoundedRange { last: 2 }.length(3), 1);
    assert_eq!(RightBoundedRange { last: 5 }.length(3), 4);
    assert_eq!(RightBoundedRange { last: 0 }.length(0), 1);
}

#[test]
fn right_bounded_range_for_each() {
    assert_eq!(
        visited_right(RightBoundedRange { last: 0 }, 3),
        vec![2, 1, 0]
    );
    assert_eq!(visited_right(RightBoundedRange { last: 2 }, 3), vec![2]);
    assert_eq!(
        visited_right(RightBoundedRange { last: 3 }, 2),
        vec![1, 2, 3]
    );
}

#[test]
fn bounded_range_length() {
    assert_eq!(BoundedRange { first: 0, last: 2 }.length(), 3);
    assert_eq!(BoundedRange { first: 2, last: 2 }.length(), 1);
    // a descending range has the same length
    assert_eq!(BoundedRange { first: 2, last: 0 }.length(), 3);
    assert_eq!(BoundedRange { first: -2, last: 2 }.length(), 5);
}

#[test]
fn bounded_range_for_each() {
    assert_eq!(
        visited_bounded(BoundedRange { first: 0, last: 2 }),
        vec![0, 1, 2]
    );
    assert_eq!(visited_bounded(BoundedRange { first: 2, last: 2 }), vec![2]);
    assert_eq!(
        visited_bounded(BoundedRange { first: 2, last: 0 }),
        vec![2, 1, 0]
    );
}

#[test]
fn range_display() {
    assert_eq!(
        Range::Left(LeftBoundedRange { first: 2 }).to_string(),
        "LeftBoundedRange{first: 2}"
    );
    assert_eq!(
        Range::Right(RightBoundedRange { last: 5 }).to_string(),
        "RightBoundedRange{last: 5}"
    );
    assert_eq!(
        Range::Bounded(BoundedRange { first: 1, last: 3 }).to_string(),
        "BoundedRange{first: 1, last: 3}"
    );
}

#[test]
fn type_names() {
    assert_eq!(type_name(ValueType::Boolean), "Boolean");
    assert_eq!(type_name(ValueType::String), "String");
    assert_eq!(type_name(ValueType::Number), "Number");
    assert_eq!(type_name(ValueType::Array), "Array");
    assert_eq!(type_name(ValueType::Map), "Map");
    assert_eq!(type_name(ValueType::Range), "Range");
    assert_eq!(type_name(ValueType::Null), "Null");
    assert_eq!(type_name(ValueType::Undefined), "Undefined");
}

#[test]
fn type_for_names() {
    assert_eq!("Boolean".parse::<ValueType>().unwrap(), ValueType::Boolean);
    assert_eq!("String".parse::<ValueType>().unwrap(), ValueType::String);
    assert_eq!("Number".parse::<ValueType>().unwrap(), ValueType::Number);
    assert_eq!("Array".parse::<ValueType>().unwrap(), ValueType::Array);
    assert_eq!("Map".parse::<ValueType>().unwrap(), ValueType::Map);
    assert_eq!("Range".parse::<ValueType>().unwrap(), ValueType::Range);
    assert_eq!("Null".parse::<ValueType>().unwrap(), ValueType::Null);
    assert_eq!(
        "Undefined".parse::<ValueType>().unwrap(),
        ValueType::Undefined
    );

    for t in ALL_TYPES {
        assert_eq!(type_name(t).parse::<ValueType>().unwrap(), t);
    }
}

#[test]
fn unknown_type_name_is_a_recoverable_error() {
    let error = "MissingType".parse::<ValueType>().unwrap_err();
    assert_eq!(error.to_string(), "not a type name: MissingType");
}
