//! The expression AST, its evaluation, optimization and precedence rebalancing.
//!

use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use crate::context::EvaluationContext;
use crate::types::{BoundedRange, LeftBoundedRange, Range, RightBoundedRange, ValueType};
use crate::value::{conversion_error, Value};
use crate::{ElError, FileLocation};

/// A unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperation {
    /// `+x`
    Plus,
    /// `-x`
    Minus,
    /// `!x`
    LogicalNegation,
    /// `~x`
    BitwiseNegation,
    /// `( x )`
    Group,
    /// `x..`
    LeftBoundedRange,
    /// `..x`
    RightBoundedRange,
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperation {
    /// `+`
    Addition,
    /// `-`
    Subtraction,
    /// `*`
    Multiplication,
    /// `/`
    Division,
    /// `%`
    Modulus,
    /// `&&`
    LogicalAnd,
    /// `||`
    LogicalOr,
    /// `&`
    BitwiseAnd,
    /// `^`
    BitwiseXOr,
    /// `|`
    BitwiseOr,
    /// `<<`
    BitwiseShiftLeft,
    /// `>>`
    BitwiseShiftRight,
    /// `<`
    Less,
    /// `<=`
    LessOrEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterOrEqual,
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `a..b`
    BoundedRange,
    /// `->`
    Case,
}

/// The payload of an expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionKind {
    /// A literal value.
    Literal(Value),
    /// A variable reference.
    Variable(String),
    /// An array literal.
    Array(Vec<ExpressionNode>),
    /// A map literal, keys sorted.
    Map(BTreeMap<String, ExpressionNode>),
    /// A unary operation.
    Unary {
        /// The operator.
        operation: UnaryOperation,
        /// The operand.
        operand: ExpressionNode,
    },
    /// A binary operation.
    Binary {
        /// The operator.
        operation: BinaryOperation,
        /// The left operand.
        left: ExpressionNode,
        /// The right operand.
        right: ExpressionNode,
    },
    /// A subscript `a[b]`.
    Subscript {
        /// The indexable.
        left: ExpressionNode,
        /// The index.
        right: ExpressionNode,
    },
    /// A `{{ ... }}` switch.
    Switch(Vec<ExpressionNode>),
}

#[derive(Debug)]
struct Inner {
    kind: ExpressionKind,
    location: Option<FileLocation>,
}

/// An expression node: a payload and, optionally, where in the source it came from.
///
/// Shared expression data. Equality ignores source locations.
#[derive(Debug, Clone)]
pub struct ExpressionNode {
    inner: Rc<Inner>,
}

impl PartialEq for ExpressionNode {
    fn eq(&self, other: &Self) -> bool {
        self.inner.kind == other.inner.kind
    }
}

impl ExpressionNode {
    /// Builds a node and rebalances it according to operator precedence.
    pub fn new(kind: ExpressionKind, location: Option<FileLocation>) -> Self {
        let (kind, location) = rebalance(kind, location);
        ExpressionNode {
            inner: Rc::new(Inner { kind, location }),
        }
    }

    /// Builds a node without rebalancing, for [`ExpressionNode::optimize`] and the precedence
    /// rebalancer.
    pub fn from_parts(kind: ExpressionKind, location: Option<FileLocation>) -> Self {
        ExpressionNode {
            inner: Rc::new(Inner { kind, location }),
        }
    }

    /// A literal node with no location.
    pub fn literal(value: Value) -> Self {
        Self::from_parts(ExpressionKind::Literal(value), None)
    }

    /// This node's payload.
    pub fn kind(&self) -> &ExpressionKind {
        &self.inner.kind
    }

    /// Where this node came from.
    pub fn location(&self) -> Option<FileLocation> {
        self.inner.location
    }

    /// Whether this node is a literal.
    pub fn is_literal(&self) -> bool {
        matches!(self.inner.kind, ExpressionKind::Literal(_))
    }

    /// Renders this node as source text.
    pub fn as_string(&self) -> String {
        format!("{self}")
    }

    /// Evaluates this node.
    pub fn evaluate(&self, ctx: &mut EvaluationContext) -> Result<Value, ElError> {
        self.eval_inner(ctx, false)
    }

    /// Evaluates this node, replacing subexpression `EvaluationError`s, including index errors,
    /// with `undefined`. Conversion and dereference errors still propagate.
    pub fn try_evaluate(&self, ctx: &mut EvaluationContext) -> Result<Value, ElError> {
        self.eval_inner(ctx, true)
    }

    fn eval_inner(&self, ctx: &mut EvaluationContext, try_mode: bool) -> Result<Value, ElError> {
        match self.eval_kind(ctx, try_mode) {
            Ok(v) => Ok(ctx.trace(v, self)),
            Err(ElError::Evaluation(_)) if try_mode => Ok(Value::undefined()),
            Err(e) => Err(e),
        }
    }

    fn eval_kind(&self, ctx: &mut EvaluationContext, try_mode: bool) -> Result<Value, ElError> {
        match &self.inner.kind {
            ExpressionKind::Literal(v) => Ok(v.clone()),
            ExpressionKind::Variable(name) => Ok(ctx.variable_value(name)),
            ExpressionKind::Array(elements) => {
                let mut array = Vec::with_capacity(elements.len());
                for element in elements {
                    let value = element.eval_inner(ctx, try_mode)?;
                    match value.value_type() {
                        ValueType::Range => {
                            if let Range::Bounded(range) = value.range_value(ctx)? {
                                range.for_each(|i| array.push(Value::integer(i)));
                            } else {
                                array.push(value);
                            }
                        }
                        _ => array.push(value),
                    }
                }
                Ok(Value::array(array))
            }
            ExpressionKind::Map(elements) => {
                let mut map = BTreeMap::new();
                for (key, element) in elements {
                    map.insert(key.clone(), element.eval_inner(ctx, try_mode)?);
                }
                Ok(Value::map(map))
            }
            ExpressionKind::Unary { operation, operand } => {
                let value = operand.eval_inner(ctx, try_mode)?;
                evaluate_unary(ctx, *operation, &value, self)
            }
            ExpressionKind::Binary {
                operation,
                left,
                right,
            } => evaluate_binary(ctx, *operation, left, right, self, try_mode),
            ExpressionKind::Subscript { left, right } => {
                let l = left.eval_inner(ctx, try_mode)?;
                let r = right.eval_inner(ctx, try_mode)?;
                evaluate_subscript(ctx, &l, &r, self)
            }
            ExpressionKind::Switch(cases) => {
                for case in cases {
                    let result = case.eval_inner(ctx, try_mode)?;
                    if result != Value::undefined() {
                        return Ok(result);
                    }
                }
                Ok(Value::undefined())
            }
        }
    }

