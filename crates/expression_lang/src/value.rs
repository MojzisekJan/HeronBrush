//! Dynamically typed EL values.

use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use text_utils::string_format::str_is_blank;

use crate::context::EvaluationContext;
use crate::types::{type_name, Range, ValueType};
use crate::{prepend_location, ElError, FileLocation};

/// ASCII whitespace, matching `kdl`'s default `Whitespace` set.
const WHITESPACE: &str = " \t\n\r\x0c\x0b";

/// The payload behind a [`Value`]. `MapType` is a `std::map`, so the keys stay sorted.
#[derive(Debug, Clone, PartialEq)]
enum ValueData {
    Boolean(bool),
    String(String),
    Number(f64),
    Array(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Range(Range),
    Null,
    Undefined,
}

/// A dynamically typed EL value.
///
/// Cloning shares the immutable payload through [`Rc`]. Equality is structural; diagnostic traces
/// use allocation identity through [`Value::ptr`].
#[derive(Clone)]
pub struct Value(Rc<ValueData>);

impl Value {
    /// A boolean value.
    pub fn boolean(value: bool) -> Self {
        Value(Rc::new(ValueData::Boolean(value)))
    }
    /// A string value.
    pub fn string(value: impl Into<String>) -> Self {
        Value(Rc::new(ValueData::String(value.into())))
    }
    /// A number value.
    pub fn number(value: f64) -> Self {
        Value(Rc::new(ValueData::Number(value)))
    }
    /// A number value from an integer.
    pub fn integer(value: i64) -> Self {
        Value(Rc::new(ValueData::Number(value as f64)))
    }
    /// An array value.
    pub fn array(value: Vec<Value>) -> Self {
        Value(Rc::new(ValueData::Array(value)))
    }
    /// A map value.
    pub fn map(value: BTreeMap<String, Value>) -> Self {
        Value(Rc::new(ValueData::Map(value)))
    }
    /// A range value.
    pub fn range(value: Range) -> Self {
        Value(Rc::new(ValueData::Range(value)))
    }
    /// The null value.
    pub fn null() -> Self {
        Value(Rc::new(ValueData::Null))
    }
    /// The undefined value.
    pub fn undefined() -> Self {
        Value(Rc::new(ValueData::Undefined))
    }

    /// The identity of the payload, for keying the evaluation trace. A clone shares it.
    pub fn ptr(&self) -> usize {
        Rc::as_ptr(&self.0) as *const () as usize
    }

    /// This value's type.
    pub fn value_type(&self) -> ValueType {
        match &*self.0 {
            ValueData::Boolean(_) => ValueType::Boolean,
            ValueData::String(_) => ValueType::String,
            ValueData::Number(_) => ValueType::Number,
            ValueData::Array(_) => ValueType::Array,
            ValueData::Map(_) => ValueType::Map,
            ValueData::Range(_) => ValueType::Range,
            ValueData::Null => ValueType::Null,
            ValueData::Undefined => ValueType::Undefined,
        }
    }

    /// Whether this value has one of the given types.
    pub fn has_type(&self, types: &[ValueType]) -> bool {
        types.contains(&self.value_type())
    }

    /// The name of this value's type.
    pub fn type_name(&self) -> &'static str {
        type_name(self.value_type())
    }

    /// A short single-line description, used in error messages.
    pub fn describe(&self) -> String {
        self.as_string(false)
    }

    // --- typed accessors ---------------------------------------------------------------

    /// The boolean behind this value; `null` reads as `false`.
    pub fn boolean_value(&self, ctx: &EvaluationContext) -> Result<bool, ElError> {
        match &*self.0 {
            ValueData::Boolean(b) => Ok(*b),
            ValueData::Null => Ok(false),
            _ => Err(self.dereference_error(ctx, ValueType::Boolean)),
        }
    }

    /// The string behind this value; `null` reads as `""`.
    pub fn string_value(&self, ctx: &EvaluationContext) -> Result<String, ElError> {
        match &*self.0 {
            ValueData::String(s) => Ok(s.clone()),
            ValueData::Null => Ok(String::new()),
            _ => Err(self.dereference_error(ctx, ValueType::String)),
        }
    }

    /// The number behind this value; `null` reads as `0.0`.
    pub fn number_value(&self, ctx: &EvaluationContext) -> Result<f64, ElError> {
        match &*self.0 {
            ValueData::Number(n) => Ok(*n),
            ValueData::Null => Ok(0.0),
            _ => Err(self.dereference_error(ctx, ValueType::Number)),
        }
    }

