// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/** Emscripten module augmented with dynamic linking support. */
export interface EmscriptenModule {
  HEAPU8: Uint8Array;
  loadDynamicLibrary: (
    url: string,
    opts: {loadAsync: boolean; global: boolean; nodelete: boolean},
    scope?: Record<string, unknown>,
  ) => unknown;
  ccall: (ident: string, returnType: string, argTypes: string[], args: unknown[]) => unknown;
  cwrap: (ident: string, returnType: string, argTypes: string[]) => (...args: unknown[]) => number;
  [key: `_${string}`]: ((...args: number[]) => number) | undefined;
}

/** Module config passed to Emscripten before initialization. */
export interface EmscriptenModuleConfig {
  noInitialRun: boolean;
  locateFile: (path: string) => string;
  onRuntimeInitialized: () => void;
  onAbort: (reason: string) => void;
  // Emscripten populates these after init:
  HEAPU8?: Uint8Array;
  loadDynamicLibrary?: EmscriptenModule["loadDynamicLibrary"];
  ccall?: EmscriptenModule["ccall"];
  cwrap?: EmscriptenModule["cwrap"];
  [key: string]: unknown;
}

declare global {
  interface Window {
    Module: EmscriptenModuleConfig;
    HEAPU8?: Uint8Array;
  }
}

// ── AST JSON types ──

export interface AstListNode {
  type: "list";
  name: string;
  count: number;
  children: AstJsonNode[];
}

export interface AstFieldNode {
  type: "node";
  name: string;
  fields: AstField[];
}

export type AstJsonNode = AstListNode | AstFieldNode;

export interface AstFieldBase {
  label: string;
}

export interface AstNodeField extends AstFieldBase {
  kind: "node";
  child: AstJsonNode | undefined;
}

export interface AstSpanField extends AstFieldBase {
  kind: "span";
  value: string | undefined;
}

export interface AstBoolField extends AstFieldBase {
  kind: "bool";
  value: boolean;
}

export interface AstEnumField extends AstFieldBase {
  kind: "enum";
  value: string | undefined;
}

export interface AstFlagsField extends AstFieldBase {
  kind: "flags";
  value: string[];
}

export type AstField = AstNodeField | AstSpanField | AstBoolField | AstEnumField | AstFlagsField;

// ── Format types ──

export type KeywordCase = 0 | 1 | 2; // 0=preserve, 1=upper, 2=lower

export interface FormatOptions {
  lineWidth: number;
  keywordCase: KeywordCase;
  semicolons: boolean;
}

export interface FormatResult {
  ok: boolean;
  text: string;
}

// ── AST result types ──

export interface AstResultOk {
  ok: true;
  statements: AstJsonNode[];
}

export interface AstResultError {
  ok: false;
  error: string;
}

export type AstResult = AstResultOk | AstResultError;

// ── Dialect types ──

export interface DialectBinding {
  symbol: string;
  ptr: number;
  label: string;
}

// ── Diagnostics types ──

/** Structured detail for the diagnostic message, matching `DiagnosticMessage` in Rust. */
export type DiagnosticDetail =
  | {kind: "unknown_table"; name: string}
  | {kind: "unknown_column"; column: string; table?: string}
  | {kind: "unknown_function"; name: string}
  | {kind: "function_arity"; name: string; expected: number[]; got: number}
  | null;

/** Structured detail for the help, matching `Help` in Rust. */
export type HelpDetail = {kind: "suggestion"; value: string} | null;

export interface DiagnosticEntry {
  startOffset: number;
  endOffset: number;
  /** Human-readable message (Display string). */
  message: string;
  /** Structured detail for machine consumption. `null` for parse errors. */
  detail: DiagnosticDetail;
  severity: "error" | "warning" | "info" | "hint";
  /** Human-readable help text (Display string). */
  help?: string;
  /** Structured help for machine consumption. */
  helpDetail?: HelpDetail;
  /** 1-based line number, populated by the consumer after offset conversion. */
  line?: number;
  /** 1-based column number, populated by the consumer after offset conversion. */
  col?: number;
  /** 1-based statement index (semicolon-delimited), populated by the consumer. */
  stmtIndex?: number;
}

export interface DiagnosticsResult {
  ok: boolean;
  diagnostics: DiagnosticEntry[];
}

// ── Embedded SQL types ──

export type EmbeddedLanguage = "python" | "typescript";

export interface EmbeddedHole {
  start: number;
  end: number;
  placeholder: string;
}

export interface EmbeddedFragment {
  start: number;
  end: number;
  sql: string;
  holes: EmbeddedHole[];
}

export interface EmbeddedExtractResult {
  ok: boolean;
  fragments: EmbeddedFragment[];
}

// ── Completion types ──

export interface CompletionEntry {
  label: string;
  kind: "keyword" | "function" | "class";
}

export interface CompletionsResult {
  ok: boolean;
  items: CompletionEntry[];
}
