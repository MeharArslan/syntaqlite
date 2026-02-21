// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {Theme} from "../types";
import {MonacoEditor} from "../widgets/monaco_editor";
import "./editor_pane.css";

export interface EditorPaneAttrs {
  theme: Theme;
  initialSql: string;
  onContentChange: (s: string) => void;
}

export class EditorPane implements m.ClassComponent<EditorPaneAttrs> {
  view(vnode: m.Vnode<EditorPaneAttrs>) {
    const {theme, initialSql, onContentChange} = vnode.attrs;
    return m("section.sq-workspace__pane.sq-editor-pane", [
      m(MonacoEditor, {
        theme,
        initialValue: initialSql,
        onContentChange,
        lineNumbers: "on",
        renderLineHighlight: "gutter",
      }),
    ]);
  }
}