    /// Folds constant subexpressions.
    pub fn optimize(&self, ctx: &mut EvaluationContext) -> Result<ExpressionNode, ElError> {
        let kind = match &self.inner.kind {
            ExpressionKind::Literal(v) => ExpressionKind::Literal(v.clone()),
            ExpressionKind::Variable(name) => ExpressionKind::Variable(name.clone()),
            ExpressionKind::Array(elements) => {
                let optimized: Vec<ExpressionNode> = elements
                    .iter()
                    .map(|e| e.optimize(ctx))
                    .collect::<Result<_, _>>()?;
                if optimized.iter().all(ExpressionNode::is_literal) {
                    let values: Vec<Value> = optimized
                        .iter()
                        .map(|e| e.evaluate(ctx))
                        .collect::<Result<_, _>>()?;
                    ExpressionKind::Literal(Value::array(values))
                } else {
                    ExpressionKind::Array(optimized)
                }
            }
            ExpressionKind::Map(elements) => {
                let optimized: BTreeMap<String, ExpressionNode> = elements
                    .iter()
                    .map(|(k, v)| Ok((k.clone(), v.optimize(ctx)?)))
                    .collect::<Result<_, ElError>>()?;
                if optimized.values().all(ExpressionNode::is_literal) {
                    let mut map = BTreeMap::new();
                    for (k, e) in &optimized {
                        map.insert(k.clone(), e.evaluate(ctx)?);
                    }
                    ExpressionKind::Literal(Value::map(map))
                } else {
                    ExpressionKind::Map(optimized)
                }
            }
            ExpressionKind::Unary { operation, operand } => {
                let optimized = operand.optimize(ctx)?;
                if optimized.is_literal() {
                    let value = optimized.evaluate(ctx)?;
                    ExpressionKind::Literal(evaluate_unary(ctx, *operation, &value, self)?)
                } else {
                    ExpressionKind::Unary {
                        operation: *operation,
                        operand: optimized,
                    }
                }
            }
            ExpressionKind::Binary {
                operation,
                left,
                right,
            } => {
                return optimize_binary(ctx, *operation, left, right, self);
            }
            ExpressionKind::Subscript { left, right } => {
                let opt_left = left.optimize(ctx)?;
                let opt_right = right.optimize(ctx)?;
                if opt_left.is_literal() && opt_right.is_literal() {
                    let l = opt_left.evaluate(ctx)?;
                    let r = opt_right.evaluate(ctx)?;
                    ExpressionKind::Literal(evaluate_subscript(ctx, &l, &r, self)?)
                } else {
                    ExpressionKind::Subscript {
                        left: opt_left,
                        right: opt_right,
                    }
                }
            }
            ExpressionKind::Switch(cases) => {
                if cases.is_empty() {
                    ExpressionKind::Literal(Value::undefined())
                } else {
                    let first = cases[0].optimize(ctx)?;
                    if first.is_literal() {
                        ExpressionKind::Literal(first.evaluate(ctx)?)
                    } else {
                        let mut optimized = vec![first];
                        for case in &cases[1..] {
                            optimized.push(case.optimize(ctx)?);
                        }
                        ExpressionKind::Switch(optimized)
                    }
                }
            }
        };
        Ok(ExpressionNode::from_parts(kind, self.inner.location))
    }

    /// Collects rendered nodes in preorder.
    pub fn visit_preorder(&self, out: &mut Vec<String>) {
        out.push(format!("{self}"));
        match &self.inner.kind {
            ExpressionKind::Literal(_) | ExpressionKind::Variable(_) => {}
            ExpressionKind::Array(elements) => {
                for e in elements {
                    e.visit_preorder(out);
                }
            }
            ExpressionKind::Map(elements) => {
                for e in elements.values() {
                    e.visit_preorder(out);
                }
            }
            ExpressionKind::Unary { operand, .. } => operand.visit_preorder(out),
            ExpressionKind::Binary { left, right, .. } => {
                left.visit_preorder(out);
                right.visit_preorder(out);
            }
            ExpressionKind::Subscript { left, right } => {
                left.visit_preorder(out);
                right.visit_preorder(out);
            }
            ExpressionKind::Switch(cases) => {
                for c in cases {
                    c.visit_preorder(out);
                }
            }
        }
    }
}

// --- rendering -----------------------------------------------------------------------

impl fmt::Display for ExpressionNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.kind)
    }
}

