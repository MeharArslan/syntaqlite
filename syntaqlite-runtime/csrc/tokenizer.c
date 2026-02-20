// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#include "syntaqlite/tokenizer.h"

#include <string.h>

#include "syntaqlite/dialect.h"

struct SyntaqliteTokenizer {
  SyntaqliteMemMethods mem;
  const SyntaqliteDialect* dialect;
  const char* source;
  uint32_t len;
  uint32_t offset;
};

SyntaqliteTokenizer* syntaqlite_tokenizer_create(
    const SyntaqliteMemMethods* mem,
    const SyntaqliteDialect* dialect) {
  SyntaqliteMemMethods m = mem ? *mem : SYNTAQLITE_MEM_METHODS_DEFAULT;
  SyntaqliteTokenizer* tok = m.xMalloc(sizeof(SyntaqliteTokenizer));
  memset(tok, 0, sizeof(*tok));
  tok->mem = m;
  tok->dialect = dialect;
  return tok;
}

void syntaqlite_tokenizer_reset(SyntaqliteTokenizer* tok,
                                const char* source,
                                uint32_t len) {
  tok->source = source;
  tok->len = len;
  tok->offset = 0;
}

int syntaqlite_tokenizer_next(SyntaqliteTokenizer* tok, SyntaqliteToken* out) {
  if (tok->offset >= tok->len) {
    return 0;
  }

  int token_type = 0;
  int64_t token_len = tok->dialect->get_token(
      (const unsigned char*)tok->source + tok->offset, &token_type);

  out->text = tok->source + tok->offset;
  out->length = (uint32_t)token_len;
  out->type = (uint32_t)token_type;

  tok->offset += (uint32_t)token_len;
  return 1;
}

void syntaqlite_tokenizer_destroy(SyntaqliteTokenizer* tok) {
  if (tok) {
    tok->mem.xFree(tok);
  }
}
