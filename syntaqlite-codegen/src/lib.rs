pub mod ast_codegen;
pub mod c_writer;
pub mod fmt_compiler;
pub mod grammar_parser;
pub mod lemon;
pub mod mkkeyword;
pub mod node_parser;
mod run;

use std::fs;
use std::path::Path;
use syntaqlite_codegen_utils::{c_extractor, c_transformer};

pub struct TokenizerExtractResult {
    pub char_map: String,
    pub upper_to_lower: String,
}

/// Parse `#define SYNTAQLITE_TK_NAME VALUE` lines from lemon's parse.h output.
/// Returns structured `(name, value)` pairs where name is the short name (e.g. "ABORT").
pub fn extract_token_defines(parse_h: &str) -> Vec<(String, u32)> {
    let mut tokens = Vec::new();
    for line in parse_h.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("#define SYNTAQLITE_TK_") {
            let mut parts = rest.split_whitespace();
            if let (Some(name), Some(value_str)) = (parts.next(), parts.next()) {
                if let Ok(value) = value_str.parse::<u32>() {
                    tokens.push((name.to_string(), value));
                }
            }
        }
    }
    tokens
}

// Embed lempar.c template (needed by the library)
const LEMPAR_C: &[u8] = include_bytes!("../sqlite/lempar.c");

pub fn extract_grammar(input_path: &str, output_path: Option<&str>) -> Result<(), String> {
    // Read input file
    let input_text = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read {}: {}", input_path, e))?;

    // Parse grammar
    let grammar = grammar_parser::LemonGrammar::parse(&input_text)
        .map_err(|e| format!("Parse error at {}:{}: {}", e.line, e.column, e.message))?;

    // Generate C header
    let c_code = generate_header(&grammar)?;

    // Write output
    if let Some(output) = output_path {
        fs::write(output, c_code).map_err(|e| format!("Failed to write {}: {}", output, e))?;
    } else {
        print!("{}", c_code);
    }

    Ok(())
}

fn generate_header(grammar: &grammar_parser::LemonGrammar) -> Result<String, String> {
    let mut w = c_writer::CWriter::new();

    // File header
    w.file_header();

    // Header guard
    let guard = "GRAMMAR_TOKENS_H";
    w.header_guard_start(guard);

    // Includes
    w.include_system("stdint.h");
    w.newline();

    w.extern_c_start();

    // Token enum
    w.section("Token Types");
    let mut token_variants: Vec<(&str, Option<i32>)> = vec![("TOKEN_INVALID", Some(0))];
    for (i, token) in grammar.tokens.iter().enumerate() {
        token_variants.push((token.name, Some((i + 1) as i32)));
    }
    w.typedef_enum("TokenType", &token_variants);
    w.newline();

    // Token count constant
    w.comment(&format!("Total number of tokens: {}", grammar.tokens.len()));
    w.line(&format!("#define TOKEN_COUNT {}", grammar.tokens.len() + 1));
    w.newline();

    w.extern_c_end();
    w.newline();

    w.header_guard_end(guard);

    Ok(w.finish())
}

