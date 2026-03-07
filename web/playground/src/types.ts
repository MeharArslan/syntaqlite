// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Re-export all library types so existing component imports from "../types" continue to work.
export type {
  EmscriptenModule,
  EmscriptenModuleConfig,
  AstFieldValue,
  AstListNode,
  AstRegularNode,
  AstJsonNode,
  AstResultOk,
  AstResultError,
  AstResult,
  KeywordCase,
  FormatOptions,
  FormatResult,
  DialectBinding,
  DiagnosticDetail,
  HelpDetail,
  DiagnosticEntry,
  DiagnosticsResult,
  CompletionEntry,
  CompletionsResult,
} from "@syntaqlite/js";

// ── Playground-only types ──

export type Theme = "dark" | "light";
export type ActiveTab = "format" | "ast" | "validation" | "schema";
export type AstViewMode = "outline" | "graph";