impl fmt::Display for ExpressionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExpressionKind::Literal(v) => write!(f, "{v}"),
            ExpressionKind::Variable(name) => f.write_str(name),
            ExpressionKind::Array(elements) => {
                f.write_str("[")?;
                for (i, e) in elements.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{e}")?;
                }
                f.write_str("]")
            }
            ExpressionKind::Map(elements) => {
                if elements.is_empty() {
                    return f.write_str("{}");
                }
                f.write_str("{ ")?;
                for (i, (key, e)) in elements.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "\"{key}\": {e}")?;
                }
                f.write_str(" }")
            }
            ExpressionKind::Unary { operation, operand } => match operation {
                UnaryOperation::Plus => write!(f, "+{operand}"),
                UnaryOperation::Minus => write!(f, "-{operand}"),
                UnaryOperation::LogicalNegation => write!(f, "!{operand}"),
                UnaryOperation::BitwiseNegation => write!(f, "~{operand}"),
                UnaryOperation::Group => write!(f, "( {operand} )"),
                UnaryOperation::LeftBoundedRange => write!(f, "{operand}.."),
                UnaryOperation::RightBoundedRange => write!(f, "..{operand}"),
            },
            ExpressionKind::Binary {
                operation,
                left,
                right,
            } => {
                let op = match operation {
                    BinaryOperation::Addition => " + ",
                    BinaryOperation::Subtraction => " - ",
                    BinaryOperation::Multiplication => " * ",
                    BinaryOperation::Division => " / ",
                    BinaryOperation::Modulus => " % ",
                    BinaryOperation::LogicalAnd => " && ",
                    BinaryOperation::LogicalOr => " || ",
                    BinaryOperation::BitwiseAnd => " & ",
                    BinaryOperation::BitwiseXOr => " ^ ",
                    BinaryOperation::BitwiseOr => " | ",
                    BinaryOperation::BitwiseShiftLeft => " << ",
                    BinaryOperation::BitwiseShiftRight => " >> ",
                    BinaryOperation::Less => " < ",
                    BinaryOperation::LessOrEqual => " <= ",
                    BinaryOperation::Greater => " > ",
                    BinaryOperation::GreaterOrEqual => " >= ",
                    BinaryOperation::Equal => " == ",
                    BinaryOperation::NotEqual => " != ",
                    BinaryOperation::BoundedRange => "..",
                    BinaryOperation::Case => " -> ",
                };
                write!(f, "{left}{op}{right}")
            }
            ExpressionKind::Subscript { left, right } => write!(f, "{left}[{right}]"),
            ExpressionKind::Switch(cases) => {
                f.write_str("{{ ")?;
                for (i, c) in cases.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{c}")?;
                }
                f.write_str(" }}")
            }
        }
    }
}

// --- precedence rebalancing ---------------------------------------------------------

fn binop_precedence(op: BinaryOperation) -> usize {
    use BinaryOperation::*;
    match op {
        Multiplication | Division | Modulus => 12,
        Addition | Subtraction => 11,
        BitwiseShiftLeft | BitwiseShiftRight => 10,
        Less | LessOrEqual | Greater | GreaterOrEqual => 9,
        Equal | NotEqual => 8,
        BitwiseAnd => 7,
        BitwiseXOr => 6,
        BitwiseOr => 5,
        LogicalAnd => 4,
        LogicalOr => 3,
        BoundedRange => 2,
        Case => 1,
    }
}

fn node_precedence(node: &ExpressionNode) -> usize {
    match &node.inner.kind {
        ExpressionKind::Binary { operation, .. } => binop_precedence(*operation),
        _ => 13,
    }
}

fn into_inner(node: ExpressionNode) -> (ExpressionKind, Option<FileLocation>) {
    match Rc::try_unwrap(node.inner) {
        Ok(inner) => (inner.kind, inner.location),
        Err(rc) => (rc.kind.clone(), rc.location),
    }
}

fn rebalance_node(node: ExpressionNode) -> ExpressionNode {
    let (kind, location) = into_inner(node);
    let (kind, location) = rebalance(kind, location);
    ExpressionNode::from_parts(kind, location)
}

/// Keeps each node precedence less than or equal to its children so depth-first evaluation respects
/// operator precedence.
fn rebalance(
    kind: ExpressionKind,
    location: Option<FileLocation>,
) -> (ExpressionKind, Option<FileLocation>) {
    let ExpressionKind::Binary {
        operation,
        left,
        right,
    } = kind
    else {
        return (kind, location);
    };
    let my_p = binop_precedence(operation);
    let lp = node_precedence(&left);
    let rp = node_precedence(&right);

    if my_p > lp.min(rp) {
        if lp < rp {
            // Rotate the left (binary) node up.
            let (left_kind, left_loc) = into_inner(left);
            let ExpressionKind::Binary {
                operation: lop,
                left: ll,
                right: lr,
            } = left_kind
            else {
                unreachable!("a child with precedence < 13 is a binary expression");
            };
            let old_this = ExpressionNode::from_parts(
                ExpressionKind::Binary {
                    operation,
                    left: lr,
                    right: rebalance_node(right),
                },
                location,
            );
            return (
                ExpressionKind::Binary {
                    operation: lop,
                    left: ll,
                    right: old_this,
                },
                left_loc,
            );
        } else {
            // Rotate the right (binary) node up.
            let (right_kind, right_loc) = into_inner(right);
            let ExpressionKind::Binary {
                operation: rop,
                left: rl,
                right: rr,
            } = right_kind
            else {
                unreachable!("a child with precedence < 13 is a binary expression");
            };
            let old_this = ExpressionNode::from_parts(
                ExpressionKind::Binary {
                    operation,
                    left: rebalance_node(left),
                    right: rl,
                },
                location,
            );
            return (
                ExpressionKind::Binary {
                    operation: rop,
                    left: old_this,
                    right: rr,
                },
                right_loc,
            );
        }
    }

    (
        ExpressionKind::Binary {
            operation,
            left,
            right,
        },
        location,
    )
}

// --- error builders ---------------------------------------------------------------

fn evaluation_error_node(node: &ExpressionNode, reason: &str) -> ElError {
    ElError::Evaluation(crate::prepend_location(
        node.location(),
        &format!(
            "Cannot evaluate expression '{}': {}",
            node.as_string(),
            reason
        ),
    ))
}

