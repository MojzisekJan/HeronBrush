//! Reader for Forge Game Data files, the entity definitions a `.map` editor loads.
//!
//! Also picks up `// @heron_brush key=value` comments above a class. FGD cannot say
//! "this entity becomes an OmniLight3D", and editors ignore comments.

use crate::math::Vec3;
use indexmap::IndexMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum FgdError {
    #[error("{path}:{line}: {message}")]
    Syntax {
        path: String,
        line: usize,
        message: String,
    },
    #[error("failed to read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum ClassKind {
    Base,
    Point,
    Solid,
}

/// One value of a `choices` property.
#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Choice {
    pub value: String,
    pub label: String,
}

/// One bit of a `flags` property.
#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Flag {
    pub value: i64,
    pub label: String,
    pub default_on: bool,
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Property {
    pub name: String,
    /// The declared FGD type, lowercased: `integer`, `float`, `string`,
    /// `choices`, `flags`, `color255`, `target_source`, and so on.
    pub ty: String,
    pub label: String,
    pub description: String,
    /// Default as written, without surrounding quotes.
    pub default: Option<String>,
    pub choices: Vec<Choice>,
    pub flags: Vec<Flag>,
}

impl Property {
    /// Sum of the flag bits that start switched on.
    pub fn default_flag_bits(&self) -> i64 {
        self.flags
            .iter()
            .filter(|f| f.default_on)
            .map(|f| f.value)
            .sum()
    }
}

/// Build instructions from a `// @heron_brush ...` comment.
#[derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Directives {
    pub entries: IndexMap<String, String>,
}

impl Directives {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    pub fn bool(&self, key: &str) -> Option<bool> {
        match self.get(key)? {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        }
    }

    pub fn list(&self, key: &str) -> Vec<String> {
        self.get(key)
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn merge_under(&mut self, base: &Directives) {
        for (k, v) in &base.entries {
            self.entries.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EntityClass {
    pub kind: ClassKind,
    pub classname: String,
    pub description: String,
    pub bases: Vec<String>,
    /// `size(min, max)` in map units, when present.
    pub size: Option<(Vec3, Vec3)>,
    /// `color(r g b)` as 0-255 components.
    pub color: Option<[u8; 3]>,
    /// Raw contents of `model(...)`, which may also be written as JSON.
    pub model: Option<String>,
    pub properties: IndexMap<String, Property>,
    pub directives: Directives,
}

impl EntityClass {
    pub fn is_solid(&self) -> bool {
        self.kind == ClassKind::Solid
    }

    pub fn is_point(&self) -> bool {
        self.kind == ClassKind::Point
    }

    /// Path from a `model({ "path": "..." })` attribute, when it has one.
    pub fn model_path(&self) -> Option<String> {
        let raw = self.model.as_ref()?;
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
            if let Some(p) = v.get("path").and_then(|p| p.as_str()) {
                return Some(p.to_string());
            }
        }
        let trimmed = raw.trim().trim_matches('"');
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }
}

#[derive(Debug, Default)]
pub struct FgdFile {
    /// Every class, base classes included, keyed by classname.
    pub classes: IndexMap<String, EntityClass>,
    pub warnings: Vec<String>,
}

impl FgdFile {
    /// Point and solid classes with base class properties folded in.
    pub fn resolved(&self) -> IndexMap<String, EntityClass> {
        let mut out = IndexMap::new();
        for (name, class) in &self.classes {
            if class.kind == ClassKind::Base {
                continue;
            }
            let mut seen = HashSet::new();
            out.insert(name.clone(), self.resolve_one(class, &mut seen));
        }
        out
    }

    fn resolve_one(&self, class: &EntityClass, seen: &mut HashSet<String>) -> EntityClass {
        let mut resolved = class.clone();
        if !seen.insert(class.classname.clone()) {
            return resolved;
        }
        // Bases come first so the class's own entries win.
        let mut merged: IndexMap<String, Property> = IndexMap::new();
        for base_name in &class.bases {
            let Some(base) = self.classes.get(base_name) else {
                continue;
            };
            let base = self.resolve_one(base, seen);
            for (k, v) in base.properties {
                merged.insert(k, v);
            }
            resolved.directives.merge_under(&base.directives);
            resolved.size = resolved.size.or(base.size);
            resolved.color = resolved.color.or(base.color);
            resolved.model = resolved.model.clone().or(base.model);
        }
        for (k, v) in &class.properties {
            merged.insert(k.clone(), v.clone());
        }
        resolved.properties = merged;
        seen.remove(&class.classname);
        resolved
    }
}

/// Reads an FGD and everything it pulls in through `@include`.
pub fn parse_file(path: &Path) -> Result<FgdFile, FgdError> {
    let mut file = FgdFile::default();
    let mut visited = HashSet::new();
    parse_into(path, &mut file, &mut visited)?;
    Ok(file)
}

fn parse_into(
    path: &Path,
    out: &mut FgdFile,
    visited: &mut HashSet<PathBuf>,
) -> Result<(), FgdError> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !visited.insert(canonical) {
        return Ok(());
    }
    let text = file_store::read_to_string(path.as_ref()).map_err(|source| FgdError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let parsed = parse_inner(&text, &path.display().to_string())?;

    for include in &parsed.includes {
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let included = dir.join(include);
        if file_store::is_file(&included) {
            parse_into(&included, out, visited)?;
        } else {
            out.warnings.push(format!(
                "{}: cannot find included file `{include}`",
                path.display()
            ));
        }
    }
    for (name, class) in parsed.file.classes {
        out.classes.insert(name, class);
    }
    out.warnings.extend(parsed.file.warnings);
    Ok(())
}

struct ParseResult {
    file: FgdFile,
    includes: Vec<String>,
}

pub fn parse(text: &str, path: &str) -> Result<FgdFile, FgdError> {
    Ok(parse_inner(text, path)?.file)
}

fn parse_inner(text: &str, path: &str) -> Result<ParseResult, FgdError> {
    Parser::new(text, path).run()
}

// -- tokenizer --------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Punct(char),
    Quoted(String),
    Bare(String),
    Comment(String),
}

impl std::fmt::Display for Tok {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tok::Punct(c) => write!(f, "`{c}`"),
            Tok::Quoted(s) => write!(f, "\"{s}\""),
            Tok::Bare(s) => write!(f, "{s}"),
            Tok::Comment(s) => write!(f, "//{s}"),
        }
    }
}