pub fn extract_tokenizer(
    tokenize_c_path: &str,
) -> Result<(String, TokenizerExtractResult), String> {
    let tokenize_content = fs::read_to_string(tokenize_c_path)
        .map_err(|e| format!("Failed to read {}: {}", tokenize_c_path, e))?;
    let tokenize_extractor = c_extractor::CExtractor::new(&tokenize_content);

    let global_c = "third_party/src/sqlite/src/global.c";
    let global_content =
        fs::read_to_string(global_c).map_err(|e| format!("Failed to read {}: {}", global_c, e))?;
    let global_extractor = c_extractor::CExtractor::new(&global_content);

    let sqliteint_h = "third_party/src/sqlite/src/sqliteInt.h";
    let sqliteint_content = fs::read_to_string(sqliteint_h)
        .map_err(|e| format!("Failed to read {}: {}", sqliteint_h, e))?;
    let sqliteint_extractor = c_extractor::CExtractor::new(&sqliteint_content);

    // Extract pieces
    let cc_defines = tokenize_extractor.extract_specific_defines(&[
        "CC_X",
        "CC_KYWD0",
        "CC_KYWD",
        "CC_DIGIT",
        "CC_DOLLAR",
        "CC_VARALPHA",
        "CC_VARNUM",
        "CC_SPACE",
        "CC_QUOTE",
        "CC_QUOTE2",
        "CC_PIPE",
        "CC_MINUS",
        "CC_LT",
        "CC_GT",
        "CC_EQ",
        "CC_BANG",
        "CC_SLASH",
        "CC_LP",
        "CC_RP",
        "CC_SEMI",
        "CC_PLUS",
        "CC_STAR",
        "CC_PERCENT",
        "CC_COMMA",
        "CC_AND",
        "CC_TILDA",
        "CC_DOT",
        "CC_ID",
        "CC_ILLEGAL",
        "CC_NUL",
        "CC_BOM",
    ])?;
    let ai_class = tokenize_extractor.extract_static_array("aiClass")?;
    let ctype_map = global_extractor.extract_static_array("sqlite3CtypeMap")?;
    let upper_to_lower = global_extractor.extract_static_array("sqlite3UpperToLower")?;
    let is_macros = sqliteint_extractor.extract_specific_defines(&[
        "sqlite3Isspace",
        "sqlite3Isdigit",
        "sqlite3Isxdigit",
    ])?;
    let id_char = tokenize_extractor.extract_defines_with_ifdef_context(&["IdChar"])?;
    let char_map = tokenize_extractor.extract_defines_with_ifdef_context(&["charMap"])?;
    let function = tokenize_extractor.extract_function("sqlite3GetToken")?;

    // Combine extracted pieces
    let combined = {
        let mut w = c_writer::CWriter::new();
        w.include_local("csrc/sqlite_compat.h")
            .include_local("syntaqlite/tokens.h")
            .include_local("csrc/sqlite_keyword.h")
            .newline()
            .fragment(&cc_defines)
            .newline()
            .fragment(&ctype_map)
            .newline()
            .fragment(&is_macros)
            .newline()
            .fragment(&id_char)
            .newline()
            .fragment(&ai_class)
            .newline()
            .fragment(&function);
        w.finish()
    };

    // Transform the combined result
    let output = c_transformer::CTransformer::new(&combined)
        .add_array_static("sqlite3CtypeMap")
        .replace_in_function("sqlite3GetToken", "keywordCode", "synq_sqlite3_keywordCode")
        .rename_function("sqlite3GetToken", "synq_sqlite3GetToken")
        .replace_all("TK_", "SYNTAQLITE_TK_")
        .finish();

    Ok((
        output,
        TokenizerExtractResult {
            char_map: char_map.text,
            upper_to_lower: upper_to_lower.text,
        },
    ))
}

