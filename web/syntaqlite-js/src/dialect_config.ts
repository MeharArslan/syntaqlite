// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import type {CflagEntry, Engine} from "./engine";

export const VERSION_OPTIONS = [
  "latest",
  "3.47.0",
  "3.46.0",
  "3.45.0",
  "3.41.0",
  "3.39.0",
  "3.38.0",
  "3.35.0",
  "3.31.0",
  "3.30.0",
  "3.28.0",
  "3.25.0",
  "3.24.0",
  "3.23.0",
] as const;

/** Parse a dotted version string to the SQLite integer encoding. */
export function versionToInt(version: string): number {
  if (version === "latest") return 0x7fffffff;
  const parts = version.split(".");
  if (parts.length !== 3) return 0x7fffffff;
  const [major, minor, patch] = parts.map(Number);
  return major * 1_000_000 + minor * 1_000 + patch;
}

export class DialectConfigManager {
  version = "latest";
  enabledCflags = new Set<string>();
  availableCflags: CflagEntry[] = [];

  loadAvailableCflags(engine: Engine): void {
    this.availableCflags = engine.getCflagList();
  }

  /** Return only cflag entries whose minVersion <= selected version. */
  get visibleCflagEntries(): CflagEntry[] {
    const ver = versionToInt(this.version);
    return this.availableCflags.filter(
      (e) => e.minVersion === 0 || e.minVersion <= ver,
    );
  }

  /** Return only cflag names whose minVersion <= selected version. */
  get visibleCflags(): string[] {
    return this.visibleCflagEntries.map((e) => e.name);
  }

  apply(engine: Engine): void {
    engine.setSqliteVersion(this.version);
    engine.clearAllCflags();
    // Only apply cflags that are still visible for the current version.
    const visible = new Set(this.visibleCflags);
    for (const suffix of this.enabledCflags) {
      if (visible.has(suffix)) {
        engine.setCflag("SYNTAQLITE_CFLAG_" + suffix);
      }
    }
  }

  reset(engine: Engine): void {
    this.version = "latest";
    this.enabledCflags.clear();
    this.apply(engine);
  }

  get configKey(): string {
    const cflags = [...this.enabledCflags].sort().join(",");
    return `${this.version}|${cflags}`;
  }
}
