// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import {DialectConfigManager} from "./dialect_config_manager";
import {DialectManager} from "./dialect_manager";
import {Engine} from "./engine";
import {ThemeManager} from "./theme_manager";

export interface Attrs {
  app: App;
}

export class App {
  theme: ThemeManager;
  runtime: Engine;
  dialect: DialectManager;
  dialectConfig: DialectConfigManager;

  constructor() {
    this.theme = new ThemeManager();
    this.runtime = new Engine();
    this.dialect = new DialectManager();
    this.dialectConfig = new DialectConfigManager();
  }
}
