// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Multi-version SQLite source analysis.
//!
//! Downloads are handled externally (bash script). This module:
//! 1. Reads pre-downloaded SQLite source trees
//! 2. Extracts code fragments using `CExtractor`
//! 3. Hashes fragments to find distinct variants
//! 4. Groups consecutive versions with identical hashes
//! 5. Produces unified diffs between consecutive variants

mod diff;
mod extract;
pub mod grammar;
mod hash;
mod keywords;

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;

use serde::Serialize;

pub use diff::VariantDiff;
pub use extract::ExtractedFragments;
pub use grammar::GrammarAnalysis;
pub use keywords::{KeywordEntry, KeywordTable, MaskDefine};

/// A parsed SQLite version number (e.g. 3.35.0 or 3.8.11.1).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(into = "String")]
pub struct SqliteVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub sub_patch: u32,
}

impl SqliteVersion {
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 3 || parts.len() > 4 {
            return Err(format!("invalid version: {s} (expected X.Y.Z or X.Y.Z.W)"));
        }
        Ok(Self {
            major: parts[0]
                .parse()
                .map_err(|_| format!("invalid major: {}", parts[0]))?,
            minor: parts[1]
                .parse()
                .map_err(|_| format!("invalid minor: {}", parts[1]))?,
            patch: parts[2]
                .parse()
                .map_err(|_| format!("invalid patch: {}", parts[2]))?,
            sub_patch: if parts.len() == 4 {
                parts[3]
                    .parse()
                    .map_err(|_| format!("invalid sub_patch: {}", parts[3]))?
            } else {
                0
            },
        })
    }

    /// SQLite version integer encoding: 3.X.Y.Z -> 3XXYYZZ, 3.X.Y -> 3XXYY00.
    pub fn version_int(&self) -> u32 {
        self.major * 1_000_000 + self.minor * 10_000 + self.patch * 100 + self.sub_patch
    }
}

impl From<SqliteVersion> for String {
    fn from(v: SqliteVersion) -> Self {
        v.to_string()
    }
}

impl fmt::Display for SqliteVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.sub_patch > 0 {
            write!(
                f,
                "{}.{}.{}.{}",
                self.major, self.minor, self.patch, self.sub_patch
            )
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

/// A group of consecutive versions that share an identical fragment.
#[derive(Debug, Clone, Serialize)]
pub struct VariantGroup {
    pub id: String,
    pub hash: String,
    pub versions: Vec<SqliteVersion>,
    #[serde(skip)]
    pub text: String,
}

impl VariantGroup {
    pub fn first(&self) -> &SqliteVersion {
        self.versions.first().unwrap()
    }

