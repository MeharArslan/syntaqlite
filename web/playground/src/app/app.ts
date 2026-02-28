// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import {DialectConfigManager} from "./dialect_config_manager";
import {DialectManager} from "./dialect_manager";
import {Engine} from "./engine";
import {SchemaContextManager} from "./schema_context";
import {ThemeManager} from "./theme_manager";
import {WindowManager} from "./window_manager";
import type {DiagnosticEntry} from "../types";

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

  constructor() {
    this.theme = new ThemeManager();
    this.runtime = new Engine();
    this.dialect = new DialectManager();
    this.dialectConfig = new DialectConfigManager();
    this.schemaContext = new SchemaContextManager();
    this.window = new WindowManager();
  }
}
