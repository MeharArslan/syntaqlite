import {describe, expect, it} from "vitest";
import {DialectConfigManager, versionToInt} from "./dialect_config_manager";

describe("versionToInt", () => {
  it("returns max int for 'latest'", () => {
    expect(versionToInt("latest")).toBe(0x7fffffff);
  });

  it("encodes 3-part version correctly", () => {
    expect(versionToInt("3.46.0")).toBe(3_046_000);
    expect(versionToInt("3.24.0")).toBe(3_024_000);
    expect(versionToInt("3.47.2")).toBe(3_047_002);
  });

  it("returns max int for malformed input", () => {
    expect(versionToInt("3.46")).toBe(0x7fffffff);
    expect(versionToInt("bad")).toBe(0x7fffffff);
  });
});

describe("DialectConfigManager", () => {
  it("visibleCflagEntries filters by version", () => {
    const mgr = new DialectConfigManager();
    mgr.availableCflags = [
      {name: "A", minVersion: 3_046_000, category: "x"},
      {name: "B", minVersion: 3_030_000, category: "x"},
    ];
    mgr.version = "3.35.0";
    expect(mgr.visibleCflagEntries.map((e) => e.name)).toEqual(["B"]);
  });

  it("visibleCflagEntries includes minVersion=0 for all versions", () => {
    const mgr = new DialectConfigManager();
    mgr.availableCflags = [
      {name: "ALWAYS", minVersion: 0, category: "x"},
      {name: "NEW", minVersion: 3_046_000, category: "x"},
    ];
    mgr.version = "3.24.0";
    expect(mgr.visibleCflagEntries.map((e) => e.name)).toEqual(["ALWAYS"]);
  });

  it("configKey is deterministic with sorted cflags", () => {
    const mgr = new DialectConfigManager();
    mgr.version = "3.46.0";
    mgr.enabledCflags = new Set(["Z", "A", "M"]);
    expect(mgr.configKey).toBe("3.46.0|A,M,Z");
  });
});
