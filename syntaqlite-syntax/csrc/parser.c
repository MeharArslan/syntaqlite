// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#include "syntaqlite/parser.h"
#include "csrc/tokens.h"
#include "syntaqlite/incremental.h"

#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#include "csrc/dialect_dispatch.h"
#include "csrc/token_wrapped.h"
#include "syntaqlite/grammar.h"
#include "syntaqlite_dialect/ast_builder.h"
#include "syntaqlite_dialect/dialect_types.h"

// ---------------------------------------------------------------------------
// Parser struct
// ---------------------------------------------------------------------------

struct SyntaqliteParser {
  SyntaqliteMemMethods mem;
  SyntaqliteGrammar grammar;
  void* lemon;
  SynqParseCtx ctx;
  const char* source;
  uint32_t source_len;
  uint32_t offset;           // Tokenizer cursor into source.
  uint32_t last_token_type;  // Last non-whitespace token fed to Lemon.
  uint32_t finished;         // 1 after EOF has been sent to Lemon.
  uint32_t had_comment;      // 1 if any comment token was seen this stmt.
  uint32_t had_error;        // Sticky error flag for current result.
  int32_t last_status;       // Last SYNTAQLITE_PARSE_* status returned.
  char error_msg[256];       // Error message buffer.
  uint32_t trace;
  uint32_t collect_tokens;
  uint32_t sealed;
  uint32_t pending_reset;  // 1 after feed_token signals completion; cleared on
                           // the next feed_token call (arena reset deferred).
  SYNQ_VEC(SyntaqliteComment) comments;
  SYNQ_VEC(SyntaqliteParserToken) tokens;
  uint32_t macro_depth;  // Nesting depth (0 = not in macro).
  SYNQ_VEC(SyntaqliteMacroRegion) macros;
};

static int32_t set_result_status(SyntaqliteParser* p, int32_t rc) {
  p->last_status = rc;
  return rc;
}

// Forward declaration — defined after feed_one_token.
static int check_macro_straddle(SyntaqliteParser* p);

// ---------------------------------------------------------------------------
// Internal: reusable state-reset helpers
// ---------------------------------------------------------------------------

// Reinitialize the Lemon parser automaton to its initial state.
// Called after real-statement completion (cmdx ::= cmd . reduces with SEMI
// as the LALR(1) lookahead, leaving SEMI shifted but ecmd ::= cmdx SEMI .
// pending).  Reinitializing discards that half-reduced state.
// NOT called for bare semicolons or error-recovery completions — those
// reduce via ecmd ::= SEMI . or ecmd ::= error SEMI . using the *next*
// token as the lookahead, so that token is already consumed by Lemon.
static void lemon_reinit(SyntaqliteParser* p) {
  SYNQ_PARSER_FINALIZE(p->grammar.tmpl, p->lemon);
  SYNQ_PARSER_INIT(p->grammar.tmpl, p->lemon, &p->ctx);
  p->last_token_type = 0;
}

// Reset all per-statement output state: arena, token/comment/macro vectors,
// context flags, and error state.  Called at the *start* of the next
// statement (not at completion) so that callers can read the previous
// statement's results between calls.
static void reset_stmt(SyntaqliteParser* p) {
  synq_parse_ctx_clear(&p->ctx);
  syntaqlite_vec_clear(&p->comments);
  syntaqlite_vec_clear(&p->tokens);
  syntaqlite_vec_clear(&p->macros);
  p->ctx.root = SYNTAQLITE_NULL_NODE;
  p->ctx.stmt_completed = 0;
  p->ctx.pending_explain_mode = 0;
  p->ctx.error = 0;
  p->ctx.saw_subquery = 0;
  p->ctx.saw_update_delete_limit = 0;
  p->had_comment = 0;
  p->had_error = 0;
  p->error_msg[0] = '\0';
  p->ctx.error_offset = 0xFFFFFFFF;
  p->ctx.error_length = 0;
  p->ctx.tokens = p->collect_tokens ? &p->tokens : NULL;
}

