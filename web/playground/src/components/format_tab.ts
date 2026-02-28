// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {App} from "../app/app";
import type {FormatOptions, FormatResult, KeywordCase} from "../types";
import {FormatOutput} from "./format_output";

export interface FormatTabAttrs {
  app: App;
  sql: string;
  active: boolean;
}

export class FormatTab implements m.ClassComponent<FormatTabAttrs> {
  private formatOptions: FormatOptions = {
    lineWidth: 80,
    keywordCase: 1 as KeywordCase,
    semicolons: true,
  };
  private formatResult: FormatResult | undefined = undefined;
  private lastSql: string | undefined = undefined;
  private lastOptionsKey: string | undefined = undefined;
  private lastDialectPtr: number | undefined = undefined;
  private lastConfigKey: string | undefined = undefined;

  view(vnode: m.Vnode<FormatTabAttrs>) {
    const {app, sql, active} = vnode.attrs;

    if (active && app.runtime.ready && app.dialect.active) {
      const optKey = `${this.formatOptions.lineWidth}:${this.formatOptions.keywordCase}:${this.formatOptions.semicolons}`;
      const dPtr = app.dialect.active.ptr;
      const cfgKey = app.dialectConfig.configKey;
      if (
        sql !== this.lastSql ||
        optKey !== this.lastOptionsKey ||
        dPtr !== this.lastDialectPtr ||
        cfgKey !== this.lastConfigKey
      ) {
        this.lastSql = sql;
        this.lastOptionsKey = optKey;
        this.lastDialectPtr = dPtr;
        this.lastConfigKey = cfgKey;
        this.formatResult = app.runtime.runFmt(sql, this.formatOptions);
      }
    }

    const result = this.formatResult;
    const text = result ? (result.ok ? result.text : `Error: ${result.text}`) : "";

    return m("div.sq-tab-panel", {class: active ? "sq-tab-panel--active" : ""}, [
      m("div.sq-panel-options", [
        m("label.sq-panel-options__width-label", [
          "Width:",
          m("span.sq-panel-options__width-value", String(this.formatOptions.lineWidth)),
        ]),
        m("input[type=range]", {
          min: 20,
          max: 240,
          value: this.formatOptions.lineWidth,
          oninput: (e: Event) => {
            this.formatOptions.lineWidth = Number((e.target as HTMLInputElement).value);
          },
        }),
        m("label", "Keywords"),
        m(
          "select",
          {
            value: String(this.formatOptions.keywordCase),
            onchange: (e: Event) => {
              this.formatOptions.keywordCase = Number(
                (e.target as HTMLSelectElement).value,
              ) as KeywordCase;
            },
          },
          [
            m("option", {value: "0"}, "Preserve"),
            m("option", {value: "1"}, "Upper"),
            m("option", {value: "2"}, "Lower"),
          ],
        ),
        m("input[type=checkbox]", {
          checked: this.formatOptions.semicolons,
          onchange: (e: Event) => {
            this.formatOptions.semicolons = (e.target as HTMLInputElement).checked;
          },
        }),
        m("label", "Semicolons"),
      ]),
      m(FormatOutput, {text, theme: app.theme.current}),
    ]);
  }
}
