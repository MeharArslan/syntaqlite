// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashSet;

use crate::util::c_writer::CWriter;
use crate::util::pascal_case;

/// Controls the `#include` paths emitted by `generate_dialect_c()`.
///
/// Two kinds of headers are included:
/// - **Internal**: dialect-private headers (`dialect_builder.h`, `dialect_meta.h`, …).
/// - **Public**: runtime/API headers (`syntaqlite/parser.h`, `syntaqlite/dialect.h`, …).
///
/// Each prefix is prepended verbatim, so include `"/"` if the prefix is a directory.
///
/// **Default** (external dialect crates): both prefixes `""`, `dialect_include_dir`
/// set to `"syntaqlite_{dialect}"` — all headers assumed discoverable from a
/// single root.
///
/// **Internal syntaqlite crate**: `internal = "csrc/sqlite/"`, `public = ""`,
/// `dialect_include_dir = "syntaqlite"` because private headers live in
/// `csrc/sqlite/` while public headers (including dialect-specific ones) are
/// all under `include/syntaqlite/`.
#[derive(Debug, Clone, Default)]
pub struct DialectCIncludes<'a> {
    /// Prefix for internal dialect headers (`dialect_builder.h`, `dialect_meta.h`, etc.).
    pub internal: &'a str,
    /// Prefix for public headers (`syntaqlite/parser.h`, `syntaqlite/dialect.h`, etc.).
    pub public: &'a str,
    /// Directory name for dialect public headers (`sqlite_tokens.h`, `sqlite_node.h`, etc.)
    /// in `#include` directives. E.g., `"syntaqlite"` (internal crate, merged with runtime)
    /// or `"syntaqlite_mydialect"` (external dialect crate, separate directory).
    pub dialect_include_dir: &'a str,
}

/// Classify a token name into a `TokenCategory` byte value.
///
/// If `keyword_names` is provided (from the mkkeywordhash table + dialect
/// extra keywords), any token whose name appears in that set is classified
/// as Keyword (1), overriding the default heuristic.
fn classify_token(name: &str, keyword_names: Option<&HashSet<String>>) -> u8 {
    let base = match name {
        "STRING" | "BLOB" => 3,               // String
        "INTEGER" | "FLOAT" | "QNUMBER" => 4, // Number
        "BITAND" | "BITOR" | "LSHIFT" | "RSHIFT" | "PLUS" | "MINUS" | "STAR" | "SLASH" | "REM"
        | "CONCAT" | "PTR" | "BITNOT" | "NE" | "EQ" | "GT" | "LE" | "LT" | "GE" => 5, // Operator
        "SEMI" | "LP" | "RP" | "COMMA" | "DOT" | "ASTERISK" => 6, // Punctuation
        "COMMENT" => 7,                       // Comment
        "VARIABLE" => 8,                      // Variable
        "AGG_FUNCTION" | "AGGFUNCTION" | "FUNCTION" => 9, // Function
        "ID" => 2,                            // Identifier
        "SPACE" | "ERROR" | "ILLEGAL" | "SPAN" | "UPLUS" | "UMINUS" | "TRUTH" | "REGISTER"
        | "VECTOR" | "SELECT_COLUMN" | "IF_NULL_ROW" | "AGG_COLUMN" => 0, // Other
        _ => 0,                               // Other (unknown tokens)
    };

    // Keyword-set override should only affect tokens that are otherwise
    // unknown ("Other") or function-like (FUNCTION/AGG_FUNCTION), so
    // structural tokens like ID/DOT/LP remain correctly classified.
    if let Some(kws) = keyword_names
        && kws.contains(name)
        && (base == 0 || base == 9)
    {
        return 1; // Keyword
    }

    base
}

/// Generate a C header with a static token categories table.
///
/// When `keyword_names` is provided, tokens whose names appear in the set
/// are classified as Keyword (1), overriding the default heuristic.
pub fn generate_token_categories_header(
    tokens: &[(String, u32)],
    keyword_names: Option<&HashSet<String>>,
) -> String {
    let max_ordinal = tokens.iter().map(|(_, v)| *v).max().unwrap_or(0);
    let count = max_ordinal as usize + 1;

    // Build ordinal → category map
    let mut categories = vec![0u8; count]; // default = Other
    for (name, ordinal) in tokens {
        categories[*ordinal as usize] = classify_token(name, keyword_names);
    }

    let mut w = CWriter::new();
    w.file_header();
    w.line(&format!("#define TOKEN_TYPE_COUNT {count}"));
    w.newline();
    w.line(&format!(
        "static const uint8_t token_categories[{count}] = {{"
    ));

    // Emit 16 values per line for readability
    for chunk in categories.chunks(16) {
        let vals: Vec<String> = chunk.iter().map(|b| format!("{b}")).collect();
        w.line(&format!("    {},", vals.join(",")));
    }

    w.line("};");
    w.finish()
}

