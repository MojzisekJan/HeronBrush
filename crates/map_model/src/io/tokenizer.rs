//! Tokenizer for `.map` files.
//!
//! A hand-written tokenizer preserves source line numbers for `.map` diagnostics.

use std::fmt;

/// A token from a `.map` file.
///
/// The text-bearing variants borrow a slice of the source rather than own a `String`:
/// for a 450 KB map that is tens of thousands of allocations saved, and the parser only
/// copies the handful of strings it actually keeps (material names, property values).
/// The token is [`Copy`], so peeking and advancing move nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token<'a> {
    /// `{`
    OpenBrace,
    /// `}`
    CloseBrace,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `[`
    OpenBracket,
    /// `]`
    CloseBracket,
    /// A double-quoted string, as written between the quotes -- escapes not yet resolved.
    /// Classic maps use [`unescape`] when storing it; idTech4 strings stay literal.
    String(&'a str),
    /// A number.
    Number(f64),
    /// A bare word: a material name, or a keyword such as `patchDef2`.
    Word(&'a str),
    /// A `//` comment, without the leading slashes.
    ///
    /// Comments are kept rather than skipped because the header lines that name the game
    /// and format are comments.
    Comment(&'a str),
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::OpenBrace => f.write_str("{"),
            Token::CloseBrace => f.write_str("}"),
            Token::OpenParen => f.write_str("("),
            Token::CloseParen => f.write_str(")"),
            Token::OpenBracket => f.write_str("["),
            Token::CloseBracket => f.write_str("]"),
            Token::String(s) => write!(f, "\"{s}\""),
            Token::Number(n) => write!(f, "{n}"),
            Token::Word(w) => f.write_str(w),
            Token::Comment(c) => write!(f, "//{c}"),
        }
    }
}

/// Resolves `\"` and `\\` in a quoted string's raw text. Anything else after a backslash
/// is kept as it is, so a Windows path in a property value survives. Borrows when there
/// is nothing to resolve.
pub fn unescape(raw: &str) -> std::borrow::Cow<'_, str> {
    if !raw.contains('\\') {
        return std::borrow::Cow::Borrowed(raw);
    }
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.clone().next() {
                Some(next @ ('"' | '\\')) => {
                    out.push(next);
                    chars.next();
                }
                _ => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    std::borrow::Cow::Owned(out)
}

/// A token together with where it came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpannedToken<'a> {
    /// The token.
    pub token: Token<'a>,
    /// The 1-based line the token starts on.
    pub line: usize,
    /// The 1-based column the token starts at.
    pub column: usize,
}

/// A problem the tokenizer could not get past.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenizeError {
    /// What went wrong.
    pub message: String,
    /// The 1-based line it happened on.
    pub line: usize,
    /// The 1-based column it happened at.
    pub column: usize,
}

impl fmt::Display for TokenizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for TokenizeError {}

/// Splits the given `.map` source into tokens.
pub fn tokenize(source: &str) -> Result<Vec<SpannedToken<'_>>, TokenizeError> {
    Tokenizer::new(source).run()
}

pub(super) struct Tokenizer<'a> {
    /// The source as bytes. The `.map` grammar's structure is all ASCII -- braces,
    /// numbers, whitespace -- so it is scanned a byte at a time; the only text that can
    /// be non-ASCII is inside a quoted string or a bare word, and those are sliced back
    /// out of `source` at the ASCII delimiters that bound them, which are char
    /// boundaries.
    bytes: &'a [u8],
    source: &'a str,
    position: usize,
    line: usize,
    column: usize,
    literal_strings: bool,
}

impl<'a> Tokenizer<'a> {
    pub(super) fn new(source: &'a str) -> Self {
        Self {
            bytes: source.as_bytes(),
            source,
            position: 0,
            line: 1,
            column: 1,
            literal_strings: false,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.position + offset).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.position += 1;
        // `\n` is 0x0A, which is never a UTF-8 continuation or lead byte, so counting it
        // per byte is exact.
        if c == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    fn error(&self, message: impl Into<String>) -> TokenizeError {
        TokenizeError {
            message: message.into(),
            line: self.line,
            column: self.column,
        }
    }

    pub(super) fn with_literal_strings(mut self, literal: bool) -> Self {
        self.literal_strings = literal;
        self
    }

    fn run(mut self) -> Result<Vec<SpannedToken<'a>>, TokenizeError> {
        let mut tokens = Vec::with_capacity(self.bytes.len() / 8);
        while let Some(token) = self.next_token()? {
            tokens.push(token);
        }
        Ok(tokens)
    }

