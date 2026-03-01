// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import * as monaco from "monaco-editor";
import type {App, Attrs} from "../app/app";
import type {Engine, EmbeddedLanguage} from "@syntaqlite/js";
import {getSqlPresetLibrary} from "./workspace/sql_presets";
import {debounce} from "../base/debounce";
import type {DiagnosticEntry} from "../types";
import {EditorPane, type LanguageMode} from "./editor_pane";
import {OutputPanel} from "./output_panel";
import {ResizeHandle} from "../widgets/resize_handle";
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

/** Return the 1-based statement index for a byte offset (semicolons delimit statements). */
function countStatements(source: string, offset: number): number {
  let count = 1;
  for (let i = 0; i < offset && i < source.length; i++) {
    if (source[i] === ";") count++;
  }
  return count;
}

export class Workspace implements m.ClassComponent<Attrs> {
  private sql = "select a, b from t where c = 1;";
  private editor: monaco.editor.IStandaloneCodeEditor | undefined = undefined;
  private debouncedUpdate: (sql: string) => void;
  private debouncedDiagnostics: (engine: Engine, sql: string) => void;
  private debouncedEmbeddedDiagnostics: (app: App, sql: string) => void;
  private presetSelectionByDialect = new Map<string, string>();
  private customSqlByDialect = new Map<string, string>();
  private appRef: App | undefined = undefined;
  private applyingPreset = false;
  private lastAppliedDialectPtr: number | undefined = undefined;
  /** Track the last dialect pointer we ran diagnostics against. */
  private lastDiagnosticDialectPtr: number | undefined = undefined;
  private lastDiagnosticConfigKey: string | undefined = undefined;
  private lastSchemaKey: string | undefined = undefined;
  /** Left pane fraction (0..1), undefined = equal split. */
  private splitFraction: number | undefined = undefined;
  private workspaceEl: HTMLElement | undefined = undefined;

  constructor() {
    this.debouncedUpdate = debounce((sql: string) => {
      this.sql = sql;
      m.redraw();
    }, 150);

    this.debouncedDiagnostics = debounce(
      (engine: Engine, sql: string) => this.updateDiagnostics(engine, sql),
      100,
    );

    this.debouncedEmbeddedDiagnostics = debounce(
      (app: App, sql: string) => this.updateEmbeddedDiagnostics(app, sql),
      100,
    );
  }