pub fn generate_dialect_c(
    dialect: &str,
    tokens: Option<&[(String, u32)]>,
    includes: &DialectCIncludes<'_>,
) -> String {
    let upper = dialect.to_uppercase();
    let ip = includes.internal;
    let pp = includes.public;
    let mut w = CWriter::new();
    w.file_header();
    w.include_local(&format!("{pp}syntaqlite/parser.h"));
    w.include_local(&format!("{pp}syntaqlite_{dialect}/{dialect}_tokens.h"));
    w.include_local(&format!("{pp}syntaqlite/dialect.h"));
    w.include_local(&format!("{ip}dialect_builder.h"));
    w.include_local(&format!("{ip}dialect_meta.h"));
    w.include_local(&format!("{ip}dialect_fmt.h"));
    if tokens.is_some() {
        w.include_local(&format!("{ip}dialect_tokens.h"));
    }
    w.include_local(&format!("{ip}sqlite_keyword.h"));
    w.include_local(&format!("{ip}sqlite_parse.h"));
    w.include_local(&format!("{ip}sqlite_tokenize.h"));
    w.newline();

    if tokens.is_some() {
        for (ctype, sym) in [
            ("const char", "zKWText[]"),
            ("const unsigned short int", "aKWOffset[]"),
            ("const unsigned char", "aKWLen[]"),
            ("const unsigned char", "aKWCode[]"),
            ("const unsigned int", "nKeyword"),
        ] {
            w.line(&format!("extern {ctype} synq_{dialect}_{sym};"));
        }
        w.newline();
    }

    w.section(&format!("{} dialect descriptor", dialect));
    w.newline();
    w.line(&format!(
        "static const SyntaqliteDialect {upper}_DIALECT = {{"
    ));
    w.line(&format!("    .name = \"{dialect}\","));
    w.newline();
    w.line("    .range_meta = range_meta_table,");
    w.line("    .tk_space = SYNTAQLITE_TK_SPACE,");
    w.line("    .tk_semi = SYNTAQLITE_TK_SEMI,");
    w.line("    .tk_comment = SYNTAQLITE_TK_COMMENT,");
    w.newline();
    w.line("    // AST metadata");
    w.line("    .node_count = sizeof(ast_meta_node_names) / sizeof(ast_meta_node_names[0]),");
    w.line("    .node_names = ast_meta_node_names,");
    w.line("    .field_meta = ast_meta_field_meta,");
    w.line("    .field_meta_counts = ast_meta_field_meta_counts,");
    w.line("    .list_tags = ast_meta_list_tags,");
    w.newline();
    w.line("    // Formatter data");
    w.line("    .fmt_strings = fmt_strings,");
    w.line("    .fmt_string_lens = fmt_string_lens,");
    w.line("    .fmt_string_count = sizeof(fmt_strings) / sizeof(fmt_strings[0]),");
    w.line("    .fmt_enum_display = fmt_enum_display,");
    w.line("    .fmt_enum_display_count = sizeof(fmt_enum_display) / sizeof(fmt_enum_display[0]),");
    w.line("    .fmt_ops = fmt_ops,");
    w.line("    .fmt_op_count = sizeof(fmt_ops) / 6,");
    w.line("    .fmt_dispatch = fmt_dispatch,");
    w.line("    .fmt_dispatch_count = sizeof(fmt_dispatch) / sizeof(fmt_dispatch[0]),");
    w.newline();
    let pascal = pascal_case(dialect);
    w.line("    // Parser lifecycle");
    w.line(&format!("    .parser_alloc = Synq{pascal}ParseAlloc,"));
    w.line(&format!("    .parser_init = Synq{pascal}ParseInit,"));
    w.line(&format!(
        "    .parser_finalize = Synq{pascal}ParseFinalize,"
    ));
    w.line(&format!("    .parser_free = Synq{pascal}ParseFree,"));
    w.line(&format!("    .parser_feed = Synq{pascal}Parse,"));
    w.line("#ifndef NDEBUG");
    w.line(&format!("    .parser_trace = Synq{pascal}ParseTrace,"));
    w.line("#endif");
    w.line(&format!(
        "    .parser_expected_tokens = Synq{pascal}ParseExpectedTokens,"
    ));
    w.line(&format!(
        "    .parser_completion_context = Synq{pascal}ParseCompletionContext,"
    ));
    w.newline();
    w.line("    // Tokenizer");
    w.line(&format!("    .get_token = Synq{pascal}GetToken,"));
    w.newline();
    w.line("    // Keyword table");
    if tokens.is_some() {
        w.line(&format!("    .keyword_text = synq_{dialect}_zKWText,"));
        w.line(&format!("    .keyword_offsets = synq_{dialect}_aKWOffset,"));
        w.line(&format!("    .keyword_lens = synq_{dialect}_aKWLen,"));
        w.line(&format!("    .keyword_codes = synq_{dialect}_aKWCode,"));
        w.line(&format!("    .keyword_count = &synq_{dialect}_nKeyword,"));
    } else {
        w.line("    .keyword_text = 0,");
        w.line("    .keyword_offsets = 0,");
        w.line("    .keyword_lens = 0,");
        w.line("    .keyword_codes = 0,");
        w.line("    .keyword_count = 0,");
    }
    w.newline();
    w.line("    // Token metadata");
    if tokens.is_some() {
        w.line("    .token_categories = token_categories,");
        w.line("    .token_type_count = TOKEN_TYPE_COUNT,");
    } else {
        w.line("    .token_categories = 0,");
        w.line("    .token_type_count = 0,");
    }
    w.newline();
    w.line("    // Function extensions (none for base dialect)");
    w.line("    .function_extensions = 0,");
    w.line("    .function_extension_count = 0,");
    w.newline();
    w.line("    // Schema contributions");
    w.line("#ifdef SYNTAQLITE_HAS_SCHEMA_CONTRIBUTIONS");
    w.line("    .schema_contributions = schema_contributions,");
    w.line("    .schema_contribution_count = sizeof(schema_contributions) / sizeof(schema_contributions[0]),");
    w.line("#else");
    w.line("    .schema_contributions = 0,");
    w.line("    .schema_contribution_count = 0,");
    w.line("#endif");
    w.line("};");
    w.newline();

    w.section("Public API");
    w.newline();
    w.line(&format!(
        "const SyntaqliteDialect* syntaqlite_{dialect}_dialect(void) {{"
    ));
    w.line(&format!("    return &{upper}_DIALECT;"));
    w.line("}");

    w.finish()
}

