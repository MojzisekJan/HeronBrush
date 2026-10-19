//! An EL-specific tokenizer and recursive-descent parser.

use std::collections::BTreeMap;

use text_utils::string_format::str_unescape;

use crate::expression::{BinaryOperation, ExpressionKind, ExpressionNode, UnaryOperation};
use crate::{ElError, FileLocation};

/// Strict rejects trailing input after the expression; lenient stops at the first token
/// that cannot continue it, leaving [`Parser::tokenizer_state`] pointing at the rest.
///
/// Expression parsing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    /// Reject anything after the expression.
    Strict,
    /// Stop at the first token that cannot continue the expression.
    Lenient,
}

/// The position where tokenization stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenizerState {
    /// The 1-based line.
    pub line: usize,
    /// The 1-based column.
    pub column: usize,
    /// The source from the current position to the end.
    pub cur: String,
}

const NUMBER_DELIM: &str = " \t\n\r(){}[],:+-*/%<>=!&|^~";
const INTEGER_DELIM: &str = " \t\n\r(){}[],:+-*/%<>=!&|^~.";

/// The kind of a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tok {
    Name,
    Str,
    Number,
    Boolean,
    OBracket,
    CBracket,
    OBrace,
    CBrace,
    OParen,
    CParen,
    Addition,
    Subtraction,
    Multiplication,
    Division,
    Modulus,
    Colon,
    Comma,
    Range,
    LogicalNegation,
    LogicalAnd,
    LogicalOr,
    Less,
    LessOrEqual,
    Equal,
    NotEqual,
    GreaterOrEqual,
    Greater,
    Case,
    BitwiseNegation,
    BitwiseAnd,
    BitwiseXOr,
    BitwiseOr,
    BitwiseShiftLeft,
    BitwiseShiftRight,
    DoubleOBrace,
    DoubleCBrace,
    Null,
    Eof,
}

fn is_literal_tok(t: Tok) -> bool {
    matches!(t, Tok::Str | Tok::Number | Tok::Boolean | Tok::Null)
}

fn is_unary_op_tok(t: Tok) -> bool {
    matches!(
        t,
        Tok::Addition | Tok::Subtraction | Tok::LogicalNegation | Tok::BitwiseNegation
    )
}

fn is_simple_term_tok(t: Tok) -> bool {
    t == Tok::Name
        || is_literal_tok(t)
        || matches!(t, Tok::OParen | Tok::OBracket | Tok::OBrace)
        || is_unary_op_tok(t)
}

fn is_compound_term_tok(t: Tok) -> bool {
    matches!(
        t,
        Tok::Addition
            | Tok::Subtraction
            | Tok::Multiplication
            | Tok::Division
            | Tok::Modulus
            | Tok::LogicalAnd
            | Tok::LogicalOr
            | Tok::Less
            | Tok::LessOrEqual
            | Tok::Equal
            | Tok::NotEqual
            | Tok::GreaterOrEqual
            | Tok::Greater
            | Tok::Case
            | Tok::BitwiseAnd
            | Tok::BitwiseXOr
            | Tok::BitwiseOr
            | Tok::BitwiseShiftLeft
            | Tok::BitwiseShiftRight
    )
}

fn binop_for_tok(t: Tok) -> BinaryOperation {
    use BinaryOperation as B;
    match t {
        Tok::Addition => B::Addition,
        Tok::Subtraction => B::Subtraction,
        Tok::Multiplication => B::Multiplication,
        Tok::Division => B::Division,
        Tok::Modulus => B::Modulus,
        Tok::LogicalAnd => B::LogicalAnd,
        Tok::LogicalOr => B::LogicalOr,
        Tok::BitwiseAnd => B::BitwiseAnd,
        Tok::BitwiseXOr => B::BitwiseXOr,
        Tok::BitwiseOr => B::BitwiseOr,
        Tok::BitwiseShiftLeft => B::BitwiseShiftLeft,
        Tok::BitwiseShiftRight => B::BitwiseShiftRight,
        Tok::Less => B::Less,
        Tok::LessOrEqual => B::LessOrEqual,
        Tok::Greater => B::Greater,
        Tok::GreaterOrEqual => B::GreaterOrEqual,
        Tok::Equal => B::Equal,
        Tok::NotEqual => B::NotEqual,
        Tok::Case => B::Case,
        Tok::Range => B::BoundedRange,
        other => unreachable!("not a binary operator token: {other:?}"),
    }
}

