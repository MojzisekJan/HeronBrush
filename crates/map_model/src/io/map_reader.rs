//! Reading `.map` files.
//!
//! Reads brace-delimited entities containing key/value properties and optional brush blocks, with
//! one plane-defined face per line.
//!
//! The reader produces a flat list of entities exactly as the file holds them. Turning
//! that into a node tree — resolving the `_tb_layer` and `_tb_group` links the file
//! carries — is a separate step, so that a file that fails to make a valid tree can still
//! be inspected.

use std::fmt;

use geometry_math::vec::{Vec2f, Vec3d};

use super::tokenizer::{Tokenizer, SpannedToken, Token, TokenizeError};
use crate::brush_face::{BrushFace, FaceColor, MaterialName, Rgb};
use crate::map_format::MapFormat;
use crate::uv::{SurfaceAttributes, UvAttributes, UvCoordSystem};

/// A `.map` file as read from disk.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MapFile {
    /// The format the faces were read in.
    pub format: MapFormat,
    /// The game named in the `// Game:` header comment, if there was one.
    pub game: Option<String>,
    /// The entities, in file order. The first is the worldspawn.
    pub entities: Vec<Entity>,
    /// Things the reader recovered from rather than failing on.
    pub warnings: Vec<Warning>,
    /// The version of the editor's own notes the file says it carries, from its
    /// `// heron_brush format N` header. `None` for a file that says nothing, which is one from
    /// another editor or from before the header existed.
    pub notes_version: Option<u32>,
}

/// An entity: a set of properties and, for a brush entity, its brushes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Entity {
    /// The entity's properties, in file order.
    pub properties: Vec<(String, String)>,
    /// What this entity is, across saves and across editors. See [`Brush::id`].
    ///
    /// Written the same way, `// heron_brush id <hex>` on the line before the entity opens. A
    /// merge otherwise matches an ordinary entity by its classname and its position
    /// among the others of that classname, so a light added above another makes every
    /// light below it look like a different light.
    pub id: Option<u64>,
    /// The entity's brushes.
    pub brushes: Vec<Brush>,
    /// The Bézier patches on this entity, from `patchDef2` and `patchDef3` blocks.
    pub patches: Vec<Patch>,
    /// The line the entity starts on.
    pub line: usize,
}

impl Entity {
    /// Returns the value of the given property, or `None` if the entity does not have it.
    pub fn property(&self, key: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Returns the entity's `classname`, or `None` if it has none.
    #[inline]
    pub fn classname(&self) -> Option<&str> {
        self.property("classname")
    }
}

/// A brush: the convex solid behind a set of face planes.
#[derive(Debug, Clone, Default)]
pub struct Brush {
    /// The brush's faces.
    pub faces: Vec<BrushFace>,
    /// What this brush is, across saves and across editors.
    ///
    /// Written as `// heron_brush id <hex>`, so another tool reads a comment and ignores it.
    /// Without one a three-way merge cannot tell an edited brush from one removed and
    /// one added, and has to guess from how much of the geometry still lines up; with
    /// one it knows, and two people editing different brushes never trouble each other.
    /// `None` for a brush from a file that carries none, which is every file written by
    /// another editor -- one is minted when it reaches the node tree.
    pub id: Option<u64>,
    /// The line the brush starts on.
    pub line: usize,
}

/// Two brushes are the same brush when they have the same faces, whatever line of
/// whatever file each was read from. See [`BrushFace`]'s own equality for why the line
/// is left out.
///
/// The id is left out too, and deliberately: it answers "is this the same brush?" where
/// a merge is asking "has this brush changed?", and folding the two together would make
/// a brush that moved between files unequal to itself.
impl PartialEq for Brush {
    fn eq(&self, other: &Self) -> bool {
        self.faces == other.faces
    }
}

/// A Bézier patch, from a `patchDef2` or `patchDef3` block.
///
/// The control points are stored row by row. Each carries a position and its UV
/// coordinates, which is how the format writes them.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Patch {
    /// The material on the patch.
    pub material_name: String,
    /// The number of control point rows.
    pub rows: usize,
    /// The number of control point columns.
    pub columns: usize,
    /// The control points, row by row.
    pub control_points: Vec<PatchControlPoint>,
    /// What this patch is, across saves and across editors. See [`Brush::id`].
    pub id: Option<u64>,
    /// The line the patch starts on.
    pub line: usize,
}

/// Two patches are the same patch when they have the same material and the same control
/// grid. As with a brush, where in the file each was read from is not part of what it is.
impl PartialEq for Patch {
    fn eq(&self, other: &Self) -> bool {
        self.material_name == other.material_name
            && self.rows == other.rows
            && self.columns == other.columns
            && self.control_points == other.control_points
    }
}

/// One control point of a [`Patch`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PatchControlPoint {
    /// The control point's position.
    pub position: Vec3d,
    /// The control point's UV coordinates.
    pub uv: Vec2f,
}

/// Something the reader worked around rather than giving up on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// What was skipped or assumed.
    pub message: String,
    /// The line it happened on.
    pub line: usize,
}

/// Why a `.map` file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// The file could not be split into tokens.
    Tokenize(TokenizeError),
    /// The tokens did not form a valid map file.
    Parse {
        /// What was expected and what was found.
        message: String,
        /// The line it happened on.
        line: usize,
    },
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Tokenize(error) => write!(f, "{error}"),
            ReadError::Parse { message, line } => write!(f, "line {line}: {message}"),
        }
    }
}

impl std::error::Error for ReadError {}

impl From<TokenizeError> for ReadError {
    fn from(error: TokenizeError) -> Self {
        ReadError::Tokenize(error)
    }
}