    pub(super) fn next_token(&mut self) -> Result<Option<SpannedToken<'a>>, TokenizeError> {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.advance();
                continue;
            }

            let (line, column) = (self.line, self.column);
            let token = match c {
                b'{' => {
                    self.advance();
                    Token::OpenBrace
                }
                b'}' => {
                    self.advance();
                    Token::CloseBrace
                }
                b'(' => {
                    self.advance();
                    Token::OpenParen
                }
                b')' => {
                    self.advance();
                    Token::CloseParen
                }
                b'[' => {
                    self.advance();
                    Token::OpenBracket
                }
                b']' => {
                    self.advance();
                    Token::CloseBracket
                }
                b'"' => self.read_string()?,
                b'/' if self.peek_at(1) == Some(b'/') => self.read_comment(),
                b'/' if self.peek_at(1) == Some(b'*') => {
                    self.skip_block_comment()?;
                    continue;
                }
                _ => self.read_word_or_number(),
            };

            return Ok(Some(SpannedToken {
                token,
                line,
                column,
            }));
        }

        Ok(None)
    }

    /// Reads a double-quoted string.
    ///
    /// Only `\"` and `\\` are escapes; anything else after a backslash is kept as it is,
    /// so a Windows path in a property value survives unmangled.
    ///
    /// A `\"` at the end of a line closes the string, and the backslash is part of the
    /// value. Compilers have no escapes, so a value may end in a backslash: Neverball's
    /// messages break lines with `\`, and 59 of its 368 maps end one that way.
    /// TrenchBroom reads it the same way, and also before a `}`, which this does not: a
    /// JSON value such as `{\"version\":1}` ends in `\"}`.
    fn read_string(&mut self) -> Result<Token<'a>, TokenizeError> {
        self.advance(); // the opening quote

        // The span is the raw text between the quotes; `\"` does not end the string but
        // is left in place for `unescape` to deal with when the value is stored. `start`
        // and the end sit against a `"`, an ASCII byte, so the slice is a valid `&str`
        // whatever UTF-8 is between.
        let start = self.position;
        loop {
            match self.peek() {
                None => return Err(self.error("unterminated string")),
                Some(b'"') => {
                    let text = &self.source[start..self.position];
                    self.advance();
                    return Ok(Token::String(text));
                }
                Some(b'\\')
                    if !self.literal_strings
                        && self.peek_at(1) == Some(b'"')
                        && matches!(self.peek_at(2), Some(b'\n' | b'\r')) =>
                {
                    self.advance(); // the backslash, which is the value's last character
                }
                Some(b'\\') if !self.literal_strings => {
                    self.advance();
                    self.advance(); // whatever the backslash escapes
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Reads a `//` comment to the end of the line.
    fn read_comment(&mut self) -> Token<'a> {
        self.advance();
        self.advance();
        let start = self.position;
        while !matches!(self.peek(), Some(b'\n') | None) {
            self.advance();
        }
        Token::Comment(&self.source[start..self.position])
    }

    /// Skips a `/* … */` comment.
    fn skip_block_comment(&mut self) -> Result<(), TokenizeError> {
        self.advance();
        self.advance();
        loop {
            match self.peek() {
                None => return Err(self.error("unterminated block comment")),
                Some(b'*') if self.peek_at(1) == Some(b'/') => {
                    self.advance();
                    self.advance();
                    return Ok(());
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Reads a bare word, returning it as a number if it parses as one.
    ///
    /// Material names can look like numbers, so the distinction is left to the parser
    /// where it knows what it is expecting; producing a `Number` when the text parses as
    /// one is a convenience, not a commitment.
    fn read_word_or_number(&mut self) -> Token<'a> {
        let start = self.position;
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace()
                || matches!(c, b'{' | b'}' | b'(' | b')' | b'[' | b']' | b'"')
            {
                break;
            }
            // A `//` starts a comment even with no space in front of it.
            if c == b'/' && self.peek_at(1) == Some(b'/') {
                break;
            }
            self.advance();
        }

        let text = &self.source[start..self.position];
        match text.parse::<f64>() {
            Ok(number) => Token::Number(number),
            Err(_) => Token::Word(text),
        }
    }
}