// Handle a statement boundary after feed_one_token returns 1.
// Reinitializes Lemon and classifies the completed statement:
//   SYNTAQLITE_PARSE_OK    — successful statement (root is set)
//   SYNTAQLITE_PARSE_ERROR — statement with syntax error(s)
//   SYNTAQLITE_PARSE_DONE  — bare semicolon (no statement produced)
static int32_t stmt_boundary(SyntaqliteParser* p) {
  lemon_reinit(p);

  // Bare semicolon — no statement produced.
  if (p->ctx.root == SYNTAQLITE_NULL_NODE && !p->had_error)
    return SYNTAQLITE_PARSE_DONE;

  if (check_macro_straddle(p) < 0)
    return SYNTAQLITE_PARSE_ERROR;

  if (p->had_error) {
    p->had_error = 0;  // consumed for this result
    return SYNTAQLITE_PARSE_ERROR;
  }
  return SYNTAQLITE_PARSE_OK;
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

SyntaqliteParser* syntaqlite_parser_create_with_grammar(
    const SyntaqliteMemMethods* mem,
    const SyntaqliteGrammar grammar) {
  SyntaqliteMemMethods m = mem ? *mem : SYNTAQLITE_MEM_METHODS_DEFAULT;
  SyntaqliteParser* p = m.xMalloc(sizeof(SyntaqliteParser));
  memset(p, 0, sizeof(*p));
  p->mem = m;
  p->grammar = grammar;
  p->lemon = SYNQ_PARSER_ALLOC(grammar.tmpl, m.xMalloc, &p->ctx);
  synq_parse_ctx_init(&p->ctx, m);
  syntaqlite_vec_init(&p->comments);
  syntaqlite_vec_init(&p->tokens);
  syntaqlite_vec_init(&p->macros);
  return p;
}

#ifndef SYNTAQLITE_OMIT_SQLITE_API
SyntaqliteParser* syntaqlite_parser_create(const SyntaqliteMemMethods* mem) {
  SyntaqliteGrammar grammar = syntaqlite_sqlite_grammar();
  return syntaqlite_parser_create_with_grammar(mem, grammar);
}
#endif

void syntaqlite_parser_reset(SyntaqliteParser* p,
                             const char* source,
                             uint32_t len) {
  // Seal the parser on first use — configuration is frozen after this.
  p->sealed = 1;

  lemon_reinit(p);
  reset_stmt(p);

  p->source = source;
  p->source_len = len;
  p->offset = 0;
  p->finished = 0;
  p->pending_reset = 0;
  p->last_status = SYNTAQLITE_PARSE_DONE;
  p->macro_depth = 0;

  p->ctx.source = source;
  p->ctx.env = &p->grammar;
}

// ---------------------------------------------------------------------------
// Internal: feed one real token to Lemon.
// Returns: 0 = keep going, 1 = statement completed, -1 = unrecoverable error.
// ---------------------------------------------------------------------------

static int feed_one_token(SyntaqliteParser* p,
                          uint32_t token_type,
                          const char* text,
                          uint32_t len,
                          uint32_t token_idx) {
  SynqParseToken minor = {
      .z = text, .n = len, .type = token_type, .token_idx = token_idx};
  SYNQ_PARSER_FEED(p->grammar.tmpl, p->lemon, (int)token_type, minor);
  p->last_token_type = token_type;

  if (p->ctx.error) {
    p->had_error = 1;
    if (p->error_msg[0] == '\0') {
      if (text) {
        p->ctx.error_offset = (uint32_t)(text - p->source);
        p->ctx.error_length = (uint32_t)len;
        snprintf(p->error_msg, sizeof(p->error_msg), "syntax error near '%.*s'",
                 len, text);
      } else {
        snprintf(p->error_msg, sizeof(p->error_msg),
                 "incomplete SQL statement");
      }
    }
    p->ctx.error = 0;  // Lemon is now driving recovery.
    return 0;
  }

  if (p->ctx.stmt_completed) {
    p->ctx.stmt_completed = 0;
    return 1;
  }

  return 0;
}

// ---------------------------------------------------------------------------
// Internal: check macro straddle after statement completion.
// ---------------------------------------------------------------------------

static int check_macro_straddle(SyntaqliteParser* p) {
  uint32_t macro_count = syntaqlite_vec_len(&p->macros);
  if (macro_count == 0)
    return 0;
  if (!p->grammar.tmpl->range_meta) {
    snprintf(p->error_msg, sizeof(p->error_msg),
             "internal error: grammar has no range_meta but macros were used");
    p->had_error = 1;
    return -1;
  }

  uint32_t node_count = syntaqlite_vec_len(&p->ctx.ast.offsets);
  const SyntaqliteMacroRegion* macros = p->macros.data;

  for (uint32_t nid = 0; nid < node_count; nid++) {
    const uint8_t* raw = (const uint8_t*)synq_arena_ptr(&p->ctx.ast, nid);
    uint32_t tag;
    memcpy(&tag, raw, sizeof(tag));
    if (tag == 0 || tag >= p->grammar.tmpl->node_count)
      continue;

    const SyntaqliteRangeMetaEntry* entry = &p->grammar.tmpl->range_meta[tag];
    if (entry->fields == NULL || entry->count == 0)
      continue;

    for (uint32_t mi = 0; mi < macro_count; mi++) {
      uint32_t r_start = macros[mi].call_offset;
      uint32_t r_end = r_start + macros[mi].call_length;

      int has_inside = 0;
      int has_outside = 0;

      for (uint8_t fi = 0; fi < entry->count; fi++) {
        if (entry->fields[fi].kind != 1)
          continue;  // Not a SourceSpan.
        const SyntaqliteSourceSpan* sp =
            (const SyntaqliteSourceSpan*)(raw + entry->fields[fi].offset);
        if (sp->length == 0)
          continue;

        uint32_t s_start = sp->offset;
        uint32_t s_end = sp->offset + sp->length;

        if (s_start >= r_start && s_end <= r_end) {
          has_inside = 1;
        } else {
          has_outside = 1;
        }
      }

      if (has_inside && has_outside) {
        snprintf(p->error_msg, sizeof(p->error_msg),
                 "macro expansion straddles node boundary");
        p->had_error = 1;
        return -1;
      }
    }
  }
  return 0;
}

// ---------------------------------------------------------------------------
// Internal: synthesize SEMI + EOF to finish parsing.
// Returns a SYNTAQLITE_PARSE_* code.
// ---------------------------------------------------------------------------

static int finish_input(SyntaqliteParser* p) {
  // No real tokens were fed (only whitespace/comments).
  if (p->last_token_type == 0) {
    p->finished = 1;
    // If comments were seen, return PARSE_OK (root will be NULL_NODE).
    // This matches SQLite's sqlite3_prepare_v2 which returns SQLITE_OK
    // for comment-only input.
    if (p->had_comment) {
      return set_result_status(p, SYNTAQLITE_PARSE_OK);
    }
    return set_result_status(p, SYNTAQLITE_PARSE_DONE);
  }

  // Synthesize SEMI if the last token wasn't one.
  if (p->last_token_type != SYNTAQLITE_TK_SEMI) {
    int rc = feed_one_token(p, SYNTAQLITE_TK_SEMI, NULL, 0, 0xFFFFFFFF);
    if (rc == 1) {
      int32_t status = stmt_boundary(p);
      if (status != SYNTAQLITE_PARSE_DONE) {
        p->finished = 1;
        return set_result_status(p, status);
      }
      // bare semicolon — fall through to EOF
    }
  }

  // Send end-of-input (EOF) to flush the final reduction.
  SynqParseToken eof = {.z = NULL, .n = 0, .type = 0, .token_idx = 0xFFFFFFFF};
  SYNQ_PARSER_FEED(p->grammar.tmpl, p->lemon, 0, eof);
  p->finished = 1;

  if (p->ctx.error) {
    p->had_error = 1;
    if (p->ctx.error_offset == 0xFFFFFFFF) {
      p->ctx.error_offset = p->offset;
    }
    if (p->error_msg[0] == '\0') {
      snprintf(p->error_msg, sizeof(p->error_msg), "incomplete SQL statement");
    }
    return set_result_status(p, SYNTAQLITE_PARSE_ERROR);
  }

  if (p->ctx.root != SYNTAQLITE_NULL_NODE) {
    if (check_macro_straddle(p) < 0)
      return set_result_status(p, SYNTAQLITE_PARSE_ERROR);
    return set_result_status(
        p, p->had_error ? SYNTAQLITE_PARSE_ERROR : SYNTAQLITE_PARSE_OK);
  }

  if (p->had_error)
    return set_result_status(p, SYNTAQLITE_PARSE_ERROR);

  return set_result_status(p, SYNTAQLITE_PARSE_DONE);
}

// ---------------------------------------------------------------------------
// High-level API
// ---------------------------------------------------------------------------

int32_t syntaqlite_parser_next(SyntaqliteParser* p) {
  reset_stmt(p);

  if (p->finished)
    return set_result_status(p, SYNTAQLITE_PARSE_DONE);

  const unsigned char* z = (const unsigned char*)p->source;

  while (p->offset < p->source_len && z[p->offset] != '\0') {
    uint32_t token_type = 0;
    int64_t token_len = SynqSqliteGetTokenVersionWrapped(
        &p->grammar, z + p->offset, &token_type);
    if (token_len <= 0)
      break;

    uint32_t tok_offset = p->offset;
    p->offset += (uint32_t)token_len;

    if (token_type == SYNTAQLITE_TK_SPACE) {
      continue;
    }

    if (token_type == SYNTAQLITE_TK_COMMENT) {
      p->had_comment = 1;
      if (p->collect_tokens) {
        SyntaqliteComment t = {tok_offset, (uint32_t)token_len,
                               z[tok_offset] == '-' ? (uint8_t)0 : (uint8_t)1};
        syntaqlite_vec_push(&p->comments, t, p->mem);
      }
      continue;
    }

    uint32_t tidx = 0xFFFFFFFF;
    if (p->collect_tokens && token_type != SYNTAQLITE_TK_SEMI) {
      SyntaqliteParserToken tp = {tok_offset, (uint32_t)token_len, token_type,
                                  0};
      syntaqlite_vec_push(&p->tokens, tp, p->mem);
      tidx = syntaqlite_vec_len(&p->tokens) - 1;
    }

    int rc = feed_one_token(p, token_type, p->source + tok_offset,
                            (uint32_t)token_len, tidx);

    // Safety net: if error recovery couldn't produce an ecmd ::= error SEMI .
    // reduction (e.g. parse_failure), force completion on the next SEMI.
    if (p->had_error && rc == 0 && token_type == SYNTAQLITE_TK_SEMI)
      rc = 1;

    if (rc == 1) {
      // Bare semicolons (ecmd ::= SEMI.) and error-recovery completions
      // (ecmd ::= error SEMI.) have root == NULL_NODE and reduce when the
      // *next* token is the LALR(1) lookahead — that token has already been
      // consumed by Lemon and must not be discarded by lemon_reinit.
      if (p->ctx.root == SYNTAQLITE_NULL_NODE && !p->had_error)
        continue;  // bare semicolon — Lemon state is clean enough
      int32_t status = stmt_boundary(p);
      return set_result_status(p, status);
    }
  }

  // End of input.
  return finish_input(p);
}

// ---------------------------------------------------------------------------
// Result accessors
// ---------------------------------------------------------------------------

uint32_t syntaqlite_result_root(SyntaqliteParser* p) {
  if (p->last_status != SYNTAQLITE_PARSE_OK) {
    return SYNTAQLITE_NULL_NODE;
  }
  return p->ctx.root;
}

uint32_t syntaqlite_result_recovery_root(SyntaqliteParser* p) {
  if (p->last_status != SYNTAQLITE_PARSE_ERROR) {
    return SYNTAQLITE_NULL_NODE;
  }
  return p->ctx.root;
}

const char* syntaqlite_result_error_msg(SyntaqliteParser* p) {
  return p->error_msg[0] ? p->error_msg : NULL;
}

uint32_t syntaqlite_result_error_offset(SyntaqliteParser* p) {
  return p->ctx.error_offset;
}

uint32_t syntaqlite_result_error_length(SyntaqliteParser* p) {
  return p->ctx.error_length;
}

const SyntaqliteComment* syntaqlite_result_comments(SyntaqliteParser* p,
                                                    uint32_t* count) {
  *count = syntaqlite_vec_len(&p->comments);
  return p->comments.data;
}

const SyntaqliteParserToken* syntaqlite_result_tokens(SyntaqliteParser* p,
                                                      uint32_t* count) {
  *count = syntaqlite_vec_len(&p->tokens);
  return p->tokens.data;
}

const SyntaqliteMacroRegion* syntaqlite_result_macros(SyntaqliteParser* p,
                                                      uint32_t* count) {
  *count = syntaqlite_vec_len(&p->macros);
  return p->macros.data;
}

// ---------------------------------------------------------------------------
// Low-level token-feeding API
// ---------------------------------------------------------------------------

int32_t syntaqlite_parser_feed_token(SyntaqliteParser* p,
                                     uint32_t token_type,
                                     const char* text,
                                     uint32_t len) {
  // Deferred reset: clear previous statement's data before processing the
  // first token of the next one.  Lemon was already reinitialized eagerly
  // by stmt_boundary() when the previous statement completed.
  if (p->pending_reset) {
    reset_stmt(p);
    p->pending_reset = 0;
  }

  // Skip whitespace silently.
  if (token_type == SYNTAQLITE_TK_SPACE) {
    return set_result_status(p, SYNTAQLITE_PARSE_DONE);
  }

  // Record comments but don't feed to Lemon.
  if (token_type == SYNTAQLITE_TK_COMMENT) {
    if (p->collect_tokens && text) {
      uint32_t tok_offset = (uint32_t)(text - p->source);
      SyntaqliteComment t = {tok_offset, len,
                             (uint8_t)(text[0] == '-' ? 0 : 1)};
      syntaqlite_vec_push(&p->comments, t, p->mem);
    }
    return set_result_status(p, SYNTAQLITE_PARSE_DONE);
  }

  // Capture non-whitespace, non-comment, non-semicolon token positions.
  uint32_t tidx = 0xFFFFFFFF;
  if (p->collect_tokens && text && token_type != SYNTAQLITE_TK_SEMI) {
    uint32_t tok_offset = (uint32_t)(text - p->source);
    SyntaqliteParserToken tp = {tok_offset, len, token_type, 0};
    syntaqlite_vec_push(&p->tokens, tp, p->mem);
    tidx = syntaqlite_vec_len(&p->tokens) - 1;
  }

  int rc = feed_one_token(p, token_type, text, len, tidx);
  if (rc < 0)
    return set_result_status(p, SYNTAQLITE_PARSE_ERROR);

  if (rc == 1) {
    // Bare semicolons (ecmd ::= SEMI.) and error-recovery completions
    // (ecmd ::= error SEMI.) have root == NULL_NODE and may have consumed
    // the next token as an LALR(1) lookahead.  Do NOT reinitialize Lemon —
    // the consumed token is already in Lemon's state and will be processed
    // normally on the next feed_token call.
    if (p->ctx.root == SYNTAQLITE_NULL_NODE) {
      if (p->had_error) {
        p->had_error = 0;
        p->pending_reset = 1;
        return set_result_status(p, SYNTAQLITE_PARSE_ERROR);
      }
      return set_result_status(p, SYNTAQLITE_PARSE_DONE);
    }

    // Real statement — cmdx ::= cmd. fired with SEMI as the lookahead,
    // leaving ecmd ::= cmdx SEMI. pending.  Reinitialize Lemon.
    int32_t status = stmt_boundary(p);
    p->pending_reset = 1;
    return set_result_status(p, status);
  }

  return set_result_status(p, SYNTAQLITE_PARSE_DONE);
}

uint32_t syntaqlite_parser_expected_tokens(SyntaqliteParser* p,
                                           uint32_t* out_tokens,
                                           uint32_t out_cap) {
  if (p == NULL || p->grammar.tmpl == NULL ||
      p->grammar.tmpl->parser_expected_tokens == NULL) {
    return 0;
  }
  return p->grammar.tmpl->parser_expected_tokens(p->lemon, out_tokens, out_cap);
}

SyntaqliteCompletionContext syntaqlite_parser_completion_context(
    SyntaqliteParser* p) {
  if (p == NULL || p->grammar.tmpl == NULL ||
      p->grammar.tmpl->parser_completion_context == NULL) {
    return SYNTAQLITE_COMPLETION_CONTEXT_UNKNOWN;
  }
  return (SyntaqliteCompletionContext)
      p->grammar.tmpl->parser_completion_context(p->lemon);
}

int32_t syntaqlite_parser_finish(SyntaqliteParser* p) {
  if (p->pending_reset) {
    // Nothing pending after a completed statement — done.
    p->pending_reset = 0;
    return set_result_status(p, SYNTAQLITE_PARSE_DONE);
  }
  return finish_input(p);
}

// ---------------------------------------------------------------------------
// Macro region tracking
// ---------------------------------------------------------------------------

void syntaqlite_parser_begin_macro(SyntaqliteParser* p,
                                   uint32_t call_offset,
                                   uint32_t call_length) {
  SyntaqliteMacroRegion region = {call_offset, call_length};
  syntaqlite_vec_push(&p->macros, region, p->mem);
  p->macro_depth++;
}

void syntaqlite_parser_end_macro(SyntaqliteParser* p) {
  if (p->macro_depth > 0) {
    p->macro_depth--;
  }
}

// ---------------------------------------------------------------------------
// AST dump
// ---------------------------------------------------------------------------

typedef SYNQ_VEC(char) DumpBuf;

static void dump_append(DumpBuf* b,
                        SyntaqliteMemMethods mem,
                        const char* s,
                        uint32_t n) {
  syntaqlite_vec_push_n(b, s, n, mem);
}

static void dump_printf(DumpBuf* b,
                        SyntaqliteMemMethods mem,
                        const char* fmt,
                        ...) {
  va_list ap;
  va_start(ap, fmt);
  int n = vsnprintf(NULL, 0, fmt, ap);
  va_end(ap);
  if (n <= 0)
    return;
  syntaqlite_vec_ensure(b, b->count + (uint32_t)n + 1, mem);
  va_start(ap, fmt);
  vsnprintf(b->data + b->count, (uint32_t)n + 1, fmt, ap);
  va_end(ap);
  b->count += (uint32_t)n;
}

static void dump_indent(DumpBuf* b, SyntaqliteMemMethods mem, uint32_t indent) {
  for (uint32_t i = 0; i < indent; i++)
    dump_append(b, mem, "  ", 2);
}

static void dump_node_recursive(DumpBuf* b,
                                SyntaqliteParser* p,
                                uint32_t node_id,
                                uint32_t indent) {
  if (node_id == SYNTAQLITE_NULL_NODE)
    return;
  uint32_t count = syntaqlite_vec_len(&p->ctx.ast.offsets);
  if (node_id >= count)
    return;

  const uint8_t* raw = (const uint8_t*)synq_arena_ptr(&p->ctx.ast, node_id);
  uint32_t tag;
  memcpy(&tag, raw, sizeof(tag));

  const SyntaqliteGrammarTemplate* g = p->grammar.tmpl;
  if (tag >= g->node_count)
    return;

  const char* name = g->node_names[tag];
  uint8_t field_count = g->field_meta_counts[tag];
  SyntaqliteMemMethods mem = p->mem;

  // List node: no field descriptors, has tag + count header.
  if (field_count == 0 && tag != 0) {
    SynqListHeader hdr;
    memcpy(&hdr, raw, sizeof(hdr));
    dump_indent(b, mem, indent);
    dump_printf(b, mem, "%s [%u items]\n", name, hdr.count);
    const uint32_t* children = (const uint32_t*)(raw + sizeof(SynqListHeader));
    for (uint32_t i = 0; i < hdr.count; i++) {
      dump_node_recursive(b, p, children[i], indent + 1);
    }
    return;
  }

  dump_indent(b, mem, indent);
  dump_printf(b, mem, "%s\n", name);

  if (field_count == 0)
    return;
  const SyntaqliteFieldMeta* fields = g->field_meta[tag];

  for (uint8_t fi = 0; fi < field_count; fi++) {
    const SyntaqliteFieldMeta* fm = &fields[fi];
    const uint8_t* field_ptr = raw + fm->offset;

    switch (fm->kind) {
      case SYNTAQLITE_FIELD_NODE_ID: {
        uint32_t child_id;
        memcpy(&child_id, field_ptr, sizeof(child_id));
        dump_indent(b, mem, indent + 1);
        if (child_id == SYNTAQLITE_NULL_NODE) {
          dump_printf(b, mem, "%s: (none)\n", fm->name);
        } else {
          dump_printf(b, mem, "%s:\n", fm->name);
          dump_node_recursive(b, p, child_id, indent + 2);
        }
        break;
      }
      case SYNTAQLITE_FIELD_SPAN: {
        SyntaqliteSourceSpan sp;
        memcpy(&sp, field_ptr, sizeof(sp));
        dump_indent(b, mem, indent + 1);
        if (sp.length == 0) {
          dump_printf(b, mem, "%s: (none)\n", fm->name);
        } else {
          dump_printf(b, mem, "%s: \"%.*s\"\n", fm->name, (int)sp.length,
                      p->source + sp.offset);
        }
        break;
      }
      case SYNTAQLITE_FIELD_BOOL: {
        uint32_t val;
        memcpy(&val, field_ptr, sizeof(val));
        dump_indent(b, mem, indent + 1);
        dump_printf(b, mem, "%s: %s\n", fm->name, val ? "TRUE" : "FALSE");
        break;
      }
      case SYNTAQLITE_FIELD_FLAGS: {
        uint8_t val = *field_ptr;
        // Mask to defined bits only — upper bits may contain struct padding.
        if (fm->display_count < 8)
          val &= (uint8_t)((1u << fm->display_count) - 1);
        dump_indent(b, mem, indent + 1);
        dump_printf(b, mem, "%s: ", fm->name);
        if (val == 0) {
          dump_append(b, mem, "(none)", 6);
        } else {
          int first = 1;
          for (int bit = 0; bit < fm->display_count; bit++) {
            if (val & (1 << bit)) {
              const char* flag_name = fm->display ? fm->display[bit] : "?";
              if (flag_name[0] == '\0')
                continue;
              if (!first)
                dump_append(b, mem, " ", 1);
              dump_append(b, mem, flag_name, (uint32_t)strlen(flag_name));
              first = 0;
            }
          }
          if (first)
            dump_append(b, mem, "(none)", 6);
        }
        dump_append(b, mem, "\n", 1);
        break;
      }
      case SYNTAQLITE_FIELD_ENUM: {
        uint32_t val;
        memcpy(&val, field_ptr, sizeof(val));
        dump_indent(b, mem, indent + 1);
        const char* label =
            (val < fm->display_count && fm->display) ? fm->display[val] : "?";
        dump_printf(b, mem, "%s: %s\n", fm->name, label);
        break;
      }
    }
  }
}

char* syntaqlite_dump_node(SyntaqliteParser* p,
                           uint32_t node_id,
                           uint32_t indent) {
  DumpBuf buf;
  syntaqlite_vec_init(&buf);
  dump_node_recursive(&buf, p, node_id, indent);
  syntaqlite_vec_push(&buf, '\0', p->mem);
  return buf.data;
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

void syntaqlite_parser_destroy(SyntaqliteParser* p) {
  if (p) {
    SYNQ_PARSER_FREE(p->grammar.tmpl, p->lemon, p->mem.xFree);
    synq_parse_ctx_free(&p->ctx);
    syntaqlite_vec_free(&p->comments, p->mem);
    syntaqlite_vec_free(&p->tokens, p->mem);
    syntaqlite_vec_free(&p->macros, p->mem);
    p->mem.xFree(p);
  }
}

// ---------------------------------------------------------------------------
// Reading results
// ---------------------------------------------------------------------------

const void* syntaqlite_parser_node(SyntaqliteParser* p, uint32_t node_id) {
  return (const void*)synq_arena_ptr(&p->ctx.ast, node_id);
}

uint32_t syntaqlite_parser_node_count(SyntaqliteParser* p) {
  return syntaqlite_vec_len(&p->ctx.ast.offsets);
}

const char* syntaqlite_parser_source(SyntaqliteParser* p) {
  return p->source;
}

uint32_t syntaqlite_parser_source_length(SyntaqliteParser* p) {
  return p->source_len;
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

int32_t syntaqlite_parser_set_trace(SyntaqliteParser* p, uint32_t enable) {
  if (p->sealed)
    return -1;
  p->trace = enable;
  if (enable) {
    SYNQ_PARSER_TRACE(p->grammar.tmpl, stderr, "parser> ");
  } else {
    SYNQ_PARSER_TRACE(p->grammar.tmpl, NULL, NULL);
  }
  return 0;
}

int32_t syntaqlite_parser_set_collect_tokens(SyntaqliteParser* p,
                                             uint32_t enable) {
  if (p->sealed)
    return -1;
  p->collect_tokens = enable;
  return 0;
}