fn invalid_operand_types(node: &ExpressionNode, lhs: &Value, rhs: &Value) -> ElError {
    evaluation_error_node(
        node,
        &format!(
            "Invalid operand types {} and {}",
            lhs.type_name(),
            rhs.type_name()
        ),
    )
}

fn invalid_type(node: &ExpressionNode, v: &Value) -> ElError {
    evaluation_error_node(node, &format!("Invalid type {}", v.type_name()))
}

fn index_error_node(node: &ExpressionNode, indexable: &Value, index_value: &Value) -> ElError {
    evaluation_error_node(
        node,
        &format!(
            "'{}' is not a compatible index for '{}'",
            index_value.describe(),
            indexable.describe()
        ),
    )
}

fn index_out_of_bounds_node(node: &ExpressionNode, indexable: &Value, index: usize) -> ElError {
    evaluation_error_node(
        node,
        &format!("{} is out of bounds for '{}'", index, indexable.describe()),
    )
}

// --- unary evaluation ------------------------------------------------------------

fn evaluate_unary(
    ctx: &mut EvaluationContext,
    op: UnaryOperation,
    operand: &Value,
    node: &ExpressionNode,
) -> Result<Value, ElError> {
    if *operand == Value::undefined() {
        return Ok(Value::undefined());
    }
    match op {
        UnaryOperation::Plus => evaluate_unary_plus(ctx, operand, node),
        UnaryOperation::Minus => evaluate_unary_minus(ctx, operand, node),
        UnaryOperation::LogicalNegation => match operand.value_type() {
            ValueType::Boolean => Ok(Value::boolean(!operand.boolean_value(ctx)?)),
            _ => Err(invalid_type(node, operand)),
        },
        UnaryOperation::BitwiseNegation => evaluate_bitwise_negation(ctx, operand, node),
        UnaryOperation::Group => Ok(ctx.trace_to_value(operand.clone(), operand)),
        UnaryOperation::LeftBoundedRange => {
            let first = operand
                .convert_to(ctx, ValueType::Number)?
                .number_value(ctx)? as i64;
            Ok(ctx.trace_to_value(
                Value::range(Range::Left(LeftBoundedRange { first })),
                operand,
            ))
        }
        UnaryOperation::RightBoundedRange => {
            let last = operand
                .convert_to(ctx, ValueType::Number)?
                .number_value(ctx)? as i64;
            Ok(ctx.trace_to_value(
                Value::range(Range::Right(RightBoundedRange { last })),
                operand,
            ))
        }
    }
}

fn evaluate_unary_plus(
    ctx: &mut EvaluationContext,
    v: &Value,
    node: &ExpressionNode,
) -> Result<Value, ElError> {
    match v.value_type() {
        ValueType::Boolean | ValueType::Number => Ok(Value::number(
            v.convert_to(ctx, ValueType::Number)?.number_value(ctx)?,
        )),
        ValueType::String => {
            if let Some(result) = v.try_convert_to(ctx, ValueType::Number)? {
                return Ok(Value::number(result.number_value(ctx)?));
            }
            Err(invalid_type(node, v))
        }
        _ => Err(invalid_type(node, v)),
    }
}

fn evaluate_unary_minus(
    ctx: &mut EvaluationContext,
    v: &Value,
    node: &ExpressionNode,
) -> Result<Value, ElError> {
    match v.value_type() {
        ValueType::Boolean | ValueType::Number => Ok(Value::number(
            -v.convert_to(ctx, ValueType::Number)?.number_value(ctx)?,
        )),
        ValueType::String => {
            if let Some(result) = v.try_convert_to(ctx, ValueType::Number)? {
                return Ok(Value::number(-result.number_value(ctx)?));
            }
            Err(invalid_type(node, v))
        }
        _ => Err(invalid_type(node, v)),
    }
}

fn evaluate_bitwise_negation(
    ctx: &mut EvaluationContext,
    v: &Value,
    node: &ExpressionNode,
) -> Result<Value, ElError> {
    match v.value_type() {
        ValueType::Number => Ok(Value::integer(!v.integer_value(ctx)?)),
        ValueType::String => {
            if let Some(result) = v.try_convert_to(ctx, ValueType::Number)? {
                return Ok(Value::integer(!result.integer_value(ctx)?));
            }
            Err(invalid_type(node, v))
        }
        _ => Err(invalid_type(node, v)),
    }
}

// --- binary evaluation ---------------------------------------------------------

fn evaluate_binary(
    ctx: &mut EvaluationContext,
    op: BinaryOperation,
    left: &ExpressionNode,
    right: &ExpressionNode,
    node: &ExpressionNode,
    try_mode: bool,
) -> Result<Value, ElError> {
    use BinaryOperation::*;
    match op {
        LogicalAnd => evaluate_logical_and(ctx, left, right, node, try_mode),
        LogicalOr => evaluate_logical_or(ctx, left, right, node, try_mode),
        Case => {
            let lhs = left.eval_inner(ctx, try_mode)?;
            if lhs.value_type() != ValueType::Undefined
                && lhs
                    .convert_to(ctx, ValueType::Boolean)?
                    .boolean_value(ctx)?
            {
                right.eval_inner(ctx, try_mode)
            } else {
                Ok(Value::undefined())
            }
        }
        _ => {
            let l = left.eval_inner(ctx, try_mode)?;
            let r = right.eval_inner(ctx, try_mode)?;
            evaluate_binary_eager(ctx, op, &l, &r, node)
        }
    }
}