/// Namespace for editor metadata comments in `.map` files.
pub(crate) const METADATA_MARKER: &str = "heron_brush";

/// Reads a node id from the body of a comment, or `None` if it is not one.
///
/// `heron_brush id <hex>`, written before the brush or patch it names. Hexadecimal because the
/// value is 64 bits with a random prefix and reads shorter that way, and because nothing
/// ever does arithmetic on it.
fn parse_node_id(comment: &str) -> Option<u64> {
    let mut words = comment.split_whitespace();
    if words.next()? != METADATA_MARKER || words.next()? != "id" {
        return None;
    }
    u64::from_str_radix(words.next()?, 16).ok()
}

/// Reads a face colour from the body of a comment, or `None` if it is not one.
///
/// `heron_brush color R G B`, `heron_brush gradient R G B R G B ANGLE` and
/// `heron_brush linear|radial ANGLE N (POSITION R G B)×N [ends SX SY EX EY]`, each with an optional trailing
/// alpha from 0 to 255 (absent means opaque), which is what `map_writer::write_face_color`
/// writes.
fn parse_face_color(comment: &str) -> Option<FaceColor> {
    // Surface metadata shares the comment but is not part of the colour payload.
    let mut words = comment.split_whitespace().take_while(|word| !matches!(*word, "pbr" | "export_hidden" | "primitive_uv"));
    if words.next()? != METADATA_MARKER {
        return None;
    }
    let kind = words.next()?;
    if kind == "linear" || kind == "radial" {
        use crate::brush_face::{ColorStop, GradientStops};
        let angle: f32 = words.next()?.parse().ok()?;
        if !angle.is_finite() { return None; }
        let count: usize = words.next()?.parse().ok()?;
        if !(2..=8).contains(&count) { return None; }
        let mut stops = Vec::new();
        for _ in 0..count {
            stops.push(ColorStop { position: words.next()?.parse().ok()?, color: Rgb::new(
                words.next()?.parse().ok()?, words.next()?.parse().ok()?, words.next()?.parse().ok()?) });
        }
        let mut next = words.next();
        // Where the gradient runs, when it was placed on the face rather than fitted to it.
        let mut ends = None;
        if next == Some("ends") {
            let mut point = || -> Option<[f32; 2]> {
                let p: [f32; 2] = [words.next()?.parse().ok()?, words.next()?.parse().ok()?];
                p.iter().all(|v| v.is_finite()).then_some(p)
            };
            ends = Some(crate::brush_face::GradientEnds { start: point()?, end: point()? });
            next = words.next();
        }
        let alpha = match next { Some(word) => word.parse().ok()?, None => 255 };
        if words.next().is_some() { return None; }
        return Some(FaceColor::MultiGradient { stops: GradientStops::new(&stops)?, radial: kind == "radial", angle, alpha, ends });
    }

    let mut number = || words.next().and_then(|word| word.parse::<f32>().ok());
    let mut rgb = || {
        Some(Rgb::new(
            number()? as u8,
            number()? as u8,
            number()? as u8,
        ))
    };

    let base = match kind {
        "color" => FaceColor::solid(rgb()?),
        "gradient" => FaceColor::gradient(rgb()?, rgb()?, number()?),
        _ => return None,
    };
    // One more number on the line is the alpha; none means the face is opaque.
    match number() {
        Some(alpha) => Some(base.with_alpha(alpha.clamp(0.0, 255.0) as u8)),
        None => Some(base),
    }
}

fn parse_primitive_uv(comment: &str) -> Option<Box<[[f64; 3]; 2]>> {
    let mut words = comment.split_whitespace();
    if words.next()? != METADATA_MARKER { return None; }
    let mut values = words.skip_while(|word| *word != "primitive_uv");
    values.next()?;
    let mut matrix = [[0.; 3]; 2];
    for value in matrix.iter_mut().flatten() {
        let parsed: f64 = values.next()?.parse().ok()?;
        if !parsed.is_finite() { return None; }
        *value = parsed;
    }
    Some(Box::new(matrix))
}

/// Reads a `.map` file.
///
/// The format is taken from the `// Format:` header comment when there is one, and
/// otherwise guessed from the worldspawn's `mapversion` property and the shape of the
/// first face line. Pass `format` to override both. Doom 3 `Version 2` selects
/// literal strings and restores entity-local brushDef2/brushDef3 planes to world space.
pub fn read_map(source: &str, format: Option<MapFormat>) -> Result<MapFile, ReadError> {
    // idTech4 uses literal strings, including backslashes before closing quotes.
    // Select its lexer before tokenization so such strings are not mis-tokenized.
    let literal_strings = source.lines().map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("//"))
        .is_some_and(|line| line.split_whitespace().next() == Some("Version"));
    let mut reader = Reader::new(source, format, literal_strings);
    let result = reader.run();
    // The eager lexer reported lexical errors before syntax errors, even later in
    // the file. Drain only on failure to preserve that diagnostic precedence.
    let mut cursor = reader.tokens.into_inner();
    if result.is_err() { cursor.finish(); }
    match cursor.error { Some(error) => Err(error.into()), None => result }
}

struct TokenCursor<'a> {
    lexer: Tokenizer<'a>,
    pending: std::collections::VecDeque<SpannedToken<'a>>,
    last_line: usize,
    error: Option<TokenizeError>,
    ended: bool,
}
impl<'a> TokenCursor<'a> {
    fn peek(&mut self, offset: usize) -> Option<SpannedToken<'a>> {
        while self.pending.len() <= offset && !self.ended {
            match self.lexer.next_token() {
                Ok(Some(token)) => { self.last_line = token.line; self.pending.push_back(token); }
                Ok(None) => self.ended = true,
                Err(error) => { self.error = Some(error); self.ended = true; }
            }
        }
        self.pending.get(offset).copied()
    }
    fn finish(&mut self) {
        self.pending.clear();
        while !self.ended {
            self.peek(0);
            self.pending.clear();
        }
    }

}

