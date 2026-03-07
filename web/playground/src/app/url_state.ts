// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import LZString from "lz-string";

export type Dialect = "sqlite" | "perfetto";
export type LanguageMode = "sql" | "python" | "typescript";
export type SchemaFormat = "simple" | "ddl";
export type OutputTab = "format" | "ast";
export type AstViewMode = "outline" | "graph";

export interface PlaygroundState {
  dialect: Dialect;
  languageMode: LanguageMode;
  sqliteVersion: string;
  cflags: string[];
  /**
   * Named preset ID, or null when the user has edited SQL (custom mode).
   * Mutually exclusive with `sql`.
   */
  preset: string | null;
  /**
   * Custom SQL content. Only populated when preset is null.
   * Mutually exclusive with `preset`.
   */
  sql: string | null;
  schemaFormat: SchemaFormat;
  schema: string;
  /** Active output tab (desktop only; mobile-only tabs are never serialized). */
  outputTab: OutputTab;
  /** AST view mode: outline list or graph. */
  astViewMode: AstViewMode;
}

export const DEFAULT_STATE: PlaygroundState = {
  dialect: "sqlite",
  languageMode: "sql",
  sqliteVersion: "latest",
  cflags: [],
  preset: null,
  sql: null,
  schemaFormat: "simple",
  schema: "",
  outputTab: "format",
  astViewMode: "outline",
};

/**
 * Parse a raw URL hash string (without the leading `#`) into a PlaygroundState.
 * Unknown or invalid params are ignored; missing params take their default value.
 * Exported so that unit tests can exercise this logic without browser globals.
 */
export function parseHash(hash: string): PlaygroundState {
  if (!hash) return {...DEFAULT_STATE};

  const params = new URLSearchParams(hash);
  const s: PlaygroundState = {...DEFAULT_STATE};

  const d = params.get("d");
  if (d === "sqlite" || d === "perfetto") s.dialect = d;

  const l = params.get("l");
  if (l === "sql" || l === "python" || l === "typescript") s.languageMode = l;

  const v = params.get("v");
  if (v) s.sqliteVersion = v;

  const f = params.get("f");
  if (f) s.cflags = f.split(",").filter(Boolean);

  const sf = params.get("sf");
  if (sf === "simple" || sf === "ddl") s.schemaFormat = sf;

  const sc = params.get("sc");
  if (sc) {
    const decoded = LZString.decompressFromEncodedURIComponent(sc);
    if (decoded) s.schema = decoded;
  }

  const ot = params.get("ot");
  if (ot === "format" || ot === "ast") s.outputTab = ot;

  const av = params.get("av");
  if (av === "outline" || av === "graph") s.astViewMode = av;

  // `s` (custom SQL) and `p` (named preset) are mutually exclusive.
  const sqlParam = params.get("s");
  const presetParam = params.get("p");
  if (sqlParam) {
    const decoded = LZString.decompressFromEncodedURIComponent(sqlParam);
    if (decoded !== null) {
      s.preset = null;
      s.sql = decoded;
    }
  } else if (presetParam) {
    s.preset = presetParam;
    s.sql = null;
  }

  return s;
}

/**
 * Serialize a PlaygroundState to a URL hash string (without the leading `#`).
 * Default values are omitted to keep URLs short.
 * Exported so that unit tests can exercise this logic without browser globals.
 */
export function serializeHash(state: PlaygroundState, customDialect = false): string {
  const params = new URLSearchParams();

  if (state.dialect !== "sqlite") params.set("d", state.dialect);
  if (state.languageMode !== "sql") params.set("l", state.languageMode);
  if (state.sqliteVersion !== "latest") params.set("v", state.sqliteVersion);
  if (state.cflags.length > 0) params.set("f", state.cflags.join(","));
  if (state.schemaFormat !== "simple") params.set("sf", state.schemaFormat);
  if (state.schema) params.set("sc", LZString.compressToEncodedURIComponent(state.schema));

  if (state.preset !== null) {
    params.set("p", state.preset);
  } else if (state.sql !== null) {
    params.set("s", LZString.compressToEncodedURIComponent(state.sql));
  }

  if (state.outputTab !== "format") params.set("ot", state.outputTab);
  if (state.astViewMode !== "outline") params.set("av", state.astViewMode);

  // Signal to recipients that a custom dialect was active when this URL was
  // generated, so they can show an explanatory notice.
  if (customDialect) params.set("cd", "1");

  return params.toString();
}

/**
 * Manages serializable playground state and URL hash persistence.
 *
 * Discrete state changes (dialect, preset, version, cflags, schema format) are
 * written to the URL immediately. Text content changes (sql, schema) are
 * debounced so keystrokes don't each trigger a rewrite.
 */
export class UrlStateManager {
  current: PlaygroundState;

  /**
   * True when the page was loaded from a URL that was shared while a custom
   * (uploaded) dialect was active.  Set once at construction from the `cd=1`
   * URL param; never written back into the URL by the recipient's session so
   * it disappears as soon as any state change rewrites the hash.
   */
  readonly hadCustomDialect: boolean;

  /**
   * Whether the current session has a custom dialect active.  Written by the
   * workspace via `setCustomDialect()` and included in URL serialization as
   * `cd=1` so recipients know the SQL may use non-standard syntax.
   */
  private dialectIsCustom = false;

  private debounceTimer: ReturnType<typeof setTimeout> | undefined;

  constructor() {
    const hash = window.location.hash.slice(1);
    const params = new URLSearchParams(hash);
    this.hadCustomDialect = params.get("cd") === "1";
    this.current = parseHash(hash);
  }

  /**
   * Inform the state manager whether a custom (non-shareable) dialect is
   * currently active.  When true, `cd=1` is included in the serialized URL so
   * recipients receive a contextual notice.
   */
  setCustomDialect(isCustom: boolean): void {
    this.dialectIsCustom = isCustom;
  }

  /**
   * Merge a partial state patch and persist to the URL hash.
   * Pure text patches (`sql`, `schema` only) are debounced at 800 ms.
   * Any patch that includes non-text fields is written immediately.
   */
  update(patch: Partial<PlaygroundState>): void {
    Object.assign(this.current, patch);
    const isTextOnly = Object.keys(patch).every((k) => k === "sql" || k === "schema");
    if (isTextOnly) {
      clearTimeout(this.debounceTimer);
      this.debounceTimer = setTimeout(() => this.flush(), 800);
    } else {
      // Cancel any pending text debounce so the write includes latest text too.
      clearTimeout(this.debounceTimer);
      this.flush();
    }
  }

  /** Force an immediate URL write, cancelling any pending debounce. */
  flush(): void {
    clearTimeout(this.debounceTimer);
    this.debounceTimer = undefined;
    const serialized = serializeHash(this.current, this.dialectIsCustom);
    const newUrl = serialized
      ? `${location.pathname}${location.search}#${serialized}`
      : location.pathname + location.search;
    history.replaceState(null, "", newUrl);
  }
}