pub fn generate_parser(
    actions_dir: &str,
    output_dir: &str,
) -> Result<(), String> {
    let grammar_bytes = concatenate_y_files(actions_dir)?;
    let work_dir = Path::new(output_dir);
    fs::create_dir_all(work_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // Step 1: Write raw grammar to working directory
    let raw_parse_y_path = work_dir.join("parse_raw.y");
    fs::write(&raw_parse_y_path, &grammar_bytes)
        .map_err(|e| format!("Failed to write parse_raw.y: {}", e))?;

    // Step 2: Extract grammar using extract_grammar
    let extracted_grammar_path = work_dir.join("parse_extracted.h");
    let raw_parse_y_str = raw_parse_y_path
        .to_str()
        .ok_or_else(|| "Invalid raw parse.y path".to_string())?;
    let extracted_grammar_str = extracted_grammar_path
        .to_str()
        .ok_or_else(|| "Invalid extracted grammar path".to_string())?;

    extract_grammar(raw_parse_y_str, Some(extracted_grammar_str))?;

    // Step 3: Write lempar.c template to working directory
    let lempar_path = work_dir.join("lempar.c");
    fs::write(&lempar_path, LEMPAR_C).map_err(|e| format!("Failed to write lempar.c: {}", e))?;

    // Step 4: Write the original grammar for lemon processing
    // (lemon needs the full .y file with rules, not just the extracted tokens)
    let parse_y_path = work_dir.join("parse.y");
    fs::write(&parse_y_path, &grammar_bytes)
        .map_err(|e| format!("Failed to write parse.y: {}", e))?;

    // Step 5: Run lemon with -T option pointing to our template
    // Spawn ourselves as a subprocess with the lemon subcommand
    // Note: lemon expects -T and the path combined as a single argument: -T/path/to/template.c
    let parse_y_str = parse_y_path
        .to_str()
        .ok_or_else(|| "Invalid parse.y path".to_string())?;
    let lempar_str = lempar_path
        .to_str()
        .ok_or_else(|| "Invalid lempar.c path".to_string())?;
    let template_arg = format!("-T{}", lempar_str);

    let status = std::process::Command::new(
        std::env::current_exe().map_err(|e| format!("Failed to get current executable: {}", e))?,
    )
    .arg("lemon")
    .arg("-l")
    .arg(&template_arg)
    .arg(parse_y_str)
    .status()
    .map_err(|e| format!("Failed to spawn lemon subprocess: {}", e))?;

    if !status.success() {
        return Err(format!("Lemon failed with exit code: {}", status));
    }

    // Verify outputs were generated
    let parse_c = work_dir.join("parse.c");
    let parse_h = work_dir.join("parse.h");

    if !parse_c.exists() {
        return Err("Lemon did not generate parse.c".to_string());
    }
    if !parse_h.exists() {
        return Err("Lemon did not generate parse.h".to_string());
    }

    Ok(())
}

/// Post-process lemon's parse.c, extracting data tables into a separate header.
///
/// Returns (parse_c_content, parse_data_h_content).
pub fn split_parse_c(parse_c: &str) -> Result<(String, String), String> {
    let extractor = c_extractor::CExtractor::new(parse_c);

    // Extract sections
    let control_defines = extractor.extract_between_markers(
        "Begin control #defines",
        "End control #defines",
    )?;
    let parsing_tables_raw = extractor.extract_between_markers(
        "Begin parsing tables",
        "End of lemon-generated parsing tables",
    )?;
    let yytestcase = extractor.extract_enclosing_ifdef("# define yytestcase")?;
    let parser_structs = extractor.extract_between_markers(
        "struct yyStackEntry",
        "typedef struct yyParser yyParser;",
    )?;
    let yyfallback = extractor.extract_enclosing_ifdef("yyFallback")?;
    let token_names = extractor.extract_enclosing_ifdef("yyTokenName")?;
    let rule_names = extractor.extract_enclosing_ifdef("yyRuleName")?;
    let rule_info_lhs = extractor.extract_static_array("yyRuleInfoLhs")?;
    let rule_info_nrhs = extractor.extract_static_array("yyRuleInfoNRhs")?;
    let reduce_actions = extractor.extract_between_markers(
        "Begin reduce actions",
        "End reduce actions",
    )?;

    // Parse control #define values for the SynqParseTables initializer.
    let defines = parse_control_defines(&control_defines);

    // Remove all extracted sections from main engine code and apply transforms.
    let main = c_transformer::CTransformer::new(parse_c)
        .remove_block_comment_containing("automatically generated by Lemon")
        .remove_text(&control_defines)
        .remove_text(&yytestcase)
        .remove_text(&parsing_tables_raw)
        .remove_text(&parser_structs)
        .remove_text(&yyfallback)
        .remove_text(&token_names)
        .remove_text(&rule_names)
        .remove_text(&rule_info_lhs.text)
        .remove_text(&rule_info_nrhs.text)
        .replace_all(
            &reduce_actions,
            "      default: yy_reduce_actions(yypParser, yyruleno, yymsp, yyLookahead, yyLookaheadToken); break;",
        )
        // Replace the data header include with a conditional include.
        .replace_all(
            "#include \"csrc/sqlite_parse_data.h\"",
            concat!(
                "#ifdef SYNTAQLITE_INLINE_PARSER_DATA_HEADER\n",
                "  #include SYNTAQLITE_INLINE_PARSER_DATA_HEADER\n",
                "#else\n",
                "  #include \"csrc/synq_lemon_generic.h\"\n",
                "#endif",
            ),
        )
        // Replace sizeof patterns with named macros.
        .replace_all(
            "#define YY_NLOOKAHEAD ((int)(sizeof(yy_lookahead)/sizeof(yy_lookahead[0])))",
            "/* YY_NLOOKAHEAD defined in data header */",
        )
        .replace_all(
            "sizeof(yyFallback)/sizeof(yyFallback[0])",
            "YY_NFALLBACK",
        )
        .replace_all(
            "(int)(sizeof(yy_action)/sizeof(yy_action[0]))",
            "YY_NACTION",
        )
        .replace_all(
            "sizeof(yy_action)/sizeof(yy_action[0])",
            "YY_NACTION",
        )
        .replace_all(
            "sizeof(yyRuleInfoLhs)/sizeof(yyRuleInfoLhs[0])",
            "YY_NRULELHS",
        )
        .replace_all(
            "(int)(sizeof(yyRuleName)/sizeof(yyRuleName[0]))",
            "YY_NRULENAME",
        )
        .replace_all(
            "sizeof(yyRuleName)/sizeof(yyRuleName[0])",
            "YY_NRULENAME",
        )
        .finish();

    // Strip the marker comment from parsing tables (keep only the data)
    let parsing_tables = parsing_tables_raw
        .lines()
        .filter(|l| !l.contains("Begin parsing tables"))
        .collect::<Vec<_>>()
        .join("\n");

    // Wrap reduce actions in a function.
    // Uses SyntaqliteParse-prefixed macros (from %name SyntaqliteParse).
    let reduce_actions_fn = [
        "static void yy_reduce_actions(",
        "  yyParser *yypParser,",
        "  unsigned int yyruleno,",
        "  yyStackEntry *yymsp,",
        "  int yyLookahead,",
        "  SyntaqliteParseTOKENTYPE yyLookaheadToken",
        "){",
        "  SyntaqliteParseARG_FETCH",
        "  (void)yyLookahead;",
        "  (void)yyLookaheadToken;",
        "  switch( yyruleno ){",
        &reduce_actions,
        "  };",
        "}",
    ]
    .join("\n");

    // Build size macros for both inline and generic paths.
    let size_macros = build_size_macros(&defines, &parsing_tables);

    // Build SynqParseTables initializer.
    let tables_initializer = build_tables_initializer(&defines);

    // Build header from extracted sections
    let mut w = c_writer::CWriter::new();
    w.file_header();
    w.include_local("csrc/ast_builder.h");
    w.include_local("csrc/parser.h");
    w.include_local("csrc/grammar_types.h");
    w.include_local("syntaqlite/tokens.h");
    w.newline();
    for section in [
        &control_defines,
        &yytestcase,
        &parser_structs,
        &parsing_tables,
        &yyfallback,
        &token_names,
        &rule_names,
        &rule_info_lhs.text,
        &rule_info_nrhs.text,
        &reduce_actions_fn,
    ] {
        w.fragment(section);
        w.newline();
    }

    // Append size macros and SynqParseTables initializer.
    w.fragment(&size_macros);
    w.newline();
    w.fragment(&tables_initializer);
    w.newline();

    Ok((main, w.finish()))
}

/// Parse `#define NAME value` lines from the control defines section.
fn parse_control_defines(text: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("#define ") {
            let mut parts = rest.splitn(2, char::is_whitespace);
            if let (Some(name), Some(value)) = (parts.next(), parts.next()) {
                let value = value.trim();
                // Only capture simple numeric values and short identifiers.
                if !value.is_empty() && !value.contains('{') {
                    map.insert(name.to_string(), value.to_string());
                }
            }
        }
    }
    map
}