struct Reader<'a> {
    tokens: std::cell::RefCell<TokenCursor<'a>>,
    format: Option<MapFormat>,
    game: Option<String>,
    notes_version: Option<u32>,
    warnings: Vec<Warning>,
    /// Every distinct material name seen so far, so faces that share one share the `Arc`.
    materials: rustc_hash::FxHashMap<String, MaterialName>,
    /// Whether the deferred brush-primitive UV warning has already been given, so a
    /// map full of `brushDef` brushes warns once rather than once a brush.
    warned_about_primitive_uv: bool,
    literal_strings: bool,
    local_plane_lines: rustc_hash::FxHashSet<usize>,
}

impl<'a> Reader<'a> {
    fn new(source: &'a str, format: Option<MapFormat>, literal_strings: bool) -> Self {
        Self {
            tokens: std::cell::RefCell::new(TokenCursor {
                lexer: Tokenizer::new(source).with_literal_strings(literal_strings),
                pending: std::collections::VecDeque::with_capacity(3),
                last_line: 1, error: None, ended: false,
            }),
            format,
            game: None,
            notes_version: None,
            warnings: Vec::new(),
            materials: rustc_hash::FxHashMap::default(),
            warned_about_primitive_uv: false,
            literal_strings,
            local_plane_lines: Default::default(),
        }
    }

    fn string(&self, raw: &'a str) -> std::borrow::Cow<'a, str> {
        if self.literal_strings { std::borrow::Cow::Borrowed(raw) } else { super::tokenizer::unescape(raw) }
    }

    fn peek(&self) -> Option<SpannedToken<'a>> {
        self.tokens.borrow_mut().peek(0)
    }

    fn line(&self) -> usize {
        self.peek().map(|t| t.line).unwrap_or_else(|| self.tokens.borrow().last_line)
    }

    fn error<T>(&self, message: impl Into<String>) -> Result<T, ReadError> {
        Err(ReadError::Parse {
            message: message.into(),
            line: self.line(),
        })
    }

    fn warn(&mut self, message: impl Into<String>, line: usize) {
        self.warnings.push(Warning {
            message: message.into(),
            line,
        });
    }

    fn advance(&mut self) -> Option<SpannedToken<'a>> {
        self.peek()?;
        self.tokens.borrow_mut().pending.pop_front()
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ReadError> {
        match self.advance() {
            Some(actual) if actual.token == *expected => Ok(()),
            Some(actual) => Err(ReadError::Parse {
                message: format!("expected `{expected}`, found `{}`", actual.token),
                line: actual.line,
            }),
            None => self.error(format!("expected `{expected}`, found end of file")),
        }
    }

    fn expect_number(&mut self) -> Result<f64, ReadError> {
        match self.advance() {
            Some(SpannedToken {
                token: Token::Number(value),
                ..
            }) => Ok(value),
            Some(actual) => Err(ReadError::Parse {
                message: format!("expected a number, found `{}`", actual.token),
                line: actual.line,
            }),
            None => self.error("expected a number, found end of file"),
        }
    }

    /// Reads a material name, interned so that the thousand faces that all name `stone`
    /// share one allocation.
    ///
    /// A material name may look like a number — `128` is a perfectly good name — so
    /// whatever token turns up is taken as text.
    fn expect_material_name(&mut self) -> Result<MaterialName, ReadError> {
        let raw: std::borrow::Cow<str> = match self.advance() {
            Some(SpannedToken {
                token: Token::Word(name),
                ..
            }) => std::borrow::Cow::Borrowed(name),
            Some(SpannedToken {
                token: Token::String(name),
                ..
            }) => self.string(name),
            Some(SpannedToken {
                token: Token::Number(value),
                ..
            }) => std::borrow::Cow::Owned(format_number(value)),
            Some(actual) => {
                return Err(ReadError::Parse {
                    message: format!("expected a material name, found `{}`", actual.token),
                    line: actual.line,
                })
            }
            None => return self.error("expected a material name, found end of file"),
        };
        Ok(match self.materials.get(raw.as_ref()) {
            Some(interned) => interned.clone(),
            None => {
                let interned = MaterialName::from(raw.as_ref());
                self.materials.insert(raw.into_owned(), interned.clone());
                interned
            }
        })
    }

    fn run(&mut self) -> Result<MapFile, ReadError> {
        self.read_header_comments();
        if self.literal_strings {
            let line = self.line();
            self.advance(); // Version, detected before tokenization.
            if self.line() != line { return self.error("Doom 3 Version requires a number on the same line"); }
            let version = self.expect_number()?;
            if version != 2.0 { return self.error(format!("unsupported Doom 3 map version {version}")); }
            self.read_header_comments();
        }

        let mut entities = Vec::new();
        // An `// heron_brush id` comment names the entity that opens after it.
        let mut pending_id: Option<u64> = None;
        while let Some(next) = self.peek() {
            match next.token {
                Token::OpenBrace => {
                    let mut entity = self.read_entity()?;
                    entity.id = pending_id.take();
                    self.restore_local_planes(&mut entity)?;
                    entities.push(entity);
                }
                Token::Comment(text) => {
                    self.advance();
                    if let Some(id) = parse_node_id(text) {
                        pending_id = Some(id);
                    }
                }
                other => {
                    let message = format!("expected an entity, found `{other}`");
                    let line = next.line;
                    return Err(ReadError::Parse { message, line });
                }
            }
        }

        // Fall back to what the worldspawn says, then to Standard.
        let mut format = self
            .format
            .or_else(|| format_from_worldspawn(&entities))
            .unwrap_or_default();

        // A face carrying the surface-flags trio means a Quake 2 dialect, even when no
        // header said so -- otherwise a save would write the trio under a format that
        // does not have it, and the face attribute editor would stay hidden.
        if !format.has_surface_attributes() && any_face_has_surface_attributes(&entities) {
            format = if format.has_explicit_uv_axes() {
                MapFormat::Quake2Valve
            } else {
                MapFormat::Quake2
            };
        }

        Ok(MapFile {
            notes_version: self.notes_version,
            format,
            game: self.game.take(),
            entities,
            warnings: std::mem::take(&mut self.warnings),
        })
    }