    /// The number behind this value, truncated to an integer.
    pub fn integer_value(&self, ctx: &EvaluationContext) -> Result<i64, ElError> {
        Ok(self.number_value(ctx)? as i64)
    }

    /// The array behind this value; `null` reads as `[]`.
    pub fn array_value(&self, ctx: &EvaluationContext) -> Result<Vec<Value>, ElError> {
        match &*self.0 {
            ValueData::Array(a) => Ok(a.clone()),
            ValueData::Null => Ok(Vec::new()),
            _ => Err(self.dereference_error(ctx, ValueType::Array)),
        }
    }

    /// The map behind this value; `null` reads as `{}`.
    pub fn map_value(&self, ctx: &EvaluationContext) -> Result<BTreeMap<String, Value>, ElError> {
        match &*self.0 {
            ValueData::Map(m) => Ok(m.clone()),
            ValueData::Null => Ok(BTreeMap::new()),
            _ => Err(self.dereference_error(ctx, ValueType::Map)),
        }
    }

    /// The range behind this value. Unlike the others, `null` is an error here.
    pub fn range_value(&self, ctx: &EvaluationContext) -> Result<Range, ElError> {
        match &*self.0 {
            ValueData::Range(r) => Ok(*r),
            _ => Err(self.dereference_error(ctx, ValueType::Range)),
        }
    }

    /// This value's array, with every element read as a string.
    pub fn as_string_list(&self, ctx: &EvaluationContext) -> Result<Vec<String>, ElError> {
        self.array_value(ctx)?
            .iter()
            .map(|entry| entry.string_value(ctx))
            .collect()
    }

    /// [`Value::as_string_list`], sorted with duplicates removed.
    pub fn as_string_set(&self, ctx: &EvaluationContext) -> Result<Vec<String>, ElError> {
        let mut list = self.as_string_list(ctx)?;
        list.sort();
        list.dedup();
        Ok(list)
    }

    /// The logical length of this value.
    pub fn length(&self) -> usize {
        match &*self.0 {
            ValueData::Boolean(_) | ValueData::Number(_) => 1,
            ValueData::String(s) => s.chars().count(),
            ValueData::Array(a) => a.len(),
            ValueData::Map(m) => m.len(),
            ValueData::Range(_) => 2,
            ValueData::Null | ValueData::Undefined => 0,
        }
    }

    // --- conversions -----------------------------------------------------------------

    /// Whether [`Value::convert_to`] would succeed for `to_type`.
    pub fn convertible_to(&self, to_type: ValueType) -> bool {
        use ValueType::*;
        match &*self.0 {
            ValueData::Boolean(_) => matches!(to_type, Boolean | String | Number),
            ValueData::String(s) => match to_type {
                Boolean | String => true,
                Number => str_is_blank(s, WHITESPACE) || str_to_double(s).is_some(),
                _ => false,
            },
            ValueData::Number(_) => matches!(to_type, Boolean | String | Number),
            ValueData::Array(_) => to_type == Array,
            ValueData::Map(_) => to_type == Map,
            ValueData::Range(_) => to_type == Range,
            ValueData::Null => matches!(to_type, Boolean | Null | Number | String | Array | Map),
            ValueData::Undefined => to_type == Undefined,
        }
    }

