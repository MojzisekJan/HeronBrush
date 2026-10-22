//! Legacy `model(...)` syntax without expressions.
//!
//! A legacy FGD or `.def` `model()` attribute
//! uses an ad-hoc syntax: a comma-separated list of cases, tried back to front, each
//! either
//!
//! - a *static* case, `"path" [skin [frame]] [key = value]` -- a fixed model, optionally
//!   guarded by an entity property equalling a string or an integer, or
//! - a *dynamic* case, `pathKey = "k1" [skinKey = "k2"] [frameKey = "k3"]` -- the model
//!   path, skin and frame each read from a named entity property.
//!
//! This parser turns that into the same [`expression_lang`] expression tree the modern `{{ … }}`
//! syntax produces, so everything downstream only ever sees one representation. The
//! reader tries the modern syntax first and falls back to this.

use std::collections::BTreeMap;

use expression_lang::expression::ExpressionKind;
use expression_lang::{BinaryOperation, ExpressionNode, FileLocation, Value};

use crate::model_definition::keys;

/// Characters that end a word or an integer, from `LegacyModelDefinitionTokenizer`.
const WORD_DELIMS: &str = " \t\n\r()[]{};,=";

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Integer(i64),
    Str(String),
    Word(String),
    Comma,
    Equals,
    Eof,
}

impl Tok {
    fn describe(&self) -> String {
        match self {
            Tok::Integer(n) => format!("integer `{n}`"),
            Tok::Str(s) => format!("quoted string `{s}`"),
            Tok::Word(w) => format!("word `{w}`"),
            Tok::Comma => "`,`".to_string(),
            Tok::Equals => "`=`".to_string(),
            Tok::Eof => "end of input".to_string(),
        }
    }
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str, line: usize) -> Self {
        Self {
            src,
            pos: 0,
            line,
            col: 1,
        }
    }

    fn cur(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn location(&self) -> FileLocation {
        FileLocation::new(self.line, self.col)
    }

    fn advance(&mut self) {
        let Some(c) = self.cur() else { return };
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        self.pos += c.len_utf8();
    }

    /// Reads the next token or reports an invalid starting character.
    fn emit(&mut self) -> Result<(Tok, FileLocation), String> {
        while let Some(c) = self.cur() {
            let location = self.location();
            match c {
                ' ' | '\t' | '\n' | '\r' => {
                    self.advance();
                }
                '=' => {
                    self.advance();
                    return Ok((Tok::Equals, location));
                }
                ',' => {
                    self.advance();
                    return Ok((Tok::Comma, location));
                }
                '"' => {
                    self.advance();
                    let start = self.pos;
                    while let Some(c) = self.cur() {
                        if c == '"' {
                            break;
                        }
                        self.advance();
                    }
                    if self.cur().is_none() {
                        return Err(prefix(location, "unterminated quoted string"));
                    }
                    let text = self.src[start..self.pos].to_string();
                    self.advance(); // the closing quote
                    return Ok((Tok::Str(text), location));
                }
                _ => {
                    let start = self.pos;
                    while let Some(c) = self.cur() {
                        if WORD_DELIMS.contains(c) {
                            break;
                        }
                        self.advance();
                    }
                    if self.pos == start {
                        return Err(prefix(location, &format!("unexpected character `{c}`")));
                    }
                    let text = &self.src[start..self.pos];
                    return Ok(match text.parse::<i64>() {
                        Ok(n) => (Tok::Integer(n), location),
                        Err(_) => (Tok::Word(text.to_string()), location),
                    });
                }
            }
        }
        Ok((Tok::Eof, self.location()))
    }
}