fn evaluate_binary_eager(
    ctx: &mut EvaluationContext,
    op: BinaryOperation,
    l: &Value,
    r: &Value,
    node: &ExpressionNode,
) -> Result<Value, ElError> {
    use BinaryOperation::*;
    match op {
        Addition => evaluate_addition(ctx, l, r, node),
        Subtraction => evaluate_algebraic(ctx, l, r, node, |a, b| a - b),
        Multiplication => evaluate_algebraic(ctx, l, r, node, |a, b| a * b),
        Division => evaluate_algebraic(ctx, l, r, node, |a, b| a / b),
        Modulus => evaluate_algebraic(ctx, l, r, node, |a, b| a % b),
        BitwiseAnd => evaluate_bitwise(ctx, l, r, node, |a, b| a & b),
        BitwiseXOr => evaluate_bitwise(ctx, l, r, node, |a, b| a ^ b),
        BitwiseOr => evaluate_bitwise(ctx, l, r, node, |a, b| a | b),
        BitwiseShiftLeft => evaluate_bitwise(ctx, l, r, node, |a, b| a.wrapping_shl(b as u32)),
        BitwiseShiftRight => evaluate_bitwise(ctx, l, r, node, |a, b| a.wrapping_shr(b as u32)),
        Less => Ok(Value::boolean(evaluate_compare(ctx, l, r, node)? < 0)),
        LessOrEqual => Ok(Value::boolean(evaluate_compare(ctx, l, r, node)? <= 0)),
        Greater => Ok(Value::boolean(evaluate_compare(ctx, l, r, node)? > 0)),
        GreaterOrEqual => Ok(Value::boolean(evaluate_compare(ctx, l, r, node)? >= 0)),
        Equal => Ok(Value::boolean(evaluate_compare(ctx, l, r, node)? == 0)),
        NotEqual => Ok(Value::boolean(evaluate_compare(ctx, l, r, node)? != 0)),
        BoundedRange => evaluate_bounded_range(ctx, l, r),
        LogicalAnd | LogicalOr | Case => unreachable!("handled by evaluate_binary"),
    }
}

/// Converts boolean, numeric and numeric-string operands to numbers and applies `f`, returning
/// `None` when conversion is unavailable.
fn try_algebraic(
    ctx: &mut EvaluationContext,
    lhs: &Value,
    rhs: &Value,
    f: impl Fn(f64, f64) -> f64,
) -> Result<Option<Value>, ElError> {
    use ValueType::*;
    if lhs.value_type() == Undefined || rhs.value_type() == Undefined {
        return Ok(Some(Value::undefined()));
    }
    let num_like = |v: &Value| v.has_type(&[Boolean, Number]);
    if num_like(lhs) && num_like(rhs) {
        let a = lhs.convert_to(ctx, Number)?.number_value(ctx)?;
        let b = rhs.convert_to(ctx, Number)?.number_value(ctx)?;
        return Ok(Some(Value::number(f(a, b))));
    }
    if num_like(lhs) && rhs.value_type() == String {
        if let Some(b) = rhs.try_convert_to(ctx, Number)? {
            let a = lhs.convert_to(ctx, Number)?.number_value(ctx)?;
            return Ok(Some(Value::number(f(a, b.number_value(ctx)?))));
        }
    }
    if lhs.value_type() == String && num_like(rhs) {
        if let Some(a) = lhs.try_convert_to(ctx, Number)? {
            let b = rhs.convert_to(ctx, Number)?.number_value(ctx)?;
            return Ok(Some(Value::number(f(a.number_value(ctx)?, b))));
        }
    }
    Ok(None)
}

fn evaluate_algebraic(
    ctx: &mut EvaluationContext,
    l: &Value,
    r: &Value,
    node: &ExpressionNode,
    f: impl Fn(f64, f64) -> f64,
) -> Result<Value, ElError> {
    match try_algebraic(ctx, l, r, f)? {
        Some(v) => Ok(v),
        None => Err(invalid_operand_types(node, l, r)),
    }
}

fn evaluate_addition(
    ctx: &mut EvaluationContext,
    lhs: &Value,
    rhs: &Value,
    node: &ExpressionNode,
) -> Result<Value, ElError> {
    use ValueType::*;
    if let Some(v) = try_algebraic(ctx, lhs, rhs, |a, b| a + b)? {
        return Ok(v);
    }
    if lhs.value_type() == String && rhs.value_type() == String {
        let a = lhs.convert_to(ctx, String)?.string_value(ctx)?;
        let b = rhs.convert_to(ctx, String)?.string_value(ctx)?;
        return Ok(Value::string(a + &b));
    }
    if lhs.value_type() == Array && rhs.value_type() == Array {
        let mut a = lhs.array_value(ctx)?;
        a.extend(rhs.array_value(ctx)?);
        return Ok(Value::array(a));
    }
    if lhs.value_type() == Map && rhs.value_type() == Map {
        let mut a = lhs.map_value(ctx)?;
        a.extend(rhs.map_value(ctx)?);
        return Ok(Value::map(a));
    }
    Err(invalid_operand_types(node, lhs, rhs))
}

fn try_bitwise(
    ctx: &mut EvaluationContext,
    lhs: &Value,
    rhs: &Value,
    f: impl Fn(i64, i64) -> i64,
) -> Result<Option<Value>, ElError> {
    if lhs.value_type() == ValueType::Undefined || rhs.value_type() == ValueType::Undefined {
        return Ok(Some(Value::undefined()));
    }
    if lhs.convertible_to(ValueType::Number) && rhs.convertible_to(ValueType::Number) {
        let a = lhs.convert_to(ctx, ValueType::Number)?.integer_value(ctx)?;
        let b = rhs.convert_to(ctx, ValueType::Number)?.integer_value(ctx)?;
        return Ok(Some(Value::integer(f(a, b))));
    }
    Ok(None)
}

