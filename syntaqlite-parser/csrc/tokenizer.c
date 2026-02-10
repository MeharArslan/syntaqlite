// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#include "syntaqlite/tokenizer.h"

#include <stdlib.h>
#include <string.h>

#include "csrc/sqlite_tokenize.h"

struct SyntaqliteTokenizer {
  SyntaqliteMemMethods mem;
  const char* source;
  uint32_t len;
  uint32_t offset;
};

static void* default_malloc(size_t size) { return malloc(size); }
static void default_free(void* ptr) { free(ptr); }

SyntaqliteTokenizer* syntaqlite_tokenizer_create(
    const SyntaqliteConfig* config) {
  SyntaqliteMemMethods mem;
  if (config && config->mem.xMalloc) {
    mem = config->mem;
  } else {
    mem.xMalloc = default_malloc;
    mem.xFree = default_free;
  }

  SyntaqliteTokenizer* tok = mem.xMalloc(sizeof(SyntaqliteTokenizer));
  memset(tok, 0, sizeof(*tok));
  tok->mem = mem;
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
  i64 token_len = synq_sqlite3GetToken(
      (const unsigned char*)tok->source + tok->offset, &token_type);

  out->text = tok->source + tok->offset;
  out->length = (uint32_t)token_len;
  out->type = (uint16_t)token_type;

  tok->offset += (uint32_t)token_len;
  return 1;
}

void syntaqlite_tokenizer_destroy(SyntaqliteTokenizer* tok) {
  if (tok) {
    tok->mem.xFree(tok);
  }
}