/// Generate the public API header for a dialect.
///
/// `dialect` is a short name like `"sqlite"` or `"perfetto"`.
///
/// The generated header is minimal: it forward-declares `SyntaqliteDialect`
/// and declares the `syntaqlite_<dialect>_dialect()` accessor. Convenience
/// wrappers (e.g. `syntaqlite_create_<dialect>_parser()`) belong in the
/// runtime headers (`parser.h`, `tokenizer.h`), not here.
pub fn generate_dialect_h(dialect: &str) -> String {
    let upper = dialect.to_uppercase();
    let guard = format!("SYNTAQLITE_{upper}_H");
    let mut w = CWriter::new();
    w.file_header();
    w.header_guard_start(&guard);
    w.include_local("syntaqlite/config.h");
    w.newline();
    w.line("#ifdef __cplusplus");
    w.line("extern \"C\" {");
    w.line("#endif");
    w.newline();
    w.line("typedef struct SyntaqliteDialect SyntaqliteDialect;");
    w.newline();
    w.line(&format!(
        "const SyntaqliteDialect* syntaqlite_{dialect}_dialect(void);"
    ));
    w.newline();
    w.line("#ifdef __cplusplus");
    w.line("}");
    w.line("#endif");
    w.newline();
    w.header_guard_end(&guard);

    w.finish()
}

#[cfg(test)]
mod tests {
    use super::{
        DialectCIncludes, generate_dialect_c, generate_dialect_h, generate_token_categories_header,
    };