    /// Reads the `// Game:` and `// Format:` comments a `.map` file may carry at the top.
    fn read_header_comments(&mut self) {
        while matches!(self.peek().map(|t| t.token), Some(Token::Comment(_))) {
            let Some(SpannedToken { token: Token::Comment(text), line, .. }) = self.advance()
            else {
                unreachable!("the peek said it was a comment")
            };
            let text = text.trim();
            if let Some(game) = text.strip_prefix("Game:") {
                self.game = Some(game.trim().to_string());
            } else if let Some(version) = text.strip_prefix(METADATA_MARKER).and_then(|rest| rest.strip_prefix(" format ")) {
                match version.trim().parse::<u32>() {
                    Ok(version) => {
                        self.notes_version = Some(version);
                        if version > crate::map_format::NOTES_VERSION {
                            self.warn(
                                format!(
                                    "this map was written by a newer Heron Brush \
                                     (notes version {version}, this one reads {}); \
                                     anything it added is not understood here and \
                                     saving will drop it",
                                    crate::map_format::NOTES_VERSION
                                ),
                                line,
                            );
                        }
                    }
                    Err(_) => self.warn(format!("`{version}` is not a version"), line),
                }
            } else if let Some(name) = text.strip_prefix("Format:") {
                match name.trim().parse::<MapFormat>() {
                    // An explicit argument wins over the header.
                    Ok(parsed) => self.format = self.format.or(Some(parsed)),
                    Err(error) => self.warn(error.to_string(), line),
                }
            }
        }
    }

    fn restore_local_planes(&mut self, entity: &mut Entity) -> Result<(), ReadError> {
        if self.local_plane_lines.is_empty() { return Ok(()); }
        let has_local = entity.brushes.iter().flat_map(|b| &b.faces).any(|f| self.local_plane_lines.contains(&f.line));
        if !has_local { return Ok(()); }
        if entity.properties.iter().rev().find(|(key,_)| key.eq_ignore_ascii_case("classname"))
            .is_some_and(|(_,value)| value.eq_ignore_ascii_case("worldspawn")) { return Ok(()); }
        let origin = entity.properties.iter().rev().find(|(key,_)| key.eq_ignore_ascii_case("origin"))
            .map(|(_,value)| value.as_str()).unwrap_or("0 0 0");
        let mut parts = origin.split_whitespace();
        let mut xyz = [0.;3];
        for value in &mut xyz {
            *value = parts.next().and_then(|v| v.parse::<f64>().ok()).filter(|v| v.is_finite())
                .ok_or_else(|| ReadError::Parse { message: "invalid origin for Doom 3 brush entity".into(), line: entity.line })?;
        }
        if parts.next().is_some() { return self.error("invalid origin for Doom 3 brush entity"); }
        let origin = Vec3d::new(xyz[0],xyz[1],xyz[2]);
        let rotated = entity.properties.iter().any(|(key,value)| {
            if key.eq_ignore_ascii_case("rotation") {
                let values: Result<Vec<f64>,_> = value.split_whitespace().map(str::parse).collect();
                return match values { Ok(values) => values.len() != 9 || values.iter().zip([1.,0.,0.,0.,1.,0.,0.,0.,1.]).any(|(a,b)| !a.is_finite() || (a-b).abs() > 1e-9), Err(_) => true };
            }
            (key.eq_ignore_ascii_case("angle") || key.eq_ignore_ascii_case("angles")) && value.split_whitespace().any(|v| v.parse::<f64>().ok() != Some(0.))
        });
        if rotated { self.warn("Doom 3 brush-entity rotation is preserved as properties but is not applied to the preview", entity.line); }
        for face in entity.brushes.iter_mut().flat_map(|b| &mut b.faces) {
            if !self.local_plane_lines.contains(&face.line) { continue; }
            let plane = geometry_math::plane::Plane3d { normal: face.plane.normal, distance: face.plane.distance + geometry_math::vec::dot(face.plane.normal, origin) };
            face.points = face.points.map(|p| p + origin);
            if !plane.distance.is_finite() || face.points.iter().any(|p| (0..3).any(|i| !p[i].is_finite())) {
                return Err(ReadError::Parse { message: "Doom 3 entity origin exceeds finite geometry bounds".into(), line: face.line });
            }
            face.plane = plane;
            // texMat stays unchanged: Doom 3's GetTextureVectors adds the
            // origin contribution when evaluating a local brush vertex.
        }
        Ok(())
    }

