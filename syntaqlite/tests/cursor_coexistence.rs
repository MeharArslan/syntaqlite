// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Tests that parser + cursor can be stored in the same struct.
//!
//! This is the key invariant of the Rc<RefCell> checkout pattern: `parse()`
//! takes `&self`, so the parser can be moved into the same struct as the
//! cursor it produced.

use syntaqlite::dialect::{
    DialectIncrementalCursor, DialectIncrementalParser, DialectParser, DialectStatementCursor,
    DialectTokenCursor, DialectTokenizer,
};
use syntaqlite::incremental::{IncrementalCursor, IncrementalParser};
use syntaqlite::{Parser, StatementCursor, TokenCursor, Tokenizer};
use syntaqlite_parser::{
    Dialect as TaggedDialect, RawIncrementalCursor, RawIncrementalParser, RawParser,
    RawStatementCursor, RawTokenCursor, RawTokenizer,
};
use syntaqlite_parser_sqlite::SqliteNodeFamily;

fn raw_dialect() -> syntaqlite_parser::DialectEnv<'static> {
    syntaqlite::dialect::sqlite()
}

fn tagged_dialect() -> TaggedDialect<'static, SqliteNodeFamily> {
    TaggedDialect::from_raw_dialect(raw_dialect())
}

// ── Raw layer ────────────────────────────────────────────────────────────

#[test]
fn raw_parser_and_cursor_coexist() {
    struct S<'a> {
        _parser: RawParser<'static>,
        cursor: RawStatementCursor<'a>,
    }

    let parser = RawParser::new(raw_dialect());
    let cursor = parser.parse("SELECT 1");
    let mut s = S {
        _parser: parser,
        cursor,
    };
    assert!(s.cursor.next_statement().unwrap().is_ok());
}

#[test]
fn raw_parser_reuse_after_cursor_drop() {
    let parser = RawParser::new(raw_dialect());
    {
        let mut c = parser.parse("SELECT 1");
        assert!(c.next_statement().unwrap().is_ok());
    }
    {
        let mut c = parser.parse("SELECT 2");
        assert!(c.next_statement().unwrap().is_ok());
    }
}

#[test]
fn raw_tokenizer_and_cursor_coexist() {
    struct S<'a> {
        _tokenizer: RawTokenizer<'static>,
        cursor: RawTokenCursor<'a>,
    }

    let tokenizer = RawTokenizer::new(raw_dialect());
    let cursor = tokenizer.tokenize("SELECT 1");
    let mut s = S {
        _tokenizer: tokenizer,
        cursor,
    };
    assert!(s.cursor.next().is_some());
}

#[test]
fn raw_tokenizer_reuse_after_cursor_drop() {
    let tokenizer = RawTokenizer::new(raw_dialect());
    {
        let mut c = tokenizer.tokenize("SELECT 1");
        assert!(c.next().is_some());
    }
    {
        let mut c = tokenizer.tokenize("SELECT 2");
        assert!(c.next().is_some());
    }
}

#[test]
fn raw_incremental_parser_and_cursor_coexist() {
    struct S<'a> {
        _parser: RawIncrementalParser<'static>,
        cursor: RawIncrementalCursor<'a>,
    }

    let parser = RawIncrementalParser::new(raw_dialect());
    let cursor = parser.feed("SELECT 1");
    let mut s = S {
        _parser: parser,
        cursor,
    };
    assert!(s.cursor.finish().is_ok());
}

#[test]
fn raw_incremental_reuse_after_cursor_drop() {
    let parser = RawIncrementalParser::new(raw_dialect());
    {
        let mut c = parser.feed("SELECT 1");
        assert!(c.finish().is_ok());
    }
    {
        let mut c = parser.feed("SELECT 2");
        assert!(c.finish().is_ok());
    }
}

// ── Typed (dialect-generic) layer ────────────────────────────────────────

#[test]
fn dialect_parser_and_cursor_coexist() {
    struct S<'a> {
        _parser: DialectParser<'static, SqliteNodeFamily>,
        cursor: DialectStatementCursor<'a, SqliteNodeFamily>,
    }

    let parser = DialectParser::from_dialect(tagged_dialect());
    let cursor = parser.parse("SELECT 1");
    let mut s = S {
        _parser: parser,
        cursor,
    };
    assert!(s.cursor.next_statement().unwrap().is_ok());
}

#[test]
fn dialect_tokenizer_and_cursor_coexist() {
    struct S<'a> {
        _tokenizer: DialectTokenizer<'static, SqliteNodeFamily>,
        cursor: DialectTokenCursor<'a, SqliteNodeFamily>,
    }

    let tokenizer = DialectTokenizer::from_dialect(tagged_dialect());
    let cursor = tokenizer.tokenize("SELECT 1");
    let mut s = S {
        _tokenizer: tokenizer,
        cursor,
    };
    assert!(s.cursor.next().is_some());
}

#[test]
fn dialect_incremental_parser_and_cursor_coexist() {
    struct S<'a> {
        _parser: DialectIncrementalParser<'static, SqliteNodeFamily>,
        cursor: DialectIncrementalCursor<'a, SqliteNodeFamily>,
    }

    let parser = DialectIncrementalParser::from_dialect(tagged_dialect());
    let cursor = parser.feed("SELECT 1");
    let mut s = S {
        _parser: parser,
        cursor,
    };
    assert!(s.cursor.finish().is_ok());
}

// ── SQLite convenience layer ─────────────────────────────────────────────

#[test]
fn sqlite_parser_and_cursor_coexist() {
    struct S<'a> {
        _parser: Parser,
        cursor: StatementCursor<'a>,
    }

    let parser = Parser::new();
    let cursor = parser.parse("SELECT 1");
    let mut s = S {
        _parser: parser,
        cursor,
    };
    assert!(s.cursor.next_statement().unwrap().is_ok());
}

#[test]
fn sqlite_tokenizer_and_cursor_coexist() {
    struct S<'a> {
        _tokenizer: Tokenizer,
        cursor: TokenCursor<'a>,
    }

    let tokenizer = Tokenizer::new();
    let cursor = tokenizer.tokenize("SELECT 1");
    let mut s = S {
        _tokenizer: tokenizer,
        cursor,
    };
    assert!(s.cursor.next().is_some());
}

#[test]
fn sqlite_incremental_parser_and_cursor_coexist() {
    struct S<'a> {
        _parser: IncrementalParser,
        cursor: IncrementalCursor<'a>,
    }

    let parser = IncrementalParser::new();
    let cursor = parser.feed("SELECT 1");
    let mut s = S {
        _parser: parser,
        cursor,
    };
    assert!(s.cursor.finish().is_ok());
}
