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
  private astResult: AstResult | undefined = undefined;
  private lastSql: string | undefined = undefined;
  private lastDialectPtr: number | undefined = undefined;
  private lastConfigKey: string | undefined = undefined;

  view(vnode: m.Vnode<AstTabAttrs>) {
    const {app, sql, active} = vnode.attrs;

    if (active && app.runtime.ready && app.dialect.active) {
      const isEmbedded = app.languageMode !== "sql";
      const fragIdx = app.selectedFragmentIndex;
      const cacheKey = `${sql}:${app.languageMode}:${fragIdx}`;
      const dPtr = app.dialect.active.ptr;
      const cfgKey = app.dialectConfig.configKey;
      if (cacheKey !== this.lastSql || dPtr !== this.lastDialectPtr || cfgKey !== this.lastConfigKey) {
        this.lastSql = cacheKey;
        this.lastDialectPtr = dPtr;
        this.lastConfigKey = cfgKey;

        if (isEmbedded && app.embeddedFragments.length > 0) {
          if (fragIdx >= 0 && fragIdx < app.embeddedFragments.length) {
            this.astResult = app.runtime.runAstJson(app.embeddedFragments[fragIdx].sqlText);
          } else {
            // Merge all fragments' ASTs.
            const allStatements: import("../types").AstJsonNode[] = [];
            for (const f of app.embeddedFragments) {
              const r = app.runtime.runAstJson(f.sqlText);
              if (r.ok) {
                allStatements.push(...r.statements);
              }
            }
            this.astResult = {ok: true, statements: allStatements};
          }
        } else {
          this.astResult = app.runtime.runAstJson(sql);
        }
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