    fn read_entity(&mut self) -> Result<Entity, ReadError> {
        let line = self.line();
        self.expect(&Token::OpenBrace)?;

        let mut entity = Entity {
            line,
            ..Entity::default()
        };
        // An `// heron_brush id` comment names the brush or patch that opens after it, and an
        // `// heron_brush color` there is the colour of every face of it.
        let mut pending_id: Option<u64> = None;
        let mut pending_color: Option<crate::brush_face::FaceColor> = None;

        loop {
            match self.peek().map(|t| t.token) {
                None => return self.error("unterminated entity"),
                Some(Token::CloseBrace) => {
                    self.advance();
                    return Ok(entity);
                }
                Some(Token::String(_)) => {
                    let Some(SpannedToken { token: Token::String(key), .. }) = self.advance()
                    else {
                        unreachable!("the peek said it was a string")
                    };
                    let key = self.string(key).into_owned();
                    if self.literal_strings && (key.eq_ignore_ascii_case("origin") || key.eq_ignore_ascii_case("classname"))
                        && entity.brushes.iter().flat_map(|b| &b.faces).any(|f| self.local_plane_lines.contains(&f.line))
                    { return self.error("Doom 3 origin and classname must precede local brush blocks"); }
                    match self.advance() {
                        Some(SpannedToken {
                            token: Token::String(value),
                            ..
                        }) => entity
                            .properties
                            .push((key, self.string(value).into_owned())),
                        Some(actual) => {
                            return Err(ReadError::Parse {
                                message: format!(
                                    "expected a value for property \"{key}\", found `{}`",
                                    actual.token
                                ),
                                line: actual.line,
                            })
                        }
                        None => return self.error("expected a property value"),
                    }
                }
                Some(Token::OpenBrace) => match self.read_brush_or_patch()? {
                    BrushOrPatch::Brush(mut brush) => {
                        brush.id = pending_id.take();
                        // A face that said nothing takes the brush's colour; one that
                        // spoke for itself keeps what it said.
                        if let Some(color) = pending_color.take() {
                            for face in &mut brush.faces {
                                face.color.get_or_insert_with(|| Box::new(color));
                            }
                        }
                        entity.brushes.push(brush);
                    }
                    BrushOrPatch::Patch(mut patch) => {
                        patch.id = pending_id.take();
                        pending_color = None;
                        entity.patches.push(patch);
                    }
                },
                Some(Token::Comment(_)) => {
                    let Some(SpannedToken { token: Token::Comment(text), .. }) = self.advance()
                    else {
                        unreachable!("the peek said it was a comment");
                    };
                    // An id belongs to the block that opens next. Any other comment
                    // between them leaves it alone, so `// heron_brush id ...` followed by the
                    // lineage's own `// brush 0` still reaches the brush.
                    if let Some(id) = parse_node_id(text) {
                        pending_id = Some(id);
                    } else if let Some(color) = parse_face_color(text) {
                        pending_color = Some(color);
                    }
                }
                Some(_) => {
                    let other = self.advance().expect("the peek found a token").token;
                    return self.error(format!(
                        "expected a property or a brush, found `{other}`"
                    ));
                }
            }
        }
    }

    fn read_brush_or_patch(&mut self) -> Result<BrushOrPatch, ReadError> {
        let line = self.line();
        self.expect(&Token::OpenBrace)?;

        // A brush body starts with `(`; anything else is a keyword naming what kind of
        // block this is.
        if matches!(self.peek().map(|t| t.token), Some(Token::Word(_))) {
            let Some(SpannedToken { token: Token::Word(keyword), .. }) = self.advance() else {
                unreachable!("the peek said it was a word")
            };
            return match keyword {
                "patchDef2" => Ok(BrushOrPatch::Patch(self.read_patch(line, false)?)),
                "patchDef3" => Ok(BrushOrPatch::Patch(self.read_patch(line, true)?)),
                // Quake 3's `brushDef` and Doom 3's `brushDef3` give each face a 2x3
                // texture matrix rather than the offset/scale/rotation the other formats
                // use. The matrix is retained until material dimensions are known.
                // `brushDef3` also gives the plane
                // outright rather than three points on it.
                "brushDef" => Ok(BrushOrPatch::Brush(self.read_primitive_brush(line, false)?)),
                "brushDef2" | "brushDef3" => Ok(BrushOrPatch::Brush(self.read_primitive_brush(line, true)?)),
                other => self.error(format!("unknown block type `{other}`")),
            };
        }

        Ok(BrushOrPatch::Brush(self.read_brush(line)?))
    }

    fn read_brush(&mut self, line: usize) -> Result<BrushOrPatch2, ReadError> {
        let mut faces = Vec::new();
        loop {
            match self.peek().map(|t| t.token) {
                None => return self.error("unterminated brush"),
                Some(Token::CloseBrace) => {
                    self.advance();
                    return Ok(Brush { faces, id: None, line });
                }
                Some(Token::Comment(_)) => {
                    self.advance();
                }
                Some(_) => faces.push(self.read_face()?),
            }
        }
    }

    /// Reads a `brushDef` or `brushDef3` block: `{ <faces> }` followed by the `}` that
    /// closes the whole `{ brushDef { ... } }`, the way [`Self::read_patch`] does.
    fn read_primitive_brush(
        &mut self,
        line: usize,
        doom3: bool,
    ) -> Result<BrushOrPatch2, ReadError> {
        self.expect(&Token::OpenBrace)?;
        let mut faces = Vec::new();
        loop {
            match self.peek().map(|t| t.token) {
                None => return self.error("unterminated brushDef"),
                Some(Token::CloseBrace) => {
                    self.advance();
                    break;
                }
                Some(Token::Comment(_)) => {
                    self.advance();
                }
                Some(_) => faces.push(self.read_primitive_face(doom3)?),
            }
        }
        self.expect(&Token::CloseBrace)?;

        if !faces.is_empty() && !self.warned_about_primitive_uv {
            self.warned_about_primitive_uv = true;
            self.warn(
                "brush-primitive UV matrices (brushDef / brushDef3) are preserved; \
                 texturing uses placeholder alignment until material dimensions are available"
                    .to_string(),
                line,
            );
        }
        Ok(Brush { faces, id: None, line })
    }

