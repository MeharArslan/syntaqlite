// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// ── Public API ───────────────────────────────────────────────────────────

/// A structured parse error from a `.synq` file.
#[derive(Debug, Clone)]
pub(crate) struct SynqParseError {
    pub(crate) message: String,
}

impl std::fmt::Display for SynqParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Parse a single .synq file's contents into a list of items.
pub(crate) fn parse_synq_file(input: &str) -> Result<Vec<Item>, SynqParseError> {
    let tokens = tokenize(input).map_err(|message| SynqParseError { message })?;
    Parser::new(tokens)
        .parse_file()
        .map_err(|message| SynqParseError { message })
}

// ── AST ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaKind {
    Table,
    View,
    Function,
    Import,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct SchemaParam {
    pub(crate) key: String,
    pub(crate) field: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct SchemaAnnotation {
    pub(crate) kind: SchemaKind,
    pub(crate) params: Vec<SchemaParam>,
}

#[derive(Debug)]
pub(crate) enum Item {
    Node {
        name: String,
        fields: Vec<Field>,
        fmt: Option<Vec<Fmt>>,
        schema: Option<SchemaAnnotation>,
    },
    Enum {
        name: String,
        variants: Vec<String>,
    },
    Flags {
        name: String,
        flags: Vec<(String, u32)>,
    },
    List {
        name: String,
        child_type: String,
        fmt: Option<Vec<Fmt>>,
    },
    Abstract {
        name: String,
        members: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Storage {
    Index,
    Inline,
}

#[derive(Debug)]
pub(crate) struct Field {
    pub(crate) name: String,
    pub(crate) storage: Storage,
    pub(crate) type_name: String,
}

#[derive(Debug)]
pub(crate) enum Fmt {
    Text(String),
    Child(String),
    Span(String),
    Line,
    SoftLine,
    HardLine,
    Group(Vec<Self>),
    Nest(Vec<Self>),
    IfSet {
        field: String,
        then: Vec<Self>,
        els: Option<Vec<Self>>,
    },
    IfFlag {
        field: String,
        bit: Option<String>,
        then: Vec<Self>,
        els: Option<Vec<Self>>,
    },
    IfEnum {
        field: String,
        variant: String,
        then: Vec<Self>,
        els: Option<Vec<Self>>,
    },
    IfSpan {
        field: String,
        then: Vec<Self>,
        els: Option<Vec<Self>>,
    },
    Clause {
        keyword: String,
        field: String,
    },
    Switch {
        field: String,
        cases: Vec<(String, Vec<Self>)>,
        default: Option<Vec<Self>>,
    },
    EnumDisplay {
        field: String,
        mappings: Vec<(String, String)>,
    },
    ForEach {
        sep: Option<Vec<Self>>,
        body: Vec<Self>,
    },
}

// ── Tokens ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    Str(String),
    Int(u32),
    LBrace,
    RBrace,
    LParen,
    RParen,
    Colon,
    Dot,
    Comma,
    Eq,
    Eof,
}

// ── Tokenizer ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let b = input.as_bytes();
    let (mut i, mut line, mut col) = (0, 1usize, 1usize);

    let advance = |i: &mut usize, line: &mut usize, col: &mut usize| -> Option<u8> {
        let ch = *b.get(*i)?;
        *i += 1;
        if ch == b'\n' {
            *line += 1;
            *col = 1;
        } else {
            *col += 1;
        }
        Some(ch)
    };

    let mut tokens = Vec::new();
    loop {
        // skip whitespace and # comments
        loop {
            match b.get(i) {
                Some(b' ' | b'\t' | b'\n' | b'\r') => {
                    advance(&mut i, &mut line, &mut col);
                }
                Some(b'#') => {
                    while let Some(ch) = advance(&mut i, &mut line, &mut col) {
                        if ch == b'\n' {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
        let Some(&ch) = b.get(i) else {
            tokens.push(Token::Eof);
            break;
        };
        match ch {
            b'{' => {
                advance(&mut i, &mut line, &mut col);
                tokens.push(Token::LBrace);
            }
            b'}' => {
                advance(&mut i, &mut line, &mut col);
                tokens.push(Token::RBrace);
            }
            b'(' => {
                advance(&mut i, &mut line, &mut col);
                tokens.push(Token::LParen);
            }
            b')' => {
                advance(&mut i, &mut line, &mut col);
                tokens.push(Token::RParen);
            }
            b':' => {
                advance(&mut i, &mut line, &mut col);
                tokens.push(Token::Colon);
            }
            b'.' => {
                advance(&mut i, &mut line, &mut col);
                tokens.push(Token::Dot);
            }
            b',' => {
                advance(&mut i, &mut line, &mut col);
                tokens.push(Token::Comma);
            }
            b'=' => {
                advance(&mut i, &mut line, &mut col);
                tokens.push(Token::Eq);
            }
            b'"' => {
                advance(&mut i, &mut line, &mut col);
                let mut s = String::new();
                loop {
                    match advance(&mut i, &mut line, &mut col) {
                        Some(b'"') => break,
                        Some(b'\\') => match advance(&mut i, &mut line, &mut col) {
                            Some(b'"') => s.push('"'),
                            Some(b'\\') => s.push('\\'),
                            Some(b'n') => s.push('\n'),
                            _ => return Err(format!("{line}:{col}: bad escape")),
                        },
                        Some(ch) => s.push(ch as char),
                        None => return Err(format!("{line}:{col}: unterminated string")),
                    }
                }
                tokens.push(Token::Str(s));
            }
            _ if ch.is_ascii_alphabetic() || ch == b'_' => {
                let start = i;
                while b
                    .get(i)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
                {
                    advance(&mut i, &mut line, &mut col);
                }
                tokens.push(Token::Ident(String::from_utf8_lossy(&b[start..i]).into()));
            }
            _ if ch.is_ascii_digit() => {
                let start = i;
                while b.get(i).is_some_and(u8::is_ascii_digit) {
                    advance(&mut i, &mut line, &mut col);
                }
                let n = std::str::from_utf8(&b[start..i])
                    .expect("digit sequence is valid UTF-8")
                    .parse()
                    .expect("digit sequence is a valid u32");
                tokens.push(Token::Int(n));
            }
            _ => return Err(format!("{}:{}: unexpected '{}'", line, col, ch as char)),
        }
    }
    Ok(tokens)
}

// ── Parser ───────────────────────────────────────────────────────────────

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    const fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }
    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if t != Token::Eof {
            self.pos += 1;
        }
        t
    }
    fn at(&self, s: &str) -> bool {
        matches!(self.peek(), Token::Ident(n) if n == s)
    }
    fn at_tok(&self, t: &Token) -> bool {
        self.peek() == t
    }
    fn expect(&mut self, t: &Token) -> Result<(), String> {
        let got = self.advance();
        if got == *t {
            Ok(())
        } else {
            Err(format!("expected {t:?}, got {got:?}"))
        }
    }
    fn ident(&mut self) -> Result<String, String> {
        match self.advance() {
            Token::Ident(s) => Ok(s),
            t => Err(format!("expected ident, got {t:?}")),
        }
    }
    fn string(&mut self) -> Result<String, String> {
        match self.advance() {
            Token::Str(s) => Ok(s),
            t => Err(format!("expected string, got {t:?}")),
        }
    }
    fn int(&mut self) -> Result<u32, String> {
        match self.advance() {
            Token::Int(n) => Ok(n),
            t => Err(format!("expected int, got {t:?}")),
        }
    }

    fn parse_file(&mut self) -> Result<Vec<Item>, String> {
        let mut items = Vec::new();
        while !self.at_tok(&Token::Eof) {
            items.push(self.parse_item()?);
        }
        Ok(items)
    }

    fn parse_item(&mut self) -> Result<Item, String> {
        if self.at("node") {
            self.advance();
            return self.parse_node_item();
        }
        if self.at("enum") {
            self.advance();
            return self.parse_enum();
        }
        if self.at("flags") {
            self.advance();
            return self.parse_flags();
        }
        if self.at("list") {
            self.advance();
            return self.parse_list();
        }
        if self.at("abstract") {
            self.advance();
            return self.parse_abstract();
        }
        Err(format!("expected item keyword, got {:?}", self.peek()))
    }

    fn parse_node_item(&mut self) -> Result<Item, String> {
        let name = self.ident()?;
        self.expect(&Token::LBrace)?;
        let mut fields = Vec::new();
        let mut fmt = None;
        let mut schema = None;
        loop {
            if self.at_tok(&Token::RBrace) {
                self.advance();
                break;
            }
            if self.at("fmt") {
                self.advance();
                self.expect(&Token::LBrace)?;
                fmt = Some(self.parse_fmt_seq()?);
                self.expect(&Token::RBrace)?;
            } else if self.at("session_schema") {
                self.advance();
                self.expect(&Token::LBrace)?;
                schema = Some(self.parse_schema(&name, &fields)?);
                self.expect(&Token::RBrace)?;
            } else {
                let name = self.ident()?;
                self.expect(&Token::Colon)?;
                let storage = if self.at("index") {
                    self.advance();
                    Storage::Index
                } else if self.at("inline") {
                    self.advance();
                    Storage::Inline
                } else {
                    return Err(format!("expected storage class, got {:?}", self.peek()));
                };
                let type_name = self.ident()?;
                fields.push(Field {
                    name,
                    storage,
                    type_name,
                });
            }
        }
        Ok(Item::Node {
            name,
            fields,
            fmt,
            schema,
        })
    }

    fn parse_schema(
        &mut self,
        node_name: &str,
        fields: &[Field],
    ) -> Result<SchemaAnnotation, String> {
        let kind_str = self.ident()?;
        let kind = match kind_str.as_str() {
            "table" => SchemaKind::Table,
            "view" => SchemaKind::View,
            "function" => SchemaKind::Function,
            "import" => SchemaKind::Import,
            _ => {
                return Err(format!(
                    "unknown schema kind '{kind_str}' in node '{node_name}'"
                ));
            }
        };
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while !self.at_tok(&Token::RParen) {
            if !params.is_empty() {
                self.expect(&Token::Comma)?;
            }
            let key = self.ident()?;
            self.expect(&Token::Colon)?;
            let field = self.ident()?;
            // Validate that the referenced field exists.
            if !fields.iter().any(|f| f.name == field) {
                return Err(format!(
                    "schema annotation in '{node_name}' references unknown field '{field}'"
                ));
            }
            params.push(SchemaParam { key, field });
        }
        self.advance(); // consume RParen
        Ok(SchemaAnnotation { kind, params })
    }

    fn parse_enum(&mut self) -> Result<Item, String> {
        let name = self.ident()?;
        self.expect(&Token::LBrace)?;
        let mut variants = Vec::new();
        while !self.at_tok(&Token::RBrace) {
            variants.push(self.ident()?);
        }
        self.advance();
        Ok(Item::Enum { name, variants })
    }

    fn parse_flags(&mut self) -> Result<Item, String> {
        let name = self.ident()?;
        self.expect(&Token::LBrace)?;
        let mut flags = Vec::new();
        while !self.at_tok(&Token::RBrace) {
            let n = self.ident()?;
            self.expect(&Token::Eq)?;
            let v = self.int()?;
            flags.push((n, v));
        }
        self.advance();
        Ok(Item::Flags { name, flags })
    }

    fn parse_abstract(&mut self) -> Result<Item, String> {
        let name = self.ident()?;
        self.expect(&Token::LBrace)?;
        let mut members = Vec::new();
        while !self.at_tok(&Token::RBrace) {
            members.push(self.ident()?);
        }
        self.advance();
        Ok(Item::Abstract { name, members })
    }

    fn parse_list(&mut self) -> Result<Item, String> {
        let name = self.ident()?;
        self.expect(&Token::LBrace)?;
        let child_type = self.ident()?;
        let fmt = if self.at("fmt") {
            self.advance();
            self.expect(&Token::LBrace)?;
            let items = self.parse_fmt_seq()?;
            self.expect(&Token::RBrace)?;
            Some(items)
        } else {
            None
        };
        self.expect(&Token::RBrace)?;
        Ok(Item::List {
            name,
            child_type,
            fmt,
        })
    }

    // ── fmt parsing ──────────────────────────────────────────────────

    fn parse_fmt_seq(&mut self) -> Result<Vec<Fmt>, String> {
        let mut items = Vec::new();
        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            items.push(self.parse_fmt()?);
        }
        Ok(items)
    }

    fn braced(&mut self) -> Result<Vec<Fmt>, String> {
        self.expect(&Token::LBrace)?;
        let items = self.parse_fmt_seq()?;
        self.expect(&Token::RBrace)?;
        Ok(items)
    }

    fn optional_else(&mut self) -> Result<Option<Vec<Fmt>>, String> {
        if self.at("else") {
            self.advance();
            Ok(Some(self.braced()?))
        } else {
            Ok(None)
        }
    }

    #[allow(clippy::too_many_lines)]
    fn parse_fmt(&mut self) -> Result<Fmt, String> {
        match self.peek().clone() {
            Token::Str(s) => {
                self.advance();
                Ok(Fmt::Text(s))
            }
            Token::Ident(ref s) => match s.as_str() {
                "child" => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let f = self.ident()?;
                    self.expect(&Token::RParen)?;
                    Ok(Fmt::Child(f))
                }
                "span" => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let f = self.ident()?;
                    self.expect(&Token::RParen)?;
                    Ok(Fmt::Span(f))
                }
                "line" => {
                    self.advance();
                    Ok(Fmt::Line)
                }
                "softline" => {
                    self.advance();
                    Ok(Fmt::SoftLine)
                }
                "hardline" => {
                    self.advance();
                    Ok(Fmt::HardLine)
                }
                "group" => {
                    self.advance();
                    Ok(Fmt::Group(self.braced()?))
                }
                "nest" => {
                    self.advance();
                    Ok(Fmt::Nest(self.braced()?))
                }
                "if_set" => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let field = self.ident()?;
                    self.expect(&Token::RParen)?;
                    let then = self.braced()?;
                    let els = self.optional_else()?;
                    Ok(Fmt::IfSet { field, then, els })
                }
                "if_flag" => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let field = self.ident()?;
                    let bit = if self.at_tok(&Token::Dot) {
                        self.advance();
                        Some(self.ident()?)
                    } else {
                        None
                    };
                    self.expect(&Token::RParen)?;
                    let then = self.braced()?;
                    let els = self.optional_else()?;
                    Ok(Fmt::IfFlag {
                        field,
                        bit,
                        then,
                        els,
                    })
                }
                "if_enum" => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let field = self.ident()?;
                    self.expect(&Token::Comma)?;
                    let variant = self.ident()?;
                    self.expect(&Token::RParen)?;
                    let then = self.braced()?;
                    let els = self.optional_else()?;
                    Ok(Fmt::IfEnum {
                        field,
                        variant,
                        then,
                        els,
                    })
                }
                "if_span" => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let field = self.ident()?;
                    self.expect(&Token::RParen)?;
                    let then = self.braced()?;
                    let els = self.optional_else()?;
                    Ok(Fmt::IfSpan { field, then, els })
                }
                "clause" => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let kw = self.string()?;
                    self.expect(&Token::Comma)?;
                    let field = self.ident()?;
                    self.expect(&Token::RParen)?;
                    Ok(Fmt::Clause { keyword: kw, field })
                }
                "switch" => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let field = self.ident()?;
                    self.expect(&Token::RParen)?;
                    self.expect(&Token::LBrace)?;
                    let mut cases = Vec::new();
                    let mut default = None;
                    while !self.at_tok(&Token::RBrace) {
                        if self.at("default") {
                            self.advance();
                            default = Some(self.braced()?);
                        } else {
                            let v = self.ident()?;
                            cases.push((v, self.braced()?));
                        }
                    }
                    self.advance();
                    Ok(Fmt::Switch {
                        field,
                        cases,
                        default,
                    })
                }
                "enum_display" => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let field = self.ident()?;
                    self.expect(&Token::Comma)?;
                    self.expect(&Token::LBrace)?;
                    let mut mappings = Vec::new();
                    while !self.at_tok(&Token::RBrace) {
                        let v = self.ident()?;
                        self.expect(&Token::Eq)?;
                        let d = self.string()?;
                        mappings.push((v, d));
                    }
                    self.advance();
                    self.expect(&Token::RParen)?;
                    Ok(Fmt::EnumDisplay { field, mappings })
                }
                "for_each" => {
                    self.advance();
                    let sep = if self.at_tok(&Token::LParen) {
                        self.advance();
                        if !self.at("sep") {
                            return Err(format!("expected 'sep', got {:?}", self.peek()));
                        }
                        self.advance();
                        self.expect(&Token::Colon)?;
                        let mut items = Vec::new();
                        while !self.at_tok(&Token::RParen) {
                            items.push(self.parse_fmt()?);
                        }
                        self.advance();
                        Some(items)
                    } else {
                        None
                    };
                    let body = self.braced()?;
                    Ok(Fmt::ForEach { sep, body })
                }
                _ => Err(format!("unexpected in fmt: {:?}", self.peek())),
            },
            other => Err(format!("unexpected in fmt: {other:?}")),
        }
    }
}