fn evaluate_bitwise(
    ctx: &mut EvaluationContext,
    l: &Value,
    r: &Value,
    node: &ExpressionNode,
    f: impl Fn(i64, i64) -> i64,
) -> Result<Value, ElError> {
    match try_bitwise(ctx, l, r, f)? {
        Some(v) => Ok(v),
        None => Err(invalid_operand_types(node, l, r)),
    }
}

fn evaluate_logical_and(
    ctx: &mut EvaluationContext,
    left: &ExpressionNode,
    right: &ExpressionNode,
    node: &ExpressionNode,
    try_mode: bool,
) -> Result<Value, ElError> {
    use ValueType::*;
    let lhs = left.eval_inner(ctx, try_mode)?;
    if lhs.value_type() == Undefined {
        return Ok(Value::undefined());
    }
    if lhs.has_type(&[Boolean, Null]) {
        if !lhs.convert_to(ctx, Boolean)?.boolean_value(ctx)? {
            return Ok(Value::boolean(false));
        }
        let rhs = right.eval_inner(ctx, try_mode)?;
        if rhs.has_type(&[Boolean, Null]) {
            return Ok(Value::boolean(
                rhs.convert_to(ctx, Boolean)?.boolean_value(ctx)?,
            ));
        }
        if rhs.value_type() == Undefined {
            return Ok(Value::undefined());
        }
        return Err(invalid_operand_types(node, &lhs, &rhs));
    }
    let rhs = right.eval_inner(ctx, try_mode)?;
    if rhs.value_type() == Undefined {
        return Ok(Value::undefined());
    }
    Err(invalid_operand_types(node, &lhs, &rhs))
}

fn evaluate_logical_or(
    ctx: &mut EvaluationContext,
    left: &ExpressionNode,
    right: &ExpressionNode,
    node: &ExpressionNode,
    try_mode: bool,
) -> Result<Value, ElError> {
    use ValueType::*;
    let lhs = left.eval_inner(ctx, try_mode)?;
    if lhs.value_type() == Undefined {
        return Ok(Value::undefined());
    }
    if lhs.has_type(&[Boolean, Null]) {
        if lhs.convert_to(ctx, Boolean)?.boolean_value(ctx)? {
            return Ok(Value::boolean(true));
        }
        let rhs = right.eval_inner(ctx, try_mode)?;
        if rhs.has_type(&[Boolean, Null]) {
            return Ok(Value::boolean(
                rhs.convert_to(ctx, Boolean)?.boolean_value(ctx)?,
            ));
        }
        if rhs.value_type() == Undefined {
            return Ok(Value::undefined());
        }
        return Err(invalid_operand_types(node, &lhs, &rhs));
    }
    let rhs = right.eval_inner(ctx, try_mode)?;
    if rhs.value_type() == Undefined {
        return Ok(Value::undefined());
    }
    Err(invalid_operand_types(node, &lhs, &rhs))
}

fn compare_as_booleans(
    ctx: &mut EvaluationContext,
    lhs: &Value,
    rhs: &Value,
) -> Result<i32, ElError> {
    let a = lhs
        .convert_to(ctx, ValueType::Boolean)?
        .boolean_value(ctx)?;
    let b = rhs
        .convert_to(ctx, ValueType::Boolean)?
        .boolean_value(ctx)?;
    Ok(if a == b {
        0
    } else if a {
        1
    } else {
        -1
    })
}

fn compare_as_numbers(
    ctx: &mut EvaluationContext,
    lhs: &Value,
    rhs: &Value,
) -> Result<i32, ElError> {
    let diff = lhs.convert_to(ctx, ValueType::Number)?.number_value(ctx)?
        - rhs.convert_to(ctx, ValueType::Number)?.number_value(ctx)?;
    Ok(if diff < 0.0 {
        -1
    } else if diff > 0.0 {
        1
    } else {
        0
    })
}

fn evaluate_compare(
    ctx: &mut EvaluationContext,
    lhs: &Value,
    rhs: &Value,
    node: &ExpressionNode,
) -> Result<i32, ElError> {
    match evaluate_compare_inner(ctx, lhs, rhs, node) {
        Err(ElError::Conversion(msg)) => Err(evaluation_error_node(node, &msg)),
        other => other,
    }
}

