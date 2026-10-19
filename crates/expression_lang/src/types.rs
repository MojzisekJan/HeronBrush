//! Value types and range representations.

use std::fmt;

/// Length of the inclusive integer interval `[first, last]`, in either direction.
fn detail_length(first: i64, last: i64) -> usize {
    if first <= last {
        (last - first + 1) as usize
    } else {
        (first - last + 1) as usize
    }
}

/// Visits every integer in the inclusive interval from `first` to `last`, in either direction.
fn detail_for_each(first: i64, last: i64, mut f: impl FnMut(i64)) {
    if first <= last {
        for i in first..=last {
            f(i);
        }
    } else {
        let mut i = first;
        while i >= last {
            f(i);
            i -= 1;
        }
    }
}

/// A range with only a lower bound: `[first..]`. The upper bound is the last index of
/// whatever it is applied to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeftBoundedRange {
    /// The first index.
    pub first: i64,
}

impl LeftBoundedRange {
    /// The number of indices the range covers over an indexable of the given size.
    pub fn length(&self, indexable_size: usize) -> usize {
        detail_length(self.first, (indexable_size as i64 - 1).max(0))
    }

    /// Visits each index the range covers over an indexable of the given size.
    pub fn for_each(&self, f: impl FnMut(i64), indexable_size: usize) {
        detail_for_each(self.first, indexable_size as i64 - 1, f);
    }
}

/// A range with only an upper bound: `[..last]`. It starts at the last index of whatever
/// it is applied to and runs back to `last`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RightBoundedRange {
    /// The last index.
    pub last: i64,
}

impl RightBoundedRange {
    /// The number of indices the range covers over an indexable of the given size.
    pub fn length(&self, indexable_size: usize) -> usize {
        detail_length((indexable_size as i64 - 1).max(0), self.last)
    }

    /// Visits each index the range covers over an indexable of the given size.
    pub fn for_each(&self, f: impl FnMut(i64), indexable_size: usize) {
        detail_for_each(indexable_size as i64 - 1, self.last, f);
    }
}

/// A range with both bounds: `[first..last]`. It may run in either direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundedRange {
    /// The first index.
    pub first: i64,
    /// The last index.
    pub last: i64,
}

impl BoundedRange {
    /// The number of indices the range covers.
    pub fn length(&self) -> usize {
        detail_length(self.first, self.last)
    }

    /// Visits each index the range covers.
    pub fn for_each(&self, f: impl FnMut(i64)) {
        detail_for_each(self.first, self.last, f);
    }
}

/// The three supported range representations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Range {
    /// `[first..]`
    Left(LeftBoundedRange),
    /// `[..last]`
    Right(RightBoundedRange),
    /// `[first..last]`
    Bounded(BoundedRange),
}

impl fmt::Display for Range {
    /// The textual range representation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Range::Left(r) => write!(f, "LeftBoundedRange{{first: {}}}", r.first),
            Range::Right(r) => write!(f, "RightBoundedRange{{last: {}}}", r.last),
            Range::Bounded(r) => {
                write!(f, "BoundedRange{{first: {}, last: {}}}", r.first, r.last)
            }
        }
    }
}

/// The eight EL value types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueType {
    /// `true` / `false`
    Boolean,
    /// a string
    String,
    /// a `f64`
    Number,
    /// an array
    Array,
    /// a string-keyed map
    Map,
    /// a range
    Range,
    /// `null`
    Null,
    /// `undefined`
    Undefined,
}

/// The name of a type, as it appears in error messages and string parsing.
pub fn type_name(t: ValueType) -> &'static str {
    match t {
        ValueType::Boolean => "Boolean",
        ValueType::String => "String",
        ValueType::Number => "Number",
        ValueType::Array => "Array",
        ValueType::Map => "Map",
        ValueType::Range => "Range",
        ValueType::Null => "Null",
        ValueType::Undefined => "Undefined",
    }
}

/// A type name that is not part of the expression language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseValueTypeError(String);

impl std::fmt::Display for ParseValueTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not a type name: {}", self.0)
    }
}

impl std::error::Error for ParseValueTypeError {}

impl std::str::FromStr for ValueType {
    type Err = ParseValueTypeError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "Boolean" => Ok(Self::Boolean),
            "String" => Ok(Self::String),
            "Number" => Ok(Self::Number),
            "Array" => Ok(Self::Array),
            "Map" => Ok(Self::Map),
            "Range" => Ok(Self::Range),
            "Null" => Ok(Self::Null),
            "Undefined" => Ok(Self::Undefined),
            other => Err(ParseValueTypeError(other.to_owned())),
        }
    }
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(type_name(*self))
    }
}
