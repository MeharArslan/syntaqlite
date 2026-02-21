// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import * as monaco from "monaco-editor";
import type {Theme} from "../types";

let registered = false;

function ensureThemesRegistered() {
  if (registered) return;
  registered = true;

  monaco.editor.defineTheme("syntaqlite-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      {token: "keyword", foreground: "d4826a", fontStyle: "bold"},
      {token: "keyword.sql", foreground: "d4826a", fontStyle: "bold"},
      {token: "operator.sql", foreground: "b0b8b4"},
      {token: "string", foreground: "d4a656"},
      {token: "string.sql", foreground: "d4a656"},
      {token: "number", foreground: "c08edb"},
      {token: "number.sql", foreground: "c08edb"},
      {token: "comment", foreground: "5a6260", fontStyle: "italic"},
      {token: "comment.sql", foreground: "5a6260", fontStyle: "italic"},
      {token: "identifier", foreground: "d4d8d6"},
      {token: "predefined.sql", foreground: "8bbcb0"},
    ],
    colors: {
      "editor.background": "#1c2021",
      "editor.foreground": "#d4d8d6",
      "editor.lineHighlightBackground": "#232729",
      "editor.selectionBackground": "#3a2e2a",
      "editorCursor.foreground": "#d4826a",
      "editorLineNumber.foreground": "#4a5052",
      "editorLineNumber.activeForeground": "#8a8f8c",
      "editor.selectionHighlightBackground": "#3a2e2a30",
    },
  });

  monaco.editor.defineTheme("syntaqlite-light", {
    base: "vs",
    inherit: true,
    rules: [
      {token: "keyword", foreground: "b8553a", fontStyle: "bold"},
      {token: "keyword.sql", foreground: "b8553a", fontStyle: "bold"},
      {token: "operator.sql", foreground: "5a6260"},
      {token: "string", foreground: "8a5d22"},
      {token: "string.sql", foreground: "8a5d22"},
      {token: "number", foreground: "7a3e9d"},
      {token: "number.sql", foreground: "7a3e9d"},
      {token: "comment", foreground: "9aa09d", fontStyle: "italic"},
      {token: "comment.sql", foreground: "9aa09d", fontStyle: "italic"},
      {token: "identifier", foreground: "1f2121"},
      {token: "predefined.sql", foreground: "b8553a"},
    ],
    colors: {
      "editor.background": "#f3f5f1",
      "editor.foreground": "#1f2121",
      "editor.lineHighlightBackground": "#eef0ec",
      "editor.selectionBackground": "#e8d4cc",
      "editorCursor.foreground": "#b8553a",
      "editorLineNumber.foreground": "#c3cabf",
      "editorLineNumber.activeForeground": "#6a6f6c",
    },
  });
}

function currentThemeName(theme: Theme): string {
  return theme === "dark" ? "syntaqlite-dark" : "syntaqlite-light";
}

const SHARED_EDITOR_OPTIONS: monaco.editor.IStandaloneEditorConstructionOptions = {
  language: "sql",
  minimap: {enabled: false},
  scrollBeyondLastLine: false,
  fontSize: 16,
  fontFamily: "'Source Code Pro', 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
  fontLigatures: true,
  automaticLayout: true,
  padding: {top: 12},
};

export interface MonacoEditorAttrs {
  theme: Theme;
  initialValue: string;
  readOnly?: boolean;
  lineNumbers?: "on" | "off";
  renderLineHighlight?: "gutter" | "none";
  onContentChange?: (text: string) => void;
}

export class MonacoEditor implements m.ClassComponent<MonacoEditorAttrs> {
  oncreate(vnode: m.VnodeDOM<MonacoEditorAttrs>) {
    const {theme, initialValue, readOnly, lineNumbers, renderLineHighlight, onContentChange} =
      vnode.attrs;
    ensureThemesRegistered();

    const opts: monaco.editor.IStandaloneEditorConstructionOptions = {
      ...SHARED_EDITOR_OPTIONS,
      value: initialValue,
      theme: currentThemeName(theme),
      lineNumbers: lineNumbers ?? "on",
      renderLineHighlight: renderLineHighlight ?? "gutter",
    };

    if (readOnly) {
      Object.assign(opts, {
        readOnly: true,
        domReadOnly: true,
        cursorWidth: 0,
        cursorStyle: "line-thin" as const,
        cursorBlinking: "solid" as const,
        occurrencesHighlight: "off" as const,
        selectionHighlight: false,
        matchBrackets: "never" as const,
        folding: false,
        glyphMargin: false,
        lineDecorationsWidth: 0,
        lineNumbersMinChars: 0,
        contextmenu: false,
        hideCursorInOverviewRuler: true,
        scrollbar: {handleMouseWheel: true},
      });
    }

    this.editor = monaco.editor.create(vnode.dom as HTMLElement, opts);
    this.lastTheme = theme;

    if (onContentChange) {
      this.editor.onDidChangeModelContent(() => {
        if (this.editor) onContentChange(this.editor.getValue());
      });
    }
  }

  onupdate(vnode: m.VnodeDOM<MonacoEditorAttrs>) {
    const {theme, initialValue, onContentChange} = vnode.attrs;

    if (theme !== this.lastTheme) {
      this.lastTheme = theme;
      if (this.editor) monaco.editor.setTheme(currentThemeName(theme));
    }

    // For read-only editors (no onContentChange), sync value from outside.
    if (!onContentChange && initialValue !== this.lastSyncedValue) {
      this.lastSyncedValue = initialValue;
      if (this.editor) this.editor.setValue(initialValue);
    }
  }

  onremove() {
    if (this.editor) {
      this.editor.dispose();
      this.editor = null;
    }
  }

  view() {
    return m("div");
  }

  private editor: monaco.editor.IStandaloneCodeEditor | null = null;
  private lastTheme: Theme | null = null;
  private lastSyncedValue: string | null = null;
}