  view(vnode: m.Vnode<Attrs>) {
    const {app} = vnode.attrs;
    this.appRef = app;
    const presetKey = app.languageMode === "sql"
      ? app.dialect.activePresetId
      : `${app.dialect.activePresetId}:${app.languageMode}`;
    const presetLibrary = getSqlPresetLibrary(presetKey);
    const selectedPresetId = this.ensurePresetSelection(presetLibrary);

    // Apply dialect-specific preset SQL and refresh analysis whenever the
    // active dialect pointer changes.
    if (app.runtime.ready && app.dialect.active) {
      const dPtr = app.dialect.active.ptr;
      const cfgKey = app.dialectConfig.configKey;
      const schemaKey = app.schemaContext.configKey;
      if (dPtr !== this.lastAppliedDialectPtr) {
        this.lastAppliedDialectPtr = dPtr;
        this.applySelectionForDialect(app.runtime, presetLibrary.dialectId, selectedPresetId, true);
        this.lastDiagnosticDialectPtr = dPtr;
        this.lastDiagnosticConfigKey = cfgKey;
        this.lastSchemaKey = schemaKey;
      } else if (
        this.editor &&
        (dPtr !== this.lastDiagnosticDialectPtr ||
          cfgKey !== this.lastDiagnosticConfigKey ||
          schemaKey !== this.lastSchemaKey)
      ) {
        this.updateDiagnostics(app.runtime, this.sql);
        this.lastDiagnosticDialectPtr = dPtr;
        this.lastDiagnosticConfigKey = cfgKey;
        this.lastSchemaKey = schemaKey;
      }
    }

    const splitStyle: Record<string, string> = {};
    if (this.splitFraction != undefined) {
      const pct = (this.splitFraction * 100).toFixed(2);
      splitStyle.gridTemplateColumns = `${pct}% 5px 1fr`;
    }

    return m("section.sq-workspace", {
      style: splitStyle,
      oncreate: (v: m.VnodeDOM) => { this.workspaceEl = v.dom as HTMLElement; },
    }, [
      m(EditorPane, {
        theme: app.theme.current,
        initialSql: this.sql,
        presets: presetLibrary.presets,
        selectedPresetId,
        languageMode: app.languageMode,
        onLanguageChange: (lang: LanguageMode) => {
          app.languageMode = lang;
          app.embeddedFragments = [];
          app.selectedFragmentIndex = -1;
          app.diagnostics = [];
          if (this.editor) {
            const model = this.editor.getModel();
            if (model) monaco.editor.setModelMarkers(model, "syntaqlite", []);
          }
          // Apply the first preset of the new language.
          const newPresetKey = lang === "sql"
            ? app.dialect.activePresetId
            : `${app.dialect.activePresetId}:${lang}`;
          const newLib = getSqlPresetLibrary(newPresetKey);
          const sel = this.ensurePresetSelection(newLib);
          this.applySelectionForDialect(app.runtime, newLib.dialectId, sel, true);
          m.redraw();
        },
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
          if (app.languageMode === "sql") {
            this.debouncedDiagnostics(app.runtime, s);
          } else {
            this.debouncedEmbeddedDiagnostics(app, s);
          }
        },
        onEditorCreated: (editor) => {
          this.editor = editor;
          app.revealDiagnostic = (d) => this.revealDiagnostic(d);
        },
      }),
      m(ResizeHandle, {
        axis: "vertical",
        onResize: (delta: number) => {
          const el = this.workspaceEl;
          if (!el) return;
          const totalWidth = el.getBoundingClientRect().width;
          const handleWidth = 5;
          const available = totalWidth - handleWidth;
          const current = this.splitFraction ?? 0.5;
          const currentPx = current * available;
          const minPx = 100;
          const newPx = Math.max(minPx, Math.min(available - minPx, currentPx + delta));
          this.splitFraction = newPx / available;
        },
        onResizeEnd: () => {
          if (this.editor) this.editor.layout();
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
    if (this.appRef && this.appRef.languageMode !== "sql") {
      this.updateEmbeddedDiagnostics(this.appRef, sql);
    } else {
      this.updateDiagnostics(engine, sql);
    }
    m.redraw();
  }

  private updateDiagnostics(engine: Engine, sql: string): void {
    if (!engine.ready || !this.editor) return;

    const model = this.editor.getModel();
    if (!model) return;
    const result = engine.runDiagnostics(sql, model.getVersionId());
    if (!result.ok) {
      monaco.editor.setModelMarkers(model, "syntaqlite", []);
      if (this.appRef) this.appRef.diagnostics = [];
      return;
    }

    // Enrich diagnostics with line/col/statement info for the details panel.
    for (const d of result.diagnostics) {
      const pos = offsetToLineCol(sql, d.startOffset);
      d.line = pos.line;
      d.col = pos.col;
      d.stmtIndex = countStatements(sql, d.startOffset);
    }

    if (this.appRef) this.appRef.diagnostics = result.diagnostics;

    const markers: monaco.editor.IMarkerData[] = result.diagnostics.map((d) => {
      const start = offsetToLineCol(sql, d.startOffset);
      const end = offsetToLineCol(sql, d.endOffset);
      return {
        severity: SEVERITY_MAP[d.severity] ?? monaco.MarkerSeverity.Error,
        message: d.help ? `${d.message}\nhelp: ${d.help}` : d.message,
        startLineNumber: start.line,
        startColumn: start.col,
        endLineNumber: end.line,
        endColumn: end.col,
      };
    });

    monaco.editor.setModelMarkers(model, "syntaqlite", markers);
  }

  private updateEmbeddedDiagnostics(app: App, source: string): void {
    if (!app.runtime.ready || !this.editor) return;

    const model = this.editor.getModel();
    if (!model) return;

    const lang = app.languageMode as EmbeddedLanguage;

    // Extract fragments.
    const extractResult = app.runtime.runEmbeddedExtract(lang, source);
    app.embeddedFragments = extractResult.ok ? extractResult.fragments : [];

    // Run embedded diagnostics.
    const version = model.getVersionId();
    const result = app.runtime.runEmbeddedDiagnostics(lang, source, version);
    if (!result.ok) {
      monaco.editor.setModelMarkers(model, "syntaqlite", []);
      app.diagnostics = [];
      return;
    }

    for (const d of result.diagnostics) {
      const pos = offsetToLineCol(source, d.startOffset);
      d.line = pos.line;
      d.col = pos.col;
      d.stmtIndex = countStatements(source, d.startOffset);
    }

    app.diagnostics = result.diagnostics;

    const markers: monaco.editor.IMarkerData[] = result.diagnostics.map((d) => {
      const start = offsetToLineCol(source, d.startOffset);
      const end = offsetToLineCol(source, d.endOffset);
      return {
        severity: SEVERITY_MAP[d.severity] ?? monaco.MarkerSeverity.Error,
        message: d.help ? `${d.message}\nhelp: ${d.help}` : d.message,
        startLineNumber: start.line,
        startColumn: start.col,
        endLineNumber: end.line,
        endColumn: end.col,
      };
    });

    monaco.editor.setModelMarkers(model, "syntaqlite", markers);
  }

  private revealDiagnostic(d: DiagnosticEntry): void {
    if (!this.editor) return;
    const pos = offsetToLineCol(this.sql, d.endOffset);
    this.editor.setPosition({lineNumber: pos.line, column: pos.col});
    this.editor.revealLineInCenter(pos.line);
    this.editor.focus();
  }
}
