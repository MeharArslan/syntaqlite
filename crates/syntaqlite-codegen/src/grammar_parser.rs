//! Lemon grammar file parser
//!
//! **NOTE:** This is a minimal, relaxed parser designed specifically to extract
//! tokens and rules from SQLite's parse.y grammar. It is NOT a general-purpose
//! Lemon parser and makes no attempt to validate grammar correctness. It simply
//! extracts the information we need while skipping everything else.

use std::iter::Peekable;
use std::str::CharIndices;

// ============================================================================
// Core Types
// ============================================================================

#[derive(Debug, Clone)]
pub struct LemonGrammar<'a> {
    pub tokens: Vec<TokenDecl<'a>>,
    pub rules: Vec<GrammarRule<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenDecl<'a> {
    pub name: &'a str,
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
    chars: Peekable<CharIndices<'a>>,
    pos: usize,      // Current byte position
    line: usize,
    column: usize,
}

impl<'a> Parser<'a> {
    fn parse_grammar(input: &'a str) -> Result<LemonGrammar<'a>> {
        let mut parser = Self {
            input,
            chars: input.char_indices().peekable(),
            pos: 0,
            line: 1,
            column: 1,
        };
        let mut tokens = Vec::new();
        let mut rules = Vec::new();
        while parser.peek().is_some() {
            parser.skip_ws();
            match parser.peek() {
                Some('%') => {
                    parser.next();
                    if parser.parse_identifier()? == "token" {
                        parser.collect_tokens(&mut tokens)?;
                    } else {
                        parser.skip_to_next_item();
                    }
                }
                Some(ch) if ch.is_alphabetic() || ch == '_' => {
                    rules.push(parser.parse_rule()?);
                    parser.skip_ws();
                    parser.skip_block();
                }
                _ => {
                    parser.next();
                }
            }
        }
        Ok(LemonGrammar { tokens, rules })
    }

    fn parse_rule(&mut self) -> Result<GrammarRule<'a>> {
        let lhs = self.parse_identifier()?;

        // Skip optional LHS alias: name(X)
        self.skip_ws();
        if self.peek() == Some('(') {
            self.skip_until(')');
            self.next();
        }

        // Expect ::=
        self.skip_ws();
        self.expect(':')?;
        self.expect(':')?;
        self.expect('=')?;

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

    fn skip_to_next_item(&mut self) {
        // Skip until we find a line that doesn't start with whitespace
        while let Some(ch) = self.peek() {
            if ch == '{' {
                self.skip_block();
            } else if ch == '\n' {
                self.next();
                if !matches!(self.peek(), Some(' ' | '\t')) {
                    break;
                }
            } else {
                self.next();
            }
        }
    }

    fn skip_until(&mut self, target: char) {
        while let Some(ch) = self.peek() {
            if ch == target {
                break;
            }
            self.next();
        }
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
        self.next(); // /
        self.next(); // /
        while let Some(ch) = self.next() {
            if ch == '\n' {
                break;
            }
        }
    }

    fn skip_block_comment(&mut self) {
        self.next(); // /
        self.next(); // *
        while let Some(ch) = self.next() {
            if ch == '*' && self.pos < self.input.len() {
                if self.input[self.pos..].chars().next() == Some('/') {
                    self.next(); // /
                    break;
                }
            }
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, ch)| *ch)
    }

    fn next(&mut self) -> Option<char> {
        self.chars.next().map(|(pos, ch)| {
            self.pos = pos + ch.len_utf8();
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            ch
        })
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

    #[test]
    fn test_parse_sqlite_grammar() {
        let input = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../third_party/src/sqlite/src/parse.y"),
        )
        .unwrap();

        let grammar = LemonGrammar::parse(&input).unwrap();

        assert!(
            grammar.tokens.len() > 0,
            "Expected to find token declarations"
        );
        assert!(grammar.rules.len() > 0, "Expected to find grammar rules");

        println!(
            "Parsed {} tokens and {} rules from SQLite grammar",
            grammar.tokens.len(),
            grammar.rules.len()
        );
    }
}
