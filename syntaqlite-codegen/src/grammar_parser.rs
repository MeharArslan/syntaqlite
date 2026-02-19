// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Lemon grammar file parser
//!
//! **NOTE:** This is a minimal, relaxed parser designed specifically to extract
//! tokens and rules from SQLite's parse.y grammar. It is NOT a general-purpose
//! Lemon parser and makes no attempt to validate grammar correctness. It simply
//! extracts the information we need while skipping everything else.

use std::fmt;

// ============================================================================
// Core Types
// ============================================================================

#[derive(Debug, Clone)]
pub struct LemonGrammar<'a> {
    pub tokens: Vec<TokenDecl<'a>>,
    pub rules: Vec<GrammarRule<'a>>,
    pub token_classes: Vec<TokenClass<'a>>,
    pub fallbacks: Vec<FallbackDecl<'a>>,
    pub precedences: Vec<PrecedenceDecl<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecedenceDecl<'a> {
    pub assoc: &'a str, // "left", "right", or "nonassoc"
    pub tokens: Vec<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallbackDecl<'a> {
    pub target: &'a str,
    pub tokens: Vec<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenDecl<'a> {
    pub name: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenClass<'a> {
    pub name: &'a str,
    pub tokens: &'a str, // Raw token list like "ID|STRING"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarRule<'a> {
    pub lhs: &'a str,
    pub rhs: Vec<RhsSymbol<'a>>,
    pub precedence_override: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RhsSymbol<'a> {
    pub name: &'a str,
    pub alias: Option<&'a str>,
}

// ============================================================================
// Display Implementations
// ============================================================================

impl<'a> fmt::Display for GrammarRule<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rhs = self
            .rhs
            .iter()
            .map(|sym| sym.name)
            .collect::<Vec<_>>()
            .join(" ");

        match self.precedence_override {
            Some(prec) => write!(f, "{} ::= {} [{}]", self.lhs, rhs, prec),
            None => write!(f, "{} ::= {}", self.lhs, rhs),
        }
    }
}

impl<'a> fmt::Display for TokenClass<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%token_class {}  {}", self.name, self.tokens)
    }
}

impl<'a> fmt::Display for TokenDecl<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// ============================================================================
// Error Handling
// ============================================================================

#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

pub type Result<T> = std::result::Result<T, ParseError>;

// ============================================================================
// Public API
// ============================================================================

impl<'a> LemonGrammar<'a> {
    pub fn parse(input: &'a str) -> Result<Self> {
        Parser::parse_grammar(input)
    }
}

// ============================================================================
// Parser Implementation
// ============================================================================

struct Parser<'a> {
    input: &'a str,
    pos: usize, // Current byte position
    line: usize,
    column: usize,
}

