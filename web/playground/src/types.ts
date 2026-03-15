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
  KeywordCase,
  FormatOptions,
  FormatResult,
  AstResultOk,
  AstResultError,
  AstResult,
  DialectBinding,
  DiagnosticDetail,
  HelpDetail,
  DiagnosticEntry,
  DiagnosticsResult,
  CompletionEntry,
  CompletionsResult,
} from "syntaqlite";

// ── Playground-only types ──

export type Theme = "dark" | "light";
export type ActiveTab = "format" | "ast" | "validation" | "schema";
