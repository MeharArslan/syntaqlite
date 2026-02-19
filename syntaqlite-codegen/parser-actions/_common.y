// AST building actions for syntaqlite grammar.
// These rules get merged with SQLite's parse.y during code generation.
//
// Rule signatures MUST match upstream parse.y exactly.
// Python tooling validates coverage and consistency.
//
// Conventions:
// - pCtx: Parse context (SynqParseCtx*), threaded via %extra_argument
// - pCtx->root: Set to root node ID at input rule
// - Terminals are SynqToken with .z (pointer), .n (length), .type (token ID)
// - Non-terminals default to uint32_t (node IDs)
// - synq_span(pCtx, tok) converts a SynqToken into SyntaqliteSourceSpan
// - SYNQ_NO_SPAN is the zero sentinel span

%name SyntaqliteParse
%token_prefix SYNTAQLITE_TK_
%start_symbol input
%extra_argument {SynqParseCtx* pCtx}

%include {
#include <string.h>

#include "csrc/parser.h"
#include "csrc/grammar_types.h"
#include "csrc/sqlite_parse_data.h"
#include "syntaqlite/tokens.h"

#define YYNOERRORRECOVERY 1
#define YYPARSEFREENEVERNULL 1
}

// ============ Type declarations ============
//
// %token_type and %default_type are global; individual %type declarations
// live next to the rules they describe in each action file.

%token_type {SynqToken}
%default_type {uint32_t}

// ============ Error handlers ============

%syntax_error {
  (void)yymajor;
  (void)TOKEN;
  if (pCtx) {
    pCtx->error = 1;
  }
}

%stack_overflow {
  if (pCtx) {
    pCtx->error = 1;
  }
}

// ============ Tokens ============

%token ABORT ACTION AFTER ANALYZE ASC ATTACH BEFORE BEGIN BY CASCADE CAST.
%token CONFLICT DATABASE DEFERRED DESC DETACH EACH END EXCLUSIVE EXPLAIN FAIL.
%token OR AND NOT IS ISNOT MATCH LIKE_KW BETWEEN IN ISNULL NOTNULL NE EQ.
%token GT LE LT GE ESCAPE.

// The following directive causes tokens ABORT, AFTER, ASC, etc. to
// fallback to ID if they will not parse as their original value.
// This obviates the need for the "id" nonterminal.
//
%fallback ID
  ABORT ACTION AFTER ANALYZE ASC ATTACH BEFORE BEGIN BY CASCADE CAST COLUMNKW
  CONFLICT DATABASE DEFERRED DESC DETACH DO
  EACH END EXCLUSIVE EXPLAIN FAIL FOR
  IGNORE IMMEDIATE INITIALLY INSTEAD LIKE_KW MATCH NO PLAN
  QUERY KEY OF OFFSET PRAGMA RAISE RECURSIVE RELEASE REPLACE RESTRICT ROW ROWS
  ROLLBACK SAVEPOINT TEMP TRIGGER VACUUM VIEW VIRTUAL WITH WITHOUT
  NULLS FIRST LAST
  CURRENT FOLLOWING PARTITION PRECEDING RANGE UNBOUNDED
  EXCLUDE GROUPS OTHERS TIES
  GENERATED ALWAYS
  MATERIALIZED
  REINDEX RENAME CTIME_KW IF
  .
%wildcard ANY.

%left OR.
%left AND.
%right NOT.
%left IS MATCH LIKE_KW BETWEEN IN ISNULL NOTNULL NE EQ.
%left GT LE LT GE.
%right ESCAPE.
%left BITAND BITOR LSHIFT RSHIFT.
%left PLUS MINUS.
%left STAR SLASH REM.
%left CONCAT PTR.
%left COLLATE.
%right BITNOT.
%nonassoc ON.

// ============ Token classes (match SQLite's parse.y) ============

%token_class id  ID|INDEXED.
%token_class ids  ID|STRING.
%token_class idj  ID|INDEXED|JOIN_KW.
%token_class number INTEGER|FLOAT.

// ============ Entry point ============

input ::= cmdlist(B). {
    pCtx->root = B;
}

// ============ Command list ============

cmdlist(A) ::= cmdlist ecmd(B). {
    A = B;  // Just use the last command for now
}

cmdlist(A) ::= ecmd(B). {
    A = B;
}

// ============ Command wrapper ============

ecmd(A) ::= SEMI. {
    A = SYNTAQLITE_NULL_NODE;
    pCtx->stmt_completed = 1;
}

ecmd(A) ::= cmdx(B) SEMI. {
    A = B;
    pCtx->root = B;
    synq_parse_list_flush(pCtx);
    pCtx->stmt_completed = 1;
}

cmdx(A) ::= cmd(B). {
    A = B;
}