fn tokenize(text: &str) -> Vec<(Tok, usize)> {
    let mut out = Vec::new();
    let mut line = 1usize;
    let bytes = text.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '\n' {
            line += 1;
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            i += 2;
            let start = i;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            out.push((Tok::Comment(text[start..i].to_string()), line));
            continue;
        }
        if matches!(c, '(' | ')' | '[' | ']' | ':' | ',' | '=' | '{' | '}' | '+') {
            out.push((Tok::Punct(c), line));
            i += 1;
            continue;
        }
        if c == '"' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\n' {
                    line += 1;
                }
                i += 1;
            }
            out.push((Tok::Quoted(text[start..i].to_string()), line));
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if c.is_whitespace()
                || matches!(
                    c,
                    '(' | ')' | '[' | ']' | ':' | ',' | '=' | '"' | '{' | '}' | '+'
                )
            {
                break;
            }
            i += 1;
        }
        out.push((Tok::Bare(text[start..i].to_string()), line));
    }
    out
}

/// Reads `key=value key2="two words"` out of a `@heron_brush` comment body.
fn parse_directive_body(body: &str, into: &mut Directives) {
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let key_start = i;
        while i < chars.len() && chars[i] != '=' && !chars[i].is_whitespace() {
            i += 1;
        }
        if i == key_start {
            break;
        }
        let key: String = chars[key_start..i].iter().collect();
        let mut value = String::from("true");
        if i < chars.len() && chars[i] == '=' {
            i += 1;
            if i < chars.len() && (chars[i] == '"' || chars[i] == '\'') {
                let quote = chars[i];
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                value = chars[start..i].iter().collect();
                if i < chars.len() {
                    i += 1;
                }
            } else {
                let start = i;
                while i < chars.len() && !chars[i].is_whitespace() {
                    i += 1;
                }
                value = chars[start..i].iter().collect();
            }
        }
        into.entries.insert(key.to_ascii_lowercase(), value);
    }
}

// -- parser -----------------------------------------------------------------