    const DEFAULT_INCLUDES: DialectCIncludes<'static> = DialectCIncludes {
        internal: "",
        public: "",
        dialect_include_dir: "",
    };

    #[test]
    fn c_source_exposes_dialect_function() {
        let c = generate_dialect_c("sqlite", None, &DEFAULT_INCLUDES);
        assert!(c.contains("const SyntaqliteDialect* syntaqlite_sqlite_dialect(void)"));
        // No default-alias or create-parser wrappers
        assert!(!c.contains("syntaqlite_dialect(void)"));
        assert!(!c.contains("syntaqlite_create_sqlite_parser"));
    }

    #[test]
    fn header_exposes_dialect_function() {
        let h = generate_dialect_h("sqlite");
        assert!(h.contains("const SyntaqliteDialect* syntaqlite_sqlite_dialect(void);"));
        // Forward-declares the opaque type
        assert!(h.contains("typedef struct SyntaqliteDialect SyntaqliteDialect;"));
        // No convenience wrappers — those belong in the runtime headers
        assert!(
            !h.contains("syntaqlite_create_sqlite_parser"),
            "codegen should not emit convenience wrappers"
        );
        assert!(
            !h.contains("SqliteParser"),
            "codegen should not emit C++ convenience"
        );
        assert!(
            !h.contains("SYNTAQLITE_OMIT_SQLITE_API"),
            "codegen should not emit opt-out guards"
        );
        assert!(
            !h.contains("parser.h"),
            "codegen should not include parser.h"
        );
        // No default-alias (that's SQLite-specific, not generic codegen)
        assert!(!h.contains("syntaqlite_dialect(void)"));
    }

    #[test]
    fn token_categories_header_classifies_correctly() {
        use std::collections::HashSet;

        let tokens = vec![
            ("SEMI".to_string(), 0),
            ("SELECT".to_string(), 1),
            ("ID".to_string(), 2),
            ("STRING".to_string(), 3),
            ("INTEGER".to_string(), 4),
            ("PLUS".to_string(), 5),
            ("COMMENT".to_string(), 6),
            ("VARIABLE".to_string(), 7),
            ("FUNCTION".to_string(), 8),
            ("SPACE".to_string(), 9),
        ];
        let kws: HashSet<String> = ["SELECT"].iter().map(|s| s.to_string()).collect();
        let h = generate_token_categories_header(&tokens, Some(&kws));
        assert!(h.contains("#define TOKEN_TYPE_COUNT 10"));
        assert!(h.contains("token_categories[10]"));
        // SEMI=6(punct), SELECT=1(kw), ID=2(ident), STRING=3(str),
        // INTEGER=4(num), PLUS=5(op), COMMENT=7(comment), VARIABLE=8(var),
        // FUNCTION=9(func), SPACE=0(other)
        assert!(h.contains("6,1,2,3,4,5,7,8,9,0,"));
    }

    #[test]
    fn token_categories_keyword_override() {
        use std::collections::HashSet;

        let tokens = vec![
            ("SELECT".to_string(), 0),
            ("FUNCTION".to_string(), 1),
            ("ID".to_string(), 2),
        ];
        // FUNCTION is in the keyword set, so it should be Keyword(1) not Function(9)
        let kws: HashSet<String> = ["SELECT", "FUNCTION"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let h = generate_token_categories_header(&tokens, Some(&kws));
        // SELECT=1(kw), FUNCTION=1(kw, overridden), ID=2(ident)
        assert!(h.contains("1,1,2,"));
    }

    #[test]
    fn id_never_becomes_keyword_even_if_present_in_keyword_set() {
        use std::collections::HashSet;

        let tokens = vec![("ID".to_string(), 0)];
        let kws: HashSet<String> = ["ID"].iter().map(|s| s.to_string()).collect();
        let h = generate_token_categories_header(&tokens, Some(&kws));
        assert!(h.contains("2,"));
    }

    #[test]
    fn punctuation_does_not_become_keyword_even_if_present_in_keyword_set() {
        use std::collections::HashSet;

        let tokens = vec![("DOT".to_string(), 0)];
        let kws: HashSet<String> = ["DOT"].iter().map(|s| s.to_string()).collect();
        let h = generate_token_categories_header(&tokens, Some(&kws));
        assert!(h.contains("6,"));
    }

    #[test]
    fn dialect_c_with_tokens_includes_categories() {
        let tokens = vec![("SELECT".to_string(), 0)];
        let c = generate_dialect_c("sqlite", Some(&tokens), &DEFAULT_INCLUDES);
        assert!(c.contains(".parser_expected_tokens = SynqSqliteParseExpectedTokens,"));
        assert!(c.contains(".keyword_text = synq_sqlite_zKWText,"));
        assert!(c.contains(".keyword_offsets = synq_sqlite_aKWOffset,"));
        assert!(c.contains(".keyword_lens = synq_sqlite_aKWLen,"));
        assert!(c.contains(".keyword_codes = synq_sqlite_aKWCode,"));
        assert!(c.contains(".keyword_count = &synq_sqlite_nKeyword,"));
        assert!(c.contains(".token_categories = token_categories,"));
        assert!(c.contains(".token_type_count = TOKEN_TYPE_COUNT,"));
        assert!(c.contains("dialect_tokens.h"));
    }

    #[test]
    fn dialect_c_without_tokens_uses_null() {
        let c = generate_dialect_c("sqlite", None, &DEFAULT_INCLUDES);
        assert!(c.contains(".keyword_text = 0,"));
        assert!(c.contains(".keyword_offsets = 0,"));
        assert!(c.contains(".keyword_lens = 0,"));
        assert!(c.contains(".keyword_codes = 0,"));
        assert!(c.contains(".keyword_count = 0,"));
        assert!(c.contains(".token_categories = 0,"));
        assert!(c.contains(".token_type_count = 0,"));
        assert!(!c.contains("dialect_tokens.h"));
    }

    /// Tokens not in the keyword set and not explicitly matched should be
    /// Other (0), not Keyword.  This catches dialect-specific internal tokens
    /// (e.g. TRUEFALSE, COLUMN) being falsely classified as keywords.
    #[test]
    fn unknown_tokens_default_to_other_not_keyword() {
        use std::collections::HashSet;

        let tokens = vec![
            ("SELECT".to_string(), 0),
            ("ID".to_string(), 1),
            ("TRUEFALSE".to_string(), 2), // internal token, not a keyword
            ("COLUMN".to_string(), 3),    // internal token, not a keyword
            ("FILTER".to_string(), 4),    // internal token, not a keyword
        ];
        let kws: HashSet<String> = ["SELECT"].iter().map(|s| s.to_string()).collect();
        let h = generate_token_categories_header(&tokens, Some(&kws));
        // SELECT=1(kw), ID=2(ident), TRUEFALSE=0(other), COLUMN=0(other), FILTER=0(other)
        assert!(
            h.contains("1,2,0,0,0,"),
            "unknown tokens should be Other (0), not Keyword (1); got:\n{h}"
        );
    }

    #[test]
    fn dialect_c_includes_respect_prefixes() {
        let includes = DialectCIncludes {
            internal: "csrc/sqlite/",
            public: "",
            dialect_include_dir: "syntaqlite",
        };
        let c = generate_dialect_c("sqlite", None, &includes);
        // Internal headers use the internal prefix
        assert!(c.contains("\"csrc/sqlite/dialect_builder.h\""));
        assert!(c.contains("\"csrc/sqlite/dialect_meta.h\""));
        assert!(c.contains("\"csrc/sqlite/dialect_fmt.h\""));
        // Public headers use the public prefix (empty → bare path)
        assert!(c.contains("\"syntaqlite/parser.h\""));
        assert!(c.contains("\"syntaqlite/dialect.h\""));
    }

    #[test]
    fn dialect_c_default_includes_no_prefix() {
        let c = generate_dialect_c("sqlite", None, &DEFAULT_INCLUDES);
        // Default: both prefixes empty — bare header names
        assert!(c.contains("\"dialect_builder.h\""));
        assert!(c.contains("\"syntaqlite/parser.h\""));
    }
}