    /// Converts this value to `to_type`, tracing the result to wherever this value came
    /// from so that a later error can name the source location.
    pub fn convert_to(
        &self,
        ctx: &mut EvaluationContext,
        to_type: ValueType,
    ) -> Result<Value, ElError> {
        use ValueType::*;
        let err = || self.conversion_error(ctx, to_type);
        match &*self.0 {
            ValueData::Boolean(b) => match to_type {
                Boolean => Ok(self.clone()),
                String => Ok(ctx.trace_to_value(
                    Value::string(if *b { "true" } else { "false" }),
                    self,
                )),
                Number => Ok(ctx.trace_to_value(Value::number(if *b { 1.0 } else { 0.0 }), self)),
                _ => Err(err()),
            },
            ValueData::String(s) => match to_type {
                Boolean => {
                    Ok(ctx.trace_to_value(Value::boolean(s != "false" && !s.is_empty()), self))
                }
                String => Ok(self.clone()),
                Number => {
                    if str_is_blank(s, WHITESPACE) {
                        Ok(ctx.trace_to_value(Value::number(0.0), self))
                    } else if let Some(x) = str_to_double(s) {
                        Ok(ctx.trace_to_value(Value::number(x), self))
                    } else {
                        Err(err())
                    }
                }
                _ => Err(err()),
            },
            ValueData::Number(n) => match to_type {
                Boolean => Ok(ctx.trace_to_value(Value::boolean(*n != 0.0), self)),
                String => Ok(ctx.trace_to_value(Value::string(self.describe()), self)),
                Number => Ok(self.clone()),
                _ => Err(err()),
            },
            ValueData::Array(_) if to_type == Array => Ok(self.clone()),
            ValueData::Map(_) if to_type == Map => Ok(self.clone()),
            ValueData::Range(_) if to_type == Range => Ok(self.clone()),
            ValueData::Null => match to_type {
                Boolean => Ok(ctx.trace_to_value(Value::boolean(false), self)),
                Null => Ok(self.clone()),
                Number => Ok(ctx.trace_to_value(Value::number(0.0), self)),
                String => Ok(ctx.trace_to_value(Value::string(""), self)),
                Array => Ok(ctx.trace_to_value(Value::array(Vec::new()), self)),
                Map => Ok(ctx.trace_to_value(Value::map(BTreeMap::new()), self)),
                _ => Err(err()),
            },
            ValueData::Undefined if to_type == Undefined => Ok(self.clone()),
            _ => Err(err()),
        }
    }