    /// Reads one `brushDef` / `brushDef3` face: the plane (three points, or a
    /// `( nx ny nz d )` plane for `brushDef3`), the `( ( a b c ) ( d e f ) )` texture
    /// matrix, the material, and the optional surface-attribute trio. The matrix is kept
    /// on the face as `primitive_uv` -- converting it needs the material's pixel size --
    /// and the face wears the default paraxial alignment until it is resolved.
    fn read_primitive_face(&mut self, doom3: bool) -> Result<BrushFace, ReadError> {
        let line = self.line();

        if doom3 { self.local_plane_lines.insert(line); }
        let points = if doom3 {
            self.expect(&Token::OpenParen)?;
            let normal = Vec3d::new(
                self.expect_number()?,
                self.expect_number()?,
                self.expect_number()?,
            );
            let distance = self.expect_number()?;
            self.expect(&Token::CloseParen)?;
            points_on_plane(normal, distance).ok_or(ReadError::Parse {
                message: "the brushDef3 face plane has no direction".to_string(),
                line,
            })?
        } else {
            [self.read_point()?, self.read_point()?, self.read_point()?]
        };

        // ( ( a b c ) ( d e f ) ): the two rows of the texture matrix.
        self.expect(&Token::OpenParen)?;
        let mut matrix = [[0.0; 3]; 2];
        for row in &mut matrix {
            self.expect(&Token::OpenParen)?;
            row[0] = self.expect_number()?;
            row[1] = self.expect_number()?;
            row[2] = self.expect_number()?;
            self.expect(&Token::CloseParen)?;
        }
        self.expect(&Token::CloseParen)?;

        if matrix.iter().flatten().any(|value| !value.is_finite()) {
            return self.error("non-finite brush-primitive UV matrix");
        }
        let material_name = self.expect_material_name()?;
        let surface_attributes = self.read_optional_surface_attributes()?;

        let plane = geometry_math::plane::from_points(points[0], points[1], points[2]).ok_or(
            ReadError::Parse {
                message: "the three points of the face do not define a plane".to_string(),
                line,
            },
        )?;

        let (color, surface_material, export_hidden, _) = self.read_face_notes(line);
        Ok(BrushFace {
            points,
            plane,
            material_name,
            uv_coord_system: UvCoordSystem::Paraxial,
            uv_attributes: UvAttributes::default(),
            surface_attributes,
            color: color.map(Box::new),
            surface_material,
            export_hidden,
            primitive_uv: Some(Box::new(matrix)),
            line,
        })
    }

    fn read_face(&mut self) -> Result<BrushFace, ReadError> {
        let line = self.line();

        let points = [self.read_point()?, self.read_point()?, self.read_point()?];
        let material_name = self.expect_material_name()?;

        // The shape of what follows tells the two dialects apart: Valve 220 opens with a
        // bracketed U axis, Standard goes straight to the offsets.
        let is_valve = matches!(self.peek().map(|t| t.token), Some(Token::OpenBracket));

        let (uv_coord_system, uv_attributes) = if is_valve {
            self.read_valve_uv()?
        } else {
            self.read_standard_uv()?
        };

        // The format is settled by the first face, unless the caller or header said.
        if self.format.is_none() {
            self.format = Some(if is_valve {
                MapFormat::Valve
            } else {
                MapFormat::Standard
            });
        }

        let surface_attributes = self.read_optional_surface_attributes()?;

        let plane = geometry_math::plane::from_points(points[0], points[1], points[2]).ok_or(
            ReadError::Parse {
                message: "the three points of the face do not define a plane".to_string(),
                line,
            },
        )?;

        let (color, surface_material, export_hidden, primitive_uv) = self.read_face_notes(line);

        Ok(BrushFace {
            points,
            plane,
            material_name,
            uv_coord_system,
            uv_attributes,
            surface_attributes,
            color: color.map(Box::new),
            surface_material,
            export_hidden,
            primitive_uv,
            line,
        })
    }

    fn read_face_notes(&mut self, line: usize) -> (Option<FaceColor>, Option<crate::brush_face::SurfaceMaterial>, bool, Option<Box<[[f64; 3]; 2]>>) {
        // A colour is written as a comment on the face's own line, so that a file with
        // colours in it is still a plain `.map`. Only a comment on that line and carrying
        // the editor's own marker is one; anything else is somebody's note.
        let export_hidden=self.peek().filter(|token|token.line==line).is_some_and(|token| {
            let Token::Comment(text)=token.token else{return false;};
            let mut words=text.split_whitespace();
            words.next()==Some(METADATA_MARKER) && words.any(|word|word=="export_hidden")
        });
        let surface_material = self.peek().filter(|token| token.line == line).and_then(|token| {
            let Token::Comment(text) = token.token else { return None; };
            let mut tokens = text.split_whitespace();
            if tokens.next()? != METADATA_MARKER { return None; }
            let mut words = tokens.skip_while(|word| *word != "pbr");
            words.next()?;
            let roughness = words.next()?.parse::<u8>().ok()?.min(100);
            let metallic = words.next()?.parse::<u8>().ok()?.min(100);
            let emission = words.next().and_then(|v| v.parse::<u16>().ok()).unwrap_or(0).min(800);
            Some(crate::brush_face::SurfaceMaterial { roughness, metallic, emission })
        });
        let primitive_uv = self.peek().filter(|token| token.line == line).and_then(|token| {
            let Token::Comment(text) = token.token else { return None; };
            parse_primitive_uv(text)
        });
        if primitive_uv.is_some() && !self.warned_about_primitive_uv {
            self.warned_about_primitive_uv = true;
            self.warn("brush-primitive UV matrices are preserved; texturing uses placeholder alignment until material dimensions are available", line);
        }
        let color = self.read_face_color(line);
        if (surface_material.is_some() || export_hidden || primitive_uv.is_some()) && color.is_none() { self.advance(); }

        (color, surface_material, export_hidden, primitive_uv)
    }

