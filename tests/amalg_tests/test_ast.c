// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Minimal AST dumper for amalgamation integration tests.
//
// Compiled against a generated syntaqlite_<dialect>.{h,c} amalgamation.
// Reads SQL from stdin, parses each statement, and dumps the AST.
// The DIALECT_HEADER and DIALECT_FN macros are set at compile time to
// select the dialect header and dialect accessor function.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include DIALECT_HEADER

int main(void) {
  static char buf[256 * 1024];
  size_t n = fread(buf, 1, sizeof(buf) - 1, stdin);
  buf[n] = '\0';

  SyntaqliteDialectEnv env = SYNQ_DIALECT_ENV_DEFAULT(DIALECT_FN());
  SyntaqliteParser* p =
      syntaqlite_create_parser_with_grammar(NULL, &env);
  syntaqlite_parser_reset(p, buf, (uint32_t)n);

  SyntaqliteParseResult result;
  int count = 0;

  while ((result = syntaqlite_parser_next(p)).root != SYNTAQLITE_NULL_NODE) {
    if (result.error) {
      fprintf(stderr, "parse error: %s\n",
              result.error_msg ? result.error_msg : "unknown");
      syntaqlite_parser_destroy(p);
      return 1;
    }
    if (count > 0)
      printf("----\n");
    char* dump = syntaqlite_dump_node(p, result.root, 0);
    if (dump) {
      fputs(dump, stdout);
      free(dump);
    }
    count++;
  }

  syntaqlite_parser_destroy(p);
  return 0;
}
