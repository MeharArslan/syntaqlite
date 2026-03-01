// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#include "syntaqlite/parser.h"

#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#include "csrc/dialect_dispatch.h"
#include "csrc/token_wrapped.h"
#include "syntaqlite/dialect.h"
#include "syntaqlite_ext/ast_builder.h"

// ---------------------------------------------------------------------------
// Parser struct
// ---------------------------------------------------------------------------

struct SyntaqliteParser {
  SyntaqliteMemMethods mem;
  const SyntaqliteDialect* dialect;
  void* lemon;
  SynqParseCtx ctx;
  const char* source;
  uint32_t source_len;
  uint32_t offset;      // Tokenizer cursor into source.
  int last_token_type;  // Last non-whitespace token fed to Lemon.
  int finished;         // 1 after EOF has been sent to Lemon.
  int had_error;        // Sticky error flag.
  char error_msg[256];  // Error message buffer.
  int trace;
  int collect_tokens;
  int sealed;
  SyntaqliteDialectConfig dialect_config;
  SYNQ_VEC(SyntaqliteComment) comments;
  SYNQ_VEC(SyntaqliteTokenPos) tokens;
  int macro_depth;  // Nesting depth (0 = not in macro).
  SYNQ_VEC(SyntaqliteMacroRegion) macros;
};

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

SyntaqliteParser* syntaqlite_create_parser_with_dialect(
    const SyntaqliteMemMethods* mem,
    const SyntaqliteDialect* dialect) {
  SyntaqliteMemMethods m = mem ? *mem : SYNTAQLITE_MEM_METHODS_DEFAULT;
  SyntaqliteParser* p = m.xMalloc(sizeof(SyntaqliteParser));
  memset(p, 0, sizeof(*p));
  p->mem = m;
  p->dialect = dialect;
  SyntaqliteDialectConfig default_config = SYNQ_DIALECT_CONFIG_DEFAULT;
  p->dialect_config = default_config;
  p->lemon = SYNQ_PARSER_ALLOC(dialect, m.xMalloc);
  synq_parse_ctx_init(&p->ctx, m);
  syntaqlite_vec_init(&p->comments);
  syntaqlite_vec_init(&p->tokens);
  syntaqlite_vec_init(&p->macros);
  return p;
}

void syntaqlite_parser_reset(SyntaqliteParser* p,
                             const char* source,
                             uint32_t len) {
  // Seal the parser on first use — configuration is frozen after this.
  p->sealed = 1;

  // Clear AST arena — keeps allocated memory for reuse.
  synq_parse_ctx_clear(&p->ctx);

  // Re-initialize lemon parser state (reuses allocation).
  SYNQ_PARSER_FINALIZE(p->dialect, p->lemon);
  SYNQ_PARSER_INIT(p->dialect, p->lemon);

  p->source = source;
  p->source_len = len;
  p->offset = 0;
  p->last_token_type = 0;
  p->finished = 0;
  p->had_error = 0;
  p->error_msg[0] = '\0';
  syntaqlite_vec_clear(&p->comments);
  syntaqlite_vec_clear(&p->tokens);
  p->macro_depth = 0;
  syntaqlite_vec_clear(&p->macros);

  // Reset parse context.
  p->ctx.source = source;
  p->ctx.config = &p->dialect_config;
  p->ctx.root = SYNTAQLITE_NULL_NODE;
  p->ctx.stmt_completed = 0;
  p->ctx.error = 0;
  p->ctx.error_offset = 0xFFFFFFFF;
  p->ctx.error_length = 0;
  p->ctx.tokens = p->collect_tokens ? &p->tokens : NULL;
}

// ---------------------------------------------------------------------------
// Internal: feed one real token to Lemon.
// Returns: 0 = keep going, 1 = statement completed, 2 = statement completed
// with error recovery (tree has ErrorNode holes), -1 = unrecoverable error.
// ---------------------------------------------------------------------------

