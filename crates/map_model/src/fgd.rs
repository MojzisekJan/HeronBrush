//! Reading entity definitions from an FGD file.
//!
//! An FGD tells the editor what entity classes a game has: what each one is called, what
//! it looks like in the viewport, and what properties it takes. Without one, an entity is
//! an opaque bag of strings; with one, the editor can offer a typed property editor and
//! draw a point entity at the right size.
//!
//! ## Scope
//!
//! Reads entity classes, typed properties, inheritance and expression-based model/default
//! specifications from FGD files.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use geometry_math::bbox::Bbox3d;
use geometry_math::vec::Vec3d;

use crate::model_definition::ModelDefinition;

/// What kind of entity a definition describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DefinitionKind {
    /// A class other definitions inherit from, and which is not itself placeable.
    Base,
    /// An entity placed as a point, drawn as a box or a model.
    Point,
    /// An entity made of brushes.
    Solid,
}

/// What a property holds.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PropertyType {
    /// Free text.
    String,
    /// A whole number.
    Integer,
    /// A number.
    Float,
    /// Yes or no, written as `1` or `0`. No FGD says this; the `.ent` files do.
    Boolean,
    /// One of a fixed set of values.
    Choices(Vec<Choice>),
    /// A set of independent bits, stored as one integer.
    Flags(Vec<Flag>),
    /// A name other entities target.
    TargetSource,
    /// The name of an entity this one targets.
    TargetDestination,
    /// A colour, as three numbers.
    Color,
    /// A type this reader does not know. Kept so the property still appears in the
    /// editor, as free text.
    Other(String),
}

/// One option of a `choices` property.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Choice {
    /// The value written into the map file.
    pub value: String,
    /// What the editor shows.
    pub description: String,
}

/// One bit of a `flags` property.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Flag {
    /// The bit's value, which is a power of two.
    pub value: i64,
    /// What the editor shows.
    pub description: String,
    /// Whether the bit is set by default.
    pub set_by_default: bool,
}

/// One property an entity class takes.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PropertyDefinition {
    /// The key written into the map file.
    pub key: String,
    /// What the property holds.
    pub kind: PropertyType,
    /// The name the editor shows.
    pub short_description: String,
    /// The value used when the property is not set.
    pub default: Option<String>,
    /// The help text the editor shows.
    pub long_description: Option<String>,
}

/// One entity class.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityDefinition {
    /// The `classname` this definition describes.
    pub name: String,
    /// What kind of entity it is.
    pub kind: DefinitionKind,
    /// What the editor shows about it.
    pub description: String,
    /// The classes it inherits properties from.
    pub base_classes: Vec<String>,
    /// The box a point entity is drawn as, relative to its origin.
    pub bounds: Option<Bbox3d>,
    /// The colour the editor draws it in.
    pub color: Option<[u8; 3]>,
    /// The `model(...)` expression -- what model this class draws as, which may depend on
    /// the entity's own properties. See [`crate::model_definition`].
    pub model: Option<ModelDefinition>,
    /// The properties it takes.
    pub properties: Vec<PropertyDefinition>,
}

impl EntityDefinition {
    /// Returns the property with the given key, if this class has one.
    pub fn property(&self, key: &str) -> Option<&PropertyDefinition> {
        self.properties.iter().find(|p| p.key == key)
    }

    /// Returns whether this class is one that can be placed in a map.
    #[inline]
    pub fn is_placeable(&self) -> bool {
        self.kind != DefinitionKind::Base
    }
}

/// A set of entity definitions, with their inheritance resolved.
#[derive(Debug, Clone, Default)]
pub struct EntityDefinitions {
    definitions: Vec<EntityDefinition>,
    /// Things the reader worked around rather than failing on.
    pub warnings: Vec<String>,
    /// Classes that are known, so an entity of that class still shows and edits, but
    /// that [`Self::placeable`] does not offer.
    unoffered: Vec<String>,
}

impl EntityDefinitions {
    /// Builds a set of definitions from classes that have already been resolved.
    ///
    /// What [`read_fgd`] ends with, and what [`crate::def`] hands over: a `.def` file
    /// describes the same classes in an older shape, and what a definition is does not
    /// depend on which file it was written in.
    pub fn new(definitions: Vec<EntityDefinition>, warnings: Vec<String>) -> Self {
        Self {
            definitions,
            warnings,
            unoffered: Vec::new(),
        }
    }