    /// [`Value::convert_to`], returning `None` on a conversion failure.
    pub fn try_convert_to(
        &self,
        ctx: &mut EvaluationContext,
        to_type: ValueType,
    ) -> Result<Option<Value>, ElError> {
        match self.convert_to(ctx, to_type) {
            Ok(v) => Ok(Some(v)),
            Err(ElError::Conversion(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // --- rendering -------------------------------------------------------------------

    /// Renders this value. `multiline` breaks arrays and maps across lines with tabs.
    pub fn as_string(&self, multiline: bool) -> String {
        let mut out = String::new();
        self.append_to_stream(&mut out, multiline, "");
        out
    }

    /// Appends this value's rendering to `out`.
    pub fn append_to_stream(&self, out: &mut String, multiline: bool, indent: &str) {
        match &*self.0 {
            ValueData::Boolean(b) => out.push_str(if *b { "true" } else { "false" }),
            ValueData::String(s) => {
                out.push('"');
                out.push_str(&text_utils::string_format::str_escape(s, "\\\"", '\\'));
                out.push('"');
            }
            ValueData::Number(n) => out.push_str(&format_number(*n)),
            ValueData::Array(a) => {
                if a.is_empty() {
                    out.push_str("[]");
                    return;
                }
                let child_indent =
                    if multiline { format!("{indent}\t") } else { String::new() };
                out.push('[');
                if multiline {
                    out.push('\n');
                }
                for (i, element) in a.iter().enumerate() {
                    out.push_str(&child_indent);
                    element.append_to_stream(out, multiline, &child_indent);
                    if i < a.len() - 1 {
                        out.push(',');
                        if !multiline {
                            out.push(' ');
                        }
                    }
                    if multiline {
                        out.push('\n');
                    }
                }
                if multiline {
                    out.push_str(indent);
                }
                out.push(']');
            }
            ValueData::Map(m) => {
                if m.is_empty() {
                    out.push_str("{}");
                    return;
                }
                let child_indent =
                    if multiline { format!("{indent}\t") } else { String::new() };
                out.push('{');
                out.push_str(if multiline { "\n" } else { " " });
                for (i, (key, value)) in m.iter().enumerate() {
                    out.push_str(&child_indent);
                    out.push('"');
                    out.push_str(key);
                    out.push_str("\": ");
                    value.append_to_stream(out, multiline, &child_indent);
                    if i < m.len() - 1 {
                        out.push(',');
                        if !multiline {
                            out.push(' ');
                        }
                    }
                    if multiline {
                        out.push('\n');
                    }
                }
                out.push_str(if multiline { indent } else { " " });
                out.push('}');
            }
            ValueData::Range(r) => {
                out.push('[');
                match r {
                    Range::Left(lbr) => out.push_str(&format!("{}..", lbr.first)),
                    Range::Right(rbr) => out.push_str(&format!("..{}", rbr.last)),
                    Range::Bounded(br) => out.push_str(&format!("{}..{}", br.first, br.last)),
                }
                out.push(']');
            }
            ValueData::Null => out.push_str("null"),
            ValueData::Undefined => out.push_str("undefined"),
        }
    }

    // --- indexing -------------------------------------------------------------------

    /// Whether this string or array holds an element at `index`.
    pub fn contains_index(&self, index: usize) -> bool {
        match self.value_type() {
            ValueType::String | ValueType::Array => index < self.length(),
            _ => false,
        }
    }

    /// Whether this map holds `key`. `null` reads as an empty map.
    pub fn contains_key(&self, ctx: &EvaluationContext, key: &str) -> Result<bool, ElError> {
        Ok(self.map_value(ctx)?.contains_key(key))
    }

    /// This map's keys, sorted. `null` reads as an empty map.
    pub fn keys(&self, ctx: &EvaluationContext) -> Result<Vec<String>, ElError> {
        Ok(self.map_value(ctx)?.keys().cloned().collect())
    }

    /// The element at `index`.
    pub fn at_index(&self, ctx: &EvaluationContext, index: usize) -> Result<Value, ElError> {
        match &*self.0 {
            ValueData::String(s) => match s.chars().nth(index) {
                Some(c) => Ok(Value::string(c.to_string())),
                None => Err(self.index_out_of_bounds_index(ctx, index)),
            },
            ValueData::Array(a) => match a.get(index) {
                Some(v) => Ok(v.clone()),
                None => Err(self.index_out_of_bounds_index(ctx, index)),
            },
            _ => Err(self.index_error_index(ctx, index)),
        }
    }

    /// The element at `index`, or `default` if it is out of bounds. A non-indexable value
    /// is still an error.
    pub fn at_or_default_index(
        &self,
        ctx: &EvaluationContext,
        index: usize,
        default: Value,
    ) -> Result<Value, ElError> {
        match &*self.0 {
            ValueData::String(s) => Ok(match s.chars().nth(index) {
                Some(c) => Value::string(c.to_string()),
                None => default,
            }),
            ValueData::Array(a) => Ok(a.get(index).cloned().unwrap_or(default)),
            _ => Err(self.index_error_index(ctx, index)),
        }
    }

    /// The value under `key`.
    pub fn at_key(&self, ctx: &EvaluationContext, key: &str) -> Result<Value, ElError> {
        match &*self.0 {
            ValueData::Map(m) => match m.get(key) {
                Some(v) => Ok(v.clone()),
                None => Err(self.index_out_of_bounds_key(ctx, key)),
            },
            _ => Err(self.index_error_key(ctx, key)),
        }
    }

    /// The value under `key`, or `default` if it is missing.
    pub fn at_or_default_key(
        &self,
        ctx: &EvaluationContext,
        key: &str,
        default: Value,
    ) -> Result<Value, ElError> {
        match &*self.0 {
            ValueData::Map(m) => Ok(m.get(key).cloned().unwrap_or(default)),
            _ => Err(self.index_error_key(ctx, key)),
        }
    }

    // --- error builders ------------------------------------------------------------

    fn dereference_error(&self, ctx: &EvaluationContext, to: ValueType) -> ElError {
        ElError::Dereference(prepend_location(
            ctx.location(self),
            &format!(
                "Cannot dereference value '{}' of type '{}' as type '{}'",
                self.describe(),
                self.type_name(),
                type_name(to)
            ),
        ))
    }

    fn conversion_error(&self, ctx: &EvaluationContext, to: ValueType) -> ElError {
        conversion_error(ctx.location(self), self, self.value_type(), to)
    }

    fn index_error_index(&self, ctx: &EvaluationContext, index: usize) -> ElError {
        index_error_loc_index(ctx.location(self), self, index)
    }

    fn index_error_key(&self, ctx: &EvaluationContext, key: &str) -> ElError {
        index_error_loc_key(ctx.location(self), self, key)
    }

    fn index_out_of_bounds_index(&self, ctx: &EvaluationContext, index: usize) -> ElError {
        index_out_of_bounds_loc_index(ctx.location(self), self, index)
    }

    fn index_out_of_bounds_key(&self, ctx: &EvaluationContext, key: &str) -> ElError {
        index_out_of_bounds_loc_key(ctx.location(self), self, key)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || *self.0 == *other.0
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Value({})", self.as_string(false))
    }
}

impl fmt::Display for Value {
    /// Formats the value across multiple lines.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_string(true))
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::boolean(v)
    }
}
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::string(v)
    }
}
impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::string(v)
    }
}
impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::number(v)
    }
}
impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::integer(v)
    }
}
impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::integer(v as i64)
    }
}

// --- free error builders, shared with `expression` ---------------------------------

