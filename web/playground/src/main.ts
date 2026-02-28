// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import {App} from "./app/app";
import * as monaco from "monaco-editor";
import "monaco-editor/esm/vs/basic-languages/sql/sql.contribution";
import {INPUT_MODEL_URI} from "./app/editor_models";
import type {Engine} from "@syntaqlite/js";
import {AppComponent} from "./components/app";
import "./styles/main.css";

function setupMonacoWorkers() {
  self.MonacoEnvironment = {
    getWorker(_workerId: string, label: string): Worker {
      if (label === "json") {
        return new Worker(
          new URL("monaco-editor/esm/vs/language/json/json.worker.js", import.meta.url),
          {type: "module"},
        );
      }
      if (label === "css" || label === "scss" || label === "less") {
        return new Worker(
          new URL("monaco-editor/esm/vs/language/css/css.worker.js", import.meta.url),
          {type: "module"},
        );
      }
      if (label === "html" || label === "handlebars" || label === "razor") {
        return new Worker(
          new URL("monaco-editor/esm/vs/language/html/html.worker.js", import.meta.url),
          {type: "module"},
        );
      }
      if (label === "typescript" || label === "javascript") {
        return new Worker(
          new URL("monaco-editor/esm/vs/language/typescript/ts.worker.js", import.meta.url),
          {type: "module"},
        );
      }
      return new Worker(new URL("monaco-editor/esm/vs/editor/editor.worker.js", import.meta.url), {
        type: "module",
      });
    },
  };
}

// Legend order must match SEMANTIC_TOKEN_LEGEND in syntaqlite-runtime.
const TOKEN_LEGEND: monaco.languages.SemanticTokensLegend = {
  tokenTypes: [
    "keyword", // 0
    "variable", // 1
    "string", // 2
    "number", // 3
    "operator", // 4
    "comment", // 5
    "punctuation", // 6
    "identifier", // 7
    "function", // 8
    "type", // 9
  ],
  tokenModifiers: [],
};

function registerSemanticTokensProvider(engine: Engine): void {
  monaco.languages.registerDocumentRangeSemanticTokensProvider("sql", {
    getLegend: () => TOKEN_LEGEND,

    provideDocumentRangeSemanticTokens(
      model: monaco.editor.ITextModel,
      range: monaco.Range,
    ): monaco.languages.ProviderResult<monaco.languages.SemanticTokens> {
      if (!engine.ready) return {data: new Uint32Array(0)};
      if (model.uri.toString() !== INPUT_MODEL_URI) {
        return {data: new Uint32Array(0)};
      }
      const source = model.getValue();
      const rangeStart = model.getOffsetAt(range.getStartPosition());
      const rangeEnd = model.getOffsetAt(range.getEndPosition());
      const version = model.getVersionId();
      const data = engine.runSemanticTokens(source, rangeStart, rangeEnd, version);
      return {data: data ?? new Uint32Array(0)};
    },
  });
}

function registerCompletionProvider(engine: Engine): void {
  monaco.languages.registerCompletionItemProvider("sql", {
    triggerCharacters: [" ", "\t", ";", "(", ","],
    provideCompletionItems(
      model: monaco.editor.ITextModel,
      position: monaco.Position,
    ): monaco.languages.ProviderResult<monaco.languages.CompletionList> {
      if (!engine.ready) return {suggestions: []};
      if (model.uri.toString() !== INPUT_MODEL_URI) {
        return {suggestions: []};
      }

      const source = model.getValue();
      const offset = model.getOffsetAt(position);
      const version = model.getVersionId();
      const result = engine.runCompletions(source, offset, version);
      if (!result.ok || result.items.length === 0) {
        return {suggestions: []};
      }

      const word = model.getWordUntilPosition(position);
      const range = new monaco.Range(
        position.lineNumber,
        word.startColumn,
        position.lineNumber,
        word.endColumn,
      );

      const suggestions: monaco.languages.CompletionItem[] = result.items.map((item) => ({
        label: item.label,
        insertText: item.label,
        kind:
          item.kind === "function"
            ? monaco.languages.CompletionItemKind.Function
            : item.kind === "class"
              ? monaco.languages.CompletionItemKind.Class
              : monaco.languages.CompletionItemKind.Keyword,
        range,
      }));

      return {suggestions};
    },
  });
}

function registerCodeActionProvider(app: App): void {
  monaco.languages.registerCodeActionProvider("sql", {
    provideCodeActions(
      model: monaco.editor.ITextModel,
      range: monaco.Range,
    ): monaco.languages.ProviderResult<monaco.languages.CodeActionList> {
      if (model.uri.toString() !== INPUT_MODEL_URI) return {actions: [], dispose() {}};

      const actions: monaco.languages.CodeAction[] = [];
      for (const d of app.diagnostics) {
        if (!d.helpDetail || d.helpDetail.kind !== "suggestion") continue;
        if (d.line == null || d.col == null) continue;
        const suggestion = d.helpDetail.value;

        // Check if this diagnostic overlaps the requested range.
        const dStartLine = d.line;
        const dStartCol = d.col;
        const dEnd = model.getPositionAt(d.endOffset);
        const dRange = new monaco.Range(dStartLine, dStartCol, dEnd.lineNumber, dEnd.column);
        if (!monaco.Range.areIntersectingOrTouching(range, dRange)) continue;

        actions.push({
          title: `Change to '${suggestion}'`,
          kind: "quickfix",
          isPreferred: true,
          edit: {
            edits: [{
              resource: model.uri,
              textEdit: {range: dRange, text: suggestion},
              versionId: model.getVersionId(),
            }],
          },
        });
      }
      return {actions, dispose() {}};
    },
  });
}

async function main() {
  setupMonacoWorkers();

  const app = new App();
  const root = document.getElementById("app");
  if (!root) throw new Error("missing #app element");

  m.mount(root, {view: () => m(AppComponent, {app})});

  try {
    await app.runtime.load();
    await app.dialect.loadDefault(app.runtime);
    app.dialectConfig.loadAvailableCflags(app.runtime);
    registerSemanticTokensProvider(app.runtime);
    registerCompletionProvider(app.runtime);
    registerCodeActionProvider(app);
    app.runtime.updateStatus("Ready.");
  } catch (err) {
    app.runtime.updateStatus(`Failed to initialize: ${(err as Error).message}`, true);
  }
  m.redraw();
}

main();
