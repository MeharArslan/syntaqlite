// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import {
  Engine,
  DialectManager,
  DialectConfigManager,
  SchemaContextManager,
} from "@syntaqlite/js";
import type {DiagnosticEntry, EmbeddedFragment, EmbeddedLanguage} from "@syntaqlite/js";
import {ThemeManager} from "./theme_manager";
import {WindowManager} from "./window_manager";

export interface Attrs {
  app: App;
}

export class App {
  theme: ThemeManager;
  runtime: Engine;
  dialect: DialectManager;
  dialectConfig: DialectConfigManager;
  schemaContext: SchemaContextManager;
  window: WindowManager;
  diagnostics: DiagnosticEntry[] = [];
  /** Set by the workspace to reveal a diagnostic in the editor. */
  revealDiagnostic: ((d: DiagnosticEntry) => void) | undefined = undefined;

  /** Current editor language mode. */
  languageMode: "sql" | EmbeddedLanguage = "sql";
  /** Extracted SQL fragments in embedded mode. */
  embeddedFragments: EmbeddedFragment[] = [];
  /** Selected fragment index (-1 = show all). */
  selectedFragmentIndex = -1;

  constructor() {
    this.theme = new ThemeManager();
    this.runtime = new Engine({ runtimeJsPath: "./syntaqlite-runtime.js" });
    this.dialect = new DialectManager({
      presets: [
        {
          id: "sqlite",
          label: "SQLite",
          wasmUrl: "./syntaqlite-sqlite.wasm",
          symbol: "syntaqlite_sqlite_dialect",
        },
        {
          id: "perfetto",
          label: "PerfettoSQL",
          wasmUrl: "./syntaqlite-perfetto.wasm",
          symbol: "syntaqlite_perfetto_dialect",
        },
      ],
      onDialectChanged: () => m.redraw(),
    });
    this.dialectConfig = new DialectConfigManager();
    this.schemaContext = new SchemaContextManager();
    this.window = new WindowManager();
  }
}