pub(crate) fn conversion_error(
    loc: Option<FileLocation>,
    value: &Value,
    from: ValueType,
    to: ValueType,
) -> ElError {
    ElError::Conversion(prepend_location(
        loc,
        &format!(
            "Cannot convert value '{}' of type '{}' to type '{}'",
            value.describe(),
            type_name(from),
            type_name(to)
        ),
    ))
}

pub(crate) fn evaluation_error_loc(loc: Option<FileLocation>, message: &str) -> ElError {
    ElError::Evaluation(prepend_location(loc, message))
}

pub(crate) fn index_error_loc_index(
    loc: Option<FileLocation>,
    indexable: &Value,
    index: usize,
) -> ElError {
    evaluation_error_loc(
        loc,
        &format!("{} is not a compatible index for '{}'", index, indexable.describe()),
    )
}

pub(crate) fn index_error_loc_key(
    loc: Option<FileLocation>,
    indexable: &Value,
    key: &str,
) -> ElError {
    evaluation_error_loc(
        loc,
        &format!("'{}' is not a compatible index for '{}'", key, indexable.describe()),
    )
}

pub(crate) fn index_out_of_bounds_loc_index(
    loc: Option<FileLocation>,
    indexable: &Value,
    index: usize,
) -> ElError {
    evaluation_error_loc(
        loc,
        &format!("{} is out of bounds for '{}'", index, indexable.describe()),
    )
}

pub(crate) fn index_out_of_bounds_loc_key(
    loc: Option<FileLocation>,
    indexable: &Value,
    key: &str,
) -> ElError {
    evaluation_error_loc(loc, &format!("'{}' not found in '{}'", key, indexable.describe()))
}

// --- number formatting ------------------------------------------------------------

/// Renders near-integers without decimals and other numbers with 17 significant digits.
fn format_number(n: f64) -> String {
    const ROUNDING_THRESHOLD: f64 = 0.00001;
    if (n - n.round()).abs() < ROUNDING_THRESHOLD {
        format!("{n:.0}")
    } else {
        format_g(n, 17)
    }
}

/// A `printf("%.*g")` for the cases EL hits: strip trailing zeros, pick fixed or
/// scientific notation by exponent.
fn format_g(n: f64, sig: usize) -> String {
    if n == 0.0 {
        return "0".to_string();
    }
    let neg = n.is_sign_negative();
    let sci = format!("{:.*e}", sig - 1, n.abs());
    let (mantissa, exp_str) = sci.split_once('e').expect("scientific notation has an 'e'");
    let exp: i32 = exp_str.parse().expect("a valid exponent");
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();

    let mut body = String::new();
    if exp >= -4 && (exp as i64) < sig as i64 {
        if exp >= 0 {
            let int_len = (exp + 1) as usize;
            if digits.len() <= int_len {
                body.push_str(&digits);
                body.push_str(&"0".repeat(int_len - digits.len()));
            } else {
                body.push_str(&digits[..int_len]);
                body.push('.');
                body.push_str(&digits[int_len..]);
            }
        } else {
            body.push_str("0.");
            body.push_str(&"0".repeat((-exp - 1) as usize));
            body.push_str(&digits);
        }
        if body.contains('.') {
            while body.ends_with('0') {
                body.pop();
            }
            if body.ends_with('.') {
                body.pop();
            }
        }
    } else {
        body.push_str(&digits[..1]);
        let rest = digits[1..].trim_end_matches('0');
        if !rest.is_empty() {
            body.push('.');
            body.push_str(rest);
        }
        body.push('e');
        body.push(if exp >= 0 { '+' } else { '-' });
        body.push_str(&format!("{:02}", exp.abs()));
    }

    if neg {
        format!("-{body}")
    } else {
        body
    }
}

/// The prefix of `s` that parses as a float, skipping leading whitespace and one leading
/// `+`. Mirrors `kdl::str_to_double`, which is `std::from_chars` and stops at the first
/// character it cannot use.
pub(crate) fn str_to_double(s: &str) -> Option<f64> {
    let s = s.trim_start_matches(|c| WHITESPACE.contains(c));
    let s = match s.strip_prefix('+') {
        Some(rest) if rest.starts_with(|c: char| c.is_ascii_digit() || c == '.') => rest,
        _ => s,
    };
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && b[i] == b'-' {
        i += 1;
    }
    let mantissa_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
    }
    // need at least one mantissa digit
    if !b[mantissa_start..i].iter().any(u8::is_ascii_digit) {
        return None;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let save = i;
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let exp_digits = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_digits {
            i = save;
        }
    }
    s[..i].parse::<f64>().ok()
}