#[derive(Debug, Clone)]
struct Token {
    kind: Tok,
    text: String,
    line: usize,
    column: usize,
}

impl Token {
    fn location(&self) -> Option<FileLocation> {
        Some(FileLocation::new(self.line, self.column))
    }
}

/// The EL lexer.
struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str, line: usize, column: usize) -> Self {
        Self { src, pos: 0, line, col: column }
    }

    fn remainder(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn cur_char(&self) -> Option<char> {
        self.remainder().chars().next()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.remainder().chars().nth(n)
    }

    fn eof(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn snapshot(&self) -> (usize, usize, usize) {
        (self.pos, self.line, self.col)
    }

    fn restore(&mut self, s: (usize, usize, usize)) {
        (self.pos, self.line, self.col) = s;
    }

    fn advance(&mut self) {
        let Some(c) = self.cur_char() else { return };
        match c {
            '\n' => {
                self.line += 1;
                self.col = 1;
            }
            '\r' => {
                if self.peek_at(1) == Some('\n') {
                    self.col += 1;
                } else {
                    self.line += 1;
                    self.col = 1;
                }
            }
            _ => self.col += 1,
        }
        self.pos += c.len_utf8();
    }

    fn discard_while_ws(&mut self) {
        while matches!(self.cur_char(), Some(c) if " \t\n\r".contains(c)) {
            self.advance();
        }
    }

    fn discard_until(&mut self, chars: &str) {
        while let Some(c) = self.cur_char() {
            if chars.contains(c) {
                break;
            }
            self.advance();
        }
    }

    fn read_digits(&mut self) {
        while matches!(self.cur_char(), Some(c) if c.is_ascii_digit()) {
            self.advance();
        }
    }

    /// Reads a decimal number.
    fn read_decimal(&mut self, delims: &str) -> Option<String> {
        let c0 = self.cur_char()?;
        if c0 == '+' || c0 == '-' || c0 == '.' || c0.is_ascii_digit() {
            let saved = self.snapshot();
            let start = self.pos;
            if c0 != '.' {
                self.advance();
                self.read_digits();
            }
            if self.cur_char() == Some('.') {
                self.advance();
                self.read_digits();
            }
            if matches!(self.cur_char(), Some('e' | 'E')) {
                self.advance();
                if matches!(self.cur_char(), Some(c) if c == '+' || c == '-' || c.is_ascii_digit())
                {
                    self.advance();
                    self.read_digits();
                }
            }
            if self.eof() || delims.contains(self.cur_char().unwrap()) {
                return Some(self.src[start..self.pos].to_string());
            }
            self.restore(saved);
        }
        None
    }

    /// Reads an integer.
    fn read_integer(&mut self, delims: &str) -> Option<String> {
        let c0 = self.cur_char()?;
        if c0 == '+' || c0 == '-' || c0.is_ascii_digit() {
            let saved = self.snapshot();
            let start = self.pos;
            if c0 == '+' || c0 == '-' {
                self.advance();
            }
            self.read_digits();
            if self.eof() || delims.contains(self.cur_char().unwrap()) {
                return Some(self.src[start..self.pos].to_string());
            }
            self.restore(saved);
        }
        None
    }

    /// Consumes a matching literal at the current position without requiring a following delimiter.
    fn discard(&mut self, s: &str) -> bool {
        if self.remainder().starts_with(s) {
            for _ in 0..s.chars().count() {
                self.advance();
            }
            true
        } else {
            false
        }
    }

    /// Reads a quoted string with `self.pos` immediately after the opening delimiter.
    fn read_quoted_string(&mut self, delim: char) -> Result<String, ElError> {
        let start = self.pos;
        let mut escaped = false;
        loop {
            match self.cur_char() {
                None => return Err(ElError::Parse("Unexpected end of file".into())),
                Some(c) => {
                    // The tokenizer's only escapable character is '"'.
                    let is_escaped_quote = escaped && c == '"';
                    if c == delim && !is_escaped_quote {
                        break;
                    }
                    let next_escaped = if c == '\\' { !escaped } else { false };
                    self.advance();
                    escaped = next_escaped;
                }
            }
        }
        let end = self.pos;
        self.advance(); // past the closing delimiter
        Ok(self.src[start..end].to_string())
    }

    fn tok(&self, kind: Tok, text: impl Into<String>, line: usize, column: usize) -> Token {
        Token { kind, text: text.into(), line, column }
    }

    /// Reads the next token.
    fn emit(&mut self) -> Result<Token, ElError> {
        loop {
            if self.eof() {
                return Ok(self.tok(Tok::Eof, "", self.line, self.col));
            }
            let (line, column) = (self.line, self.col);
            let start = self.pos;
            let c = self.cur_char().unwrap();
            match c {
                '[' => {
                    self.advance();
                    return Ok(self.tok(Tok::OBracket, "[", line, column));
                }
                ']' => {
                    self.advance();
                    return Ok(self.tok(Tok::CBracket, "]", line, column));
                }
                '{' => {
                    self.advance();
                    if self.cur_char() == Some('{') {
                        self.advance();
                        return Ok(self.tok(Tok::DoubleOBrace, "{{", line, column));
                    }
                    return Ok(self.tok(Tok::OBrace, "{", line, column));
                }
                '}' => {
                    self.advance();
                    if self.cur_char() == Some('}') {
                        self.advance();
                        return Ok(self.tok(Tok::DoubleCBrace, "}}", line, column));
                    }
                    return Ok(self.tok(Tok::CBrace, "}", line, column));
                }
                '(' => {
                    self.advance();
                    return Ok(self.tok(Tok::OParen, "(", line, column));
                }
                ')' => {
                    self.advance();
                    return Ok(self.tok(Tok::CParen, ")", line, column));
                }
                '+' => {
                    self.advance();
                    return Ok(self.tok(Tok::Addition, "+", line, column));
                }
                '-' => {
                    self.advance();
                    if self.cur_char() == Some('>') {
                        self.advance();
                        return Ok(self.tok(Tok::Case, "->", line, column));
                    }
                    return Ok(self.tok(Tok::Subtraction, "-", line, column));
                }
                '*' => {
                    self.advance();
                    return Ok(self.tok(Tok::Multiplication, "*", line, column));
                }
                '/' => {
                    self.advance();
                    if self.cur_char() == Some('/') {
                        self.discard_until("\n\r");
                        continue;
                    }
                    return Ok(self.tok(Tok::Division, "/", line, column));
                }
                '%' => {
                    self.advance();
                    return Ok(self.tok(Tok::Modulus, "%", line, column));
                }
                '~' => {
                    self.advance();
                    return Ok(self.tok(Tok::BitwiseNegation, "~", line, column));
                }
                '&' => {
                    self.advance();
                    if self.cur_char() == Some('&') {
                        self.advance();
                        return Ok(self.tok(Tok::LogicalAnd, "&&", line, column));
                    }
                    return Ok(self.tok(Tok::BitwiseAnd, "&", line, column));
                }
                '|' => {
                    self.advance();
                    if self.cur_char() == Some('|') {
                        self.advance();
                        return Ok(self.tok(Tok::LogicalOr, "||", line, column));
                    }
                    return Ok(self.tok(Tok::BitwiseOr, "|", line, column));
                }
                '^' => {
                    self.advance();
                    return Ok(self.tok(Tok::BitwiseXOr, "^", line, column));
                }
                '!' => {
                    self.advance();
                    if self.cur_char() == Some('=') {
                        self.advance();
                        return Ok(self.tok(Tok::NotEqual, "!=", line, column));
                    }
                    return Ok(self.tok(Tok::LogicalNegation, "!", line, column));
                }
                '<' => {
                    self.advance();
                    if self.cur_char() == Some('=') {
                        self.advance();
                        return Ok(self.tok(Tok::LessOrEqual, "<=", line, column));
                    }
                    if self.cur_char() == Some('<') {
                        self.advance();
                        return Ok(self.tok(Tok::BitwiseShiftLeft, "<<", line, column));
                    }
                    return Ok(self.tok(Tok::Less, "<", line, column));
                }
                '>' => {
                    self.advance();
                    if self.cur_char() == Some('=') {
                        self.advance();
                        return Ok(self.tok(Tok::GreaterOrEqual, ">=", line, column));
                    }
                    if self.cur_char() == Some('>') {
                        self.advance();
                        return Ok(self.tok(Tok::BitwiseShiftRight, ">>", line, column));
                    }
                    return Ok(self.tok(Tok::Greater, ">", line, column));
                }
                ':' => {
                    self.advance();
                    return Ok(self.tok(Tok::Colon, ":", line, column));
                }
                ',' => {
                    self.advance();
                    return Ok(self.tok(Tok::Comma, ",", line, column));
                }
                '\'' | '"' => {
                    let delim = c;
                    self.advance();
                    let inner = self.read_quoted_string(delim)?;
                    return Ok(self.tok(Tok::Str, inner, line, column));
                }
                ' ' | '\t' | '\n' | '\r' => {
                    self.discard_while_ws();
                    continue;
                }
                _ => {
                    if c == '.' && self.peek_at(1) == Some('.') {
                        self.advance();
                        self.advance();
                        return Ok(self.tok(Tok::Range, "..", line, column));
                    }
                    if c == '=' && self.peek_at(1) == Some('=') {
                        self.advance();
                        self.advance();
                        return Ok(self.tok(Tok::Equal, "==", line, column));
                    }
                    if let Some(text) = self.read_decimal(NUMBER_DELIM) {
                        return Ok(self.tok(Tok::Number, text, line, column));
                    }
                    if let Some(text) = self.read_integer(INTEGER_DELIM) {
                        return Ok(self.tok(Tok::Number, text, line, column));
                    }
                    if self.discard("true") {
                        return Ok(self.tok(Tok::Boolean, "true", line, column));
                    }
                    if self.discard("false") {
                        return Ok(self.tok(Tok::Boolean, "false", line, column));
                    }
                    if self.discard("null") {
                        return Ok(self.tok(Tok::Null, "null", line, column));
                    }
                    if is_letter(c) || c == '_' {
                        self.advance();
                        while matches!(self.cur_char(), Some(ch) if is_letter(ch) || ch.is_ascii_digit() || ch == '_')
                        {
                            self.advance();
                        }
                        return Ok(self.tok(
                            Tok::Name,
                            self.src[start..self.pos].to_string(),
                            line,
                            column,
                        ));
                    }
                    return Err(ElError::Parse(format!(
                        "At line {line}, column {column}: Unexpected character: '{c}'"
                    )));
                }
            }
        }
    }
}

fn is_letter(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// Reads a number token's text as an `f64`. `std::from_chars` accepts `.5` and `5.`; the
/// fallback covers the `f64::from_str` cases that do not.
fn parse_number_text(s: &str) -> f64 {
    if let Ok(n) = s.parse::<f64>() {
        return n;
    }
    let mut t = s.to_string();
    if let Some(idx) = t.find('.') {
        if idx == 0 || t[..idx].chars().all(|c| c == '+' || c == '-') {
            t.insert(idx, '0');
        }
    }
    if t.ends_with('.') {
        t.push('0');
    }
    t.parse::<f64>().unwrap_or(0.0)
}

/// The EL parser.
pub struct Parser<'a> {
    mode: ParseMode,
    lexer: Lexer<'a>,
}

impl<'a> Parser<'a> {
    /// A parser over `src`, with the first token at `line`/`column`.
    pub fn new(mode: ParseMode, src: &'a str) -> Self {
        Self { mode, lexer: Lexer::new(src, 1, 1) }
    }

    /// The current lexer position.
    pub fn tokenizer_state(&self) -> TokenizerState {
        TokenizerState {
            line: self.lexer.line,
            column: self.lexer.col,
            cur: self.lexer.remainder().to_string(),
        }
    }

    /// Parses a complete expression.
    pub fn parse(&mut self) -> Result<ExpressionNode, ElError> {
        let result = self.parse_expression()?;
        if self.mode == ParseMode::Strict {
            self.peek_expect(|k| k == Tok::Eof)?;
        }
        Ok(result)
    }

    // --- token access ----------------------------------------------------------

    fn next_raw(&mut self) -> Result<Token, ElError> {
        self.lexer.emit()
    }

    fn peek_raw(&mut self) -> Result<Token, ElError> {
        let saved = self.lexer.snapshot();
        let token = self.lexer.emit();
        self.lexer.restore(saved);
        token
    }

    fn expect(token: &Token, ok: impl Fn(Tok) -> bool) -> Result<(), ElError> {
        if ok(token.kind) {
            Ok(())
        } else {
            Err(ElError::Parse(format!(
                "At line {}, column {}: unexpected {:?} (raw data: '{}')",
                token.line, token.column, token.kind, token.text
            )))
        }
    }

    fn next_token(&mut self, ok: impl Fn(Tok) -> bool) -> Result<Token, ElError> {
        let token = self.next_raw()?;
        Self::expect(&token, ok)?;
        Ok(token)
    }

    fn peek_expect(&mut self, ok: impl Fn(Tok) -> bool) -> Result<Token, ElError> {
        let token = self.peek_raw()?;
        Self::expect(&token, ok)?;
        Ok(token)
    }

    // --- grammar -------------------------------------------------------------

    fn parse_expression(&mut self) -> Result<ExpressionNode, ElError> {
        if self.peek_raw()?.kind == Tok::OParen {
            self.parse_grouped_term()
        } else {
            self.parse_term()
        }
    }

    fn parse_grouped_term(&mut self) -> Result<ExpressionNode, ElError> {
        let token = self.next_token(|k| k == Tok::OParen)?;
        let expression = self.parse_term()?;
        self.next_token(|k| k == Tok::CParen)?;
        let lhs = ExpressionNode::new(
            ExpressionKind::Unary { operation: UnaryOperation::Group, operand: expression },
            token.location(),
        );
        if is_compound_term_tok(self.peek_raw()?.kind) {
            self.parse_compound_term(lhs)
        } else {
            Ok(lhs)
        }
    }

    fn parse_term(&mut self) -> Result<ExpressionNode, ElError> {
        self.peek_expect(|k| is_simple_term_tok(k) || k == Tok::DoubleOBrace)?;
        let lhs = self.parse_simple_term_or_switch()?;
        if is_compound_term_tok(self.peek_raw()?.kind) {
            self.parse_compound_term(lhs)
        } else {
            Ok(lhs)
        }
    }

    fn parse_simple_term_or_switch(&mut self) -> Result<ExpressionNode, ElError> {
        let token = self.peek_expect(|k| is_simple_term_tok(k) || k == Tok::DoubleOBrace)?;
        if is_simple_term_tok(token.kind) {
            self.parse_simple_term_or_subscript()
        } else {
            self.parse_switch()
        }
    }

    fn parse_simple_term_or_subscript(&mut self) -> Result<ExpressionNode, ElError> {
        let mut term = self.parse_simple_term()?;
        while self.peek_raw()?.kind == Tok::OBracket {
            term = self.parse_subscript(term)?;
        }
        Ok(term)
    }

    fn parse_simple_term(&mut self) -> Result<ExpressionNode, ElError> {
        let token = self.peek_expect(is_simple_term_tok)?;
        if is_unary_op_tok(token.kind) {
            self.parse_unary_operator()
        } else if token.kind == Tok::OParen {
            self.parse_grouped_term()
        } else if token.kind == Tok::Name {
            self.parse_variable()
        } else {
            self.parse_literal()
        }
    }

    fn parse_subscript(&mut self, lhs: ExpressionNode) -> Result<ExpressionNode, ElError> {
        let token = self.next_token(|k| k == Tok::OBracket)?;
        let location = token.location();

        let mut elements = Vec::new();
        if self.peek_raw()?.kind != Tok::CBracket {
            loop {
                elements.push(self.parse_expression_or_any_range()?);
                if self.next_token(|k| k == Tok::Comma || k == Tok::CBracket)?.kind != Tok::Comma {
                    break;
                }
            }
        } else {
            self.next_raw()?;
        }

        let rhs = if elements.len() == 1 {
            elements.pop().unwrap()
        } else {
            ExpressionNode::new(ExpressionKind::Array(elements), location)
        };
        Ok(ExpressionNode::new(
            ExpressionKind::Subscript { left: lhs, right: rhs },
            location,
        ))
    }

    fn parse_variable(&mut self) -> Result<ExpressionNode, ElError> {
        let token = self.next_token(|k| k == Tok::Name)?;
        Ok(ExpressionNode::new(
            ExpressionKind::Variable(token.text.clone()),
            token.location(),
        ))
    }

    fn parse_literal(&mut self) -> Result<ExpressionNode, ElError> {
        let token = self.peek_expect(|k| {
            is_literal_tok(k) || k == Tok::OBracket || k == Tok::OBrace
        })?;
        match token.kind {
            Tok::Str => {
                self.next_raw()?;
                let value = str_unescape(&token.text, "\\\"", '\\');
                Ok(ExpressionNode::new(
                    ExpressionKind::Literal(crate::Value::string(value)),
                    token.location(),
                ))
            }
            Tok::Number => {
                self.next_raw()?;
                Ok(ExpressionNode::new(
                    ExpressionKind::Literal(crate::Value::number(parse_number_text(&token.text))),
                    token.location(),
                ))
            }
            Tok::Boolean => {
                self.next_raw()?;
                Ok(ExpressionNode::new(
                    ExpressionKind::Literal(crate::Value::boolean(token.text == "true")),
                    token.location(),
                ))
            }
            Tok::Null => {
                self.next_raw()?;
                Ok(ExpressionNode::new(
                    ExpressionKind::Literal(crate::Value::null()),
                    token.location(),
                ))
            }
            Tok::OBracket => self.parse_array(),
            _ => self.parse_map(),
        }
    }

    fn parse_array(&mut self) -> Result<ExpressionNode, ElError> {
        let token = self.next_token(|k| k == Tok::OBracket)?;
        let location = token.location();

        let mut elements = Vec::new();
        if self.peek_raw()?.kind != Tok::CBracket {
            loop {
                elements.push(self.parse_expression_or_bounded_range()?);
                if self.next_token(|k| k == Tok::Comma || k == Tok::CBracket)?.kind != Tok::Comma {
                    break;
                }
            }
        } else {
            self.next_raw()?;
        }

        Ok(ExpressionNode::new(ExpressionKind::Array(elements), location))
    }

    fn parse_expression_or_bounded_range(&mut self) -> Result<ExpressionNode, ElError> {
        let mut expression = self.parse_expression()?;
        if self.peek_raw()?.kind == Tok::Range {
            let token = self.next_raw()?;
            expression = ExpressionNode::new(
                ExpressionKind::Binary {
                    operation: BinaryOperation::BoundedRange,
                    left: expression,
                    right: self.parse_expression()?,
                },
                token.location(),
            );
        }
        Ok(expression)
    }

    fn parse_expression_or_any_range(&mut self) -> Result<ExpressionNode, ElError> {
        if self.peek_raw()?.kind == Tok::Range {
            let token = self.next_raw()?;
            return Ok(ExpressionNode::new(
                ExpressionKind::Unary {
                    operation: UnaryOperation::RightBoundedRange,
                    operand: self.parse_expression()?,
                },
                token.location(),
            ));
        }

        let mut expression = self.parse_expression()?;
        if self.peek_raw()?.kind == Tok::Range {
            let token = self.next_raw()?;
            if is_simple_term_tok(self.peek_raw()?.kind) {
                expression = ExpressionNode::new(
                    ExpressionKind::Binary {
                        operation: BinaryOperation::BoundedRange,
                        left: expression,
                        right: self.parse_expression()?,
                    },
                    token.location(),
                );
            } else {
                expression = ExpressionNode::new(
                    ExpressionKind::Unary {
                        operation: UnaryOperation::LeftBoundedRange,
                        operand: expression,
                    },
                    token.location(),
                );
            }
        }
        Ok(expression)
    }

    fn parse_map(&mut self) -> Result<ExpressionNode, ElError> {
        let token = self.next_token(|k| k == Tok::OBrace)?;
        let location = token.location();

        let mut elements: BTreeMap<String, ExpressionNode> = BTreeMap::new();
        if self.peek_raw()?.kind != Tok::CBrace {
            loop {
                let key_token = self.next_token(|k| k == Tok::Str || k == Tok::Name)?;
                let key = key_token.text.clone();
                self.next_token(|k| k == Tok::Colon)?;
                let value = self.parse_expression()?;
                elements.entry(key).or_insert(value);
                if self.next_token(|k| k == Tok::Comma || k == Tok::CBrace)?.kind != Tok::Comma {
                    break;
                }
            }
        } else {
            self.next_raw()?;
        }

        Ok(ExpressionNode::new(ExpressionKind::Map(elements), location))
    }

    fn parse_unary_operator(&mut self) -> Result<ExpressionNode, ElError> {
        let token = self.next_token(is_unary_op_tok)?;
        let operation = match token.kind {
            Tok::Addition => UnaryOperation::Plus,
            Tok::Subtraction => UnaryOperation::Minus,
            Tok::LogicalNegation => UnaryOperation::LogicalNegation,
            Tok::BitwiseNegation => UnaryOperation::BitwiseNegation,
            _ => unreachable!("checked by next_token"),
        };
        Ok(ExpressionNode::new(
            ExpressionKind::Unary { operation, operand: self.parse_simple_term_or_switch()? },
            token.location(),
        ))
    }

    fn parse_switch(&mut self) -> Result<ExpressionNode, ElError> {
        let token = self.next_token(|k| k == Tok::DoubleOBrace)?;
        let location = token.location();

        let mut cases = Vec::new();
        let peeked = self.peek_expect(|k| is_simple_term_tok(k) || k == Tok::DoubleCBrace)?;
        if is_simple_term_tok(peeked.kind) {
            loop {
                cases.push(self.parse_expression()?);
                if self.next_token(|k| k == Tok::Comma || k == Tok::DoubleCBrace)?.kind
                    != Tok::Comma
                {
                    break;
                }
            }
        } else if peeked.kind == Tok::DoubleCBrace {
            self.next_raw()?;
        }

        Ok(ExpressionNode::new(ExpressionKind::Switch(cases), location))
    }

    fn parse_compound_term(
        &mut self,
        mut lhs: ExpressionNode,
    ) -> Result<ExpressionNode, ElError> {
        while is_compound_term_tok(self.peek_raw()?.kind) {
            let token = self.next_token(is_compound_term_tok)?;
            let operation = binop_for_tok(token.kind);
            lhs = ExpressionNode::new(
                ExpressionKind::Binary {
                    operation,
                    left: lhs,
                    right: self.parse_simple_term_or_switch()?,
                },
                token.location(),
            );
        }
        Ok(lhs)
    }
}

/// Parses `src` as an expression.
pub fn parse_expression(mode: ParseMode, src: &str) -> Result<ExpressionNode, ElError> {
    Parser::new(mode, src).parse()
}