    /// Reads the colour comment that may follow a face on its own line.
    ///
    /// The counterpart of `map_writer::write_face_color`. A comment that does not parse is
    /// left where it is rather than being an error: a `.map` file is somebody's text file,
    /// and a note that happens to start with the marker should not stop a map opening.
    fn read_face_color(&mut self, face_line: usize) -> Option<FaceColor> {
        let token = self.peek()?;
        if token.line != face_line {
            return None;
        }
        let Token::Comment(text) = token.token else {
            return None;
        };
        let color = parse_face_color(text)?;
        self.advance();
        Some(color)
    }

    fn read_point(&mut self) -> Result<Vec3d, ReadError> {
        self.expect(&Token::OpenParen)?;
        let x = self.expect_number()?;
        let y = self.expect_number()?;
        let z = self.expect_number()?;
        self.expect(&Token::CloseParen)?;
        Ok(Vec3d::new(x, y, z))
    }

    /// Reads `xoff yoff rot xscale yscale`.
    fn read_standard_uv(&mut self) -> Result<(UvCoordSystem, UvAttributes), ReadError> {
        let x_offset = self.expect_number()? as f32;
        let y_offset = self.expect_number()? as f32;
        let rotation = self.expect_number()? as f32;
        let x_scale = self.expect_number()? as f32;
        let y_scale = self.expect_number()? as f32;

        Ok((
            UvCoordSystem::Paraxial,
            UvAttributes {
                offset: Vec2f::new(x_offset, y_offset),
                scale: Vec2f::new(x_scale, y_scale),
                rotation,
            },
        ))
    }

    /// Reads `[ ux uy uz uoff ] [ vx vy vz voff ] rot xscale yscale`.
    fn read_valve_uv(&mut self) -> Result<(UvCoordSystem, UvAttributes), ReadError> {
        let (u_axis, u_offset) = self.read_uv_axis()?;
        let (v_axis, v_offset) = self.read_uv_axis()?;
        let rotation = self.expect_number()? as f32;
        let x_scale = self.expect_number()? as f32;
        let y_scale = self.expect_number()? as f32;

        Ok((
            UvCoordSystem::Parallel { u_axis, v_axis },
            UvAttributes {
                offset: Vec2f::new(u_offset, v_offset),
                scale: Vec2f::new(x_scale, y_scale),
                rotation,
            },
        ))
    }

    /// Reads one `[ x y z offset ]` axis.
    fn read_uv_axis(&mut self) -> Result<(Vec3d, f32), ReadError> {
        self.expect(&Token::OpenBracket)?;
        let x = self.expect_number()?;
        let y = self.expect_number()?;
        let z = self.expect_number()?;
        let offset = self.expect_number()? as f32;
        self.expect(&Token::CloseBracket)?;
        Ok((Vec3d::new(x, y, z), offset))
    }

    /// Reads the `content_flags surface_flags value` trio, if it is there.
    ///
    /// It is optional even within a format that allows it, so this looks ahead rather
    /// than committing: three numbers in a row can only be the trio, since the next thing
    /// otherwise is a `(` starting the next face or a `}` ending the brush.
    fn read_optional_surface_attributes(
        &mut self,
    ) -> Result<Option<SurfaceAttributes>, ReadError> {
        let all_numbers = (0..3).all(|offset| {
            matches!(
                self.tokens.borrow_mut().peek(offset).map(|t| t.token),
                Some(Token::Number(_))
            )
        });
        if !all_numbers {
            return Ok(None);
        }

        Ok(Some(SurfaceAttributes {
            content_flags: self.expect_number()? as i32,
            surface_flags: self.expect_number()? as i32,
            value: self.expect_number()? as i32,
        }))
    }

    /// Reads a `patchDef2` or `patchDef3` body.
    ///
    /// The dimensions line is `( width height contents flags value )`, with `patchDef3`
    /// adding two subdivision counts after the height. The editor uses neither the flags
    /// nor the subdivision counts, but they have to be consumed to reach the grid.
    fn read_patch(&mut self, line: usize, has_subdivisions: bool) -> Result<Patch, ReadError> {
        self.expect(&Token::OpenBrace)?;
        let material_name = self.expect_material_name()?.to_string();

        self.expect(&Token::OpenParen)?;
        let columns = self.expect_number()? as usize;
        let rows = self.expect_number()? as usize;
        if has_subdivisions {
            // The horizontal and vertical subdivision counts.
            self.expect_number()?;
            self.expect_number()?;
        }
        // Contents, flags and value.
        self.expect_number()?;
        self.expect_number()?;
        self.expect_number()?;
        self.expect(&Token::CloseParen)?;

        // The grid is written column-major: an outer group per column, holding one point
        // per row.
        let mut by_column: Vec<Vec<PatchControlPoint>> = Vec::with_capacity(columns);
        self.expect(&Token::OpenParen)?;
        for _ in 0..columns {
            self.expect(&Token::OpenParen)?;
            let mut column = Vec::with_capacity(rows);
            for _ in 0..rows {
                self.expect(&Token::OpenParen)?;
                let x = self.expect_number()?;
                let y = self.expect_number()?;
                let z = self.expect_number()?;
                let u = self.expect_number()? as f32;
                let v = self.expect_number()? as f32;
                self.expect(&Token::CloseParen)?;
                column.push(PatchControlPoint {
                    position: Vec3d::new(x, y, z),
                    uv: Vec2f::new(u, v),
                });
            }
            self.expect(&Token::CloseParen)?;
            by_column.push(column);
        }
        self.expect(&Token::CloseParen)?;

        self.expect(&Token::CloseBrace)?;
        self.expect(&Token::CloseBrace)?;

        // Store row by row, which is the order the surface evaluator wants.
        let mut control_points = Vec::with_capacity(rows * columns);
        for row in 0..rows {
            for column in &by_column {
                control_points.push(column[row]);
            }
        }

        Ok(Patch {
            id: None,
            material_name,
            rows,
            columns,
            control_points,
            line,
        })
    }

}

