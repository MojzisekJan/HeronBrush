//! Model specifications used to render point entities.
//!
//! An FGD `model(...)` attribute (or a `.def` `/*QUAKED*/` model field) is not a fixed
//! path: it is an [expression](expression_lang) evaluated against the entity's own properties, so a
//! class can pick its model, skin and frame from a `spawnflags` bit or a `targetname`.
//! [`ModelDefinition`] holds the parsed expression; [`ModelDefinition::model_specification`]
//! evaluates it for one entity into a [`ModelSpecification`] -- a path, a skin index and a
//! frame index.

use std::path::PathBuf;

use expression_lang::expression::ExpressionKind;
use expression_lang::{EvaluationContext, ExpressionNode, ValueType, VariableStore, VariableTable};
use geometry_math::vec::Vec3d;

/// The keys a `model(...)` map may carry.
pub mod keys {
    /// The model file.
    pub const PATH: &str = "path";
    /// Which skin of the model.
    pub const SKIN: &str = "skin";
    /// Which animation frame.
    pub const FRAME: &str = "frame";
    /// The scale, a number or `"x y z"`.
    pub const SCALE: &str = "scale";
}

/// A model to draw, once a [`ModelDefinition`] has been evaluated for an entity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ModelSpecification {
    /// The model file, relative to the game path. Empty when the class names no model.
    pub path: PathBuf,
    /// Which skin.
    pub skin_index: usize,
    /// Which animation frame.
    pub frame_index: usize,
}

impl ModelSpecification {
    /// A specification with just a path.
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), skin_index: 0, frame_index: 0 }
    }
}

/// The `model(...)` expression for one entity class.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelDefinition {
    expression: ExpressionNode,
}

impl Default for ModelDefinition {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelDefinition {
    /// A definition that names no model.
    pub fn new() -> Self {
        Self { expression: ExpressionNode::literal(expression_lang::Value::undefined()) }
    }

    /// A definition from an already-parsed expression.
    pub fn from_expression(expression: ExpressionNode) -> Self {
        Self { expression }
    }

    /// The expression, for the app that wants to optimize or inspect it.
    pub fn expression(&self) -> &ExpressionNode {
        &self.expression
    }

    /// Folds `other` in as a fallback: this definition is tried first, then `other`. A
    /// class inheriting from a base with its own `model(...)` gets both.
    pub fn append(&mut self, other: ModelDefinition) {
        let location = self.expression.location();
        let cases = vec![self.expression.clone(), other.expression];
        self.expression = ExpressionNode::new(ExpressionKind::Switch(cases), location);
    }

    /// Evaluates the expression against `variables` (the entity's properties) and reads
    /// the result as a model.
    pub fn model_specification(
        &self,
        variables: &dyn VariableStore,
    ) -> Result<ModelSpecification, String> {
        let mut context = EvaluationContext::with_variables(variables);
        let value = self.expression.evaluate(&mut context).map_err(expression_lang::ElError::into_message)?;
        Ok(convert_to_model(&mut context, &value))
    }

    /// [`ModelDefinition::model_specification`] against an entity's property list, so a
    /// caller need not build a variable store.
    pub fn resolve(&self, properties: &[(String, String)]) -> Result<ModelSpecification, String> {
        self.model_specification(&variable_table(properties))
    }

    /// [`safe_get_model_scale`] against an entity's property list. `1, 1, 1` unless the
    /// model map carries a `scale`, or `default_scale_expression` yields one.
    pub fn resolve_scale(
        &self,
        properties: &[(String, String)],
        default_scale_expression: Option<&ExpressionNode>,
    ) -> Vec3d {
        safe_get_model_scale(self, &variable_table(properties), default_scale_expression)
    }

    /// Evaluates the expression with no variables, swallowing evaluation errors -- what
    /// the entity browser shows before an entity is placed. `defaultModelSpecification`.
    pub fn default_model_specification(&self) -> Result<ModelSpecification, String> {
        let mut context = EvaluationContext::new();
        let value =
            self.expression.try_evaluate(&mut context).map_err(expression_lang::ElError::into_message)?;
        Ok(convert_to_model(&mut context, &value))
    }

