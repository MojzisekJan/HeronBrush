//! Shared expression test helpers.
#![allow(dead_code)]

use std::collections::BTreeMap;

use expression_lang::expression::ExpressionKind;
use expression_lang::{
    BinaryOperation, EvaluationContext, ExpressionNode, UnaryOperation, Value, VariableTable,
};
use expression_lang::{parse_expression, ParseMode};

pub fn lit(value: impl Into<Value>) -> ExpressionNode {
    ExpressionNode::new(ExpressionKind::Literal(value.into()), None)
}

pub fn var(name: &str) -> ExpressionNode {
    ExpressionNode::new(ExpressionKind::Variable(name.to_string()), None)
}

pub fn arr(elements: Vec<ExpressionNode>) -> ExpressionNode {
    ExpressionNode::new(ExpressionKind::Array(elements), None)
}

pub fn map_(pairs: Vec<(&str, ExpressionNode)>) -> ExpressionNode {
    let elements: BTreeMap<String, ExpressionNode> =
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    ExpressionNode::new(ExpressionKind::Map(elements), None)
}

fn unary(operation: UnaryOperation, operand: ExpressionNode) -> ExpressionNode {
    ExpressionNode::new(ExpressionKind::Unary { operation, operand }, None)
}

fn binary(
    operation: BinaryOperation,
    left: ExpressionNode,
    right: ExpressionNode,
) -> ExpressionNode {
    ExpressionNode::new(ExpressionKind::Binary { operation, left, right }, None)
}

pub fn plus(o: ExpressionNode) -> ExpressionNode {
    unary(UnaryOperation::Plus, o)
}
pub fn minus(o: ExpressionNode) -> ExpressionNode {
    unary(UnaryOperation::Minus, o)
}
pub fn log_neg(o: ExpressionNode) -> ExpressionNode {
    unary(UnaryOperation::LogicalNegation, o)
}
pub fn bit_neg(o: ExpressionNode) -> ExpressionNode {
    unary(UnaryOperation::BitwiseNegation, o)
}
pub fn grp(o: ExpressionNode) -> ExpressionNode {
    unary(UnaryOperation::Group, o)
}
pub fn lb_rng(o: ExpressionNode) -> ExpressionNode {
    unary(UnaryOperation::LeftBoundedRange, o)
}
pub fn rb_rng(o: ExpressionNode) -> ExpressionNode {
    unary(UnaryOperation::RightBoundedRange, o)
}

pub fn add(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::Addition, l, r)
}
pub fn sub(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::Subtraction, l, r)
}
pub fn mul(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::Multiplication, l, r)
}
pub fn div(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::Division, l, r)
}
pub fn r#mod(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::Modulus, l, r)
}
pub fn log_and(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::LogicalAnd, l, r)
}
pub fn log_or(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::LogicalOr, l, r)
}
pub fn bit_and(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::BitwiseAnd, l, r)
}
pub fn bit_or(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::BitwiseOr, l, r)
}
pub fn bit_xor(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::BitwiseXOr, l, r)
}
pub fn bit_shl(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::BitwiseShiftLeft, l, r)
}
pub fn bit_shr(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::BitwiseShiftRight, l, r)
}
pub fn ls(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::Less, l, r)
}
pub fn ls_eq(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::LessOrEqual, l, r)
}
pub fn gr(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::Greater, l, r)
}
pub fn gr_eq(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::GreaterOrEqual, l, r)
}
pub fn eq(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::Equal, l, r)
}
pub fn neq(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::NotEqual, l, r)
}
pub fn b_rng(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::BoundedRange, l, r)
}
pub fn cs(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    binary(BinaryOperation::Case, l, r)
}
pub fn scr(l: ExpressionNode, r: ExpressionNode) -> ExpressionNode {
    ExpressionNode::new(ExpressionKind::Subscript { left: l, right: r }, None)
}
pub fn swt(cases: Vec<ExpressionNode>) -> ExpressionNode {
    ExpressionNode::new(ExpressionKind::Switch(cases), None)
}

// --- value builders ------------------------------------------------------------

pub fn v_array(elements: Vec<Value>) -> Value {
    Value::array(elements)
}

pub fn v_map(pairs: Vec<(&str, Value)>) -> Value {
    Value::map(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

// --- evaluation helpers -------------------------------------------------------

pub fn table(vars: &[(&str, Value)]) -> VariableTable {
    vars.iter().map(|(k, v)| (*k, v.clone())).collect()
}

/// Evaluates a test expression.
pub fn evaluate_with(expr: &str, vars: &[(&str, Value)]) -> Result<Value, String> {
    let t = table(vars);
    let mut ctx = EvaluationContext::with_variables(&t);
    let node = parse_expression(ParseMode::Strict, expr).expect("parse should succeed");
    node.evaluate(&mut ctx).map_err(|e| e.into_message())
}

pub fn evaluate(expr: &str) -> Result<Value, String> {
    evaluate_with(expr, &[])
}

/// Evaluates a test expression with recoverable evaluation errors.
pub fn try_evaluate_with(expr: &str, vars: &[(&str, Value)]) -> Result<Value, String> {
    let t = table(vars);
    let mut ctx = EvaluationContext::with_variables(&t);
    let node = parse_expression(ParseMode::Strict, expr).expect("parse should succeed");
    node.try_evaluate(&mut ctx).map_err(|e| e.into_message())
}

pub fn ok(v: impl Into<Value>) -> Result<Value, String> {
    Ok(v.into())
}

pub fn err(s: &str) -> Result<Value, String> {
    Err(s.to_string())
}
