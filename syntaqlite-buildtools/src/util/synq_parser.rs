// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// ── Public API ───────────────────────────────────────────────────────────

/// Parse a single .synq file's contents into a list of items.
pub fn parse_synq_file(input: &str) -> Result<Vec<Item>, String> {
    let tokens = tokenize(input)?;
    Parser::new(tokens).parse_file()
}

// ── AST ──────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum Item {
    Node {
        name: String,
        fields: Vec<Field>,
        fmt: Option<Vec<Fmt>>,
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
pub enum Storage {
    Index,
    Inline,
}

#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub storage: Storage,
    pub type_name: String,
}

#[derive(Debug)]
pub enum Fmt {
    Text(String),
    Child(String),
    Span(String),
    Line,
    SoftLine,
    HardLine,
    Group(Vec<Fmt>),
    Nest(Vec<Fmt>),
    IfSet {
        field: String,
        then: Vec<Fmt>,
        els: Option<Vec<Fmt>>,
    },
    IfFlag {
        field: String,
        bit: Option<String>,
        then: Vec<Fmt>,
        els: Option<Vec<Fmt>>,
    },
    IfEnum {
        field: String,
        variant: String,
        then: Vec<Fmt>,
        els: Option<Vec<Fmt>>,
    },
    IfSpan {
        field: String,
        then: Vec<Fmt>,
        els: Option<Vec<Fmt>>,
    },
    Clause {
        keyword: String,
        field: String,
    },
    Switch {
        field: String,
        cases: Vec<(String, Vec<Fmt>)>,
        default: Option<Vec<Fmt>>,
    },
    EnumDisplay {
        field: String,
        mappings: Vec<(String, String)>,
    },
    ForEach {
        sep: Option<Vec<Fmt>>,
        body: Vec<Fmt>,
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
                            _ => return Err(format!("{}:{}: bad escape", line, col)),
                        },
                        Some(ch) => s.push(ch as char),
                        None => return Err(format!("{}:{}: unterminated string", line, col)),
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
                while b.get(i).is_some_and(|c| c.is_ascii_digit()) {
                    advance(&mut i, &mut line, &mut col);
                }
                let n = std::str::from_utf8(&b[start..i]).unwrap().parse().unwrap();
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
    fn new(tokens: Vec<Token>) -> Self {
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
            Err(format!("expected {:?}, got {:?}", t, got))
        }
    }
    fn ident(&mut self) -> Result<String, String> {
        match self.advance() {
            Token::Ident(s) => Ok(s),
            t => Err(format!("expected ident, got {:?}", t)),
        }
    }
    fn string(&mut self) -> Result<String, String> {
        match self.advance() {
            Token::Str(s) => Ok(s),
            t => Err(format!("expected string, got {:?}", t)),
        }
    }
    fn int(&mut self) -> Result<u32, String> {
        match self.advance() {
            Token::Int(n) => Ok(n),
            t => Err(format!("expected int, got {:?}", t)),
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
        Ok(Item::Node { name, fields, fmt })
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
            other => Err(format!("unexpected in fmt: {:?}", other)),
        }
    }
}