/// Generate the dialect dispatch header for amalgamation builds.
///
/// Produces a header like `sqlite_dialect_dispatch.h` that defines the
/// `SYNQ_PARSER_ALLOC`, etc. macros to call the dialect's parser/tokenizer
/// functions directly (bypassing function pointer indirection).
pub fn generate_dialect_dispatch_h(dialect: &str) -> String {
    let upper = dialect.to_uppercase();
    let guard = format!("SYNTAQLITE_{upper}_DIALECT_DISPATCH_H");
    let mut w = CWriter::new();
    w.file_header();
    w.header_guard_start(&guard);
    let pascal = pascal_case(dialect);
    w.line(&format!(
        "#define SYNQ_PARSER_ALLOC(d, m)          Synq{pascal}ParseAlloc(m)"
    ));
    w.line(&format!(
        "#define SYNQ_PARSER_INIT(d, p)           Synq{pascal}ParseInit(p)"
    ));
    w.line(&format!(
        "#define SYNQ_PARSER_FINALIZE(d, p)       Synq{pascal}ParseFinalize(p)"
    ));
    w.line(&format!(
        "#define SYNQ_PARSER_FREE(d, p, f)        Synq{pascal}ParseFree(p, f)"
    ));
    w.line(&format!(
        "#define SYNQ_PARSER_FEED(d, p, t, m, c)  Synq{pascal}Parse(p, t, m, c)"
    ));
    w.line(&format!(
        "#define SYNQ_PARSER_TRACE(d, f, s)       Synq{pascal}ParseTrace(f, s)"
    ));
    w.line(&format!(
        "#define SYNQ_GET_TOKEN(d, cfg, z, t)     Synq{pascal}GetToken(cfg, z, t)"
    ));
    w.newline();
    w.header_guard_end(&guard);
    w.finish()
}