static int feed_one_token(SyntaqliteParser* p,
                          int token_type,
                          const char* text,
                          int len,
                          uint32_t token_idx) {
  SynqParseToken minor = {
      .z = text, .n = len, .type = token_type, .token_idx = token_idx};
  SYNQ_PARSER_FEED(p->dialect, p->lemon, token_type, minor, &p->ctx);
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
  if (!p->dialect->range_meta)
    return 0;

  uint32_t node_count = syntaqlite_vec_len(&p->ctx.ast.offsets);
  const SyntaqliteMacroRegion* macros = p->macros.data;

  for (uint32_t nid = 0; nid < node_count; nid++) {
    const uint8_t* raw = (const uint8_t*)synq_arena_ptr(&p->ctx.ast, nid);
    uint32_t tag;
    memcpy(&tag, raw, sizeof(tag));
    if (tag == 0 || tag >= p->dialect->node_count)
      continue;

    const SyntaqliteRangeMetaEntry* entry = &p->dialect->range_meta[tag];
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
// Returns: 0 = done, 1 = statement completed, 2 = statement completed with
// error recovery (tree has ErrorNode holes), -1 = unrecoverable error.
// ---------------------------------------------------------------------------

static int finish_input(SyntaqliteParser* p) {
  // Nothing to do if no tokens were ever fed.
  if (p->last_token_type == 0) {
    p->finished = 1;
    return 0;
  }

  // Synthesize SEMI if the last token wasn't one.
  if (p->last_token_type != p->dialect->tk_semi) {
    int rc = feed_one_token(p, p->dialect->tk_semi, NULL, 0, 0xFFFFFFFF);
    if (rc == 1) {
      if (p->ctx.root != SYNTAQLITE_NULL_NODE) {
        p->finished = 1;
        if (check_macro_straddle(p) < 0)
          return -1;
        return p->had_error ? 2 : 1;
      }
      if (p->had_error) {
        p->finished = 1;
        return -1;  // no tree to return
      }
      // null root, no error = bare semicolon, fall through to EOF
    }
  }

  // Send end-of-input (EOF) to flush the final reduction. LALR(1) parsers
  // need one token of lookahead — the EOF provides it, triggering any
  // pending reduce (e.g. ecmd ::= cmdx SEMI).
  SynqParseToken eof = {.z = NULL, .n = 0, .type = 0, .token_idx = 0xFFFFFFFF};
  SYNQ_PARSER_FEED(p->dialect, p->lemon, 0, eof, &p->ctx);
  p->finished = 1;

  if (p->ctx.error) {
    p->had_error = 1;
    // Only set the offset if we don't already have one from an earlier error.
    if (p->ctx.error_offset == 0xFFFFFFFF) {
      p->ctx.error_offset = p->offset;
    }
    if (p->error_msg[0] == '\0') {
      snprintf(p->error_msg, sizeof(p->error_msg), "incomplete SQL statement");
    }
    return -1;
  }

  if (p->ctx.root != SYNTAQLITE_NULL_NODE) {
    if (check_macro_straddle(p) < 0)
      return -1;
    return p->had_error ? 2 : 1;
  }

  // Error recovery via `ecmd ::= error SEMI.` leaves ctx.root=NULL and
  // ctx.error=0, but had_error=1. Without this check that case returns 0
  // (no statement, no error), silently swallowing the error.
  if (p->had_error)
    return -1;

  return 0;
}

// ---------------------------------------------------------------------------
// High-level API
// ---------------------------------------------------------------------------

SyntaqliteParseResult syntaqlite_parser_next(SyntaqliteParser* p) {
  SyntaqliteParseResult result = {
      SYNTAQLITE_NULL_NODE, 0, NULL, 0xFFFFFFFF, 0, 0, 0};

  if (p->finished) {
    if (p->had_error) {
      result.error = 1;
      result.error_msg = p->error_msg;
      result.error_offset = p->ctx.error_offset;
      result.error_length = p->ctx.error_length;
    }
    return result;
  }

  // Reset per-statement state.
  p->ctx.root = SYNTAQLITE_NULL_NODE;
  p->ctx.stmt_completed = 0;
  p->ctx.error = 0;
  p->ctx.saw_subquery = 0;
  p->ctx.saw_update_delete_limit = 0;
  // Clear the error message buffer for the new statement. We do this here
  // (not when returning the error) because result.error_msg returns a pointer
  // into this buffer — clearing it before the function returns would give the
  // caller a pointer to an empty string.
  p->error_msg[0] = '\0';

  const unsigned char* z = (const unsigned char*)p->source;

  while (p->offset < p->source_len && z[p->offset] != '\0') {
    int token_type = 0;
    int64_t token_len = SynqSqliteGetTokenVersionWrapped(
        p->dialect, &p->dialect_config, z + p->offset, &token_type);
    if (token_len <= 0)
      break;

    uint32_t tok_offset = p->offset;
    p->offset += (uint32_t)token_len;

    // Skip whitespace.
    if (token_type == p->dialect->tk_space) {
      continue;
    }

    // Capture comments as comments when collect_tokens is enabled.
    if (token_type == p->dialect->tk_comment) {
      if (p->collect_tokens) {
        SyntaqliteComment t = {tok_offset, (uint32_t)token_len,
                               z[tok_offset] == '-' ? (uint8_t)0 : (uint8_t)1};
        syntaqlite_vec_push(&p->comments, t, p->mem);
      }
      continue;
    }

    // Capture non-whitespace, non-comment, non-semicolon token positions.
    // Semicolons are statement separators, not part of the AST — including
    // them would desync the token cursor with format ops.
    uint32_t tidx = 0xFFFFFFFF;
    if (p->collect_tokens && token_type != p->dialect->tk_semi) {
      SyntaqliteTokenPos tp = {tok_offset, (uint32_t)token_len,
                               (uint32_t)token_type, 0};
      syntaqlite_vec_push(&p->tokens, tp, p->mem);
      tidx = syntaqlite_vec_len(&p->tokens) - 1;
    }

    int rc = feed_one_token(p, token_type, p->source + tok_offset,
                            (int)token_len, tidx);

    // After a syntax error where SEMI is the triggering token, Lemon
    // discards it during error recovery and then keeps consuming tokens from
    // subsequent statements looking for a replacement SEMI.  Short-circuit
    // by reinitialising Lemon so the next statement starts with a clean
    // parser state.  When the triggering token is NOT a SEMI, Lemon's
    // natural `ecmd ::= error SEMI` recovery will find the real SEMI.
    if (p->had_error && rc == 0 && token_type == p->dialect->tk_semi) {
      SYNQ_PARSER_FINALIZE(p->dialect, p->lemon);
      SYNQ_PARSER_INIT(p->dialect, p->lemon);
      p->last_token_type = 0;
      rc = 1;
    }

    if (rc == 1) {
      if (p->ctx.root == SYNTAQLITE_NULL_NODE && !p->had_error) {
        continue;  // bare semicolon
      }
      if (p->had_error) {
        // Return the recovered tree (if any) alongside the error.
        result.root = p->ctx.root;
        result.saw_subquery = p->ctx.saw_subquery;
        result.saw_update_delete_limit = p->ctx.saw_update_delete_limit;
        result.error = 1;
        result.error_msg = p->error_msg;
        result.error_offset = p->ctx.error_offset;
        result.error_length = p->ctx.error_length;
        // Clear had_error (not error_msg): the caller reads error_msg through
        // the returned pointer; clearing the buffer here would give an empty
        // string. error_msg is reset at the start of the next statement.
        p->had_error = 0;
        return result;
      }
      result.root = p->ctx.root;
      result.saw_subquery = p->ctx.saw_subquery;
      result.saw_update_delete_limit = p->ctx.saw_update_delete_limit;
      return result;
    }
  }

  // End of input.
  int rc = finish_input(p);
  if (rc < 0) {
    result.error = 1;
    result.error_msg = p->error_msg;
    result.error_offset = p->ctx.error_offset;
    result.error_length = p->ctx.error_length;
    // Clear had_error (not error_msg): the caller reads error_msg through the
    // returned pointer; clearing it here would give an empty string. The
    // buffer is reset at the start of the next statement instead.
    p->had_error = 0;
  } else if (rc == 2) {
    // Error recovery succeeded: tree exists but has ErrorNode holes.
    result.root = p->ctx.root;
    result.saw_subquery = p->ctx.saw_subquery;
    result.saw_update_delete_limit = p->ctx.saw_update_delete_limit;
    result.error = 1;
    result.error_msg = p->error_msg;
    result.error_offset = p->ctx.error_offset;
    result.error_length = p->ctx.error_length;
    p->had_error = 0;
  } else if (rc == 1) {
    result.root = p->ctx.root;
    result.saw_subquery = p->ctx.saw_subquery;
    result.saw_update_delete_limit = p->ctx.saw_update_delete_limit;
  }
  return result;
}

// ---------------------------------------------------------------------------
// Low-level token-feeding API
// ---------------------------------------------------------------------------

int syntaqlite_parser_feed_token(SyntaqliteParser* p,
                                 int token_type,
                                 const char* text,
                                 int len) {
  // Skip whitespace silently.
  if (token_type == p->dialect->tk_space) {
    return 0;
  }

  // Record comments as comments but don't feed to Lemon.
  if (token_type == p->dialect->tk_comment) {
    if (p->collect_tokens && text) {
      uint32_t tok_offset = (uint32_t)(text - p->source);
      SyntaqliteComment t = {tok_offset, (uint32_t)len,
                             (uint8_t)(text[0] == '-' ? 0 : 1)};
      syntaqlite_vec_push(&p->comments, t, p->mem);
    }
    return 0;
  }

  // Capture non-whitespace, non-comment, non-semicolon token positions.
  uint32_t tidx = 0xFFFFFFFF;
  if (p->collect_tokens && text && token_type != p->dialect->tk_semi) {
    uint32_t tok_offset = (uint32_t)(text - p->source);
    SyntaqliteTokenPos tp = {tok_offset, (uint32_t)len, (uint32_t)token_type,
                             0};
    syntaqlite_vec_push(&p->tokens, tp, p->mem);
    tidx = syntaqlite_vec_len(&p->tokens) - 1;
  }

  // Reset per-statement state if starting fresh.
  if (p->last_token_type == 0 || p->ctx.root != SYNTAQLITE_NULL_NODE) {
    p->ctx.root = SYNTAQLITE_NULL_NODE;
    p->ctx.stmt_completed = 0;
    p->ctx.error = 0;
    p->ctx.saw_subquery = 0;
    p->ctx.saw_update_delete_limit = 0;
  }

  int rc = feed_one_token(p, token_type, text, len, tidx);
  if (rc < 0)
    return rc;

  if (rc == 1 && p->ctx.root == SYNTAQLITE_NULL_NODE) {
    // Bare semicolon — not a real statement.
    return 0;
  }

  if (rc == 1 && check_macro_straddle(p) < 0) {
    return -1;
  }

  return rc;
}

SyntaqliteParseResult syntaqlite_parser_result(SyntaqliteParser* p) {
  SyntaqliteParseResult result = {
      SYNTAQLITE_NULL_NODE, 0, NULL, 0xFFFFFFFF, 0, 0, 0};
  if (p->had_error) {
    result.error = 1;
    result.error_msg = p->error_msg;
    result.error_offset = p->ctx.error_offset;
    result.error_length = p->ctx.error_length;
  }
  if (p->ctx.root != SYNTAQLITE_NULL_NODE) {
    result.root = p->ctx.root;
    result.saw_subquery = p->ctx.saw_subquery;
    result.saw_update_delete_limit = p->ctx.saw_update_delete_limit;
  }
  return result;
}

int syntaqlite_parser_expected_tokens(SyntaqliteParser* p,
                                      int* out_tokens,
                                      int out_cap) {
  if (p == NULL || p->dialect == NULL ||
      p->dialect->parser_expected_tokens == NULL) {
    return 0;
  }
  return p->dialect->parser_expected_tokens(p->lemon, out_tokens, out_cap);
}

uint32_t syntaqlite_parser_completion_context(SyntaqliteParser* p) {
  if (p == NULL || p->dialect == NULL ||
      p->dialect->parser_completion_context == NULL) {
    return 0;
  }
  return p->dialect->parser_completion_context(p->lemon);
}

int syntaqlite_parser_finish(SyntaqliteParser* p) {
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

const SyntaqliteMacroRegion* syntaqlite_parser_macro_regions(
    SyntaqliteParser* p,
    uint32_t* count) {
  *count = syntaqlite_vec_len(&p->macros);
  return p->macros.data;
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

  const SyntaqliteDialect* d = p->dialect;
  if (tag == SYNTAQLITE_ERROR_NODE_TAG) {
    const SyntaqliteErrorNode* e = (const SyntaqliteErrorNode*)raw;
    SyntaqliteMemMethods mem = p->mem;
    dump_indent(b, mem, indent);
    dump_printf(b, mem, "ErrorNode { offset: %u, length: %u }\n", e->offset,
                e->length);
    return;
  }
  if (tag >= d->node_count)
    return;

  const char* name = d->node_names[tag];
  uint8_t field_count = d->field_meta_counts[tag];
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
  const SyntaqliteFieldMeta* fields = d->field_meta[tag];

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
          dump_printf(b, mem, "%s: null\n", fm->name);
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
        dump_indent(b, mem, indent + 1);
        dump_printf(b, mem, "%s: ", fm->name);
        if (val == 0) {
          dump_append(b, mem, "(none)", 6);
        } else {
          int first = 1;
          for (int bit = 0; bit < 8; bit++) {
            if (val & (1 << bit)) {
              const char* flag_name = (bit < fm->display_count && fm->display)
                                          ? fm->display[bit]
                                          : "?";
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
  // NUL-terminate
  syntaqlite_vec_push(&buf, '\0', p->mem);
  return buf.data;
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

void syntaqlite_parser_destroy(SyntaqliteParser* p) {
  if (p) {
    SYNQ_PARSER_FREE(p->dialect, p->lemon, p->mem.xFree);
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

int syntaqlite_parser_set_trace(SyntaqliteParser* p, int enable) {
  if (p->sealed)
    return -1;
  p->trace = enable;
  if (enable) {
    SYNQ_PARSER_TRACE(p->dialect, stderr, "parser> ");
  } else {
    SYNQ_PARSER_TRACE(p->dialect, NULL, NULL);
  }
  return 0;
}

int syntaqlite_parser_set_collect_tokens(SyntaqliteParser* p, int enable) {
  if (p->sealed)
    return -1;
  p->collect_tokens = enable;
  return 0;
}

int syntaqlite_parser_set_dialect_config(
    SyntaqliteParser* p,
    const SyntaqliteDialectConfig* config) {
  if (p->sealed)
    return -1;
  p->dialect_config = *config;
  return 0;
}

const SyntaqliteComment* syntaqlite_parser_comments(SyntaqliteParser* p,
                                                    uint32_t* count) {
  *count = syntaqlite_vec_len(&p->comments);
  return p->comments.data;
}

const SyntaqliteTokenPos* syntaqlite_parser_tokens(SyntaqliteParser* p,
                                                   uint32_t* count) {
  *count = syntaqlite_vec_len(&p->tokens);
  return p->tokens.data;
}
