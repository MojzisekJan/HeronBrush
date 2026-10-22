//! Reader for the Quake `.map` format as level editors write it.
//!
//! Handles the Standard (Quake) and Valve 220 texture formats, the extra
//! Quake II / III face attributes, and the `_tb_` layer and group entities.
//! `patchDef` and `brushDef` blocks are recognised and skipped, with a warning.

use crate::math::{Plane, Vec2, Vec3};
use indexmap::IndexMap;
use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum MapError {
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

/// How a face's texture coordinates were written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UvFormat {
    /// `texture offset_x offset_y rotation scale_x scale_y`
    Standard,
    /// `texture [ ux uy uz ox ] [ vx vy vz oy ] rotation scale_x scale_y`
    Valve220,
}

#[derive(Debug, Clone)]
pub struct Face {
    pub plane: Plane,
    /// The three points as written, kept for round-tripping and diagnostics.
    pub points: [Vec3; 3],
    pub texture: String,
    pub format: UvFormat,
    /// U and V axes. Only meaningful for [`UvFormat::Valve220`].
    pub uv_axes: [Vec3; 2],
    pub offset: Vec2,
    pub rotation: f64,
    pub scale: Vec2,
    /// Quake II / III surface attributes, when the face carries them.
    pub surface: Option<SurfaceAttribs>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SurfaceAttribs {
    pub content_flags: i64,
    pub surface_flags: i64,
    pub value: i64,
}

#[derive(Debug, Clone, Default)]
pub struct Brush {
    pub faces: Vec<Face>,
}

impl Brush {
    pub fn planes(&self) -> Vec<Plane> {
        self.faces.iter().map(|f| f.plane).collect()
    }
}

/// A bezier patch: a grid of control points that bends.
///
/// Quake 3 draws these instead of brushes wherever something has to curve. The
/// grid is odd sized in both directions, and every three by three corner of it
/// is one biquadratic bezier surface.
#[derive(Debug, Clone, Default)]
pub struct Patch {
    pub texture: String,
    pub width: usize,
    pub height: usize,
    /// `width * height` control points, row by row.
    pub points: Vec<Vec3>,
    /// One UV per control point, bent along with the surface.
    pub uvs: Vec<Vec2>,
    /// What `patchDef3` asked for, when the map says.
    pub subdivisions: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Default)]
pub struct Entity {
    pub properties: IndexMap<String, String>,
    pub brushes: Vec<Brush>,
    pub patches: Vec<Patch>,
    /// Imported triangle surfaces in world map coordinates, supplied by the editor.
    pub external_surfaces: Vec<crate::geometry::Surface>,
    /// Optional source mesh and placement, used when instancing preserves behavior.
    pub external_asset: Option<crate::instancing::ModelAsset>,
    /// Index into [`MapFile::entities`] of the source entity, before any
    /// layer or group entities were folded away.
    pub source_index: usize,
}

impl Entity {
    pub fn classname(&self) -> &str {
        self.properties
            .get("classname")
            .map(String::as_str)
            .unwrap_or("")
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.properties.get(key).map(String::as_str)
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.trim().parse().ok())
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key)
            .and_then(|v| v.trim().parse::<f64>().ok())
            .map(|v| v as i64)
    }

    /// Parses a whitespace separated vector property such as `origin`.
    pub fn get_vec3(&self, key: &str) -> Option<Vec3> {
        let v = self.get(key)?;
        let mut it = v.split_whitespace().filter_map(|t| t.parse::<f64>().ok());
        Some(Vec3::new(it.next()?, it.next()?, it.next()?))
    }
}

/// A map layer or group. Their brushes are merged into worldspawn;
/// the node hierarchy is rebuilt from these entries.
#[derive(Debug, Clone)]
pub struct Group {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub kind: GroupKind,
    pub omit_from_export: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    Layer,
    Group,
}

#[derive(Debug, Default)]
pub struct MapFile {
    pub entities: Vec<Entity>,
    pub groups: Vec<Group>,
    pub warnings: Vec<String>,
}

