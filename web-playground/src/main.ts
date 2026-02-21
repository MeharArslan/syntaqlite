// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import {App} from "./app/app";
import {registerSemanticTokensProvider} from "./app/semantic_tokens";
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

async function main() {
  setupMonacoWorkers();

  const app = new App();
  const root = document.getElementById("app");
  if (!root) throw new Error("missing #app element");

  m.mount(root, {view: () => m(AppComponent, {app})});

  try {
    await app.runtime.load();
    await app.dialect.loadDefault(app.runtime);
    registerSemanticTokensProvider(app.runtime);
    app.runtime.updateStatus("Ready.");
  } catch (err) {
    app.runtime.updateStatus(`Failed to initialize: ${(err as Error).message}`, true);
  }
  m.redraw();
}

main();