/// Generate forward declarations for the Lemon-generated parser functions.
///
/// Produces `sqlite_parse.h` with declarations for `SynqSqliteParseAlloc`,
/// `SynqSqliteParseFree`, etc.  Needed by the amalgamation so that
/// `dialect.c` (emitted before `sqlite_parse.c`) can reference the symbols.
pub fn generate_parse_h(dialect: &str, includes: &DialectCIncludes<'_>) -> String {
    let pascal = pascal_case(dialect);
    let upper = dialect.to_uppercase();
    let pp = includes.public;
    let guard = format!("SYNTAQLITE_{upper}_PARSE_H");
    let mut w = CWriter::new();
    w.file_header();
    w.header_guard_start(&guard);
    w.line("#include <stddef.h>");
    w.line("#include <stdint.h>");
    w.line("#include <stdio.h>");
    w.newline();
    w.include_local(&format!("{pp}syntaqlite_ext/ast_builder.h"));
    w.newline();
    w.line("#ifdef __cplusplus");
    w.line("extern \"C\" {");
    w.line("#endif");
    w.newline();
    w.line(&format!(
        "void* Synq{pascal}ParseAlloc(void* (*mallocProc)(size_t));"
    ));
    w.line(&format!("void Synq{pascal}ParseInit(void* parser);"));
    w.line(&format!("void Synq{pascal}ParseFinalize(void* parser);"));
    w.line(&format!(
        "void Synq{pascal}ParseFree(void* parser, void (*freeProc)(void*));"
    ));
    w.line(&format!(
        "void Synq{pascal}Parse(void* parser, int token_type, SynqParseToken minor,"
    ));
    w.line(&format!(
        "{}SynqParseCtx* pCtx);",
        " ".repeat(5 + 4 + pascal.len() + 5 + 1)
    ));
    w.line(&format!(
        "int Synq{pascal}ParseExpectedTokens(void* parser, int* out_tokens, int out_cap);"
    ));
    w.line(&format!(
        "uint32_t Synq{pascal}ParseCompletionContext(void* parser);"
    ));
    w.line("#ifndef NDEBUG");
    w.line(&format!(
        "void Synq{pascal}ParseTrace(FILE* trace_file, char* prompt);"
    ));
    w.line("#endif");
    w.newline();
    w.line("#ifdef __cplusplus");
    w.line("}");
    w.line("#endif");
    w.newline();
    w.header_guard_end(&guard);
    w.finish()
}

/// Generate forward declaration for the tokenizer function.
pub fn generate_tokenize_h(dialect: &str, includes: &DialectCIncludes<'_>) -> String {
    let pascal = pascal_case(dialect);
    let upper = dialect.to_uppercase();
    let pp = includes.public;
    let guard = format!("SYNTAQLITE_INTERNAL_{upper}_TOKENIZE_H");
    let mut w = CWriter::new();
    w.file_header();
    w.header_guard_start(&guard);
    w.include_local(&format!("{pp}syntaqlite_ext/sqlite_compat.h"));
    w.include_local(&format!("{pp}syntaqlite/dialect_config.h"));
    w.newline();
    w.line(&format!(
        "i64 Synq{pascal}GetToken(const SyntaqliteDialectConfig* config, const unsigned char* z, int* tokenType);"
    ));
    w.newline();
    w.header_guard_end(&guard);
    w.finish()
}