    /// The model's scale for one entity. A `scale` in the model map wins; otherwise
    /// `default_scale_expression` (the game configuration's own) is tried; otherwise
    /// `1, 1, 1`.
    pub fn scale(
        &self,
        variables: &dyn VariableStore,
        default_scale_expression: Option<&ExpressionNode>,
    ) -> Result<Vec3d, String> {
        let mut context = EvaluationContext::with_variables(variables);
        let value = self.expression.evaluate(&mut context).map_err(expression_lang::ElError::into_message)?;

        if value.value_type() == ValueType::Map {
            let scale_value = value
                .at_or_default_key(&context, keys::SCALE, expression_lang::Value::null())
                .map_err(expression_lang::ElError::into_message)?;
            if let Some(scale) = convert_to_scale(&mut context, &scale_value)? {
                return Ok(scale);
            }
        }

        if let Some(expression) = default_scale_expression {
            let default_value =
                expression.evaluate(&mut context).map_err(expression_lang::ElError::into_message)?;
            if let Some(scale) = convert_to_scale(&mut context, &default_value)? {
                return Ok(scale);
            }
        }

        Ok(Vec3d::new(1.0, 1.0, 1.0))
    }
}

/// [`ModelDefinition::scale`], or `1, 1, 1` if anything goes wrong. `safeGetModelScale`.
pub fn safe_get_model_scale(
    definition: &ModelDefinition,
    variables: &dyn VariableStore,
    default_scale_expression: Option<&ExpressionNode>,
) -> Vec3d {
    definition
        .scale(variables, default_scale_expression)
        .unwrap_or_else(|_| Vec3d::new(1.0, 1.0, 1.0))
}

/// Parses `expression` as a model definition. A convenience over [`expression_lang::parse_expression`].
pub fn parse_model_definition(expression: &str) -> Result<ModelDefinition, String> {
    expression_lang::parse_expression(expression_lang::ParseMode::Strict, expression)
        .map(ModelDefinition::from_expression)
        .map_err(expression_lang::ElError::into_message)
}

/// Runs `expression_lang`'s constant folder over a model expression. `optimizeModelExpression`.
pub fn optimize_model_expression(expression: &ExpressionNode) -> Result<ExpressionNode, String> {
    let mut context = EvaluationContext::new();
    expression.optimize(&mut context).map_err(expression_lang::ElError::into_message)
}

/// What a `model(...)` / `studio(...)` / `iconsprite(...)` / `sprite(...)` argument
/// parsed to, along with a note if it used the deprecated legacy syntax.
pub struct ParsedModel {
    /// The definition.
    pub definition: ModelDefinition,
    /// A deprecation message, set only when the argument used the legacy syntax.
    pub deprecation: Option<String>,
}

/// Parses a `model(...)` argument the way `parseModelDefinition` does: as an [`expression_lang`]
/// expression first, then, if that fails, as the legacy `"path" skin frame, …` syntax.
/// The constants are folded either way. `line` is where the argument sits in the source,
/// for the messages. An `Err` is a value that parsed as neither.
pub fn parse_model_argument(arguments: &str, line: usize) -> Result<ParsedModel, String> {
    let (expression, deprecation) =
        match expression_lang::parse_expression(expression_lang::ParseMode::Strict, arguments) {
            Ok(node) => (node, None),
            Err(modern_error) => {
                match crate::legacy_model_definition::parse_legacy_model_definition(
                    arguments, line,
                ) {
                    Ok(node) => {
                        let note = format!(
                            "the legacy model syntax is deprecated, replace it with `{}`",
                            node.as_string()
                        );
                        (node, Some(note))
                    }
                    Err(_) => return Err(modern_error.into_message()),
                }
            }
        };

    let expression = optimize_model_expression(&expression).unwrap_or(expression);
    Ok(ParsedModel { definition: ModelDefinition::from_expression(expression), deprecation })
}

// --- reading an evaluated value as a model -----------------------------------------

fn model_path(context: &EvaluationContext, value: &expression_lang::Value) -> PathBuf {
    if value.value_type() != ValueType::String {
        return PathBuf::new();
    }
    let string = value.string_value(context).unwrap_or_default();
    let string = string.strip_prefix(':').unwrap_or(&string);
    PathBuf::from(string)
}

fn model_index(context: &mut EvaluationContext, value: &expression_lang::Value) -> usize {
    if !value.convertible_to(ValueType::Number) {
        return 0;
    }
    match value.convert_to(context, ValueType::Number).and_then(|n| n.integer_value(context)) {
        Ok(n) => n.max(0) as usize,
        Err(_) => 0,
    }
}

fn convert_to_model(context: &mut EvaluationContext, value: &expression_lang::Value) -> ModelSpecification {
    match value.value_type() {
        ValueType::Map => {
            let path_value = value
                .at_or_default_key(context, keys::PATH, expression_lang::Value::null())
                .unwrap_or_else(|_| expression_lang::Value::null());
            let skin_value = value
                .at_or_default_key(context, keys::SKIN, expression_lang::Value::null())
                .unwrap_or_else(|_| expression_lang::Value::null());
            let frame_value = value
                .at_or_default_key(context, keys::FRAME, expression_lang::Value::null())
                .unwrap_or_else(|_| expression_lang::Value::null());
            ModelSpecification {
                path: model_path(context, &path_value),
                skin_index: model_index(context, &skin_value),
                frame_index: model_index(context, &frame_value),
            }
        }
        ValueType::String => ModelSpecification {
            path: model_path(context, value),
            skin_index: 0,
            frame_index: 0,
        },
        _ => ModelSpecification::default(),
    }
}

// --- reading an evaluated value as a scale ----------------------------------------

fn parse_vec3(string: &str) -> Option<Vec3d> {
    let parts: Vec<f64> = string
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|part| !part.is_empty())
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    match parts[..] {
        [x, y, z] => Some(Vec3d::new(x, y, z)),
        _ => None,
    }
}

