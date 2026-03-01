// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

export { Engine, type EngineConfig, type CflagEntry } from "./engine";
export { DialectManager, BUILTIN_PRESETS, type DialectPreset, type DialectManagerConfig } from "./dialect";
export { DialectConfigManager, VERSION_OPTIONS, versionToInt } from "./dialect_config";
export { SchemaContextManager, parseSimple, type SchemaFormat, type SessionContextPayload } from "./schema";
export type {
  EmscriptenModule,
  EmscriptenModuleConfig,
  AstListNode,
  AstFieldNode,
  AstJsonNode,
  AstFieldBase,
  AstNodeField,
  AstSpanField,
  AstBoolField,
  AstEnumField,
  AstFlagsField,
  AstField,
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
  EmbeddedLanguage,
  EmbeddedHole,
  EmbeddedFragment,
  EmbeddedExtractResult,
} from "./types";
