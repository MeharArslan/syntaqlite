// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {App} from "../app/app";
import type {ActiveTab} from "../types";
import {renderValidationBadge, renderValidationTab, renderSchemaTab} from "./details_shared";
import {AstTab} from "./ast_tab";
import {FormatTab} from "./format_tab";
import "./output_panel.css";

export interface OutputPanelAttrs {
  app: App;
  sql: string;
}

const DESKTOP_TABS: {key: ActiveTab; label: string}[] = [
  {key: "format", label: "Formatted"},
  {key: "ast", label: "AST"},
];

const MOBILE_TABS: {key: ActiveTab; label: string}[] = [
  {key: "validation", label: "Validation"},
  {key: "schema", label: "Schema"},
  {key: "format", label: "Formatted"},
  {key: "ast", label: "AST"},
];

export class OutputPanel implements m.ClassComponent<OutputPanelAttrs> {
  private activeTab: ActiveTab = "format";

  view(vnode: m.Vnode<OutputPanelAttrs>) {
    const {app, sql} = vnode.attrs;
    const mobile = app.window.isMobile;
    const tabs = mobile ? MOBILE_TABS : DESKTOP_TABS;

    // On first mobile render, default to validation tab.
    if (mobile && this.activeTab === "format") {
      this.activeTab = "validation";
    }

    // Guard: if we switched from mobile to desktop while on a mobile-only tab,
    // fall back to "format".
    if (!mobile && (this.activeTab === "validation" || this.activeTab === "schema")) {
      this.activeTab = "format";
    }

    return m("section.sq-workspace__pane.sq-viewer-pane", [
      m(
        "nav.sq-tab-bar",
        tabs.map((t) =>
          m(
            "button.sq-tab-bar__tab",
            {
              class: this.activeTab === t.key ? "sq-tab-bar__tab--active" : "",
              onclick: () => {
                this.activeTab = t.key;
              },
            },
            t.key === "validation"
              ? [t.label, renderValidationBadge(app.diagnostics)]
              : t.label,
          ),
        ),
      ),
      this.activeTab === "validation"
        ? renderValidationTab(app.diagnostics, app.revealDiagnostic)
        : this.activeTab === "schema"
          ? renderSchemaTab(app)
          : undefined,
      m(FormatTab, {app, sql, active: this.activeTab === "format"}),
      m(AstTab, {app, sql, active: this.activeTab === "ast"}),
    ]);
  }
}
