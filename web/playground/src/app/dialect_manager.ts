// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {DialectBinding} from "../types";
import type {Engine} from "./engine";

export interface DialectPreset {
  id: string;
  label: string;
  wasm: string;
  symbol: string;
}

export const DIALECT_PRESETS: DialectPreset[] = [
  {
    id: "sqlite",
    label: "SQLite",
    wasm: "./syntaqlite-sqlite.wasm",
    symbol: "syntaqlite_sqlite_dialect",
  },
  {
    id: "perfetto",
    label: "PerfettoSQL",
    wasm: "./syntaqlite-perfetto.wasm",
    symbol: "syntaqlite_perfetto_dialect",
  },
];

export class DialectManager {
  activePresetId: string = DIALECT_PRESETS[0].id;
  active: DialectBinding | undefined = undefined;
  customLabel: string | undefined = undefined;

  private cache = new Map<string, DialectBinding>();

  async loadDefault(engine: Engine): Promise<void> {
    await this.selectPreset(engine, DIALECT_PRESETS[0]);
  }

  async selectPreset(engine: Engine, preset: DialectPreset): Promise<void> {
    if (!engine.ready) {
      engine.updateStatus("Runtime is not initialized yet.", true);
      m.redraw();
      return;
    }
    try {
      const binding = await this.loadCached(engine, preset.wasm, preset.symbol, preset.label);
      engine.setDialectPointer(binding.ptr);
      this.active = binding;
      this.activePresetId = preset.id;
      this.customLabel = undefined;
      engine.updateStatus(`Dialect: ${preset.label}`);
    } catch (err) {
      engine.updateStatus(`Failed to load ${preset.label}: ${(err as Error).message}`, true);
    }
    m.redraw();
  }

  async loadFromFile(engine: Engine, file: File, symbol: string): Promise<string | undefined> {
    if (!engine.ready) {
      return "Runtime is not initialized yet.";
    }
    const url = URL.createObjectURL(file);
    try {
      const binding = await engine.loadDialectFromUrl(url, symbol);
      binding.label = file.name;
      engine.setDialectPointer(binding.ptr);
      this.active = binding;
      this.activePresetId = "custom";
      this.customLabel = file.name;
      engine.updateStatus(`Dialect: ${file.name}`);
      return undefined;
    } catch (err) {
      return (err as Error).message;
    } finally {
      URL.revokeObjectURL(url);
    }
  }

  private async loadCached(
    engine: Engine,
    wasm: string,
    symbol: string,
    label: string,
  ): Promise<DialectBinding> {
    const cached = this.cache.get(symbol);
    if (cached) return cached;
    const binding = await engine.loadDialectFromUrl(wasm, symbol);
    binding.label = label;
    this.cache.set(symbol, binding);
    return binding;
  }
}
