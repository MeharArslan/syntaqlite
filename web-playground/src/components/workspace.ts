// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import * as monaco from "monaco-editor";
import type {Attrs} from "../app/app";
import type {Engine} from "../app/engine";
import {getSqlPresetLibrary} from "./workspace/sql_presets";
import {debounce} from "../base/debounce";
import type {DiagnosticEntry} from "../types";
import {EditorPane} from "./editor_pane";
import {OutputPanel} from "./output_panel";
import "./workspace.css";

const SEVERITY_MAP: Record<DiagnosticEntry["severity"], monaco.MarkerSeverity> = {
  error: monaco.MarkerSeverity.Error,
  warning: monaco.MarkerSeverity.Warning,
  info: monaco.MarkerSeverity.Info,
  hint: monaco.MarkerSeverity.Hint,
};
const CUSTOM_PRESET_ID = "custom";

/** Convert a byte offset to a 1-based line and column in `source`. */
function offsetToLineCol(source: string, offset: number): {line: number; col: number} {
  const clamped = Math.min(offset, source.length);
  let line = 1;
  let col = 1;
  for (let i = 0; i < clamped; i++) {
    if (source[i] === "\n") {
      line++;
      col = 1;
    } else {
      col++;
    }
  }
  return {line, col};
}

export class Workspace implements m.ClassComponent<Attrs> {
  private sql = "select a, b from t where c = 1;";
  private editor: monaco.editor.IStandaloneCodeEditor | null = null;
  private debouncedUpdate: (sql: string) => void;
  private debouncedDiagnostics: (engine: Engine, sql: string) => void;
  private presetSelectionByDialect = new Map<string, string>();
  private customSqlByDialect = new Map<string, string>();
  private applyingPreset = false;
  private lastAppliedDialectPtr: number | null = null;
  /** Track the last dialect pointer we ran diagnostics against. */
  private lastDiagnosticDialectPtr: number | null = null;
  private lastDiagnosticConfigKey: string | null = null;

  constructor() {
    this.debouncedUpdate = debounce((sql: string) => {
      this.sql = sql;
      m.redraw();
    }, 150);

    this.debouncedDiagnostics = debounce(
      (engine: Engine, sql: string) => this.updateDiagnostics(engine, sql),
      100,
    );
  }

  view(vnode: m.Vnode<Attrs>) {
    const {app} = vnode.attrs;
    const presetLibrary = getSqlPresetLibrary(app.dialect.activePresetId);
    const selectedPresetId = this.ensurePresetSelection(presetLibrary);

    // Apply dialect-specific preset SQL and refresh analysis whenever the
    // active dialect pointer changes.
    if (app.runtime.ready && app.dialect.active) {
      const dPtr = app.dialect.active.ptr;
      const cfgKey = app.dialectConfig.configKey;
      if (dPtr !== this.lastAppliedDialectPtr) {
        this.lastAppliedDialectPtr = dPtr;
        this.applySelectionForDialect(app.runtime, presetLibrary.dialectId, selectedPresetId, true);
        this.lastDiagnosticDialectPtr = dPtr;
        this.lastDiagnosticConfigKey = cfgKey;
      } else if (
        this.editor &&
        (dPtr !== this.lastDiagnosticDialectPtr || cfgKey !== this.lastDiagnosticConfigKey)
      ) {
        this.updateDiagnostics(app.runtime, this.sql);
        this.lastDiagnosticDialectPtr = dPtr;
        this.lastDiagnosticConfigKey = cfgKey;
      }
    }

    return m("section.sq-workspace", [
      m(EditorPane, {
        theme: app.theme.current,
        initialSql: this.sql,
        presets: presetLibrary.presets,
        selectedPresetId,
        onPresetChange: (presetId: string) => {
          this.presetSelectionByDialect.set(presetLibrary.dialectId, presetId);
          this.applySelectionForDialect(app.runtime, presetLibrary.dialectId, presetId);
        },
        onContentChange: (s: string) => {
          if (!this.applyingPreset) {
            const current = this.presetSelectionByDialect.get(presetLibrary.dialectId);
            if (current !== CUSTOM_PRESET_ID) {
              this.presetSelectionByDialect.set(presetLibrary.dialectId, CUSTOM_PRESET_ID);
              m.redraw();
            }
            this.customSqlByDialect.set(presetLibrary.dialectId, s);
          }
          this.debouncedUpdate(s);
          this.debouncedDiagnostics(app.runtime, s);
        },
        onEditorCreated: (editor) => {
          this.editor = editor;
        },
      }),
      m(OutputPanel, {app, sql: this.sql}),
    ]);
  }

  private ensurePresetSelection(presetLibrary: ReturnType<typeof getSqlPresetLibrary>): string {
    const {dialectId, presets} = presetLibrary;
    if (presets.length === 0) return "";
    const selected = this.presetSelectionByDialect.get(dialectId);
    if (selected === CUSTOM_PRESET_ID) return selected;
    if (selected && presets.some((preset) => preset.id === selected)) return selected;
    const first = presets[0].id;
    this.presetSelectionByDialect.set(dialectId, first);
    return first;
  }

  private applyPresetById(
    engine: Engine,
    dialectId: string,
    presetId: string,
    forceEditorRefresh = false,
  ): void {
    const presetLibrary = getSqlPresetLibrary(dialectId);
    const preset = presetLibrary.presets.find((item) => item.id === presetId);
    if (!preset) return;

    this.applySql(engine, preset.sql, forceEditorRefresh);
  }

  private applySelectionForDialect(
    engine: Engine,
    dialectId: string,
    presetId: string,
    forceEditorRefresh = false,
  ): void {
    if (presetId === CUSTOM_PRESET_ID) {
      const customSql = this.customSqlByDialect.get(dialectId);
      if (customSql !== undefined) {
        this.applySql(engine, customSql, forceEditorRefresh);
        return;
      }
      const fallback = getSqlPresetLibrary(dialectId).presets[0];
      if (fallback) {
        this.presetSelectionByDialect.set(dialectId, fallback.id);
        this.applySql(engine, fallback.sql, forceEditorRefresh);
      }
      return;
    }
    this.applyPresetById(engine, dialectId, presetId, forceEditorRefresh);
  }

  private applySql(engine: Engine, sql: string, forceEditorRefresh = false): void {
    this.applyingPreset = true;
    try {
      this.sql = sql;
      if (this.editor && (forceEditorRefresh || this.editor.getValue() !== sql)) {
        // Force setValue on dialect switch so Monaco requests fresh semantic tokens.
        this.editor.setValue(sql);
      }
    } finally {
      this.applyingPreset = false;
    }
    this.updateDiagnostics(engine, sql);
    m.redraw();
  }

  private updateDiagnostics(engine: Engine, sql: string): void {
    if (!engine.ready || !this.editor) return;

    const model = this.editor.getModel();
    if (!model) return;
    const result = engine.runDiagnostics(sql, model.getVersionId());
    if (!result.ok) {
      monaco.editor.setModelMarkers(model, "syntaqlite", []);
      return;
    }

    const markers: monaco.editor.IMarkerData[] = result.diagnostics.map((d) => {
      const start = offsetToLineCol(sql, d.startOffset);
      const end = offsetToLineCol(sql, d.endOffset);
      return {
        severity: SEVERITY_MAP[d.severity] ?? monaco.MarkerSeverity.Error,
        message: d.message,
        startLineNumber: start.line,
        startColumn: start.col,
        endLineNumber: end.line,
        endColumn: end.col,
      };
    });

    monaco.editor.setModelMarkers(model, "syntaqlite", markers);
  }
}
