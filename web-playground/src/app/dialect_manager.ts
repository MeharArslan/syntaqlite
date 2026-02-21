// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {DialectBinding} from "../types";
import {
  BUILTIN_DIALECT_SYMBOL,
  BUILTIN_DIALECT_WASM_PATH,
  type Engine,
  dialectSymbolFromName,
} from "./engine";

export class DialectManager {
  builtin: DialectBinding | null = null;
  uploaded: DialectBinding | null = null;
  active: DialectBinding | null = null;

  activate(engine: Engine, binding: DialectBinding): void {
    engine.setDialectPointer(binding.ptr);
    this.active = binding;
  }

  async loadBuiltin(engine: Engine): Promise<void> {
    if (!engine.ready) {
      engine.updateStatus("Runtime is not initialized yet.", true);
      m.redraw();
      return;
    }
    const binding = await engine.loadDialectFromUrl(
      BUILTIN_DIALECT_WASM_PATH,
      BUILTIN_DIALECT_SYMBOL,
    );
    binding.label = "Built-in SQLite";
    this.activate(engine, binding);
    this.builtin = binding;
  }

  async loadFromFile(engine: Engine, file: File, dialectName: string): Promise<void> {
    if (!engine.ready) {
      engine.updateStatus("Runtime is not initialized yet.", true);
      m.redraw();
      return;
    }
    const symbol = dialectSymbolFromName(dialectName);
    const url = URL.createObjectURL(file);
    try {
      const binding = await engine.loadDialectFromUrl(url, symbol);
      binding.label = file.name;
      this.activate(engine, binding);
      this.uploaded = binding;
      engine.updateStatus(`Dialect: ${file.name}`);
    } catch (err) {
      engine.updateStatus(`Failed to load dialect: ${(err as Error).message}`, true);
    } finally {
      URL.revokeObjectURL(url);
    }
    m.redraw();
  }

  clearUpload(engine: Engine): void {
    if (!engine.ready) {
      engine.updateStatus("Runtime is not initialized yet.", true);
      m.redraw();
      return;
    }
    this.uploaded = null;
    try {
      if (this.builtin) {
        this.activate(engine, this.builtin);
        engine.updateStatus("Using built-in SQLite dialect.");
      } else {
        engine.clearDialectPointer();
        this.active = null;
        engine.updateStatus("Dialect cleared.");
      }
    } catch (err) {
      engine.updateStatus(`Failed to restore built-in dialect: ${(err as Error).message}`, true);
    }
    m.redraw();
  }
}