    /// Returns every placeable class, in the order the file defined them, leaving out
    /// the ones [`Self::set_unoffered`] names.
    pub fn placeable(&self) -> impl Iterator<Item = &EntityDefinition> {
        self.definitions
            .iter()
            .filter(|d| d.is_placeable() && !self.unoffered.contains(&d.name))
    }

    /// Names the classes [`Self::placeable`] leaves out. They stay in [`Self::all`] and
    /// [`Self::find`].
    pub fn set_unoffered(&mut self, names: Vec<String>) {
        self.unoffered = names;
    }

    /// Returns every class, including the base classes.
    #[inline]
    pub fn all(&self) -> &[EntityDefinition] {
        &self.definitions
    }

    /// Returns the definition for the given `classname`.
    pub fn find(&self, name: &str) -> Option<&EntityDefinition> {
        self.definitions.iter().find(|d| d.name == name)
    }
}

/// Why an FGD file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FgdError {
    /// What went wrong.
    pub message: String,
    /// The 1-based line it happened on.
    pub line: usize,
}

impl fmt::Display for FgdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for FgdError {}

/// Reads an FGD file.
///
/// Inheritance is resolved as part of reading: a class's own properties come first,
/// followed by those of its base classes that it does not itself define, which is the
/// order an entity inspector shows them in.
pub fn read_fgd(source: &str) -> Result<EntityDefinitions, FgdError> {
    let mut parser = Parser::new(source);
    let raw = parser.parse_all()?;
    let mut warnings = parser.warnings;
    for path in &parser.includes {
        warnings.push(format!(
            "`@include \"{path}\"` was skipped: reading from a string has no directory to resolve it against"
        ));
    }
    Ok(resolve_inheritance(raw, warnings))
}

/// Reads an FGD file from disk, following `@include "other.fgd"` directives.
///
/// Resolves includes relative to their declaring file. Absolute paths, upward escapes and recursive
/// includes are rejected. Definitions are merged before inheritance is resolved.
pub fn read_fgd_file(path: impl AsRef<Path>) -> Result<EntityDefinitions, FgdError> {
    let path = path.as_ref();
    let mut visited = Vec::new();
    let (raw, warnings) = read_fgd_including(path, &mut visited)?;
    Ok(resolve_inheritance(dedup_by_name(raw), warnings))
}

/// Reads one FGD file and, recursively, everything it includes. `visited` holds the files
/// on the current include chain, so a cycle is broken.
fn read_fgd_including(
    path: &Path,
    visited: &mut Vec<PathBuf>,
) -> Result<(Vec<EntityDefinition>, Vec<String>), FgdError> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if visited.contains(&canonical) {
        return Ok((
            Vec::new(),
            vec![format!(
                "`@include` skipped: `{}` includes itself",
                path.display()
            )],
        ));
    }
    visited.push(canonical);

    let source = file_store::read_to_string(path).map_err(|error| FgdError {
        message: format!("cannot read `{}`: {error}", path.display()),
        line: 0,
    })?;

    let mut parser = Parser::new(&source);
    let mut definitions = parser.parse_all()?;
    let mut warnings = std::mem::take(&mut parser.warnings);
    let includes = std::mem::take(&mut parser.includes);
    let directory = path.parent().unwrap_or_else(|| Path::new("."));

    // Included classes go first, so this file can inherit from them.
    let mut all = Vec::new();
    for include in includes {
        let include_path = Path::new(&include);
        let escapes = include_path.is_absolute()
            || include_path
                .components()
                .any(|c| c == std::path::Component::ParentDir);
        if escapes {
            warnings.push(format!(
                "`@include \"{include}\"` was skipped: only a path under `{}` is read",
                directory.display()
            ));
            continue;
        }
        let (included, included_warnings) =
            read_fgd_including(&directory.join(include_path), visited)?;
        all.extend(included);
        warnings.extend(included_warnings);
    }
    all.append(&mut definitions);

    visited.pop();
    Ok((all, warnings))
}

/// Keeps the last definition of each class name, so a file that includes another and then
/// redefines one of its classes wins.
fn dedup_by_name(definitions: Vec<EntityDefinition>) -> Vec<EntityDefinition> {
    let mut last: HashMap<String, usize> = HashMap::new();
    for (index, definition) in definitions.iter().enumerate() {
        last.insert(definition.name.clone(), index);
    }
    definitions
        .into_iter()
        .enumerate()
        .filter(|(index, definition)| last.get(&definition.name) == Some(index))
        .map(|(_, definition)| definition)
        .collect()
}

