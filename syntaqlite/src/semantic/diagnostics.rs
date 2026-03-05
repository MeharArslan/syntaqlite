// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Diagnostic types for semantic analysis.
//!
//! [`Diagnostic`] values carry byte-offset spans, structured messages, and
//! optional "did you mean?" suggestions. Both parse errors and semantic
//! issues (unknown table, wrong arity, etc.) are represented uniformly.

/// A diagnostic message associated with a source range.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Byte offset of the start of the diagnostic range.
    pub start_offset: usize,
    /// Byte offset of the end of the diagnostic range.
    pub end_offset: usize,
    /// Structured diagnostic message.
    pub message: DiagnosticMessage,
    /// Severity level.
    pub severity: Severity,
    /// Optional structured help attached to the diagnostic.
    pub help: Option<Help>,
}

/// Structured diagnostic message.
///
/// Each variant carries the identifiers needed for machine-readable
/// consumption; [`fmt::Display`](std::fmt::Display) produces the human-readable form.
#[derive(Debug, Clone)]
pub enum DiagnosticMessage {
    /// Referenced table name was not found in any catalog layer.
    UnknownTable {
        /// The unresolved table name.
        name: String,
    },
    /// Referenced column name was not found in the current scope.
    UnknownColumn {
        /// The unresolved column name.
        column: String,
        /// Optional table qualifier used by the query.
        table: Option<String>,
    },
    /// Function name was not found in the function catalog.
    UnknownFunction {
        /// The unresolved function name.
        name: String,
    },
    /// Function exists, but the call arity is invalid.
    FunctionArity {
        /// The function name.
        name: String,
        /// Accepted fixed arities.
        expected: Vec<usize>,
        /// Arity supplied by the call.
        got: usize,
    },
    /// Catch-all for parse errors and other unstructured messages.
    Other(String),
}

impl std::fmt::Display for DiagnosticMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTable { name } => write!(f, "unknown table '{name}'"),
            Self::UnknownColumn {
                column,
                table: Some(t),
            } => {
                write!(f, "unknown column '{column}' in table '{t}'")
            }
            Self::UnknownColumn {
                column,
                table: None,
            } => {
                write!(f, "unknown column '{column}'")
            }
            Self::UnknownFunction { name } => write!(f, "unknown function '{name}'"),
            Self::FunctionArity {
                name,
                expected,
                got,
            } => {
                let expected_str: Vec<String> = expected.iter().map(ToString::to_string).collect();
                write!(
                    f,
                    "function '{name}' expects {} argument(s), got {got}",
                    expected_str.join(" or ")
                )
            }
            Self::Other(msg) => f.write_str(msg),
        }
    }
}

impl DiagnosticMessage {
    /// Returns `true` for parse errors (`Other`), `false` for semantic diagnostics.
    pub fn is_parse_error(&self) -> bool {
        matches!(self, Self::Other(_))
    }

    /// Write the structured JSON representation into `out`.
    ///
    /// This is the machine-readable detail object; callers also emit
    /// `"message"` with the [`fmt::Display`](std::fmt::Display) string alongside it.
    #[cfg(feature = "json")]
    pub fn write_json(&self, out: &mut String) {
        out.push_str(&serde_json::to_string(self).expect("DiagnosticMessage serialization failed"));
    }
}

/// Structured help information attached to a diagnostic.
#[derive(Debug, Clone)]
pub enum Help {
    /// A "did you mean?" suggestion with the corrected identifier.
    Suggestion(String),
}

impl std::fmt::Display for Help {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Help::Suggestion(s) => write!(f, "did you mean '{s}'?"),
        }
    }
}

impl Help {
    /// Write the structured JSON representation into `out`.
    #[cfg(feature = "json")]
    pub fn write_json(&self, out: &mut String) {
        out.push_str(&serde_json::to_string(self).expect("Help serialization failed"));
    }
}

impl Diagnostic {
    /// Write the full diagnostic as a JSON object into `out`.
    #[cfg(feature = "json")]
    pub fn write_json(&self, out: &mut String) {
        out.push_str(&serde_json::to_string(self).expect("Diagnostic serialization failed"));
    }
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Blocking issue that should fail validation.
    Error,
    /// Suspicious but non-fatal issue.
    Warning,
    /// Informational diagnostic.
    Info,
    /// Lowest-severity diagnostic, usually a suggestion.
    Hint,
}

// ── JSON serialization (feature = "json") ────────────────────────────

#[cfg(feature = "json")]
impl serde::Serialize for DiagnosticMessage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            Self::Other(_) => serializer.serialize_none(),
            Self::UnknownTable { name } => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("kind", "unknown_table")?;
                m.serialize_entry("name", name)?;
                m.end()
            }
            Self::UnknownColumn { column, table } => {
                let len = if table.is_some() { 3 } else { 2 };
                let mut m = serializer.serialize_map(Some(len))?;
                m.serialize_entry("kind", "unknown_column")?;
                m.serialize_entry("column", column)?;
                if let Some(t) = table {
                    m.serialize_entry("table", t)?;
                }
                m.end()
            }
            Self::UnknownFunction { name } => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("kind", "unknown_function")?;
                m.serialize_entry("name", name)?;
                m.end()
            }
            Self::FunctionArity {
                name,
                expected,
                got,
            } => {
                let mut m = serializer.serialize_map(Some(4))?;
                m.serialize_entry("kind", "function_arity")?;
                m.serialize_entry("name", name)?;
                m.serialize_entry("expected", expected)?;
                m.serialize_entry("got", got)?;
                m.end()
            }
        }
    }
}

#[cfg(feature = "json")]
impl serde::Serialize for Help {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            Self::Suggestion(value) => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("kind", "suggestion")?;
                m.serialize_entry("value", value)?;
                m.end()
            }
        }
    }
}

/// Serializes as a lowercase string (e.g. `"error"`, `"warning"`).
#[cfg(feature = "json")]
impl serde::Serialize for Severity {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Hint => "hint",
        })
    }
}

/// Serializes with a distinct `"message"` (Display) and `"detail"` (structured)
/// field, matching the shape expected by LSP and WASM consumers.
#[cfg(feature = "json")]
impl serde::Serialize for Diagnostic {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let len = if self.help.is_some() { 7 } else { 5 };
        let mut m = serializer.serialize_map(Some(len))?;
        m.serialize_entry("startOffset", &self.start_offset)?;
        m.serialize_entry("endOffset", &self.end_offset)?;
        m.serialize_entry("message", &self.message.to_string())?;
        m.serialize_entry("detail", &self.message)?;
        m.serialize_entry("severity", &self.severity)?;
        if let Some(ref help) = self.help {
            m.serialize_entry("help", &help.to_string())?;
            m.serialize_entry("helpDetail", help)?;
        }
        m.end()
    }
}