impl MapFile {
    pub fn worldspawn(&self) -> Option<&Entity> {
        self.entities.first()
    }

    /// Layer or group an entity belongs to, from `_tb_layer` / `_tb_group`.
    pub fn group_of(&self, entity: &Entity) -> Option<&Group> {
        let id = entity
            .get_i64("_tb_group")
            .or_else(|| entity.get_i64("_tb_layer"))?;
        self.groups.iter().find(|g| g.id == id)
    }
}

pub fn parse_file(path: &std::path::Path) -> Result<MapFile, MapError> {
    let text = std::fs::read_to_string(path).map_err(|source| MapError::Io {
        path: path.display().to_string(),
        source,
    })?;
    parse(&text, &path.display().to_string())
}

pub fn parse(text: &str, path: &str) -> Result<MapFile, MapError> {
    Parser::new(text, path).run()
}

// -- tokenizer --------------------------------------------------------------

/// A token pointing into the map text rather than owning a copy of it. Forty
/// thousand brushes is seven and a half million tokens, nearly all of them
/// numbers, and an allocation each cost more than the rest of the parser.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Tok<'a> {
    Punct(char),
    Quoted(&'a str),
    Bare(&'a str),
}

impl fmt::Display for Tok<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::Punct(c) => write!(f, "`{c}`"),
            Tok::Quoted(s) => write!(f, "\"{s}\""),
            Tok::Bare(s) => write!(f, "{s}"),
        }
    }
}

/// What each byte means to the tokenizer. One load instead of a chain of
/// comparisons the processor cannot predict.
const SPACE: u8 = 1;
const PUNCT: u8 = 2;
const QUOTE: u8 = 4;
const ENDS_A_WORD: u8 = SPACE | PUNCT | QUOTE;

const CLASS: [u8; 256] = {
    let mut table = [0u8; 256];
    table[b' ' as usize] = SPACE;
    table[b'\t' as usize] = SPACE;
    table[b'\r' as usize] = SPACE;
    table[b'\n' as usize] = SPACE;
    table[0x0b] = SPACE;
    table[0x0c] = SPACE;
    table[b'{' as usize] = PUNCT;
    table[b'}' as usize] = PUNCT;
    table[b'(' as usize] = PUNCT;
    table[b')' as usize] = PUNCT;
    table[b'[' as usize] = PUNCT;
    table[b']' as usize] = PUNCT;
    table[b'"' as usize] = QUOTE;
    table
};

/// Hands out one token at a time. Collecting them first cost a fifth of a
/// gigabyte on a forty thousand brush level, all of it read once.
struct Lexer<'a> {
    text: &'a str,
    at: usize,
    line: usize,
    ahead: Option<(Tok<'a>, usize)>,
}

impl<'a> Lexer<'a> {
    fn new(text: &'a str) -> Self {
        Lexer {
            text,
            at: 0,
            line: 1,
            ahead: None,
        }
    }

    fn peek(&mut self) -> Option<Tok<'a>> {
        self.fill();
        self.ahead.map(|(t, _)| t)
    }

    fn next(&mut self) -> Option<Tok<'a>> {
        self.fill();
        self.ahead.take().map(|(t, _)| t)
    }

    /// The line the next token is on, for error messages.
    fn line(&mut self) -> usize {
        self.fill();
        self.ahead.map(|(_, l)| l).unwrap_or(self.line)
    }

    fn fill(&mut self) {
        if self.ahead.is_none() {
            self.ahead = self.pull();
        }
    }

    fn pull(&mut self) -> Option<(Tok<'a>, usize)> {
        let bytes = self.text.as_bytes();
        while self.at < bytes.len() {
            let b = bytes[self.at];
            let class = CLASS[b as usize];
            if class & SPACE != 0 {
                self.line += (b == b'\n') as usize;
                self.at += 1;
                continue;
            }
            if b == b'/' && self.at + 1 < bytes.len() && bytes[self.at + 1] == b'/' {
                while self.at < bytes.len() && bytes[self.at] != b'\n' {
                    self.at += 1;
                }
                continue;
            }
            if class & PUNCT != 0 {
                self.at += 1;
                return Some((Tok::Punct(b as char), self.line));
            }
            if class & QUOTE != 0 {
                self.at += 1;
                let start = self.at;
                let line = self.line;
                while self.at < bytes.len() && bytes[self.at] != b'"' {
                    if bytes[self.at] == b'\\'
                        && self.at + 1 < bytes.len()
                        && matches!(bytes[self.at + 1], b'"' | b'\\')
                    {
                        self.at += 2;
                    } else {
                        self.line += (bytes[self.at] == b'\n') as usize;
                        self.at += 1;
                    }
                }
                let word = &self.text[start..self.at];
                self.at += 1; // closing quote
                return Some((Tok::Quoted(word), line));
            }
            let start = self.at;
            while self.at < bytes.len() && CLASS[bytes[self.at] as usize] & ENDS_A_WORD == 0 {
                self.at += 1;
            }
            return Some((Tok::Bare(&self.text[start..self.at]), self.line));
        }
        None
    }
}