/// Build `#define` macros for array sizes used by the engine.
fn build_size_macros(
    defines: &std::collections::HashMap<String, String>,
    parsing_tables: &str,
) -> String {
    // Extract YY_ACTTAB_COUNT from the parsing tables section.
    let acttab_count = parsing_tables
        .lines()
        .find_map(|l| {
            let l = l.trim();
            l.strip_prefix("#define YY_ACTTAB_COUNT")
                .map(|v| v.trim().trim_matches(|c| c == '(' || c == ')').trim().to_string())
        })
        .unwrap_or_else(|| "0".to_string());

    let ntoken = defines.get("YYNTOKEN").map(|s| s.as_str()).unwrap_or("0");

    // Count fallback entries: YYNTOKEN entries when YYFALLBACK is defined.
    let n_fallback = if defines.contains_key("YYFALLBACK") {
        ntoken.to_string()
    } else {
        "0".to_string()
    };

    // YY_NLOOKAHEAD = acttab_count + ntoken + 1 (standard Lemon formula).
    // But the actual array size in bytes is what sizeof gives.
    // For the inline path we use sizeof; the values here are informational.
    let mut s = String::new();
    s.push_str("/* Size macros for engine code (inline path uses sizeof, generic uses these) */\n");
    s.push_str(&format!(
        "#define YY_NLOOKAHEAD ((int)(sizeof(yy_lookahead)/sizeof(yy_lookahead[0])))\n"
    ));
    s.push_str(&format!(
        "#define YY_NACTION ((int)(sizeof(yy_action)/sizeof(yy_action[0])))\n"
    ));
    s.push_str(&format!(
        "#define YY_NFALLBACK ((int)(sizeof(yyFallback)/sizeof(yyFallback[0])))\n"
    ));
    s.push_str(&format!(
        "#define YY_NRULELHS ((int)(sizeof(yyRuleInfoLhs)/sizeof(yyRuleInfoLhs[0])))\n"
    ));
    s.push_str("#ifdef NDEBUG\n");
    s.push_str("#define YY_NRULENAME 0\n");
    s.push_str("#else\n");
    s.push_str(&format!(
        "#define YY_NRULENAME ((int)(sizeof(yyRuleName)/sizeof(yyRuleName[0])))\n"
    ));
    s.push_str("#endif\n");

    // Store numeric values for SynqParseTables fields.
    s.push_str(&format!("\n#define YY_N_ACTION_VAL {}\n", acttab_count));
    s.push_str(&format!("#define YY_N_FALLBACK_VAL {}\n", n_fallback));

    s
}