/// Fills in each class's inherited properties: for a key it does not define it takes each
/// base class's, `spawnflags` is merged bit by bit, and its box, colour and model are
/// taken where it gives none. Shared with the `.def` reader, whose `base(...)` directives
/// name base classes the same way.
pub(crate) fn resolve_inheritance(
    definitions: Vec<EntityDefinition>,
    mut warnings: Vec<String>,
) -> EntityDefinitions {
    let by_name: HashMap<String, EntityDefinition> = definitions
        .iter()
        .map(|d| (d.name.clone(), d.clone()))
        .collect();

    let resolved = definitions
        .iter()
        .map(|definition| {
            let mut result = definition.clone();
            let mut seen = vec![definition.name.clone()];

            // Breadth-first through the base classes, so a class nearer the definition
            // wins over one further away.
            let mut queue: Vec<String> = definition.base_classes.clone();
            while let Some(name) = queue.first().cloned() {
                queue.remove(0);
                if seen.contains(&name) {
                    // A cycle, or a base class named twice. Either way, stop.
                    continue;
                }
                seen.push(name.clone());

                let Some(base) = by_name.get(&name) else {
                    warnings.push(format!(
                        "\"{}\" inherits from \"{name}\", which is not defined",
                        definition.name
                    ));
                    continue;
                };

                for property in &base.properties {
                    match result.properties.iter_mut().find(|p| p.key == property.key) {
                        None => result.properties.push(property.clone()),
                        // Merges inherited spawnflags bit by bit, with explicitly declared class
                        // flags taking precedence.
                        Some(existing)
                            if existing.key == SPAWNFLAGS_KEY
                                && matches!(existing.kind, PropertyType::Flags(_))
                                && matches!(property.kind, PropertyType::Flags(_)) =>
                        {
                            if let (
                                PropertyType::Flags(class_flags),
                                PropertyType::Flags(base_flags),
                            ) = (&existing.kind, &property.kind)
                            {
                                let merged = merge_spawnflags(class_flags, base_flags);
                                existing.kind = PropertyType::Flags(merged);
                            }
                        }
                        Some(_) => {}
                    }
                }
                // A class takes its base's appearance where it gives none of its own.
                result.bounds = result.bounds.or(base.bounds);
                result.color = result.color.or(base.color);
                result.model = result.model.clone().or_else(|| base.model.clone());

                queue.extend(base.base_classes.clone());
            }

            result
        })
        .collect();

    EntityDefinitions::new(resolved, warnings)
}

/// The `spawnflags` key, the one `flags` property that is merged across inheritance.
const SPAWNFLAGS_KEY: &str = "spawnflags";

/// Merges spawnflag bits 1 through 24, preferring the class definition and filling absent bits from
/// its base.
fn merge_spawnflags(class_flags: &[Flag], base_flags: &[Flag]) -> Vec<Flag> {
    let mut merged = Vec::new();
    for bit in 0..24 {
        let value = 1i64 << bit;
        let class_flag = class_flags.iter().find(|f| f.value == value);
        let base_flag = base_flags.iter().find(|f| f.value == value);
        match (class_flag, base_flag) {
            (Some(flag), _) => merged.push(flag.clone()),
            (None, Some(flag)) => merged.push(flag.clone()),
            (None, None) => {}
        }
    }
    merged
}

