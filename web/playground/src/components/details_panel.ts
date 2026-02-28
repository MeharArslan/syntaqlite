// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {Attrs} from "../app/app";
import type {DiagnosticEntry} from "../types";
import type {SchemaFormat} from "../app/schema_context";
import {ResizeHandle} from "../widgets/resize_handle";
import {Table} from "../widgets/table";
import type {TableColumn} from "../widgets/table";
import "./details_panel.css";

type DetailsTab = "validation" | "schema";

const TABS: {key: DetailsTab; label: string}[] = [
  {key: "validation", label: "Validation"},
  {key: "schema", label: "Schema"},
];

const FORMAT_OPTIONS: {value: SchemaFormat; label: string}[] = [
  {value: "simple", label: "Simple list"},
  {value: "ddl", label: "DDL"},
];

const FORMAT_PLACEHOLDER: Record<SchemaFormat, string> = {
  simple: "table_name: col1, col2\nusers: id, name, email",
  ddl: "CREATE TABLE users (\n  id INTEGER PRIMARY KEY,\n  name TEXT\n);",
};

const FORMAT_HELP: Record<SchemaFormat, string> = {
  simple: "One table per line: table_name: col1, col2",
  ddl: "Paste CREATE TABLE statements",
};

const SEVERITY_ICON: Record<DiagnosticEntry["severity"], string> = {
  error: "✕",
  warning: "⚠",
  info: "ℹ",
  hint: "○",
};

const VALIDATION_COLUMNS: TableColumn<DiagnosticEntry>[] = [
  {
    key: "severity",
    label: "",
    width: "36px",
    align: "center",
    render: (d) =>
      m("span", {class: `sq-validation-severity sq-validation-severity--${d.severity}`},
        SEVERITY_ICON[d.severity],
      ),
  },
  {
    key: "message",
    label: "Message",
    render: (d) => d.message,
  },
];

const DEFAULT_HEIGHT = 160;
const MIN_HEIGHT = 60;
const MAX_HEIGHT_RATIO = 0.6;

export class DetailsPanel implements m.ClassComponent<Attrs> {
  private collapsed = false;
  private activeTab: DetailsTab = "validation";
  private panelHeight: number | undefined = undefined;
  private panelEl: HTMLElement | undefined = undefined;

  view(vnode: m.Vnode<Attrs>) {
    const {app} = vnode.attrs;
    const style: Record<string, string> = {};
    if (!this.collapsed) {
      style.height = `${this.panelHeight ?? DEFAULT_HEIGHT}px`;
    }
    return m("div.sq-details-panel", {
      class: this.collapsed ? "sq-details-panel--collapsed" : "",
      style,
      oncreate: (v: m.VnodeDOM) => { this.panelEl = v.dom as HTMLElement; },
    }, [
      this.collapsed
        ? undefined
        : m(ResizeHandle, {
            axis: "horizontal",
            onResize: (delta: number) => {
              const current = this.panelHeight ?? this.panelEl?.getBoundingClientRect().height ?? 120;
              const maxHeight = window.innerHeight * MAX_HEIGHT_RATIO;
              this.panelHeight = Math.max(MIN_HEIGHT, Math.min(maxHeight, current - delta));
            },
          }),
      m("div.sq-details-panel__header", [
        m(
          "nav.sq-tab-bar",
          TABS.map((t) =>
            m(
              "button.sq-tab-bar__tab",
              {
                class: this.activeTab === t.key ? "sq-tab-bar__tab--active" : "",
                onclick: () => { this.activeTab = t.key; },
              },
              t.key === "validation"
                ? [t.label, this.renderValidationBadge(app.diagnostics)]
                : t.label,
            ),
          ),
        ),
        m(
          "button.sq-details-panel__toggle",
          {onclick: () => { this.collapsed = !this.collapsed; }},
          this.collapsed ? "\u25B4 Show" : "\u25BE Hide",
        ),
      ]),
      this.collapsed
        ? undefined
        : this.activeTab === "validation"
          ? this.renderValidationTab(app.diagnostics, app.revealDiagnostic)
          : this.activeTab === "schema"
            ? m("div.sq-details-panel__body", [
                m("div.sq-details-panel__options", [
                  m("label", "Format"),
                  m("select", {
                    value: app.schemaContext.format,
                    onchange: (e: Event) => {
                      app.schemaContext.format = (e.target as HTMLSelectElement).value as SchemaFormat;
                      app.schemaContext.apply(app.runtime);
                    },
                  }, FORMAT_OPTIONS.map((o) =>
                    m("option", {value: o.value}, o.label),
                  )),
                ]),
                m("textarea.sq-details-panel__textarea", {
                  placeholder: FORMAT_PLACEHOLDER[app.schemaContext.format],
                  rows: 3,
                  value: app.schemaContext.rawText,
                  oninput: (e: Event) => {
                    app.schemaContext.rawText = (e.target as HTMLTextAreaElement).value;
                    app.schemaContext.apply(app.runtime);
                    m.redraw();
                  },
                }),
                app.schemaContext.parseError
                  ? m("span.sq-details-panel__error", app.schemaContext.parseError)
                  : m("span.sq-details-panel__help", FORMAT_HELP[app.schemaContext.format]),
              ])
            : undefined,
    ]);
  }

  private renderValidationBadge(diagnostics: DiagnosticEntry[]): m.Children {
    const errorCount = diagnostics.filter((d) => d.severity === "error").length;
    const warnCount = diagnostics.filter((d) => d.severity === "warning").length;
    if (errorCount === 0 && warnCount === 0) return undefined;
    return [
      errorCount > 0
        ? m("span.sq-validation-badge.sq-validation-badge--error", String(errorCount))
        : undefined,
      warnCount > 0
        ? m("span.sq-validation-badge.sq-validation-badge--warning", String(warnCount))
        : undefined,
    ];
  }

  private renderValidationTab(
    diagnostics: DiagnosticEntry[],
    revealDiagnostic?: (d: DiagnosticEntry) => void,
  ): m.Children {
    const emptyContent = m("div.sq-validation-empty", [
      m("span.sq-validation-empty__icon", "✓"),
      "No issues found",
    ]);

    return m(
      "div.sq-details-panel__body.sq-details-panel__body--flush",
      m(Table, {
        columns: VALIDATION_COLUMNS,
        rows: diagnostics,
        rowKey: (_, i) => i,
        emptyContent,
        onRowClick: revealDiagnostic ? (d) => revealDiagnostic(d) : undefined,
      }),
    );
  }
}