/// Build a `static const SynqParseTables SQLITE_PARSE_TABLES = { ... }` initializer.
fn build_tables_initializer(defines: &std::collections::HashMap<String, String>) -> String {
    let get = |key: &str| -> &str {
        defines.get(key).map(|s| s.as_str()).unwrap_or("0")
    };

    let mut s = String::new();
    s.push_str("#include \"syntaqlite/dialect.h\"\n\n");
    s.push_str("static const SynqParseTables SQLITE_PARSE_TABLES = {\n");
    s.push_str("    .yy_action = yy_action,\n");
    s.push_str("    .yy_lookahead = yy_lookahead,\n");
    s.push_str("    .yy_shift_ofst = yy_shift_ofst,\n");
    s.push_str("    .yy_reduce_ofst = yy_reduce_ofst,\n");
    s.push_str("    .yy_default = yy_default,\n");
    s.push_str("#ifdef YYFALLBACK\n");
    s.push_str("    .yy_fallback = yyFallback,\n");
    s.push_str("#else\n");
    s.push_str("    .yy_fallback = NULL,\n");
    s.push_str("#endif\n");
    s.push_str("    .yy_rule_lhs = yyRuleInfoLhs,\n");
    s.push_str("    .yy_rule_nrhs = yyRuleInfoNRhs,\n");
    s.push_str("\n");
    s.push_str("    .n_action = YY_N_ACTION_VAL,\n");
    s.push_str("    .n_lookahead = YY_NLOOKAHEAD,\n");
    s.push_str("    .n_fallback = YY_N_FALLBACK_VAL,\n");
    s.push_str("\n");
    s.push_str(&format!("    .nocode = {},\n", get("YYNOCODE")));
    s.push_str(&format!("    .wildcard = {},\n", get("YYWILDCARD")));
    s.push_str(&format!("    .nstate = {},\n", get("YYNSTATE")));
    s.push_str(&format!("    .nrule = {},\n", get("YYNRULE")));
    s.push_str(&format!("    .nrule_with_action = {},\n", get("YYNRULE_WITH_ACTION")));
    s.push_str(&format!("    .ntoken = {},\n", get("YYNTOKEN")));
    s.push_str(&format!("    .max_shift = {},\n", get("YY_MAX_SHIFT")));
    s.push_str(&format!("    .min_shiftreduce = {},\n", get("YY_MIN_SHIFTREDUCE")));
    s.push_str(&format!("    .max_shiftreduce = {},\n", get("YY_MAX_SHIFTREDUCE")));
    s.push_str(&format!("    .error_action = {},\n", get("YY_ERROR_ACTION")));
    s.push_str(&format!("    .accept_action = {},\n", get("YY_ACCEPT_ACTION")));
    s.push_str(&format!("    .no_action = {},\n", get("YY_NO_ACTION")));
    s.push_str(&format!("    .min_reduce = {},\n", get("YY_MIN_REDUCE")));
    s.push_str(&format!("    .max_reduce = {},\n", get("YY_MAX_REDUCE")));
    s.push_str("    .acttab_count = YY_N_ACTION_VAL,\n");
    // shift_count and reduce_count come from the parsing tables section, not control defines.
    // We use the #defines that are in the parsing tables area.
    s.push_str("    .shift_count = YY_SHIFT_COUNT,\n");
    s.push_str("    .reduce_count = YY_REDUCE_COUNT,\n");
    s.push_str("\n");
    s.push_str("#ifndef NDEBUG\n");
    s.push_str("    .token_names = yyTokenName,\n");
    s.push_str("    .rule_names = yyRuleName,\n");
    s.push_str("#else\n");
    s.push_str("    .token_names = NULL,\n");
    s.push_str("    .rule_names = NULL,\n");
    s.push_str("#endif\n");
    s.push_str("};\n");

    s
}

