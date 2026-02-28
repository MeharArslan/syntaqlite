// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import type {Engine} from "./engine";

interface TableEntry {
  name: string;
  columns: string[];
}

interface SessionContextPayload {
  tables: TableEntry[];
  views: never[];
  functions: never[];
}

/**
 * Manages user-defined schema context for validation.
 *
 * Parses a simple text format (one table per line):
 *   table_name: col1, col2, col3
 *
 * Columns are optional — a line with just a table name is valid.
 */
export class SchemaContextManager {
  rawText = "";
  private lastAppliedKey = "";

  /** Stable key for change detection (like DialectConfigManager.configKey). */
  get configKey(): string {
    return this.rawText;
  }

  /** Parse the raw text into the JSON payload for the WASM API. */
  private parse(): SessionContextPayload {
    const tables: TableEntry[] = [];
    for (const raw of this.rawText.split("\n")) {
      const line = raw.trim();
      if (line === "" || line.startsWith("#")) continue;

      const colonIdx = line.indexOf(":");
      if (colonIdx === -1) {
        // Table name only, no columns.
        const name = line.trim();
        if (name) tables.push({name, columns: []});
      } else {
        const name = line.slice(0, colonIdx).trim();
        const colsPart = line.slice(colonIdx + 1).trim();
        const columns = colsPart
          ? colsPart.split(",").map((c) => c.trim()).filter(Boolean)
          : [];
        if (name) tables.push({name, columns});
      }
    }
    return {tables, views: [], functions: []};
  }

  /** Apply the current schema to the engine. Returns true if changed. */
  apply(engine: Engine): boolean {
    const key = this.configKey;
    if (key === this.lastAppliedKey) return false;
    this.lastAppliedKey = key;

    if (this.rawText.trim() === "") {
      engine.clearSessionContext();
    } else {
      const payload = this.parse();
      engine.setSessionContext(JSON.stringify(payload));
    }
    return true;
  }

  reset(engine: Engine): void {
    this.rawText = "";
    this.lastAppliedKey = "";
    engine.clearSessionContext();
  }
}