fn scale_value(
    context: &mut EvaluationContext,
    value: &expression_lang::Value,
) -> Result<Option<Vec3d>, String> {
    if value.value_type() == ValueType::Number {
        let scale = value.number_value(context).map_err(expression_lang::ElError::into_message)?;
        return Ok(Some(Vec3d::new(scale, scale, scale)));
    }

    if value.value_type() != ValueType::String {
        return Ok(None);
    }

    let string = value.string_value(context).map_err(expression_lang::ElError::into_message)?;
    if string.trim().is_empty() {
        return Ok(None);
    }

    if let Some(scale) = parse_vec3(&string) {
        return Ok(Some(scale));
    }

    if !value.convertible_to(ValueType::Number) {
        return Ok(None);
    }
    let scale = value
        .convert_to(context, ValueType::Number)
        .and_then(|n| n.number_value(context))
        .map_err(expression_lang::ElError::into_message)?;
    Ok(Some(Vec3d::new(scale, scale, scale)))
}

fn convert_to_scale(
    context: &mut EvaluationContext,
    value: &expression_lang::Value,
) -> Result<Option<Vec3d>, String> {
    if value.value_type() == ValueType::Array {
        let array = value.array_value(context).map_err(expression_lang::ElError::into_message)?;
        for entry in &array {
            if let Some(scale) = scale_value(context, entry)? {
                return Ok(Some(scale));
            }
        }
        return Ok(None);
    }
    scale_value(context, value)
}

// -- convenience for callers that want a plain empty store -------------------------

/// An empty variable table, for `default_model_specification`-style calls that pass one.
pub fn no_variables() -> VariableTable {
    VariableTable::new()
}

/// An entity's properties as a variable table, every value a string -- which is what a
/// `.map` file holds and what the model expression reads.
fn variable_table(properties: &[(String, String)]) -> VariableTable {
    properties
        .iter()
        .map(|(key, value)| (key.clone(), expression_lang::Value::string(value.clone())))
        .collect()
}
