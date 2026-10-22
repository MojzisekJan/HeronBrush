//! Entity-definition expression language (EL).
//!
//! EL evaluates FGD defaults, model specifications and entity-property interpolation. It supports
//! scalar and collection literals, arithmetic, bitwise and logical operators, comparisons, ranges,
//! subscripting, switches and case expressions.
//!
//! ## Modules
//!
//! [`types`] and [`value`] define values; [`parser`] reads expressions; [`expression`] evaluates
//! and optimises them. [`context`] supplies variables, [`interpolate()`] expands strings and
//! [`ElError`] reports failures.
//!
//! ## Evaluation
//!
//! Evaluation returns `Result<_, ElError>`. Values use shared immutable storage and structural
//! equality, while diagnostic traces track value identity. Indexing an empty collection reports an
//! out-of-bounds result.

#![forbid(unsafe_code)]

pub mod context;
pub mod expression;
pub mod interpolate;
pub mod parser;
pub mod types;
pub mod value;

pub use context::{
    stores_equal, EvaluationContext, NullVariableStore, VariableStore, VariableTable,
};
pub use expression::{BinaryOperation, ExpressionKind, ExpressionNode, UnaryOperation};
pub use interpolate::interpolate;
pub use parser::{parse_expression, ParseMode, Parser, TokenizerState};
pub use types::{
    type_name, BoundedRange, LeftBoundedRange, ParseValueTypeError, Range, RightBoundedRange,
    ValueType,
};
pub use value::Value;

use std::fmt;

/// A one-based source location.
///
/// `column` is optional because some callers only track lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileLocation {
    /// The 1-based line.
    pub line: usize,
    /// The 1-based column, if known.
    pub column: Option<usize>,
}

impl FileLocation {
    /// A location with a line and a column.
    pub fn new(line: usize, column: usize) -> Self {
        Self {
            line,
            column: Some(column),
        }
    }
}

/// Prepends `At line L, column C: ` (or `At unknown location: `) to a message.
pub fn prepend_location(location: Option<FileLocation>, message: &str) -> String {
    let mut out = String::from("At ");
    match location {
        Some(loc) => {
            out.push_str(&format!("line {}", loc.line));
            if let Some(column) = loc.column {
                out.push_str(&format!(", column {column}"));
            }
        }
        None => out.push_str("unknown location"),
    }
    out.push(':');
    if !message.is_empty() {
        out.push(' ');
        out.push_str(message);
    }
    out
}

/// An error raised while parsing, evaluating or interpolating an expression.
///
/// [`ExpressionNode::try_evaluate`](expression::ExpressionNode::try_evaluate) catches only
/// [`ElError::Evaluation`], including indexing errors. Conversion and dereference errors propagate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElError {
    /// A parse failure (`ParserException`).
    Parse(String),
    /// `EvaluationError`, `IndexError` or `IndexOutOfBoundsError`.
    Evaluation(String),
    /// `ConversionError`: a value could not be converted to a requested type.
    Conversion(String),
    /// `DereferenceError`: a value was read as the wrong type.
    Dereference(String),
    /// `InterpolationError` and the failures `interpolate` reports.
    Interpolation(String),
}

impl ElError {
    /// The diagnostic message.
    pub fn message(&self) -> &str {
        match self {
            ElError::Parse(s)
            | ElError::Evaluation(s)
            | ElError::Conversion(s)
            | ElError::Dereference(s)
            | ElError::Interpolation(s) => s,
        }
    }

    /// Consumes the error and returns its message.
    pub fn into_message(self) -> String {
        match self {
            ElError::Parse(s)
            | ElError::Evaluation(s)
            | ElError::Conversion(s)
            | ElError::Dereference(s)
            | ElError::Interpolation(s) => s,
        }
    }
}

impl fmt::Display for ElError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for ElError {}