/// What a brace-delimited block inside an entity turned out to be.
enum BrushOrPatch {
    Brush(Brush),
    Patch(Patch),
}

/// The alias the brush reader returns, kept separate so its name reads well at the call
/// site.
type BrushOrPatch2 = Brush;

/// Three points on the plane a `brushDef3` face names as `( nx ny nz d )`.
///
/// idTech4 writes the plane as `n . p + d = 0`; normalization must scale `d` too. The three
/// are wound so that [`geometry_math::plane::from_points`] gives `n` back, which is the outward
/// normal -- the same thing a three-point face's winding carries. `None` when the normal
/// is too small to be a direction.
fn points_on_plane(normal: Vec3d, distance: f64) -> Option<[Vec3d; 3]> {
    let length = geometry_math::vec::length(normal);
    if !length.is_finite() || length < 1e-6 || !distance.is_finite() {
        return None;
    }
    let n = normal / length;
    let anchor = n * (-distance / length);

    // Any vector off the normal will do; the axis the normal leans on least is safest.
    let (ax, ay, az) = (n.x().abs(), n.y().abs(), n.z().abs());
    let helper = if ax <= ay && ax <= az {
        Vec3d::new(1.0, 0.0, 0.0)
    } else if ay <= az {
        Vec3d::new(0.0, 1.0, 0.0)
    } else {
        Vec3d::new(0.0, 0.0, 1.0)
    };
    let u = geometry_math::vec::normalize(geometry_math::vec::cross(n, helper));
    let v = geometry_math::vec::cross(n, u); // n, u, v right-handed
    let span = 64.0;
    // `geometry_math::plane::from_points` takes the normal as `cross(p3 - p1, p2 - p1)`, so the
    // second and third points are `v` then `u` for that cross product to come out `n`.
    Some([anchor, anchor + v * span, anchor + u * span])
}

/// Whether any brush face carried the `content surface value` trio.
fn any_face_has_surface_attributes(entities: &[Entity]) -> bool {
    entities.iter().any(|entity| {
        entity
            .brushes
            .iter()
            .any(|brush| brush.faces.iter().any(|face| face.surface_attributes.is_some()))
    })
}

/// Reads the format from the worldspawn's `mapversion`, which is 220 for Valve 220.
fn format_from_worldspawn(entities: &[Entity]) -> Option<MapFormat> {
    let version = entities.first()?.property("mapversion")?;
    (version.trim() == "220").then_some(MapFormat::Valve)
}

/// Formats a number the way a `.map` file writes it: no trailing zeros, no exponent.
pub(crate) fn format_number(value: f64) -> String {
    if value == value.trunc() && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        let formatted = format!("{value}");
        formatted
    }
}

#[cfg(test)]
mod streaming_tests {
    use super::*;

    #[test]
    fn lookahead_keeps_at_most_three_tokens() {
        let source = "( 1 2 3 ) // comment\n".repeat(100);
        let reader = Reader::new(&source, None, false);
        let mut cursor = reader.tokens.into_inner();
        let mut count = 0;
        while cursor.peek(0).is_some() {
            cursor.peek(2);
            assert!(cursor.pending.len() <= 3);
            cursor.pending.pop_front();
            count += 1;
        }
        assert_eq!(count, 600);
        assert!(cursor.error.is_none());
    }

    #[test]
    fn later_lexical_errors_still_take_precedence_over_syntax_errors() {
        for source in ["unexpected\n\"unterminated", "unexpected\n/* unterminated"] {
            let expected = super::super::tokenizer::tokenize(source).unwrap_err();
            assert_eq!(read_map(source, None), Err(ReadError::Tokenize(expected)));
        }
        assert!(matches!(read_map("unexpected", None), Err(ReadError::Parse { line: 1, .. })));
    }

    #[test]
    fn streaming_matches_eager_tokens_and_spans_in_both_string_modes() {
        let source = "// header\n{ \"name\" \"žluťoučký\" ( 1 -2 .5 ) [ 0 1 0 ] /* ignored */ stone }";
        for literal in [false, true] {
            let mut lexer = Tokenizer::new(source).with_literal_strings(literal);
            let mut expected = Vec::new();
            while let Some(token) = lexer.next_token().unwrap() { expected.push(token); }
            let reader = Reader::new(source, None, literal);
            let mut cursor = reader.tokens.into_inner();
            for token in expected {
                cursor.peek(2);
                assert_eq!(cursor.peek(0), Some(token));
                cursor.pending.pop_front();
            }
            assert_eq!(cursor.peek(0), None);
            assert!(cursor.error.is_none());
        }
    }
}