fn prefix(location: FileLocation, message: &str) -> String {
    format!("line {}: {message}", location.line)
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    peeked: Option<(Tok, FileLocation)>,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str, line: usize) -> Self {
        Self {
            lexer: Lexer::new(src, line),
            peeked: None,
        }
    }

    fn peek(&mut self) -> Result<(Tok, FileLocation), String> {
        if self.peeked.is_none() {
            self.peeked = Some(self.lexer.emit()?);
        }
        Ok(self.peeked.clone().unwrap())
    }

    fn next(&mut self) -> Result<(Tok, FileLocation), String> {
        match self.peeked.take() {
            Some(token) => Ok(token),
            None => self.lexer.emit(),
        }
    }

    fn expect(&mut self, want: &Tok) -> Result<FileLocation, String> {
        let (token, location) = self.next()?;
        if &token == want {
            Ok(location)
        } else {
            Err(prefix(
                location,
                &format!("expected {}, but got {}", want.describe(), token.describe()),
            ))
        }
    }

    /// `parseModelDefinition`. The cases are reversed so the switch tries them the way the
    /// legacy runtime did: last one first.
    fn parse(&mut self) -> Result<ExpressionNode, String> {
        let (token, start) = self.peek()?;
        if token == Tok::Eof {
            return Ok(ExpressionNode::literal(Value::undefined()));
        }

        let mut cases = Vec::new();
        loop {
            match self.peek()?.0 {
                Tok::Str(_) => cases.push(self.parse_static()?),
                Tok::Word(_) => cases.push(self.parse_dynamic()?),
                other => {
                    return Err(prefix(
                        self.peek()?.1,
                        &format!(
                            "expected a quoted path or a `pathKey`, but got {}",
                            other.describe()
                        ),
                    ));
                }
            }
            if self.peek()?.0 == Tok::Comma {
                self.next()?;
            } else {
                break;
            }
        }

        let (trailing, location) = self.peek()?;
        if trailing != Tok::Eof {
            return Err(prefix(
                location,
                &format!(
                    "unexpected {} after the model definition",
                    trailing.describe()
                ),
            ));
        }

        cases.reverse();
        Ok(ExpressionNode::new(
            ExpressionKind::Switch(cases),
            Some(start),
        ))
    }

    /// `parseStaticModelDefinition`: `"path" [skin [frame]] [key = value]`.
    fn parse_static(&mut self) -> Result<ExpressionNode, String> {
        let (token, start) = self.next()?;
        let Tok::Str(path) = token else {
            return Err(prefix(start, "expected a quoted path"));
        };

        let mut map = BTreeMap::new();
        map.insert(keys::PATH.to_string(), Value::string(path));

        let mut indices = Vec::new();
        if let Tok::Integer(skin) = self.peek()?.0 {
            self.next()?;
            indices.push(skin);
            if let Tok::Integer(frame) = self.peek()?.0 {
                self.next()?;
                indices.push(frame);
            }
        }
        if let Some(&skin) = indices.first() {
            map.insert(keys::SKIN.to_string(), Value::integer(skin));
            if let Some(&frame) = indices.get(1) {
                map.insert(keys::FRAME.to_string(), Value::integer(frame));
            }
        }

        let model = ExpressionNode::new(ExpressionKind::Literal(Value::map(map)), Some(start));

        let Tok::Word(key) = self.peek()?.0 else {
            return Ok(model);
        };
        let key_location = self.next()?.1;
        self.expect(&Tok::Equals)?;

        let (value_token, value_location) = self.next()?;
        let value = match value_token {
            Tok::Str(text) => Value::string(text),
            Tok::Integer(number) => Value::integer(number),
            other => {
                return Err(prefix(
                    value_location,
                    &format!(
                        "expected a quoted string or an integer, but got {}",
                        other.describe()
                    ),
                ));
            }
        };

        let premise = ExpressionNode::new(
            ExpressionKind::Binary {
                operation: BinaryOperation::Equal,
                left: ExpressionNode::new(ExpressionKind::Variable(key), Some(key_location)),
                right: ExpressionNode::new(ExpressionKind::Literal(value), Some(value_location)),
            },
            Some(key_location),
        );

        Ok(ExpressionNode::new(
            ExpressionKind::Binary {
                operation: BinaryOperation::Case,
                left: premise,
                right: model,
            },
            Some(start),
        ))
    }

    /// `parseDynamicModelDefinition`: `pathKey = "k" [skinKey = "k"] [frameKey = "k"]`.
    fn parse_dynamic(&mut self) -> Result<ExpressionNode, String> {
        let location = self.peek()?.1;

        let mut map = BTreeMap::new();
        map.insert(keys::PATH.to_string(), self.parse_named_value("pathKey")?);

        while let Tok::Word(word) = self.peek()?.0 {
            if word.eq_ignore_ascii_case("skinKey") {
                map.insert(keys::SKIN.to_string(), self.parse_named_value("skinKey")?);
            } else if word.eq_ignore_ascii_case("frameKey") {
                map.insert(keys::FRAME.to_string(), self.parse_named_value("frameKey")?);
            } else {
                return Err(prefix(
                    self.peek()?.1,
                    &format!("expected `skinKey` or `frameKey`, but found `{word}`"),
                ));
            }
        }

        Ok(ExpressionNode::new(
            ExpressionKind::Map(map),
            Some(location),
        ))
    }

    /// `parseNamedValue`: `name = "property"`, yielding a reference to that property.
    fn parse_named_value(&mut self, name: &str) -> Result<ExpressionNode, String> {
        let (token, location) = self.next()?;
        match &token {
            Tok::Word(word) if word.eq_ignore_ascii_case(name) => {}
            other => {
                return Err(prefix(
                    location,
                    &format!("expected `{name}`, but got {}", other.describe()),
                ));
            }
        }
        self.expect(&Tok::Equals)?;

        let (value, value_location) = self.next()?;
        let Tok::Str(property) = value else {
            return Err(prefix(
                value_location,
                &format!(
                    "expected a quoted property name, but got {}",
                    value.describe()
                ),
            ));
        };
        Ok(ExpressionNode::new(
            ExpressionKind::Variable(property),
            Some(location),
        ))
    }
}

/// Parses legacy model syntax, using `line` as its diagnostic source location.
pub fn parse_legacy_model_definition(text: &str, line: usize) -> Result<ExpressionNode, String> {
    Parser::new(text, line).parse()
}