impl<'a> Parser<'a> {
    fn parse_grammar(input: &'a str) -> Result<LemonGrammar<'a>> {
        let mut parser = Self {
            input,
            pos: 0,
            line: 1,
            column: 1,
        };
        let mut tokens = Vec::new();
        let mut rules = Vec::new();
        let mut token_classes = Vec::new();
        let mut fallbacks = Vec::new();
        let mut precedences = Vec::new();
        while parser.peek().is_some() {
            parser.skip_ws();
            match parser.peek() {
                Some('%') => {
                    parser.next();
                    let directive = parser.parse_identifier()?;
                    match directive {
                        "token" => {
                            parser.collect_tokens(&mut tokens)?;
                        }
                        "token_class" => {
                            parser.parse_token_class(&mut token_classes)?;
                        }
                        "fallback" => {
                            parser.parse_fallback(&mut fallbacks)?;
                        }
                        "left" | "right" | "nonassoc" => {
                            parser.parse_precedence(directive, &mut precedences)?;
                        }
                        "ifdef" => {
                            // Skip entire ifdef block (EXCLUDE these rules)
                            parser.skip_ifdef_block()?;
                        }
                        "ifndef" => {
                            // Keep contents, just skip the directive line (INCLUDE these rules)
                            parser.skip_to_eol();
                        }
                        "endif" => {
                            // Skip endif directive (already handled by skip_ifdef_block or matching ifndef)
                            parser.skip_to_eol();
                        }
                        _ => {
                            parser.skip_to_eol();
                        }
                    }
                }
                Some(ch) if ch.is_alphabetic() || ch == '_' => {
                    // Check if current line contains ::= (is a rule) or not (bare tokens)
                    let rest = &parser.input[parser.pos..];
                    let line_end = rest.find('\n').unwrap_or(rest.len());
                    let current_line = &rest[..line_end];

                    if current_line.contains("::=") {
                        rules.push(parser.parse_rule()?);
                        parser.skip_ws();
                        parser.skip_block();
                    } else {
                        // Bare token list, skip the line
                        parser.skip_to_eol();
                    }
                }
                _ => {
                    parser.next();
                }
            }
        }
        Ok(LemonGrammar {
            tokens,
            rules,
            token_classes,
            fallbacks,
            precedences,
        })
    }

    fn parse_rule(&mut self) -> Result<GrammarRule<'a>> {
        let lhs = self.parse_identifier()?;

        // Skip optional LHS alias: name(X)
        self.skip_ws();
        if self.peek() == Some('(') {
            self.advance_until(')');
            self.next(); // consume ')'
        }

        // Expect ::=
        self.skip_ws();
        self.expect_multi(&[':', ':', '='])?;

        // Parse RHS symbols
        let mut rhs = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some('.') => {
                    self.next();
                    break;
                }
                Some('{') => self.skip_block(),
                Some(ch) if ch.is_alphabetic() || ch == '_' => {
                    rhs.push(self.parse_rhs_symbol()?);
                }
                _ => break,
            }
        }

        // Check for [PRECEDENCE] or {DIRECTIVE} after the dot
        self.skip_ws();
        let precedence_override = match self.peek() {
            Some('[') => {
                self.next();
                self.skip_ws();
                let prec = self.parse_identifier()?;
                self.skip_ws();
                self.expect(']')?;
                Some(prec)
            }
            Some('{') => {
                self.skip_block();
                None
            }
            _ => None,
        };

        Ok(GrammarRule {
            lhs,
            rhs,
            precedence_override,
        })
    }

    fn parse_rhs_symbol(&mut self) -> Result<RhsSymbol<'a>> {
        // Parse identifier, allowing | for alternatives (e.g., COMMIT|END)
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' || ch == '|' {
                self.next();
            } else {
                break;
            }
        }
        let name = &self.input[start..self.pos];

        // Check for alias: symbol(X)
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.next();
        }
        let alias = if self.peek() == Some('(') {
            self.next();
            self.skip_ws();
            let a = self.parse_identifier()?;
            self.skip_ws();
            self.expect(')')?;
            Some(a)
        } else {
            None
        };

        Ok(RhsSymbol { name, alias })
    }

    fn parse_identifier(&mut self) -> Result<&'a str> {
        let start = self.pos;
        if !matches!(self.peek(), Some(ch) if ch.is_alphabetic() || ch == '_') {
            return Err(self.error("Expected identifier"));
        }
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                self.next();
            } else {
                break;
            }
        }
        Ok(&self.input[start..self.pos])
    }

    fn collect_tokens(&mut self, tokens: &mut Vec<TokenDecl<'a>>) -> Result<()> {
        // Collect token names from %token directive until we hit a '.'
        loop {
            // Skip horizontal whitespace
            while matches!(self.peek(), Some(' ' | '\t')) {
                self.next();
            }

            match self.peek() {
                Some(ch) if ch.is_alphabetic() || ch == '_' => {
                    tokens.push(TokenDecl {
                        name: self.parse_identifier()?,
                    });
                }
                Some('/') => {
                    // Skip comments
                    self.skip_ws();
                }
                Some('\n') => {
                    self.next();
                    // Stop if next line doesn't start with whitespace
                    if !matches!(self.peek(), Some(' ' | '\t')) {
                        break;
                    }
                }
                Some('.') => {
                    self.next();
                    break;
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn parse_token_class(&mut self, token_classes: &mut Vec<TokenClass<'a>>) -> Result<()> {
        // Format: %token_class name  TOKENS.
        self.skip_ws();
        let name = self.parse_identifier()?;
        self.skip_ws();

        // Capture the token list (e.g., "ID|STRING")
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch == '.' {
                let tokens = &self.input[start..self.pos].trim();
                self.next(); // consume '.'
                token_classes.push(TokenClass { name, tokens });
                return Ok(());
            }
            if ch == '\n' {
                break;
            }
            self.next();
        }

        // If no '.' found, treat rest of line as tokens
        let tokens = &self.input[start..self.pos].trim();
        token_classes.push(TokenClass { name, tokens });
        Ok(())
    }

    fn parse_fallback(&mut self, fallbacks: &mut Vec<FallbackDecl<'a>>) -> Result<()> {
        self.skip_ws();
        let target = self.parse_identifier()?;
        let tokens = self.collect_identifiers_until_dot(true)?;
        fallbacks.push(FallbackDecl { target, tokens });
        Ok(())
    }

    fn parse_precedence(
        &mut self,
        assoc: &'a str,
        precedences: &mut Vec<PrecedenceDecl<'a>>,
    ) -> Result<()> {
        let tokens = self.collect_identifiers_until_dot(false)?;
        precedences.push(PrecedenceDecl { assoc, tokens });
        Ok(())
    }

    /// Collect identifier tokens until '.', optionally handling %ifdef/%ifndef/%endif inline.
    fn collect_identifiers_until_dot(&mut self, handle_ifdefs: bool) -> Result<Vec<&'a str>> {
        let mut names = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some('.') => {
                    self.next();
                    break;
                }
                Some('%') if handle_ifdefs => {
                    self.next();
                    let directive = self.parse_identifier()?;
                    match directive {
                        "ifdef" => self.skip_ifdef_block()?,
                        "ifndef" | "endif" => self.skip_to_eol(),
                        _ => self.skip_to_eol(),
                    }
                }
                Some(ch) if ch.is_alphabetic() || ch == '_' => {
                    names.push(self.parse_identifier()?);
                }
                _ => break,
            }
        }
        Ok(names)
    }

    fn skip_block(&mut self) {
        if self.peek() != Some('{') {
            return;
        }
        let mut depth = 0;
        while let Some(ch) = self.next() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    return;
                }
            }
        }
    }

    /// Advance until target character is found (stops before the character).
    /// Returns true if found, false if end of input reached.
    fn advance_until(&mut self, target: char) -> bool {
        let rest = &self.input[self.pos..];
        let byte_count = rest.find(target).unwrap_or(rest.len());
        let found = byte_count < rest.len();

        let end_pos = (self.pos + byte_count).min(self.input.len());
        let skipped = &self.input[self.pos..end_pos];

        for ch in skipped.chars() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }

        self.pos = end_pos;
        found
    }

    fn skip_to_eol(&mut self) {
        if self.advance_until('\n') {
            self.next(); // consume the newline
        }
    }

    fn skip_ifdef_block(&mut self) -> Result<()> {
        self.skip_to_eol();

        while self.advance_until('%') {
            self.next(); // consume %
            if self.parse_identifier().ok() == Some("endif") {
                self.skip_to_eol();
                return Ok(());
            }
        }
        Ok(())
    }

    fn skip_ws(&mut self) {
        loop {
            match self.peek() {
                Some(ch) if ch.is_whitespace() => {
                    self.next();
                }
                Some('/') if self.pos + 1 < self.input.len() => {
                    match self.input[self.pos + 1..].chars().next() {
                        Some('/') => self.skip_line_comment(),
                        Some('*') => self.skip_block_comment(),
                        _ => return,
                    }
                }
                _ => return,
            }
        }
    }

    fn skip_line_comment(&mut self) {
        self.expect_multi(&['/', '/'])
            .expect("caller should have verified //");
        while let Some(ch) = self.next() {
            if ch == '\n' {
                break;
            }
        }
    }

    fn skip_block_comment(&mut self) {
        self.expect_multi(&['/', '*'])
            .expect("caller should have verified /*");
        while let Some(ch) = self.next() {
            if ch == '*' && self.pos < self.input.len() && self.input[self.pos..].starts_with('/') {
                self.next();
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.input[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn expect(&mut self, expected: char) -> Result<()> {
        match self.peek() {
            Some(ch) if ch == expected => {
                self.next();
                Ok(())
            }
            Some(ch) => Err(self.error(&format!("Expected '{}', got '{}'", expected, ch))),
            None => Err(self.error(&format!("Expected '{}', got EOF", expected))),
        }
    }

    fn expect_multi(&mut self, expected: &[char]) -> Result<()> {
        for &ch in expected {
            self.expect(ch)?;
        }
        Ok(())
    }

    fn error(&self, message: &str) -> ParseError {
        ParseError {
            line: self.line,
            column: self.column,
            message: message.to_string(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_token_decl() {
        let input = "%token PLUS\n%token MINUS";
        let grammar = LemonGrammar::parse(input).unwrap();
        assert_eq!(grammar.tokens.len(), 2);
        assert_eq!(grammar.tokens[0].name, "PLUS");
        assert_eq!(grammar.tokens[1].name, "MINUS");
    }

    #[test]
    fn test_parse_simple_rule() {
        let input = "expr ::= term.";
        let grammar = LemonGrammar::parse(input).unwrap();
        assert_eq!(grammar.rules.len(), 1);
        assert_eq!(grammar.rules[0].lhs, "expr");
        assert_eq!(grammar.rules[0].rhs.len(), 1);
        assert_eq!(grammar.rules[0].rhs[0].name, "term");
    }

    #[test]
    fn test_parse_rule_with_alias() {
        let input = "expr ::= expr(A) PLUS expr(B).";
        let grammar = LemonGrammar::parse(input).unwrap();
        assert_eq!(grammar.rules[0].rhs.len(), 3);
        assert_eq!(grammar.rules[0].rhs[0].alias, Some("A"));
        assert_eq!(grammar.rules[0].rhs[2].alias, Some("B"));
    }

    #[test]
    fn test_skip_action_block() {
        let input = "expr ::= term. { /* action code */ }";
        let grammar = LemonGrammar::parse(input).unwrap();
        assert_eq!(grammar.rules.len(), 1);
    }

    #[test]
    fn test_precedence_override() {
        let input = "expr ::= expr PLUS expr. [PLUS]";
        let grammar = LemonGrammar::parse(input).unwrap();
        assert_eq!(grammar.rules[0].precedence_override, Some("PLUS"));
    }
}