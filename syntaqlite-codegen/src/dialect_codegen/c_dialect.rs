// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::util::pascal_case;
use crate::writers::c_writer::CWriter;

pub fn generate_dialect_c(dialect: &str) -> String {
    let upper = dialect.to_uppercase();
    let mut w = CWriter::new();
    w.file_header();
    w.include_local("syntaqlite/parser.h");
    w.include_local(&format!("syntaqlite_{dialect}/{dialect}_tokens.h"));
    w.include_local("syntaqlite/dialect.h");
    w.include_local("csrc/dialect_builder.h");
    w.include_local("csrc/dialect_meta.h");
    w.include_local("csrc/dialect_fmt.h");
    w.include_local("csrc/sqlite_parse.h");
    w.include_local("csrc/sqlite_tokenize.h");
    w.newline();

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
    w.newline();
    w.line("    // Tokenizer");
    w.line(&format!("    .get_token = Synq{pascal}GetToken,"));
    w.line("};");
    w.newline();

    w.section("Public API");
    w.newline();
    w.line(&format!(
        "const SyntaqliteDialect* syntaqlite_{dialect}_dialect(void) {{"
    ));
    w.line(&format!("    return &{upper}_DIALECT;"));
    w.line("}");
    w.newline();
    w.line("#ifndef SYNTAQLITE_NO_DEFAULT_DIALECT_SYMBOL");
    w.line("const SyntaqliteDialect* syntaqlite_dialect(void) {");
    w.line(&format!("    return syntaqlite_{dialect}_dialect();"));
    w.line("}");
    w.line("#endif");
    w.newline();
    w.line("#ifndef SYNTAQLITE_NO_DIALECT_CREATE_PARSER_API");
    w.line(&format!(
        "SyntaqliteParser* syntaqlite_create_{dialect}_parser(const SyntaqliteMemMethods* mem) {{"
    ));
    w.line(&format!(
        "    return syntaqlite_create_parser_with_dialect(mem, &{upper}_DIALECT);"
    ));
    w.line("}");
    w.line("#endif");

    w.finish()
}

/// Generate the public API header for a dialect.
///
/// `dialect` is a short name like `"sqlite"` or `"perfetto"`.
pub fn generate_dialect_h(dialect: &str) -> String {
    let upper = dialect.to_uppercase();
    let guard = format!("SYNTAQLITE_{upper}_H");
    let mut w = CWriter::new();
    w.file_header();
    w.line(&format!("#ifndef {guard}"));
    w.line(&format!("#define {guard}"));
    w.newline();
    w.include_local("syntaqlite/config.h");
    w.newline();
    w.line("#ifdef __cplusplus");
    w.line("extern \"C\" {");
    w.line("#endif");
    w.newline();
    w.line("typedef struct SyntaqliteDialect SyntaqliteDialect;");
    w.line("typedef struct SyntaqliteParser SyntaqliteParser;");
    w.newline();
    w.line(&format!(
        "const SyntaqliteDialect* syntaqlite_{dialect}_dialect(void);"
    ));
    w.line("#ifndef SYNTAQLITE_NO_DEFAULT_DIALECT_SYMBOL");
    w.line("const SyntaqliteDialect* syntaqlite_dialect(void);");
    w.line("#endif");
    w.line("#ifndef SYNTAQLITE_NO_DIALECT_CREATE_PARSER_API");
    w.line(&format!(
        "SyntaqliteParser* syntaqlite_create_{dialect}_parser(const SyntaqliteMemMethods* mem);"
    ));
    w.line("#endif");
    w.newline();
    w.line("#ifdef __cplusplus");
    w.line("}");
    w.line("#endif");
    w.newline();
    w.line("#if defined(__cplusplus) && __cplusplus >= 201703L");
    w.include_local("syntaqlite/parser.h");
    w.newline();
    w.line("namespace syntaqlite {");
    w.newline();
    let pascal = pascal_case(dialect);
    w.line("#ifndef SYNTAQLITE_NO_DIALECT_CREATE_PARSER_API");
    w.line(&format!("inline Parser {pascal}Parser() {{"));
    w.line(&format!(
        "  return Parser(syntaqlite_create_{dialect}_parser(nullptr));"
    ));
    w.line("}");
    w.line("#endif");
    w.newline();
    w.line("}  // namespace syntaqlite");
    w.line("#endif");
    w.newline();
    w.line(&format!("#endif  // {guard}"));

    w.finish()
}

#[cfg(test)]
mod tests {
    use super::{generate_dialect_c, generate_dialect_h};

    #[test]
    fn c_source_exposes_default_symbol_guard() {
        let c = generate_dialect_c("sqlite");
        assert!(c.contains("#ifndef SYNTAQLITE_NO_DEFAULT_DIALECT_SYMBOL"));
        assert!(c.contains("const SyntaqliteDialect* syntaqlite_dialect(void)"));
    }

    #[test]
    fn header_exposes_default_symbol_guard() {
        let h = generate_dialect_h("sqlite");
        assert!(h.contains("#ifndef SYNTAQLITE_NO_DEFAULT_DIALECT_SYMBOL"));
        assert!(h.contains("const SyntaqliteDialect* syntaqlite_dialect(void);"));
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
    w.line(&format!("#ifndef {guard}"));
    w.line(&format!("#define {guard}"));
    w.newline();
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
        "#define SYNQ_GET_TOKEN(d, z, t)          Synq{pascal}GetToken(z, t)"
    ));
    w.newline();
    w.line(&format!("#endif  // {guard}"));
    w.finish()
}

/// Generate forward declarations for the Lemon-generated parser functions.
///
/// Produces `sqlite_parse.h` with declarations for `SynqSqliteParseAlloc`,
/// `SynqSqliteParseFree`, etc.  Needed by the amalgamation so that
/// `dialect.c` (emitted before `sqlite_parse.c`) can reference the symbols.
pub fn generate_parse_h(dialect: &str) -> String {
    let pascal = pascal_case(dialect);
    let upper = dialect.to_uppercase();
    let guard = format!("SYNTAQLITE_{upper}_PARSE_H");
    let mut w = CWriter::new();
    w.file_header();
    w.line(&format!("#ifndef {guard}"));
    w.line(&format!("#define {guard}"));
    w.newline();
    w.line("#include <stddef.h>");
    w.line("#include <stdio.h>");
    w.newline();
    w.include_local("syntaqlite_ext/ast_builder.h");
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
    w.line(&format!("#endif  // {guard}"));
    w.finish()
}

/// Generate forward declaration for the tokenizer function.
pub fn generate_tokenize_h(dialect: &str) -> String {
    let pascal = pascal_case(dialect);
    let upper = dialect.to_uppercase();
    let guard = format!("SYNTAQLITE_INTERNAL_{upper}_TOKENIZE_H");
    let mut w = CWriter::new();
    w.file_header();
    w.line(&format!("#ifndef {guard}"));
    w.line(&format!("#define {guard}"));
    w.newline();
    w.include_local("syntaqlite_ext/sqlite_compat.h");
    w.newline();
    w.line(&format!(
        "i64 Synq{pascal}GetToken(const unsigned char* z, int* tokenType);"
    ));
    w.newline();
    w.line(&format!("#endif  // {guard}"));
    w.finish()
}