    pub fn last(&self) -> &SqliteVersion {
        self.versions.last().unwrap()
    }
}

/// Analysis of a single fragment across all versions.
#[derive(Debug, Clone, Serialize)]
pub struct FragmentAnalysis {
    #[serde(skip)]
    pub fragment_name: String,
    pub variants: Vec<VariantGroup>,
    pub diffs: Vec<VariantDiff>,
    pub errors: Vec<(SqliteVersion, String)>,
}

/// Keyword changes between consecutive versions.
#[derive(Debug, Clone, Serialize)]
pub struct KeywordAddition {
    pub version: SqliteVersion,
    pub added: Vec<String>,
}

/// Analysis of keyword table changes across versions.
#[derive(Debug, Clone, Serialize)]
pub struct KeywordAnalysis {
    pub total_keywords_latest: usize,
    pub additions: Vec<KeywordAddition>,
    #[serde(skip)]
    pub per_version: Vec<(SqliteVersion, KeywordTable)>,
}

/// Complete analysis result.
#[derive(Debug, Serialize)]
pub struct VersionAnalysis {
    pub versions: Vec<SqliteVersion>,
    pub fragments: BTreeMap<String, FragmentAnalysis>,
    pub keywords: KeywordAnalysis,
    pub grammar: Option<GrammarAnalysis>,
}

/// Required source files for a single SQLite version.
struct VersionSources {
    tokenize_c: String,
    global_c: String,
    sqliteint_h: String,
    mkkeywordhash_c: String,
    parse_y: Option<String>,
}

/// Run the full analysis pipeline on a directory of pre-downloaded SQLite sources.
///
/// Expected directory layout:
/// ```text
/// sqlite_source_dir/
///   3.24.0/src/tokenize.c
///   3.24.0/src/global.c
///   3.24.0/src/sqliteInt.h
///   3.24.0/tool/mkkeywordhash.c
///   3.25.0/src/...
/// ```
pub fn analyze_versions(
    sqlite_source_dir: &Path,
    output_dir: &Path,
) -> Result<VersionAnalysis, String> {
    let versions = discover_versions(sqlite_source_dir)?;
    if versions.is_empty() {
        return Err(format!(
            "no version directories found in {}",
            sqlite_source_dir.display()
        ));
    }

    eprintln!("Found {} versions: {}", versions.len(), {
        let first = &versions[0];
        let last = versions.last().unwrap();
        if versions.len() > 2 {
            format!("{first} .. {last}")
        } else {
            versions
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        }
    });

    // Extract fragments from each version.
    let fragment_names = extract::FRAGMENT_NAMES;
    let mut per_version_fragments: Vec<(SqliteVersion, Result<ExtractedFragments, String>)> =
        Vec::new();
    let mut per_version_keywords: Vec<(SqliteVersion, Result<KeywordTable, String>)> = Vec::new();
    let mut per_version_grammar: Vec<(SqliteVersion, Option<String>)> = Vec::new();

    for version in &versions {
        let sources = load_version_sources(sqlite_source_dir, version)?;

        eprintln!("Extracting fragments from {version}...");
        let fragments = extract::extract_fragments(
            &sources.tokenize_c,
            &sources.global_c,
            &sources.sqliteint_h,
        );
        per_version_fragments.push((version.clone(), fragments));

        let kw = keywords::parse_keyword_table(&sources.mkkeywordhash_c);
        per_version_keywords.push((version.clone(), kw));

        per_version_grammar.push((version.clone(), sources.parse_y));
    }

    // Analyze each fragment: hash, group, diff.
    let mut all_fragments = BTreeMap::new();
    for name in fragment_names {
        let texts: Vec<(SqliteVersion, Result<String, String>)> = per_version_fragments
            .iter()
            .map(|(v, res)| {
                let text = match res {
                    Ok(f) => f.get(name).map(|s| s.to_string()),
                    Err(e) => Err(e.clone()),
                };
                (v.clone(), text)
            })
            .collect();

        let analysis = analyze_fragment(name, &texts);

        // Write variant files.
        if !analysis.variants.is_empty() {
            write_variant_files(output_dir, name, &analysis.variants)?;
        }

        all_fragments.insert(name.to_string(), analysis);
    }

    // Analyze keywords.
    let kw_analysis = analyze_keywords(&per_version_keywords);

    // Write keyword files.
    write_keyword_files(output_dir, &per_version_keywords)?;

    // Analyze grammar (parse.y).
    let grammar_analysis = analyze_grammar(&per_version_grammar);

    Ok(VersionAnalysis {
        versions,
        fragments: all_fragments,
        keywords: kw_analysis,
        grammar: grammar_analysis,
    })
}

fn discover_versions(dir: &Path) -> Result<Vec<SqliteVersion>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    let mut versions = Vec::new();
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            let name = entry.file_name();
            if let Ok(v) = SqliteVersion::parse(name.to_string_lossy().as_ref()) {
                versions.push(v);
            }
        }
    }
    versions.sort();
    Ok(versions)
}

fn load_version_sources(base: &Path, version: &SqliteVersion) -> Result<VersionSources, String> {
    let dir = base.join(version.to_string());
    let read = |rel: &str| -> Result<String, String> {
        let path = dir.join(rel);
        fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
    };
    let read_optional = |rel: &str| -> Option<String> {
        let path = dir.join(rel);
        fs::read_to_string(&path).ok()
    };
    Ok(VersionSources {
        tokenize_c: read("src/tokenize.c")?,
        global_c: read("src/global.c")?,
        sqliteint_h: read("src/sqliteInt.h")?,
        mkkeywordhash_c: read("tool/mkkeywordhash.c")?,
        parse_y: read_optional("src/parse.y"),
    })
}

fn analyze_fragment(
    name: &str,
    texts: &[(SqliteVersion, Result<String, String>)],
) -> FragmentAnalysis {
    let mut errors = Vec::new();
    let mut successful: Vec<(SqliteVersion, String, String)> = Vec::new(); // (version, hash, text)

    for (version, result) in texts {
        match result {
            Ok(text) => {
                let h = hash::normalized_hash(text);
                successful.push((version.clone(), h, text.clone()));
            }
            Err(e) => {
                errors.push((version.clone(), e.clone()));
            }
        }
    }

    let variants = diff::group_variants(&successful);
    let diffs = diff::compute_diffs(&variants);

    FragmentAnalysis {
        fragment_name: name.to_string(),
        variants,
        diffs,
        errors,
    }
}

