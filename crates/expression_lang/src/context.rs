//! Evaluation contexts and variable stores.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use crate::expression::ExpressionNode;
use crate::value::Value;
use crate::FileLocation;

/// A store of named values available to an expression.
pub trait VariableStore {
    /// A boxed copy of this store.
    fn box_clone(&self) -> Box<dyn VariableStore>;
    /// How many names the store holds.
    fn len(&self) -> usize;
    /// Whether the store contains no bindings.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// The value for `name`.
    fn value(&self, name: &str) -> Value;
    /// Every name, sorted.
    fn names(&self) -> Vec<String>;
    /// Sets `name` to `value`.
    fn set(&mut self, name: String, value: Value);
}

impl Clone for Box<dyn VariableStore> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

fn fmt_store(store: &dyn VariableStore, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("{\n")?;
    for (index, name) in store.names().iter().enumerate() {
        if index > 0 {
            f.write_str(", ")?;
        }
        write!(f, "  {name}: {}", store.value(name))?;
    }
    f.write_str("}")
}

impl fmt::Display for dyn VariableStore + '_ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_store(self, f)
    }
}

impl fmt::Display for VariableTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_store(self, f)
    }
}

impl fmt::Display for NullVariableStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_store(self, f)
    }
}

/// Whether two stores hold the same names bound to the same values, whatever their type.
pub fn stores_equal(lhs: &dyn VariableStore, rhs: &dyn VariableStore) -> bool {
    let names = lhs.names();
    names == rhs.names() && names.iter().all(|n| lhs.value(n) == rhs.value(n))
}

/// A map of variable names to values.
#[derive(Clone, Default)]
pub struct VariableTable {
    variables: BTreeMap<String, Value>,
}

impl VariableTable {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// A table holding the given bindings.
    pub fn from_bindings(variables: BTreeMap<String, Value>) -> Self {
        Self { variables }
    }
}

impl<K: Into<String>, V: Into<Value>> FromIterator<(K, V)> for VariableTable {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self {
            variables: iter
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }
}

impl VariableStore for VariableTable {
    fn box_clone(&self) -> Box<dyn VariableStore> {
        Box::new(self.clone())
    }
    fn len(&self) -> usize {
        self.variables.len()
    }
    fn value(&self, name: &str) -> Value {
        self.variables
            .get(name)
            .cloned()
            .unwrap_or_else(Value::undefined)
    }
    fn names(&self) -> Vec<String> {
        self.variables.keys().cloned().collect()
    }
    fn set(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }
}

/// An empty store that returns `null` for every name.
#[derive(Clone, Default)]
pub struct NullVariableStore;

impl VariableStore for NullVariableStore {
    fn box_clone(&self) -> Box<dyn VariableStore> {
        Box::new(NullVariableStore)
    }
    fn len(&self) -> usize {
        0
    }
    fn value(&self, _name: &str) -> Value {
        Value::null()
    }
    fn names(&self) -> Vec<String> {
        Vec::new()
    }
    fn set(&mut self, _name: String, _value: Value) {}
}

/// The state threaded through an evaluation: the variables in scope, and a trace mapping
/// each value produced back to the expression that produced it, so that an error can name
/// a source location.
///
/// Trace keys use value identity. Keeping the value in the map keeps its allocation alive while its
/// address is used as a key.
pub struct EvaluationContext {
    variables: Box<dyn VariableStore>,
    trace: HashMap<usize, (Value, ExpressionNode)>,
}

impl EvaluationContext {
    /// A context with no variables.
    pub fn new() -> Self {
        Self {
            variables: Box::new(VariableTable::new()),
            trace: HashMap::new(),
        }
    }

    /// A context over a copy of the given variable store.
    pub fn with_variables(variables: &dyn VariableStore) -> Self {
        Self {
            variables: variables.box_clone(),
            trace: HashMap::new(),
        }
    }

    /// The value bound to `name`.
    pub fn variable_value(&self, name: &str) -> Value {
        self.variables.value(name)
    }

    /// The expression `value` was traced to, if any.
    pub fn expression(&self, value: &Value) -> Option<ExpressionNode> {
        self.trace.get(&value.ptr()).map(|(_, e)| e.clone())
    }

    /// The source location `value` was traced to, if any.
    pub fn location(&self, value: &Value) -> Option<FileLocation> {
        self.expression(value).and_then(|e| e.location())
    }

    /// Records the expression that produced `value` and returns the value. The first recorded
    /// expression wins.
    pub fn trace(&mut self, value: Value, expression: &ExpressionNode) -> Value {
        self.trace
            .entry(value.ptr())
            .or_insert_with(|| (value.clone(), expression.clone()));
        value
    }

    /// Records that `value` came from wherever `original` came from, and returns `value`.
    pub fn trace_to_value(&mut self, value: Value, original: &Value) -> Value {
        match self.expression(original) {
            Some(expression) => self.trace(value, &expression),
            None => value,
        }
    }
}

impl Default for EvaluationContext {
    fn default() -> Self {
        Self::new()
    }
}
