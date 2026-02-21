// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {App} from "../app/app";
import type {AstResult, AstViewMode} from "../types";
import {AstOutline} from "./ast_outline";
import {AstGraph} from "./ast_tree/ast_tree";

export interface AstTabAttrs {
  app: App;
  sql: string;
  active: boolean;
}

export class AstTab implements m.ClassComponent<AstTabAttrs> {
  private astViewMode: AstViewMode = "outline";
  private astShowEmpty = false;
  private astResult: AstResult | null = null;
  private lastSql: string | null = null;
  private lastDialectPtr: number | null = null;

  view(vnode: m.Vnode<AstTabAttrs>) {
    const {app, sql, active} = vnode.attrs;

    if (active && app.runtime.ready && app.dialect.active) {
      const dPtr = app.dialect.active.ptr;
      if (sql !== this.lastSql || dPtr !== this.lastDialectPtr) {
        this.lastSql = sql;
        this.lastDialectPtr = dPtr;
        this.astResult = app.runtime.runAstJson(sql);
      }
    }

    const isGraph = this.astViewMode === "graph";
    return m(
      "div.sq-tab-panel",
      {
        class: [active ? "sq-tab-panel--active" : "", isGraph ? "sq-tab-panel--graph" : ""]
          .filter(Boolean)
          .join(" "),
      },
      [
        m("div.sq-panel-options", [
          m("label", "View"),
          m(
            "select",
            {
              value: this.astViewMode,
              onchange: (e: Event) => {
                this.astViewMode = (e.target as HTMLSelectElement).value as AstViewMode;
              },
            },
            [m("option", {value: "outline"}, "Outline"), m("option", {value: "graph"}, "Graph")],
          ),
          m("input[type=checkbox]", {
            checked: this.astShowEmpty,
            onchange: (e: Event) => {
              this.astShowEmpty = (e.target as HTMLInputElement).checked;
            },
          }),
          m("label", "Show empty fields"),
        ]),
        isGraph
          ? m(AstGraph, {
              result: this.astResult,
              showEmpty: this.astShowEmpty,
              theme: app.theme.current,
            })
          : m(AstOutline, {result: this.astResult, showEmpty: this.astShowEmpty}),
      ],
    );
  }
}