fn analyze_keywords(
    per_version: &[(SqliteVersion, Result<KeywordTable, String>)],
) -> KeywordAnalysis {
    let mut additions = Vec::new();
    let mut prev_names: Option<std::collections::BTreeSet<String>> = None;
    let mut valid_tables = Vec::new();

    for (version, result) in per_version {
        if let Ok(table) = result {
            let names: std::collections::BTreeSet<String> =
                table.keywords.iter().map(|k| k.name.clone()).collect();
            if let Some(prev) = &prev_names {
                let added: Vec<String> = names.difference(prev).cloned().collect();
                if !added.is_empty() {
                    additions.push(KeywordAddition {
                        version: version.clone(),
                        added,
                    });
                }
            }
            prev_names = Some(names);
            valid_tables.push((version.clone(), table.clone()));
        }
    }

    let total = valid_tables
        .last()
        .map(|(_, t)| t.keywords.len())
        .unwrap_or(0);

    KeywordAnalysis {
        total_keywords_latest: total,
        additions,
        per_version: valid_tables,
    }
}

fn analyze_grammar(per_version: &[(SqliteVersion, Option<String>)]) -> Option<GrammarAnalysis> {
    let mut summaries = Vec::new();
    let mut errors = Vec::new();
    let mut any_found = false;

    for (version, parse_y) in per_version {
        let Some(source) = parse_y else {
            continue;
        };
        any_found = true;

        eprintln!("Parsing grammar for {version}...");
        match grammar::extract_grammar_summary(source) {
            Ok(summary) => summaries.push((version.clone(), summary)),
            Err(e) => errors.push((version.clone(), e)),
        }
    }

    if !any_found {
        eprintln!("No parse.y files found; skipping grammar analysis.");
        return None;
    }

    let diffs = grammar::compute_grammar_diffs(&summaries);

    Some(GrammarAnalysis {
        per_version: summaries,
        diffs,
        errors,
    })
}

fn write_variant_files(
    output_dir: &Path,
    fragment_name: &str,
    variants: &[VariantGroup],
) -> Result<(), String> {
    let dir = output_dir.join("variants");
    fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;

    if variants.len() == 1 {
        let path = dir.join(format!("{fragment_name}.c"));
        fs::write(&path, &variants[0].text)
            .map_err(|e| format!("write {}: {e}", path.display()))?;
    } else {
        for v in variants {
            let path = dir.join(format!("{fragment_name}_{}.c", v.id));
            fs::write(&path, &v.text).map_err(|e| format!("write {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

fn write_keyword_files(
    output_dir: &Path,
    per_version: &[(SqliteVersion, Result<KeywordTable, String>)],
) -> Result<(), String> {
    let dir = output_dir.join("variants").join("keywords");
    fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;

    for (version, result) in per_version {
        if let Ok(table) = result {
            let ver_str = version.to_string().replace('.', "_");
            let path = dir.join(format!("keywords_{ver_str}"));
            let mut content = String::new();
            for kw in &table.keywords {
                content.push_str(&format!(
                    "{}\t{}\t{}\t{}\n",
                    kw.name, kw.token, kw.mask_expr, kw.priority
                ));
            }
            fs::write(&path, &content).map_err(|e| format!("write {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version() {
        let v = SqliteVersion::parse("3.35.0").unwrap();
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 35);
        assert_eq!(v.patch, 0);
        assert_eq!(v.sub_patch, 0);
        assert_eq!(v.version_int(), 3_350_000);
        assert_eq!(v.to_string(), "3.35.0");
    }

    #[test]
    fn parse_version_with_patch() {
        let v = SqliteVersion::parse("3.51.2").unwrap();
        assert_eq!(v.version_int(), 3_510_200);
    }

    #[test]
    fn parse_version_four_part() {
        let v = SqliteVersion::parse("3.8.11.1").unwrap();
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 8);
        assert_eq!(v.patch, 11);
        assert_eq!(v.sub_patch, 1);
        assert_eq!(v.version_int(), 3_081_101);
        assert_eq!(v.to_string(), "3.8.11.1");
    }

    #[test]
    fn version_ordering() {
        let a = SqliteVersion::parse("3.24.0").unwrap();
        let b = SqliteVersion::parse("3.35.0").unwrap();
        let c = SqliteVersion::parse("3.35.1").unwrap();
        assert!(a < b);
        assert!(b < c);
    }

    #[test]
    fn version_ordering_four_part() {
        let a = SqliteVersion::parse("3.8.11").unwrap();
        let b = SqliteVersion::parse("3.8.11.1").unwrap();
        let c = SqliteVersion::parse("3.9.0").unwrap();
        assert!(a < b);
        assert!(b < c);
    }
}