/// Generate keyword hash lookup table
///
/// Returns (tables_header_content, keyword_function_content).
pub fn generate_keyword_hash(
    extract_result: &TokenizerExtractResult,
) -> Result<(String, String), String> {
    // Run mkkeywordhash as a subprocess and capture its output
    let output = std::process::Command::new(
        std::env::current_exe().map_err(|e| format!("Failed to get current executable: {}", e))?,
    )
    .arg("mkkeyword")
    .output()
    .map_err(|e| format!("Failed to spawn mkkeyword subprocess: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "mkkeyword failed with exit code: {}",
            output.status
        ));
    }

    // Convert output to string for processing
    let generated_code = String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 in mkkeyword output: {}", e))?;

    // Remove unwanted SQLite-specific wrapper functions using CTransformer
    let processed_code = c_transformer::CTransformer::new(&generated_code)
        .remove_function("sqlite3KeywordCode")
        .remove_function("sqlite3_keyword_name")
        .remove_function("sqlite3_keyword_count")
        .remove_function("sqlite3_keyword_check")
        .remove_lines_matching("#define SQLITE_N_KEYWORD")
        .rename_function("keywordCode", "synq_sqlite3_keywordCode")
        .remove_static("synq_sqlite3_keywordCode")
        .replace_all("TK_", "SYNTAQLITE_TK_")
        .finish();

    // Split the processed code into tables and function
    let (tables_content, function_content) = split_keyword_code(&processed_code, extract_result)?;

    Ok((tables_content, function_content))
}

/// Split keyword code into tables header and function implementation
fn split_keyword_code(
    code: &str,
    extract_result: &TokenizerExtractResult,
) -> Result<(String, String), String> {
    // Use CExtractor to split by the function
    let extractor = c_extractor::CExtractor::new(code);
    let split = extractor.split_by_function("synq_sqlite3_keywordCode")?;

    let mut tables = c_writer::CWriter::new();
    tables.line("#ifndef SQLITE_KEYWORD_TABLES_H");
    tables.line("#define SQLITE_KEYWORD_TABLES_H");
    tables.newline();
    tables.include_local("syntaqlite/tokens.h");
    tables.newline();
    tables.fragment(&split.before);
    tables.newline();
    tables.line("#endif /* SQLITE_KEYWORD_TABLES_H */");

    // Build the function C file
    let mut func = c_writer::CWriter::new();
    func.include_local("csrc/sqlite_compat.h");
    func.include_local("csrc/sqlite_keyword_tables.h");
    func.newline();

    // Add char mapping arrays (upper_to_lower and char_map)
    func.fragment(&extract_result.upper_to_lower);
    func.newline();
    func.fragment(&extract_result.char_map);
    func.newline();

    // Add the function
    func.fragment(&split.function.text);
    func.newline();

    // Add anything after the function (if any)
    if !split.after.trim().is_empty() {
        func.fragment(&split.after);
        func.newline();
    }

    Ok((tables.finish(), func.finish()))
}

/// Read all .y files from a directory, sort by name, and concatenate their contents.
fn concatenate_y_files(dir: &str) -> Result<Vec<u8>, String> {
    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        return Err(format!("{} is not a directory", dir));
    }

    let mut y_files: Vec<_> = fs::read_dir(dir_path)
        .map_err(|e| format!("Failed to read directory {}: {}", dir, e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("y") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    y_files.sort();

    if y_files.is_empty() {
        return Err(format!("No .y files found in {}", dir));
    }

    let mut combined = Vec::new();
    for path in &y_files {
        let content =
            fs::read(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        combined.extend_from_slice(&content);
        combined.push(b'\n');
    }

    Ok(combined)
}
