// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {App} from "../app/app";
import type {ActiveTab} from "../types";
import {AstTab} from "./ast_tab";
import {FormatTab} from "./format_tab";
import "./output_panel.css";

export interface OutputPanelAttrs {
  app: App;
  sql: string;
}

const TABS: {key: ActiveTab; label: string}[] = [
  {key: "format", label: "Formatted"},
  {key: "ast", label: "AST"},
];

export class OutputPanel implements m.ClassComponent<OutputPanelAttrs> {
  private activeTab: ActiveTab = "format";

  view(vnode: m.Vnode<OutputPanelAttrs>) {
    const {app, sql} = vnode.attrs;
    return m("section.sq-workspace__pane.sq-viewer-pane", [
      m(
        "nav.sq-tab-bar",
        TABS.map((t) =>
          m(
            "button.sq-tab-bar__tab",
            {
              class: this.activeTab === t.key ? "sq-tab-bar__tab--active" : "",
              onclick: () => {
                this.activeTab = t.key;
              },
            },
            t.label,
          ),
        ),
      ),
      m(FormatTab, {app, sql, active: this.activeTab === "format"}),
      m(AstTab, {app, sql, active: this.activeTab === "ast"}),
    ]);
  }
}
