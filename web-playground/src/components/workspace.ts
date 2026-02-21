// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {Attrs} from "../app/app";
import {debounce} from "../base/debounce";
import {EditorPane} from "./editor_pane";
import {OutputPanel} from "./output_panel";
import "./workspace.css";

export class Workspace implements m.ClassComponent<Attrs> {
  private sql = "select a, b from t where c = 1;";
  private debouncedUpdate: (sql: string) => void;

  constructor() {
    this.debouncedUpdate = debounce((sql: string) => {
      this.sql = sql;
      m.redraw();
    }, 150);
  }

  view(vnode: m.Vnode<Attrs>) {
    const {app} = vnode.attrs;
    return m("section.sq-workspace", [
      m(EditorPane, {
        theme: app.theme.current,
        initialSql: this.sql,
        onContentChange: (s: string) => this.debouncedUpdate(s),
      }),
      m(OutputPanel, {app, sql: this.sql}),
    ]);
  }
}