struct Parser<'a> {
    toks: Vec<(Tok, usize)>,
    pos: usize,
    path: &'a str,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str, path: &'a str) -> Self {
        Parser {
            toks: tokenize(text),
            pos: 0,
            path,
        }
    }

    fn line(&self) -> usize {
        self.toks
            .get(self.pos.min(self.toks.len().saturating_sub(1)))
            .map(|(_, l)| *l)
            .unwrap_or(0)
    }

    fn err<T>(&self, message: impl Into<String>) -> Result<T, FgdError> {
        Err(FgdError::Syntax {
            path: self.path.to_string(),
            line: self.line(),
            message: message.into(),
        })
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(t, _)| t)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).map(|(t, _)| t.clone());
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn eat_punct(&mut self, c: char) -> bool {
        if self.peek() == Some(&Tok::Punct(c)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_punct(&mut self, c: char) -> Result<(), FgdError> {
        if self.eat_punct(c) {
            Ok(())
        } else {
            let found = self
                .peek()
                .map(|t| t.to_string())
                .unwrap_or_else(|| "end of file".into());
            self.err(format!("expected `{c}`, found {found}"))
        }
    }

    /// Reads a string that may be split across `+` continuations.
    fn joined_string(&mut self) -> Option<String> {
        let mut out = match self.peek() {
            Some(Tok::Quoted(s)) => {
                let s = s.clone();
                self.pos += 1;
                s
            }
            _ => return None,
        };
        while self.peek() == Some(&Tok::Punct('+')) {
            self.pos += 1;
            match self.peek() {
                Some(Tok::Quoted(s)) => {
                    out.push_str(s);
                    self.pos += 1;
                }
                _ => break,
            }
        }
        Some(out)
    }

    fn run(mut self) -> Result<ParseResult, FgdError> {
        let mut file = FgdFile::default();
        let mut includes = Vec::new();
        let mut pending = Directives::default();

        while let Some(tok) = self.peek().cloned() {
            match tok {
                Tok::Comment(body) => {
                    self.pos += 1;
                    let trimmed = body.trim();
                    if let Some(rest) = trimmed.strip_prefix("@heron_brush") {
                        let rest = rest.trim_start_matches(':').trim();
                        parse_directive_body(rest, &mut pending);
                    }
                }
                Tok::Bare(word) if word.starts_with('@') => {
                    self.pos += 1;
                    let keyword = word[1..].to_ascii_lowercase();
                    match keyword.as_str() {
                        "include" => {
                            if let Some(Tok::Quoted(p)) = self.next() {
                                includes.push(p);
                            }
                            pending = Directives::default();
                        }
                        "baseclass" | "pointclass" | "solidclass" => {
                            let kind = match keyword.as_str() {
                                "baseclass" => ClassKind::Base,
                                "pointclass" => ClassKind::Point,
                                _ => ClassKind::Solid,
                            };
                            let mut class = self.parse_class(kind)?;
                            class.directives = std::mem::take(&mut pending);
                            file.classes.insert(class.classname.clone(), class);
                        }
                        // @mapsize, @MaterialExclusion, @autovis and friends.
                        _ => {
                            self.skip_attribute_lists();
                            pending = Directives::default();
                        }
                    }
                }
                other => {
                    let other = other.to_string();
                    self.pos += 1;
                    if !other.is_empty() {
                        file.warnings.push(format!(
                            "{}:{}: ignored stray token {other}",
                            self.path,
                            self.line()
                        ));
                    }
                }
            }
        }

        Ok(ParseResult { file, includes })
    }

    /// Steps over `(...)` and `[...]` groups until the next `@` keyword.
    fn skip_attribute_lists(&mut self) {
        loop {
            match self.peek() {
                Some(Tok::Punct('(')) => self.skip_group('(', ')'),
                Some(Tok::Punct('[')) => self.skip_group('[', ']'),
                Some(Tok::Bare(w)) if w.starts_with('@') => return,
                Some(Tok::Comment(_)) | None => return,
                _ => {
                    self.pos += 1;
                }
            }
        }
    }

    fn skip_group(&mut self, open: char, close: char) {
        if !self.eat_punct(open) {
            return;
        }
        let mut depth = 1usize;
        while let Some(tok) = self.next() {
            match tok {
                Tok::Punct(c) if c == open => depth += 1,
                Tok::Punct(c) if c == close => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                _ => {}
            }
        }
    }

    /// Everything between the balanced parens, as raw text.
    fn raw_group(&mut self) -> String {
        let mut out = String::new();
        if !self.eat_punct('(') {
            return out;
        }
        let mut depth = 1usize;
        while let Some(tok) = self.next() {
            match &tok {
                Tok::Punct('(') | Tok::Punct('{') | Tok::Punct('[') => depth += 1,
                Tok::Punct(')') | Tok::Punct('}') | Tok::Punct(']') => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            match tok {
                Tok::Punct(c) => out.push(c),
                Tok::Quoted(s) => {
                    out.push('"');
                    out.push_str(&s);
                    out.push('"');
                }
                Tok::Bare(s) => {
                    if !out.is_empty() && !out.ends_with(['(', '{', '[', ':', ',']) {
                        out.push(' ');
                    }
                    out.push_str(&s);
                }
                Tok::Comment(_) => {}
            }
        }
        out
    }

    fn parse_class(&mut self, kind: ClassKind) -> Result<EntityClass, FgdError> {
        let mut class = EntityClass {
            kind,
            classname: String::new(),
            description: String::new(),
            bases: Vec::new(),
            size: None,
            color: None,
            model: None,
            properties: IndexMap::new(),
            directives: Directives::default(),
        };

        // Class attributes, up to the `=`.
        while !self.eat_punct('=') {
            match self.peek().cloned() {
                Some(Tok::Bare(name)) => {
                    self.pos += 1;
                    let attr = name.to_ascii_lowercase();
                    let body = self.raw_group();
                    match attr.as_str() {
                        "base" => {
                            class.bases = body
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                        }
                        "size" => class.size = parse_size(&body),
                        "color" => class.color = parse_color(&body),
                        "model" | "studio" | "studioprop" | "iconsprite" | "sprite" | "decal"
                            if class.model.is_none() =>
                        {
                            class.model = Some(body);
                        }
                        _ => {}
                    }
                }
                Some(Tok::Comment(_)) => {
                    self.pos += 1;
                }
                Some(t) => {
                    let t = t.to_string();
                    return self.err(format!("unexpected {t} in a class header"));
                }
                None => return self.err("class header ended early"),
            }
        }

        class.classname = match self.next() {
            Some(Tok::Bare(name)) => name,
            Some(t) => return self.err(format!("expected a classname, found {t}")),
            None => return self.err("expected a classname"),
        };

        if self.eat_punct(':') {
            class.description = self.joined_string().unwrap_or_default();
        }

        if self.eat_punct('[') {
            loop {
                match self.peek().cloned() {
                    None => return self.err("unterminated property list"),
                    Some(Tok::Punct(']')) => {
                        self.pos += 1;
                        break;
                    }
                    Some(Tok::Comment(_)) => {
                        self.pos += 1;
                    }
                    Some(Tok::Bare(_)) => {
                        let prop = self.parse_property()?;
                        class.properties.insert(prop.name.clone(), prop);
                    }
                    Some(t) => {
                        let t = t.to_string();
                        return self.err(format!("unexpected {t} in a property list"));
                    }
                }
            }
        }

        Ok(class)
    }

    fn parse_property(&mut self) -> Result<Property, FgdError> {
        let name = match self.next() {
            Some(Tok::Bare(n)) => n,
            Some(t) => return self.err(format!("expected a property name, found {t}")),
            None => return self.err("expected a property name"),
        };

        let ty = self.raw_group().trim().to_ascii_lowercase();
        let mut prop = Property {
            name,
            ty,
            label: String::new(),
            description: String::new(),
            default: None,
            choices: Vec::new(),
            flags: Vec::new(),
        };

        // `readonly` and `report` modifiers sit between the type and the colon.
        while matches!(
            self.peek(),
            Some(Tok::Bare(w)) if matches!(w.to_ascii_lowercase().as_str(), "readonly" | "report")
        ) {
            self.pos += 1;
        }

        if self.eat_punct(':') {
            prop.label = self.joined_string().unwrap_or_default();
            if self.eat_punct(':') {
                prop.default = self.scalar_value();
                if self.eat_punct(':') {
                    prop.description = self.joined_string().unwrap_or_default();
                }
            }
        }

        if self.eat_punct('=') {
            self.expect_punct('[')?;
            let is_flags = prop.ty == "flags";
            loop {
                match self.peek().cloned() {
                    None => return self.err("unterminated choice list"),
                    Some(Tok::Punct(']')) => {
                        self.pos += 1;
                        break;
                    }
                    Some(Tok::Comment(_)) => {
                        self.pos += 1;
                    }
                    _ => {
                        let value = match self.scalar_value() {
                            Some(v) => v,
                            None => return self.err("expected a choice value"),
                        };
                        self.expect_punct(':')?;
                        let label = self.joined_string().unwrap_or_default();
                        if is_flags {
                            let mut default_on = false;
                            if self.eat_punct(':') {
                                default_on = self
                                    .scalar_value()
                                    .map(|v| v.trim() != "0" && !v.trim().is_empty())
                                    .unwrap_or(false);
                            }
                            prop.flags.push(Flag {
                                value: value.trim().parse().unwrap_or(0),
                                label,
                                default_on,
                            });
                        } else {
                            // Some FGDs carry a trailing description on choices.
                            if self.eat_punct(':') {
                                let _ = self.scalar_value();
                            }
                            prop.choices.push(Choice { value, label });
                        }
                    }
                }
            }
        }

        Ok(prop)
    }

    /// A number or quoted string used as a default or choice key.
    fn scalar_value(&mut self) -> Option<String> {
        match self.peek().cloned() {
            Some(Tok::Quoted(_)) => self.joined_string(),
            Some(Tok::Bare(s)) => {
                self.pos += 1;
                Some(s)
            }
            // A negative number arrives as `-` glued to the digits already, but
            // guard against a stray sign token anyway.
            _ => None,
        }
    }
}

fn parse_size(body: &str) -> Option<(Vec3, Vec3)> {
    let nums: Vec<f64> = body
        .replace(',', " ")
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    match nums.len() {
        6 => Some((
            Vec3::new(nums[0], nums[1], nums[2]),
            Vec3::new(nums[3], nums[4], nums[5]),
        )),
        3 => Some((
            Vec3::new(-nums[0] * 0.5, -nums[1] * 0.5, -nums[2] * 0.5),
            Vec3::new(nums[0] * 0.5, nums[1] * 0.5, nums[2] * 0.5),
        )),
        _ => None,
    }
}

fn parse_color(body: &str) -> Option<[u8; 3]> {
    let nums: Vec<f64> = body
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    if nums.len() < 3 {
        return None;
    }
    Some([
        nums[0].clamp(0.0, 255.0) as u8,
        nums[1].clamp(0.0, 255.0) as u8,
        nums[2].clamp(0.0, 255.0) as u8,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
@BaseClass = Targetname
[
    targetname(target_source) : "Name"
]

// @heron_brush node=OmniLight3D auto=true
@PointClass base(Targetname) size(-8 -8 -8, 8 8 8) color(255 255 40)
    model({ "path": "models/lamp.glb" }) = light_omni : "Omni light"
[
    light_color(color255) : "Color" : "255 255 255"
    light_energy(float) : "Energy" : "1.5" : "How bright"
    shadow_enabled(choices) : "Shadows" : 1 =
    [
        0 : "Off"
        1 : "On"
    ]
    spawnflags(flags) =
    [
        1 : "Start off" : 0
        2 : "Flicker" : 1
    ]
]

// @heron_brush node=AnimatableBody3D script=res://scripts/door.gd collision=convex
@SolidClass base(Targetname) = func_door : "A door" []
"#;

    #[test]
    fn reads_classes_properties_and_directives() {
        let fgd = parse(SAMPLE, "t.fgd").unwrap();
        assert_eq!(fgd.classes.len(), 3);

        let light = &fgd.classes["light_omni"];
        assert_eq!(light.kind, ClassKind::Point);
        assert_eq!(light.description, "Omni light");
        assert_eq!(light.bases, vec!["Targetname"]);
        assert_eq!(light.color, Some([255, 255, 40]));
        assert_eq!(light.size.unwrap().1, Vec3::splat(8.0));
        assert_eq!(light.model_path().as_deref(), Some("models/lamp.glb"));
        assert_eq!(light.directives.get("node"), Some("OmniLight3D"));
        assert_eq!(light.directives.bool("auto"), Some(true));

        let energy = &light.properties["light_energy"];
        assert_eq!(energy.ty, "float");
        assert_eq!(energy.default.as_deref(), Some("1.5"));
        assert_eq!(energy.description, "How bright");

        let shadows = &light.properties["shadow_enabled"];
        assert_eq!(shadows.choices.len(), 2);
        assert_eq!(shadows.default.as_deref(), Some("1"));

        let flags = &light.properties["spawnflags"];
        assert_eq!(flags.flags.len(), 2);
        assert_eq!(flags.default_flag_bits(), 2);

        let door = &fgd.classes["func_door"];
        assert_eq!(door.kind, ClassKind::Solid);
        assert_eq!(door.directives.get("script"), Some("res://scripts/door.gd"));
    }

    #[test]
    fn base_class_properties_are_inherited() {
        let fgd = parse(SAMPLE, "t.fgd").unwrap();
        let resolved = fgd.resolved();
        assert!(!resolved.contains_key("Targetname"));
        assert!(resolved["light_omni"].properties.contains_key("targetname"));
        // The class's own properties still come after the inherited ones.
        let keys: Vec<&str> = resolved["light_omni"]
            .properties
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys[0], "targetname");
    }

    #[test]
    fn directives_parse_quoted_values() {
        let mut d = Directives::default();
        parse_directive_body(r#"node=Area3D name="my thing" flag"#, &mut d);
        assert_eq!(d.get("node"), Some("Area3D"));
        assert_eq!(d.get("name"), Some("my thing"));
        assert_eq!(d.bool("flag"), Some(true));
    }
}