/// Reads the FGD grammar from the source text.
struct Parser<'a> {
    chars: Vec<char>,
    position: usize,
    line: usize,
    warnings: Vec<String>,
    /// The paths named by `@include` directives, in the order they were read.
    includes: Vec<String>,
    source: &'a str,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            position: 0,
            line: 1,
            warnings: Vec::new(),
            includes: Vec::new(),
            source,
        }
    }

    fn parse_all(&mut self) -> Result<Vec<EntityDefinition>, FgdError> {
        let mut definitions = Vec::new();
        loop {
            self.skip_trivia();
            if self.at_end() {
                let _ = self.source;
                return Ok(definitions);
            }
            if self.peek() != Some('@') {
                return Err(self.error("expected a class definition beginning with `@`"));
            }
            if let Some(definition) = self.parse_class()? {
                definitions.push(definition);
            }
        }
    }

    /// Reads one `@Class ... = name : "description" [ ... ]`.
    ///
    /// Returns `None` for a directive this reader does not handle, such as
    /// `@include`, after skipping it.
    fn parse_class(&mut self) -> Result<Option<EntityDefinition>, FgdError> {
        self.advance(); // the `@`
        let keyword = self.read_word();

        let kind = match keyword.to_ascii_lowercase().as_str() {
            "baseclass" => DefinitionKind::Base,
            "pointclass" => DefinitionKind::Point,
            "solidclass" => DefinitionKind::Solid,
            // `@include "other.fgd"` pulls another file's classes in. The path is kept
            // here; [`read_fgd_file`] resolves it relative to the including file.
            "include" => {
                self.skip_trivia();
                match self.read_string() {
                    Ok(path) => self.includes.push(path),
                    Err(_) => self
                        .warnings
                        .push(format!("line {}: `@include` with no file path", self.line)),
                }
                self.skip_directive();
                return Ok(None);
            }
            // Directives that describe the file rather than a class.
            "mapsize" | "materialexclusion" | "autovisgroup" => {
                self.warnings
                    .push(format!("line {}: skipped `@{keyword}`", self.line));
                self.skip_directive();
                return Ok(None);
            }
            _ => {
                // Report what was written, not the lowercased form that was matched.
                return Err(self.error(format!("unknown class type `@{keyword}`")));
            }
        };

        let mut definition = EntityDefinition {
            name: String::new(),
            kind,
            description: String::new(),
            base_classes: Vec::new(),
            bounds: None,
            color: None,
            model: None,
            properties: Vec::new(),
        };

        // The attributes between the class type and the `=`.
        loop {
            self.skip_trivia();
            match self.peek() {
                Some('=') => {
                    self.advance();
                    break;
                }
                None => return Err(self.error("the class definition ends before its name")),
                _ => self.parse_attribute(&mut definition)?,
            }
        }

        self.skip_trivia();
        definition.name = self.read_word();
        if definition.name.is_empty() {
            return Err(self.error("the class has no name"));
        }

        // An optional `: "description"`, which may be several strings joined by `+`.
        self.skip_trivia();
        if self.peek() == Some(':') {
            self.advance();
            definition.description = self.read_joined_string()?;
        }

        // The property list, which a class without properties may leave out.
        self.skip_trivia();
        if self.peek() == Some('[') {
            definition.properties = self.parse_properties()?;
        }

        Ok(Some(definition))
    }

    /// Reads one `name(arguments)` attribute of a class.
    fn parse_attribute(&mut self, definition: &mut EntityDefinition) -> Result<(), FgdError> {
        let name = self.read_word();
        if name.is_empty() {
            return Err(self.error("expected a class attribute"));
        }

        self.skip_trivia();
        if self.peek() != Some('(') {
            // An attribute with no arguments, such as a bare flag.
            return Ok(());
        }
        let arguments = self.read_parenthesized()?;

        match name.to_ascii_lowercase().as_str() {
            "base" => {
                definition.base_classes = arguments
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "size" => definition.bounds = parse_size(&arguments),
            "color" => definition.color = parse_color(&arguments),
            "model" | "studio" | "iconsprite" | "sprite" => {
                self.read_model(&name, &arguments, definition);
            }
            // Anything else describes appearance in ways the editor does not use yet.
            _ => {}
        }
        Ok(())
    }

    /// Reads `model()`, `studio()`, `iconsprite()` and `sprite()` arguments as EL expressions, with
    /// legacy path/skin/frame syntax as a fallback.
    fn read_model(&mut self, name: &str, arguments: &str, definition: &mut EntityDefinition) {
        if arguments.trim().is_empty() {
            return;
        }

        match crate::model_definition::parse_model_argument(arguments, self.line) {
            Ok(parsed) => {
                if let Some(note) = parsed.deprecation {
                    self.warnings
                        .push(format!("line {}: `{name}(…)`: {note}", self.line));
                }
                definition.model = Some(parsed.definition);
            }
            Err(error) => {
                self.warnings.push(format!(
                    "line {}: `{name}({})` did not parse as a model expression: {error}",
                    self.line,
                    arguments.trim(),
                ));
            }
        }
    }

    /// Reads the `[ ... ]` list of property definitions.
    fn parse_properties(&mut self) -> Result<Vec<PropertyDefinition>, FgdError> {
        self.advance(); // the `[`
        let mut properties = Vec::new();

        loop {
            self.skip_trivia();
            match self.peek() {
                None => return Err(self.error("the property list is never closed")),
                Some(']') => {
                    self.advance();
                    return Ok(properties);
                }
                _ => properties.push(self.parse_property()?),
            }
        }
    }

    /// Reads one `key(type) : "name" : default : "help"` property.
    fn parse_property(&mut self) -> Result<PropertyDefinition, FgdError> {
        let key = self.read_word();
        if key.is_empty() {
            return Err(self.error("expected a property name"));
        }

        self.skip_trivia();
        let type_name = if self.peek() == Some('(') {
            self.read_parenthesized()?.trim().to_ascii_lowercase()
        } else {
            "string".to_string()
        };

        let mut property = PropertyDefinition {
            key,
            kind: PropertyType::String,
            short_description: String::new(),
            default: None,
            long_description: None,
        };

        // `readonly` and similar markers sit between the type and the colon.
        self.skip_trivia();
        while self.peek().is_some_and(|c| c.is_alphabetic()) {
            self.read_word();
            self.skip_trivia();
        }

        // Up to three colon-separated fields: name, default, help.
        for field in 0..3 {
            self.skip_trivia();
            if self.peek() != Some(':') {
                break;
            }
            self.advance();
            self.skip_trivia();

            // A field may be left empty, as in `key(integer) : "Name" : : "Help"`.
            if self.peek() == Some(':') {
                continue;
            }
            let value = self.read_value()?;
            match field {
                0 => property.short_description = value,
                1 => property.default = Some(value),
                _ => property.long_description = Some(value),
            }
        }

        // `choices` and `flags` carry their options in a bracketed list.
        property.kind = match type_name.as_str() {
            "integer" => PropertyType::Integer,
            "float" => PropertyType::Float,
            "string" => PropertyType::String,
            "target_source" => PropertyType::TargetSource,
            "target_destination" => PropertyType::TargetDestination,
            "color255" | "color1" => PropertyType::Color,
            "choices" => PropertyType::Choices(self.parse_choices()?),
            "flags" => PropertyType::Flags(self.parse_flags()?),
            other => PropertyType::Other(other.to_string()),
        };

        Ok(property)
    }

    /// Reads the `= [ value : "name" ]` list of a `choices` property.
    fn parse_choices(&mut self) -> Result<Vec<Choice>, FgdError> {
        let Some(entries) = self.parse_option_list()? else {
            return Ok(Vec::new());
        };
        Ok(entries
            .into_iter()
            .map(|(value, description, _)| Choice { value, description })
            .collect())
    }

    /// Reads the `= [ bit : "name" : default ]` list of a `flags` property.
    fn parse_flags(&mut self) -> Result<Vec<Flag>, FgdError> {
        let Some(entries) = self.parse_option_list()? else {
            return Ok(Vec::new());
        };
        Ok(entries
            .into_iter()
            .map(|(value, description, default)| Flag {
                value: value.trim().parse().unwrap_or(0),
                description,
                set_by_default: default.as_deref().map(str::trim) == Some("1"),
            })
            .collect())
    }

    /// Reads a `= [ ... ]` list, returning each entry's value, name and third field.
    #[allow(clippy::type_complexity)]
    fn parse_option_list(
        &mut self,
    ) -> Result<Option<Vec<(String, String, Option<String>)>>, FgdError> {
        self.skip_trivia();
        if self.peek() != Some('=') {
            return Ok(None);
        }
        self.advance();

        self.skip_trivia();
        if self.peek() != Some('[') {
            return Err(self.error("expected `[` after `=` in an option list"));
        }
        self.advance();

        let mut entries = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                None => return Err(self.error("the option list is never closed")),
                Some(']') => {
                    self.advance();
                    return Ok(Some(entries));
                }
                _ => {}
            }

            let value = self.read_value()?;
            let mut description = String::new();
            let mut third = None;

            self.skip_trivia();
            if self.peek() == Some(':') {
                self.advance();
                self.skip_trivia();
                description = self.read_value()?;

                self.skip_trivia();
                if self.peek() == Some(':') {
                    self.advance();
                    self.skip_trivia();
                    third = Some(self.read_value()?);
                }
            }

            entries.push((value, description, third));
        }
    }

    /* ========== lexing ========== */

    fn at_end(&self) -> bool {
        self.position >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.position + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.position += 1;
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn error(&self, message: impl Into<String>) -> FgdError {
        FgdError {
            message: message.into(),
            line: self.line,
        }
    }

    /// Skips whitespace and `//` comments.
    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while self.peek().is_some_and(|c| c != '\n') {
                        self.advance();
                    }
                }
                _ => return,
            }
        }
    }

    /// Reads an identifier: letters, digits, and the punctuation FGD allows in one.
    fn read_word(&mut self) -> String {
        let mut value = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '*' | '+' | '#') {
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }
        value
    }

    /// Reads a quoted string, resolving its escapes.
    fn read_string(&mut self) -> Result<String, FgdError> {
        self.advance(); // the opening quote
        let mut value = String::new();
        loop {
            match self.advance() {
                None => return Err(self.error("unterminated string")),
                Some('"') => return Ok(value),
                Some('\\') => match self.peek() {
                    Some(escaped @ ('"' | '\\')) => {
                        self.advance();
                        value.push(escaped);
                    }
                    // FGD files carry Windows paths, where a lone backslash is literal.
                    _ => value.push('\\'),
                },
                Some(c) => value.push(c),
            }
        }
    }

    /// Reads a value: a quoted string, possibly continued with `+`, or a bare word.
    fn read_value(&mut self) -> Result<String, FgdError> {
        self.skip_trivia();
        if self.peek() == Some('"') {
            self.read_joined_string()
        } else {
            Ok(self.read_word())
        }
    }

    /// Reads one or more quoted strings joined by `+`, which is how FGD writes a long
    /// help text across several lines.
    fn read_joined_string(&mut self) -> Result<String, FgdError> {
        self.skip_trivia();
        let mut value = self.read_string()?;
        loop {
            self.skip_trivia();
            if self.peek() != Some('+') {
                return Ok(value);
            }
            self.advance();
            self.skip_trivia();
            if self.peek() != Some('"') {
                return Err(self.error("expected a string after `+`"));
            }
            value.push_str(&self.read_string()?);
        }
    }

    /// Reads the text between a matched pair of parentheses.
    fn read_parenthesized(&mut self) -> Result<String, FgdError> {
        self.advance(); // the `(`
        let mut value = String::new();
        let mut depth = 1usize;

        loop {
            match self.advance() {
                None => return Err(self.error("unterminated `(`")),
                Some('(') => {
                    depth += 1;
                    value.push('(');
                }
                Some(')') => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(value);
                    }
                    value.push(')');
                }
                Some(c) => value.push(c),
            }
        }
    }

    /// Skips a directive this reader does not handle, to the end of its line or its
    /// bracketed body.
    fn skip_directive(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                '\n' => {
                    self.advance();
                    return;
                }
                '[' => {
                    let mut depth = 0usize;
                    loop {
                        match self.advance() {
                            None => return,
                            Some('[') => depth += 1,
                            Some(']') => {
                                depth -= 1;
                                if depth == 0 {
                                    return;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
    }
}

/// Reads a `size(-16 -16 -24, 16 16 32)` attribute.
///
/// A size with one triple means a box that size centred on the origin.
fn parse_size(arguments: &str) -> Option<Bbox3d> {
    let parts: Vec<&str> = arguments.split(',').collect();
    match parts.len() {
        1 => {
            let size = parse_vec3(parts[0])?;
            Some(Bbox3d::new_repaired(-size / 2.0, size / 2.0))
        }
        2 => {
            let min = parse_vec3(parts[0])?;
            let max = parse_vec3(parts[1])?;
            Some(Bbox3d::new_repaired(min, max))
        }
        _ => None,
    }
}

/// Reads a `color(255 128 0)` attribute.
fn parse_color(arguments: &str) -> Option<[u8; 3]> {
    let numbers: Vec<f64> = arguments
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    if numbers.len() != 3 {
        return None;
    }
    Some([
        numbers[0].clamp(0.0, 255.0) as u8,
        numbers[1].clamp(0.0, 255.0) as u8,
        numbers[2].clamp(0.0, 255.0) as u8,
    ])
}

/// Reads three whitespace-separated numbers.
fn parse_vec3(text: &str) -> Option<Vec3d> {
    let numbers: Vec<f64> = text
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    (numbers.len() == 3).then(|| Vec3d::new(numbers[0], numbers[1], numbers[2]))
}
