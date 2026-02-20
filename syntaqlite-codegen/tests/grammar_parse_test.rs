// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use syntaqlite_codegen::grammar_parser::LemonGrammar;

/// Comprehensive integration test for grammar parsing and validation.
///
/// This test verifies:
/// 1. Base SQLite grammar parses successfully
/// 2. All action files parse successfully
/// 3. Token class declarations match base grammar
/// 4. All action rules match base grammar signatures
/// 5. Combined grammar is valid
///
/// Note: Only runs when full workspace is checked out (local dev/CI).
#[test]
fn test_grammar_integration() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let base_dir = PathBuf::from(manifest_dir);

    let sqlite_parse_y = base_dir.join("../third_party/src/sqlite/src/parse.y");
    let parser_actions_dir = base_dir.join("parser-actions");
    let common_y = base_dir.join("parser-actions/_common.y");

    // Skip if workspace files not available
    if !sqlite_parse_y.exists() || !parser_actions_dir.exists() {
        println!("Skipping: workspace files not available");
        return;
    }

    // Parse base SQLite grammar
    let base_content = fs::read_to_string(&sqlite_parse_y).expect("Failed to read parse.y");
    let base_grammar = LemonGrammar::parse(&base_content)
        .map_err(|e| {
            format!(
                "Base grammar parse error at {}:{}: {}",
                e.line, e.column, e.message
            )
        })
        .expect("Base grammar should parse");

    println!(
        "Base grammar: {} tokens, {} rules, {} token classes",
        base_grammar.tokens.len(),
        base_grammar.rules.len(),
        base_grammar.token_classes.len()
    );

    // Parse all action files
    let mut action_files: Vec<PathBuf> = fs::read_dir(&parser_actions_dir)
        .expect("Failed to read parser-actions")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()?.to_str()? == "y" {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    action_files.sort();

    assert!(!action_files.is_empty(), "Should have action files");

    // Read all action file contents
    let action_contents: Vec<_> = action_files
        .iter()
        .map(|path| {
            let filename = path.file_name().unwrap().to_str().unwrap().to_string();
            let content = fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("Failed to read {}: {}", filename, e));
            (filename, content)
        })
        .collect();

    // Parse all action files
    let action_grammars: Vec<_> = action_contents
        .iter()
        .map(|(filename, content)| {
            let grammar = LemonGrammar::parse(content).unwrap_or_else(|e| {
                panic!(
                    "Failed to parse {} at {}:{}: {}",
                    filename, e.line, e.column, e.message
                )
            });
            (filename.as_str(), grammar)
        })
        .collect();

    // Collect all rules
    let all_action_rules: Vec<_> = action_grammars
        .iter()
        .flat_map(|(filename, grammar)| grammar.rules.iter().map(move |r| (r, *filename)))
        .collect();

    // Combine all contents
    let combined_actions: String = action_contents
        .iter()
        .map(|(_, content)| content.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    println!(
        "Action files: {} files, {} total rules",
        action_files.len(),
        all_action_rules.len()
    );

    // Verify token classes match
    if common_y.exists() {
        let common_content = fs::read_to_string(&common_y).expect("Failed to read _common.y");
        let common_grammar =
            LemonGrammar::parse(&common_content).expect("Failed to parse _common.y");

        let base_classes: HashMap<_, _> = base_grammar
            .token_classes
            .iter()
            .map(|tc| (tc.name, tc.tokens))
            .collect();

        let our_classes: HashMap<_, _> = common_grammar
            .token_classes
            .iter()
            .map(|tc| (tc.name, tc.tokens))
            .collect();

        for (name, tokens) in &our_classes {
            match base_classes.get(name) {
                Some(base_tokens) if base_tokens == tokens => {}
                Some(base_tokens) => {
                    panic!(
                        "Token class '{}' mismatch: base has '{}', ours has '{}'",
                        name, base_tokens, tokens
                    );
                }
                None => {
                    panic!("Token class '{}' not in base grammar", name);
                }
            }
        }

        println!("Token classes: {} match base grammar", our_classes.len());
    }

    // Verify all action rules match base grammar
    let base_rules: HashMap<String, Vec<String>> = {
        let mut map = HashMap::new();
        for rule in &base_grammar.rules {
            map.entry(rule.lhs.to_string())
                .or_insert_with(Vec::new)
                .push(rule.to_string());
        }
        map
    };

    let mut mismatches = Vec::new();
    for (rule, filename) in &all_action_rules {
        let sig = rule.to_string();
        if let Some(base_sigs) = base_rules.get(rule.lhs) {
            if !base_sigs.contains(&sig) {
                mismatches.push(format!("{}: rule not in base: {}", filename, sig));
            }
        } else {
            mismatches.push(format!(
                "{}: non-terminal '{}' not in base",
                filename, rule.lhs
            ));
        }
    }

    if !mismatches.is_empty() {
        eprintln!("Rule mismatches ({}):", mismatches.len());
        for mismatch in mismatches.iter().take(10) {
            eprintln!("  {}", mismatch);
        }
        if mismatches.len() > 10 {
            eprintln!("  ... and {} more", mismatches.len() - 10);
        }
        panic!("{} action rules don't match base grammar", mismatches.len());
    }

    println!(
        "Action rules: all {} match base grammar",
        all_action_rules.len()
    );

    // Verify combined grammar parses
    let combined_grammar =
        LemonGrammar::parse(&combined_actions).expect("Combined action files should parse");

    println!("Combined actions: {} rules", combined_grammar.rules.len());

    // Verify full concatenation parses
    let mut full = base_content.clone();
    full.push_str("\n\n");
    full.push_str(&combined_actions);

    let full_grammar = LemonGrammar::parse(&full).expect("Full combined grammar should parse");

    println!("Full grammar: {} rules", full_grammar.rules.len());

    // Verify %token declarations match
    let base_token_names: Vec<&str> = base_grammar.tokens.iter().map(|t| t.name).collect();
    let action_token_names: Vec<&str> = combined_grammar.tokens.iter().map(|t| t.name).collect();

    if base_token_names != action_token_names {
        let base_set: std::collections::HashSet<&str> = base_token_names.iter().copied().collect();
        let action_set: std::collections::HashSet<&str> =
            action_token_names.iter().copied().collect();
        let missing: Vec<&&str> = base_set.difference(&action_set).collect();
        let extra: Vec<&&str> = action_set.difference(&base_set).collect();
        panic!(
            "Token declarations mismatch.\n  Missing from actions: {:?}\n  Extra in actions: {:?}",
            missing, extra
        );
    }
    println!("Token declarations: {} match", base_token_names.len());

    // Verify %fallback declarations match
    assert_eq!(
        base_grammar.fallbacks.len(),
        combined_grammar.fallbacks.len(),
        "Different number of %fallback declarations: base has {}, actions have {}",
        base_grammar.fallbacks.len(),
        combined_grammar.fallbacks.len()
    );

    for (base_fb, action_fb) in base_grammar
        .fallbacks
        .iter()
        .zip(combined_grammar.fallbacks.iter())
    {
        assert_eq!(
            base_fb.target, action_fb.target,
            "Fallback target mismatch: base has '{}', actions have '{}'",
            base_fb.target, action_fb.target
        );

        let base_fb_set: std::collections::HashSet<&str> = base_fb.tokens.iter().copied().collect();
        let action_fb_set: std::collections::HashSet<&str> =
            action_fb.tokens.iter().copied().collect();

        if base_fb_set != action_fb_set {
            let missing: Vec<&&str> = base_fb_set.difference(&action_fb_set).collect();
            let extra: Vec<&&str> = action_fb_set.difference(&base_fb_set).collect();
            panic!(
                "Fallback '{}' tokens mismatch.\n  Missing from actions: {:?}\n  Extra in actions: {:?}",
                base_fb.target, missing, extra
            );
        }
        println!(
            "Fallback '{}': {} tokens match",
            base_fb.target,
            base_fb.tokens.len()
        );
    }

    // Verify %left/%right/%nonassoc precedence declarations match
    assert_eq!(
        base_grammar.precedences.len(),
        combined_grammar.precedences.len(),
        "Different number of precedence declarations: base has {}, actions have {}",
        base_grammar.precedences.len(),
        combined_grammar.precedences.len()
    );

    for (i, (base_prec, action_prec)) in base_grammar
        .precedences
        .iter()
        .zip(combined_grammar.precedences.iter())
        .enumerate()
    {
        assert_eq!(
            base_prec.assoc, action_prec.assoc,
            "Precedence #{} associativity mismatch: base has '%{}', actions have '%{}'",
            i, base_prec.assoc, action_prec.assoc
        );
        assert_eq!(
            base_prec.tokens,
            action_prec.tokens,
            "Precedence '%{} {}' tokens mismatch:\n  base:    {:?}\n  actions: {:?}",
            base_prec.assoc,
            base_prec.tokens.join(" "),
            base_prec.tokens,
            action_prec.tokens
        );
    }
    println!(
        "Precedence declarations: {} match",
        base_grammar.precedences.len()
    );

    println!("All checks passed");
}