fn cmp_i32(o: std::cmp::Ordering) -> i32 {
    match o {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

fn evaluate_compare_inner(
    ctx: &mut EvaluationContext,
    lhs: &Value,
    rhs: &Value,
    node: &ExpressionNode,
) -> Result<i32, ElError> {
    use ValueType::*;
    match lhs.value_type() {
        Boolean => match rhs.value_type() {
            Boolean | Number | String => compare_as_booleans(ctx, lhs, rhs),
            Null | Undefined => Ok(1),
            _ => Err(invalid_operand_types(node, lhs, rhs)),
        },
        Number => match rhs.value_type() {
            Boolean => compare_as_booleans(ctx, lhs, rhs),
            Number | String => compare_as_numbers(ctx, lhs, rhs),
            Null | Undefined => Ok(1),
            _ => Err(invalid_operand_types(node, lhs, rhs)),
        },
        String => match rhs.value_type() {
            Boolean => compare_as_booleans(ctx, lhs, rhs),
            Number => compare_as_numbers(ctx, lhs, rhs),
            String => {
                let a = lhs.string_value(ctx)?;
                let b = rhs.convert_to(ctx, String)?.string_value(ctx)?;
                Ok(cmp_i32(a.as_bytes().cmp(b.as_bytes())))
            }
            Null | Undefined => Ok(1),
            _ => Err(invalid_operand_types(node, lhs, rhs)),
        },
        Null => Ok(if rhs.value_type() == Null { 0 } else { -1 }),
        Undefined => Ok(if rhs.value_type() == Undefined { 0 } else { -1 }),
        Array => match rhs.value_type() {
            Array => {
                let a = lhs.array_value(ctx)?;
                let b = rhs.array_value(ctx)?;
                range_lex_compare_values(ctx, node, &a, &b)
            }
            Null | Undefined => Ok(1),
            _ => Err(invalid_operand_types(node, lhs, rhs)),
        },
        Map => match rhs.value_type() {
            Map => {
                let a: Vec<(std::string::String, Value)> =
                    lhs.map_value(ctx)?.into_iter().collect();
                let b: Vec<(std::string::String, Value)> =
                    rhs.map_value(ctx)?.into_iter().collect();
                range_lex_compare_pairs(ctx, node, &a, &b)
            }
            Null | Undefined => Ok(1),
            _ => Err(invalid_operand_types(node, lhs, rhs)),
        },
        Range => match rhs.value_type() {
            Null | Undefined => Ok(1),
            _ => Err(invalid_operand_types(node, lhs, rhs)),
        },
    }
}

fn range_lex_compare_values(
    ctx: &mut EvaluationContext,
    node: &ExpressionNode,
    a: &[Value],
    b: &[Value],
) -> Result<i32, ElError> {
    let mut i = 0;
    while i < a.len() && i < b.len() {
        let c = evaluate_compare_inner(ctx, &a[i], &b[i], node)?;
        if c < 0 {
            return Ok(-1);
        }
        if c > 0 {
            return Ok(1);
        }
        i += 1;
    }
    Ok(if i < a.len() {
        1
    } else if i < b.len() {
        -1
    } else {
        0
    })
}

fn range_lex_compare_pairs(
    ctx: &mut EvaluationContext,
    node: &ExpressionNode,
    a: &[(String, Value)],
    b: &[(String, Value)],
) -> Result<i32, ElError> {
    let pair_less = |ctx: &mut EvaluationContext,
                     l: &(String, Value),
                     r: &(String, Value)|
     -> Result<bool, ElError> {
        if l.0 < r.0 {
            Ok(true)
        } else if r.0 < l.0 {
            Ok(false)
        } else {
            Ok(evaluate_compare_inner(ctx, &l.1, &r.1, node)? < 0)
        }
    };
    let mut i = 0;
    while i < a.len() && i < b.len() {
        if pair_less(ctx, &a[i], &b[i])? {
            return Ok(-1);
        }
        if pair_less(ctx, &b[i], &a[i])? {
            return Ok(1);
        }
        i += 1;
    }
    Ok(if i < a.len() {
        1
    } else if i < b.len() {
        -1
    } else {
        0
    })
}

fn evaluate_bounded_range(
    ctx: &mut EvaluationContext,
    lhs: &Value,
    rhs: &Value,
) -> Result<Value, ElError> {
    if lhs.value_type() == ValueType::Undefined || rhs.value_type() == ValueType::Undefined {
        return Ok(Value::undefined());
    }
    let from = lhs.convert_to(ctx, ValueType::Number)?.number_value(ctx)? as i64;
    let to = rhs.convert_to(ctx, ValueType::Number)?.number_value(ctx)? as i64;
    Ok(Value::range(Range::Bounded(BoundedRange {
        first: from,
        last: to,
    })))
}

// --- subscripting -------------------------------------------------------------

fn compute_index(index: i64, size: usize) -> usize {
    if size == 0 {
        return 0;
    }
    let s = size as i64;
    if (index >= 0 && index < s) || (index < 0 && index >= -s) {
        ((s + index % s) % s) as usize
    } else {
        size
    }
}

fn compute_index_from_value(
    ctx: &mut EvaluationContext,
    index_value: &Value,
    size: usize,
) -> Result<usize, ElError> {
    let n = index_value
        .convert_to(ctx, ValueType::Number)?
        .number_value(ctx)? as i64;
    Ok(compute_index(n, size))
}

fn compute_index_array(
    ctx: &mut EvaluationContext,
    index_value: &Value,
    size: usize,
    out: &mut Vec<usize>,
) -> Result<(), ElError> {
    match index_value.value_type() {
        ValueType::Array => {
            for entry in index_value.array_value(ctx)? {
                compute_index_array(ctx, &entry, size, out)?;
            }
        }
        ValueType::Range => match index_value.range_value(ctx)? {
            Range::Left(r) => r.for_each(|i| out.push(compute_index(i, size)), size),
            Range::Right(r) => r.for_each(|i| out.push(compute_index(i, size)), size),
            Range::Bounded(r) => r.for_each(|i| out.push(compute_index(i, size))),
        },
        _ => out.push(compute_index_from_value(ctx, index_value, size)?),
    }
    Ok(())
}

fn compute_index_array_vec(
    ctx: &mut EvaluationContext,
    index_value: &Value,
    size: usize,
) -> Result<Vec<usize>, ElError> {
    let mut out = Vec::new();
    compute_index_array(ctx, index_value, size, &mut out)?;
    Ok(out)
}

fn evaluate_subscript(
    ctx: &mut EvaluationContext,
    lhs: &Value,
    rhs: &Value,
    node: &ExpressionNode,
) -> Result<Value, ElError> {
    use ValueType::*;
    match lhs.value_type() {
        String => {
            let s = lhs.string_value(ctx)?;
            let chars: Vec<char> = s.chars().collect();
            match rhs.value_type() {
                Boolean | Number => {
                    let index = compute_index_from_value(ctx, rhs, chars.len())?;
                    Ok(Value::string(
                        chars.get(index).map(|c| c.to_string()).unwrap_or_default(),
                    ))
                }
                Array | Range => {
                    let indices = compute_index_array_vec(ctx, rhs, chars.len())?;
                    let mut result = std::string::String::new();
                    for idx in indices {
                        if let Some(c) = chars.get(idx) {
                            result.push(*c);
                        }
                    }
                    Ok(Value::string(result))
                }
                _ => Err(index_error_node(node, lhs, rhs)),
            }
        }
        Array => {
            let array = lhs.array_value(ctx)?;
            match rhs.value_type() {
                Boolean | Number => {
                    let index = compute_index_from_value(ctx, rhs, array.len())?;
                    array
                        .get(index)
                        .cloned()
                        .ok_or_else(|| index_out_of_bounds_node(node, lhs, index))
                }
                Array | Range => {
                    let indices = compute_index_array_vec(ctx, rhs, array.len())?;
                    let mut result = Vec::with_capacity(indices.len());
                    for idx in indices {
                        match array.get(idx) {
                            Some(v) => result.push(v.clone()),
                            None => return Err(index_out_of_bounds_node(node, lhs, idx)),
                        }
                    }
                    Ok(Value::array(result))
                }
                _ => Err(index_error_node(node, lhs, rhs)),
            }
        }
        Map => {
            let map = lhs.map_value(ctx)?;
            match rhs.value_type() {
                String => {
                    let key = rhs.string_value(ctx)?;
                    Ok(map.get(&key).cloned().unwrap_or_else(Value::undefined))
                }
                Array => {
                    let keys = rhs.array_value(ctx)?;
                    let mut result = BTreeMap::new();
                    for key_value in &keys {
                        if key_value.value_type() != String {
                            return Err(conversion_error(
                                ctx.location(key_value),
                                key_value,
                                key_value.value_type(),
                                String,
                            ));
                        }
                        let key = key_value.string_value(ctx)?;
                        if let Some(v) = map.get(&key) {
                            result.insert(key, v.clone());
                        }
                    }
                    Ok(Value::map(result))
                }
                _ => Err(index_error_node(node, lhs, rhs)),
            }
        }
        _ => Err(index_error_node(node, lhs, rhs)),
    }
}

// --- optimize for binary -----------------------------------------------------

#[allow(unused_assignments)]
fn optimize_binary(
    ctx: &mut EvaluationContext,
    op: BinaryOperation,
    left: &ExpressionNode,
    right: &ExpressionNode,
    node: &ExpressionNode,
) -> Result<ExpressionNode, ElError> {
    use BinaryOperation::*;

    let mut opt_left: Option<ExpressionNode> = None;
    let mut opt_right: Option<ExpressionNode> = None;

    let value = match op {
        LogicalAnd | LogicalOr => {
            // Reproduce the short-circuit rules against optimized-then-evaluated operands.
            let lhs = {
                let o = left.optimize(ctx)?;
                let v = o.evaluate(ctx)?;
                opt_left = Some(o);
                v
            };
            optimize_logical(ctx, op, lhs, right, node, &mut opt_right)?
        }
        Case => {
            let lhs = {
                let o = left.optimize(ctx)?;
                let v = o.evaluate(ctx)?;
                opt_left = Some(o);
                v
            };
            if lhs.value_type() != ValueType::Undefined
                && lhs
                    .convert_to(ctx, ValueType::Boolean)?
                    .boolean_value(ctx)?
            {
                let o = right.optimize(ctx)?;
                let v = o.evaluate(ctx)?;
                opt_right = Some(o);
                v
            } else {
                Value::undefined()
            }
        }
        _ => {
            let l = {
                let o = left.optimize(ctx)?;
                let v = o.evaluate(ctx)?;
                opt_left = Some(o);
                v
            };
            let r = {
                let o = right.optimize(ctx)?;
                let v = o.evaluate(ctx)?;
                opt_right = Some(o);
                v
            };
            evaluate_binary_eager(ctx, op, &l, &r, node)?
        }
    };

    let is_literal = opt_left.as_ref().is_none_or(ExpressionNode::is_literal)
        && opt_right.as_ref().is_none_or(ExpressionNode::is_literal);

    if is_literal {
        return Ok(ExpressionNode::from_parts(
            ExpressionKind::Literal(value),
            node.inner.location,
        ));
    }

    let final_left = match opt_left {
        Some(e) => e,
        None => left.optimize(ctx)?,
    };
    let final_right = match opt_right {
        Some(e) => e,
        None => right.optimize(ctx)?,
    };
    Ok(ExpressionNode::from_parts(
        ExpressionKind::Binary {
            operation: op,
            left: final_left,
            right: final_right,
        },
        node.inner.location,
    ))
}

fn optimize_logical(
    ctx: &mut EvaluationContext,
    op: BinaryOperation,
    lhs: Value,
    right: &ExpressionNode,
    node: &ExpressionNode,
    opt_right: &mut Option<ExpressionNode>,
) -> Result<Value, ElError> {
    use ValueType::*;
    let is_and = op == BinaryOperation::LogicalAnd;

    if lhs.value_type() == Undefined {
        return Ok(Value::undefined());
    }
    if lhs.has_type(&[Boolean, Null]) {
        let lhs_value = lhs.convert_to(ctx, Boolean)?.boolean_value(ctx)?;
        if is_and && !lhs_value {
            return Ok(Value::boolean(false));
        }
        if !is_and && lhs_value {
            return Ok(Value::boolean(true));
        }
        let rhs = {
            let o = right.optimize(ctx)?;
            let v = o.evaluate(ctx)?;
            *opt_right = Some(o);
            v
        };
        if rhs.has_type(&[Boolean, Null]) {
            return Ok(Value::boolean(
                rhs.convert_to(ctx, Boolean)?.boolean_value(ctx)?,
            ));
        }
        if rhs.value_type() == Undefined {
            return Ok(Value::undefined());
        }
        return Err(invalid_operand_types(node, &lhs, &rhs));
    }
    let rhs = {
        let o = right.optimize(ctx)?;
        let v = o.evaluate(ctx)?;
        *opt_right = Some(o);
        v
    };
    if rhs.value_type() == Undefined {
        return Ok(Value::undefined());
    }
    Err(invalid_operand_types(node, &lhs, &rhs))
}
