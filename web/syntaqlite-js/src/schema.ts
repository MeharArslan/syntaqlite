// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import type {Engine} from "./engine";

export type SchemaFormat = "simple" | "ddl";

interface TableEntry {
  name: string;
  columns: string[];
}

export interface SessionContextPayload {
  tables: TableEntry[];
  views: never[];
  functions: never[];
}

/** Parse "simple" schema format into a session context payload. */
export function parseSimple(rawText: string): SessionContextPayload {
  const tables: TableEntry[] = [];
  for (const raw of rawText.split("\n")) {
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

/**
 * Manages user-defined schema context for validation.
 *
 * Supports two formats:
 *   simple — one table per line: `table_name: col1, col2, col3`
 *   ddl    — `CREATE TABLE name (col1 TYPE, col2 TYPE, ...);`
 */
export class SchemaContextManager {
  rawText = "";
  format: SchemaFormat = "simple";
  parseError: string | undefined = undefined;
  private lastAppliedKey = "";

  /** Stable key for change detection (like DialectConfigManager.configKey). */
  get configKey(): string {
    return `${this.format}:${this.rawText}`;
  }

  /** Apply the current schema to the engine. Returns true if changed. */
  apply(engine: Engine): boolean {
    const key = this.configKey;
    if (key === this.lastAppliedKey) return false;
    this.lastAppliedKey = key;

    if (this.rawText.trim() === "") {
      this.parseError = undefined;
      engine.clearSessionContext();
    } else if (this.format === "ddl") {
      const result = engine.setSessionContextDdl(this.rawText);
      if (result.ok) {
        this.parseError = undefined;
      } else {
        this.parseError = result.error;
      }
    } else {
      this.parseError = undefined;
      const payload = parseSimple(this.rawText);
      engine.setSessionContext(JSON.stringify(payload));
    }
    return true;
  }

  reset(engine: Engine): void {
    this.rawText = "";
    this.parseError = undefined;
    this.lastAppliedKey = "";
    engine.clearSessionContext();
  }
}