// Quake strings escape quotes and backslashes, but retain unknown escapes
// such as Windows path separators. Keep this consistent with map_model.
fn unescape(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && matches!(chars.peek(), Some('"' | '\\')) {
            result.push(chars.next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

// -- parser -----------------------------------------------------------------

/// What one block inside an entity turned out to be.
enum Block {
    Brush(Brush),
    Patch(Patch),
    Skipped,
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    path: &'a str,
    warnings: Vec<String>,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str, path: &'a str) -> Self {
        Parser {
            lexer: Lexer::new(text),
            path,
            warnings: Vec::new(),
        }
    }

    fn line(&mut self) -> usize {
        self.lexer.line()
    }

    fn err<T>(&mut self, message: impl Into<String>) -> Result<T, MapError> {
        Err(MapError::Syntax {
            path: self.path.to_string(),
            line: self.line(),
            message: message.into(),
        })
    }

    fn peek(&mut self) -> Option<Tok<'a>> {
        self.lexer.peek()
    }

    fn next(&mut self) -> Option<Tok<'a>> {
        self.lexer.next()
    }

    fn eat_punct(&mut self, c: char) -> bool {
        if self.peek() == Some(Tok::Punct(c)) {
            self.next();
            true
        } else {
            false
        }
    }

    fn expect_punct(&mut self, c: char) -> Result<(), MapError> {
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

    fn number(&mut self) -> Result<f64, MapError> {
        match self.next() {
            Some(Tok::Bare(s)) => match s.parse::<f64>() {
                Ok(v) => Ok(v),
                Err(_) => self.err(format!("`{s}` is not a number")),
            },
            Some(t) => self.err(format!("expected a number, found {t}")),
            None => self.err("expected a number, found end of file"),
        }
    }

    fn peek_is_number(&mut self) -> bool {
        matches!(self.peek(), Some(Tok::Bare(s)) if s.parse::<f64>().is_ok())
    }

    fn run(mut self) -> Result<MapFile, MapError> {
        let mut map = MapFile::default();
        let mut source_index = 0usize;

        while self.peek().is_some() {
            if !self.eat_punct('{') {
                let found = self.peek().map(|t| t.to_string()).unwrap_or_default();
                return self.err(format!(
                    "expected an entity to start with `{{`, found {found}"
                ));
            }
            let entity = self.parse_entity(source_index)?;
            source_index += 1;
            self.classify_entity(&mut map, entity);
        }

        map.warnings = self.warnings;
        Ok(map)
    }

    /// Sorts a finished entity into either the group table or the entity list.
    fn classify_entity(&mut self, map: &mut MapFile, mut entity: Entity) {
        let is_container =
            entity.classname() == "func_group" && entity.properties.contains_key("_tb_type");
        if !is_container {
            map.entities.push(entity);
            return;
        }

        let tb_type = entity.get("_tb_type").unwrap_or_default().to_string();
        let id = entity.get_i64("_tb_id").unwrap_or(-1);
        let kind = if tb_type == "_tb_layer" {
            GroupKind::Layer
        } else {
            GroupKind::Group
        };
        let raw_name = entity.get("_tb_name").unwrap_or("Unnamed");

        let mut name = match kind {
            GroupKind::Layer => format!("layer_{id}"),
            GroupKind::Group => format!("group_{id}"),
        };
        if raw_name != "Unnamed" {
            name.push('_');
            name.push_str(&sanitize_node_name(raw_name));
        }

        let parent_id = entity
            .get_i64("_tb_group")
            .or_else(|| entity.get_i64("_tb_layer"));

        let omit = entity
            .get("_tb_layer_omit_from_export")
            .map(|v| v != "0")
            .unwrap_or(false);

        // Structural brushes inside a layer or group belong to worldspawn,
        // unless the layer is marked as not for export.
        if !omit {
            if let Some(world) = map.entities.first_mut() {
                world.brushes.append(&mut entity.brushes);
                world.patches.append(&mut entity.patches);
            }
        }

        map.groups.push(Group {
            id,
            parent_id,
            name,
            kind,
            omit_from_export: omit,
        });
    }

    fn parse_entity(&mut self, source_index: usize) -> Result<Entity, MapError> {
        let mut entity = Entity {
            source_index,
            ..Default::default()
        };
        loop {
            match self.peek() {
                None => return self.err("unterminated entity"),
                Some(Tok::Punct('}')) => {
                    self.next();
                    return Ok(entity);
                }
                Some(Tok::Quoted(_)) => {
                    let key = match self.next() {
                        Some(Tok::Quoted(k)) => k,
                        _ => unreachable!(),
                    };
                    let value = match self.next() {
                        Some(Tok::Quoted(v)) => v,
                        Some(t) => {
                            return self.err(format!("expected a value for `{key}`, found {t}"))
                        }
                        None => return self.err(format!("expected a value for `{key}`")),
                    };
                    entity.properties.insert(unescape(key), unescape(value));
                }
                Some(Tok::Punct('{')) => {
                    self.next();
                    match self.parse_block()? {
                        Block::Brush(brush) => entity.brushes.push(brush),
                        Block::Patch(patch) => entity.patches.push(patch),
                        Block::Skipped => {}
                    }
                }
                Some(t) => {
                    let t = t.to_string();
                    return self.err(format!("unexpected {t} inside an entity"));
                }
            }
        }
    }

    /// Reads a `patchDef2` or `patchDef3` block, the `{` already eaten.
    ///
    /// `patchDef3` states how finely it wants to be divided; `patchDef2` leaves
    /// that to whoever draws it.
    fn parse_patch(&mut self, kind: &str) -> Result<Patch, MapError> {
        self.expect_punct('{')?;
        let texture = match self.next() {
            Some(Tok::Bare(t)) => t.to_string(),
            Some(Tok::Quoted(t)) => t.to_string(),
            Some(t) => return self.err(format!("expected a patch texture, found {t}")),
            None => return self.err("expected a patch texture"),
        };

        self.expect_punct('(')?;
        let mut header = Vec::new();
        while self.peek_is_number() {
            header.push(self.number()?);
        }
        self.expect_punct(')')?;
        if header.len() < 2 {
            return self.err("a patch needs a width and a height");
        }
        let width = header[0].max(0.0) as usize;
        let height = header[1].max(0.0) as usize;
        if width < 3 || height < 3 || width.is_multiple_of(2) || height.is_multiple_of(2) {
            return self.err(format!(
                "a patch grid has to be odd and at least 3 by 3, not {width} by {height}"
            ));
        }
        let subdivisions = if kind == "patchDef3" && header.len() >= 4 {
            Some((header[2].max(1.0) as usize, header[3].max(1.0) as usize))
        } else {
            None
        };

        let mut patch = Patch {
            texture,
            width,
            height,
            subdivisions,
            ..Default::default()
        };
        self.expect_punct('(')?;
        for _ in 0..width {
            self.expect_punct('(')?;
            for _ in 0..height {
                self.expect_punct('(')?;
                let x = self.number()?;
                let y = self.number()?;
                let z = self.number()?;
                let u = self.number()?;
                let v = self.number()?;
                self.expect_punct(')')?;
                patch.points.push(Vec3::new(x, y, z));
                patch.uvs.push(Vec2::new(u, v));
            }
            self.expect_punct(')')?;
        }
        self.expect_punct(')')?;
        self.expect_punct('}')?;
        self.expect_punct('}')?;
        Ok(patch)
    }

    /// Parses one block inside an entity: a brush, a patch, or something this
    /// reader does not know.
    fn parse_block(&mut self) -> Result<Block, MapError> {
        if let Some(Tok::Bare(word)) = self.peek() {
            if word.starts_with("patchDef") {
                let kind = word.to_string();
                self.next();
                return Ok(Block::Patch(self.parse_patch(&kind)?));
            }
            if word.starts_with("brushDef") {
                let line = self.line();
                self.warnings.push(format!(
                    "{}:{line}: skipped a `{word}` block, which this build does not support yet",
                    self.path
                ));
                self.skip_block()?;
                return Ok(Block::Skipped);
            }
        }

        let mut brush = Brush::default();
        loop {
            match self.peek() {
                None => return self.err("unterminated brush"),
                Some(Tok::Punct('}')) => {
                    self.next();
                    return Ok(Block::Brush(brush));
                }
                Some(Tok::Punct('(')) => {
                    if let Some(face) = self.parse_face()? {
                        brush.faces.push(face);
                    }
                }
                Some(t) => {
                    let t = t.to_string();
                    return self.err(format!("unexpected {t} inside a brush"));
                }
            }
        }
    }

    /// Consumes the rest of the current block, including nested braces.
    fn skip_block(&mut self) -> Result<(), MapError> {
        let mut depth = 1usize;
        while let Some(tok) = self.next() {
            match tok {
                Tok::Punct('{') => depth += 1,
                Tok::Punct('}') => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
        self.err("unterminated block")
    }

    fn point(&mut self) -> Result<Vec3, MapError> {
        self.expect_punct('(')?;
        let x = self.number()?;
        let y = self.number()?;
        let z = self.number()?;
        self.expect_punct(')')?;
        Ok(Vec3::new(x, y, z))
    }

    fn parse_face(&mut self) -> Result<Option<Face>, MapError> {
        let p0 = self.point()?;
        let p1 = self.point()?;
        let p2 = self.point()?;

        let texture = match self.next() {
            Some(Tok::Bare(s)) => s,
            Some(Tok::Quoted(s)) => s,
            Some(t) => return self.err(format!("expected a texture name, found {t}")),
            None => return self.err("expected a texture name"),
        };

        let mut face = Face {
            plane: Plane {
                normal: Vec3::Z,
                d: 0.0,
            },
            points: [p0, p1, p2],
            texture: texture.to_string(),
            format: UvFormat::Standard,
            uv_axes: [Vec3::ZERO; 2],
            offset: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            surface: None,
        };

        if self.peek() == Some(Tok::Punct('[')) {
            face.format = UvFormat::Valve220;
            for axis in 0..2 {
                self.expect_punct('[')?;
                let x = self.number()?;
                let y = self.number()?;
                let z = self.number()?;
                let offset = self.number()?;
                self.expect_punct(']')?;
                face.uv_axes[axis] = Vec3::new(x, y, z);
                face.offset[axis] = offset;
            }
        } else {
            face.offset = Vec2::new(self.number()?, self.number()?);
        }

        face.rotation = self.number()?;
        face.scale = Vec2::new(self.number()?, self.number()?);

        // Quake II and III append content flags, surface flags and a value.
        if self.peek_is_number() {
            let content_flags = self.number()? as i64;
            let surface_flags = self.number()? as i64;
            let value = self.number()? as i64;
            face.surface = Some(SurfaceAttribs {
                content_flags,
                surface_flags,
                value,
            });
        }

        // A degenerate plane means the three points are collinear. Warn and drop
        // the face rather than poisoning the whole brush.
        match Plane::from_points(p0, p1, p2) {
            Some(plane) => {
                face.plane = plane;
                Ok(Some(face))
            }
            None => {
                let line = self.line();
                self.warnings.push(format!(
                    "{}:{line}: dropped a face with collinear plane points",
                    self.path
                ));
                Ok(None)
            }
        }
    }
}

/// Turns a map layer or group name into something usable as a Godot node name.
pub fn sanitize_node_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

#[cfg(test)]
mod tests {

    /// CRLF and LF have to read the same, or a team is one commit away from a
    /// level that builds differently depending on who saved it last.
    #[test]
    fn carriage_returns_change_nothing() {
        let text = "\
// Game: heron_brush
{
\"classname\" \"worldspawn\"
\"_tb_textures\" \"textures\"
{
( -16 -16 -16 ) ( -16 -15 -16 ) ( -16 -16 -15 ) floor [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -16 -16 -16 ) ( -16 -16 -15 ) ( -15 -16 -16 ) floor [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -16 -16 -16 ) ( -15 -16 -16 ) ( -16 -15 -16 ) floor [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 16 16 16 ) ( 16 17 16 ) ( 17 16 16 ) floor [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 16 16 16 ) ( 17 16 16 ) ( 16 16 17 ) floor [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 16 16 16 ) ( 16 16 17 ) ( 16 17 16 ) floor [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
}
}
";
        let unix = parse(text, "unix.map").unwrap();
        let windows = parse(&text.replace('\n', "\r\n"), "windows.map").unwrap();

        assert_eq!(unix.entities.len(), 1);
        assert_eq!(unix.entities.len(), windows.entities.len());
        // Values, not just counts: a stray carriage return would ride along on
        // the end of the last quoted string on the line.
        for (a, b) in unix.entities.iter().zip(&windows.entities) {
            assert_eq!(a.properties, b.properties);
            assert_eq!(a.brushes.len(), b.brushes.len());
            for (x, y) in a.brushes.iter().zip(&b.brushes) {
                assert_eq!(x.faces.len(), y.faces.len());
                for (f, g) in x.faces.iter().zip(&y.faces) {
                    assert_eq!(f.texture, g.texture);
                    assert_eq!(f.points, g.points);
                }
            }
        }
        assert_eq!(windows.entities[0].classname(), "worldspawn");
    }
    use super::*;

    const VALVE: &str = r#"
// Game: heron_brush
// Format: Valve
{
"mapversion" "220"
"classname" "worldspawn"
{
( -64 -64 -16 ) ( -64 -63 -16 ) ( -64 -64 -15 ) stone [ 0 -1 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -64 -64 -16 ) ( -64 -64 -15 ) ( -63 -64 -16 ) stone [ 1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( -64 -64 -16 ) ( -63 -64 -16 ) ( -64 -63 -16 ) stone [ -1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 64 64 16 ) ( 64 65 16 ) ( 65 64 16 ) stone [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
( 64 64 16 ) ( 65 64 16 ) ( 64 64 17 ) stone [ -1 0 0 0 ] [ 0 0 -1 0 ] 0 1 1
( 64 64 16 ) ( 64 64 17 ) ( 64 65 16 ) stone [ 0 1 0 0 ] [ 0 0 -1 0 ] 0 1 1
}
}
{
"classname" "light_omni"
"origin" "0 0 64"
"light_energy" "2.5"
}
"#;

    #[test]
    fn reads_escaped_json_and_preserves_unknown_path_escapes() {
        let map = parse(
            r#"{
"classname" "func_animate"
"animation_curves" "{\"version\":1,\"label\":\"věž\"}"
"path" "C:\maps\tower"
"slash" "a\\b"
}"#,
            "escaped.map",
        )
        .unwrap();
        assert_eq!(
            map.entities[0].get("animation_curves"),
            Some(r#"{"version":1,"label":"věž"}"#)
        );
        assert_eq!(map.entities[0].get("path"), Some(r"C:\maps\tower"));
        assert_eq!(map.entities[0].get("slash"), Some(r"a\b"));
        assert!(parse(r#"{"classname" "unterminated\""#, "bad.map").is_err());
    }

    #[test]
    fn reads_valve_220_brushes_and_point_entities() {
        let map = parse(VALVE, "test.map").unwrap();
        assert_eq!(map.entities.len(), 2);
        let world = &map.entities[0];
        assert_eq!(world.classname(), "worldspawn");
        assert_eq!(world.brushes.len(), 1);
        assert_eq!(world.brushes[0].faces.len(), 6);
        assert_eq!(world.brushes[0].faces[0].format, UvFormat::Valve220);
        assert_eq!(world.brushes[0].faces[0].texture, "stone");

        let light = &map.entities[1];
        assert_eq!(light.classname(), "light_omni");
        assert_eq!(light.get_vec3("origin"), Some(Vec3::new(0.0, 0.0, 64.0)));
        assert_eq!(light.get_f64("light_energy"), Some(2.5));
    }

    #[test]
    fn reads_standard_uv_faces() {
        let text = r#"
{
"classname" "worldspawn"
{
( 0 0 0 ) ( 1 0 0 ) ( 0 1 0 ) wall 8 16 45 2 -2
}
}
"#;
        let map = parse(text, "t.map").unwrap();
        let face = &map.entities[0].brushes[0].faces[0];
        assert_eq!(face.format, UvFormat::Standard);
        assert_eq!(face.offset, Vec2::new(8.0, 16.0));
        assert_eq!(face.rotation, 45.0);
        assert_eq!(face.scale, Vec2::new(2.0, -2.0));
    }

    #[test]
    fn reads_quake2_surface_attributes() {
        let text = r#"
{
"classname" "worldspawn"
{
( 0 0 0 ) ( 1 0 0 ) ( 0 1 0 ) wall 0 0 0 1 1 1 2 3
}
}
"#;
        let map = parse(text, "t.map").unwrap();
        let attribs = map.entities[0].brushes[0].faces[0].surface.unwrap();
        assert_eq!(
            (attribs.content_flags, attribs.surface_flags, attribs.value),
            (1, 2, 3)
        );
    }

    #[test]
    fn folds_layers_into_worldspawn_and_records_the_hierarchy() {
        let text = r#"
{
"classname" "worldspawn"
}
{
"classname" "func_group"
"_tb_type" "_tb_layer"
"_tb_name" "Detail Pass"
"_tb_id" "3"
{
( 0 0 0 ) ( 1 0 0 ) ( 0 1 0 ) wall 0 0 0 1 1
( 0 0 0 ) ( 0 1 0 ) ( 0 0 1 ) wall 0 0 0 1 1
( 0 0 0 ) ( 0 0 1 ) ( 1 0 0 ) wall 0 0 0 1 1
( 8 8 8 ) ( 8 9 8 ) ( 9 8 8 ) wall 0 0 0 1 1
( 8 8 8 ) ( 9 8 8 ) ( 8 8 9 ) wall 0 0 0 1 1
( 8 8 8 ) ( 8 8 9 ) ( 8 9 8 ) wall 0 0 0 1 1
}
}
"#;
        let map = parse(text, "t.map").unwrap();
        assert_eq!(map.entities.len(), 1);
        assert_eq!(map.entities[0].brushes.len(), 1);
        assert_eq!(map.groups.len(), 1);
        assert_eq!(map.groups[0].name, "layer_3_Detail_Pass");
        assert_eq!(map.groups[0].kind, GroupKind::Layer);
    }

    #[test]
    fn quoted_texture_names_with_spaces() {
        let text = r#"
{
"classname" "worldspawn"
{
( 0 0 0 ) ( 1 0 0 ) ( 0 1 0 ) "my tex/with space" [ 1 0 0 0 ] [ 0 -1 0 0 ] 0 1 1
}
}
"#;
        let map = parse(text, "t.map").unwrap();
        assert_eq!(
            map.entities[0].brushes[0].faces[0].texture,
            "my tex/with space"
        );
    }
}
