//! `${...}` string interpolation.

use text_utils::string_utils::str_find_next_delimited_string;

use crate::context::{EvaluationContext, VariableStore};
use crate::parser::{parse_expression, ParseMode};
use crate::types::ValueType;

/// Returns ordered `(start, length)` byte spans of `${...}` groups, including their delimiters.
fn find_expressions(s: &str) -> Result<Vec<(usize, usize)>, String> {
    let mut result = Vec::new();
    let mut total_offset = 0usize;
    let mut rest = s;
    while let Some(part) = str_find_next_delimited_string(rest, "${", "}", None) {
        match part.length {
            None => {
                return Err(format!(
                    "At position {}: Unterminated expression",
                    total_offset + part.start
                ));
            }
            Some(len) => {
                result.push((total_offset + part.start, len));
                let cut = part.start + len;
                rest = &rest[cut..];
                total_offset += cut;
            }
        }
    }
    Ok(result)
}

/// Replaces each `${expr}` with its string value evaluated against `variables`.
///
/// An unterminated `${`, a parse error, an evaluation error, or a result that will not
/// convert to a string all come back as `Err`.
pub fn interpolate(variables: &dyn VariableStore, s: &str) -> Result<String, String> {
    let positions = find_expressions(s)?;

    let mut ctx = EvaluationContext::with_variables(variables);

    let mut expressions = Vec::with_capacity(positions.len());
    for &(start, length) in &positions {
        let expression_str = &s[start + 2..start + length - 1];
        let node =
            parse_expression(ParseMode::Strict, expression_str).map_err(|e| e.into_message())?;
        expressions.push(node);
    }

    let mut values = Vec::with_capacity(expressions.len());
    for node in &expressions {
        values.push(node.evaluate(&mut ctx).map_err(|e| e.into_message())?);
    }

    let mut result = String::new();
    let mut previous_end = 0usize;
    for (i, &(start, length)) in positions.iter().enumerate() {
        result.push_str(&s[previous_end..start]);
        let converted = values[i]
            .convert_to(&mut ctx, ValueType::String)
            .map_err(|e| e.into_message())?;
        result.push_str(&converted.string_value(&ctx).map_err(|e| e.into_message())?);
        previous_end = start + length;
    }
    result.push_str(&s[previous_end..]);
    Ok(result)
}
