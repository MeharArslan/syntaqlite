// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {DiagnosticEntry, SchemaFormat, Engine} from "@syntaqlite/js";
import {Table} from "../widgets/table";
import type {TableColumn} from "../widgets/table";

export const SEVERITY_ICON: Record<DiagnosticEntry["severity"], string> = {
  error: "\u2715",
  warning: "\u26A0",
  info: "\u2139",
  hint: "\u25CB",
};

export const VALIDATION_COLUMNS: TableColumn<DiagnosticEntry>[] = [
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
    key: "location",
    label: "Location",
    width: "80px",
    render: (d) => d.line != null && d.col != null ? `${d.line}:${d.col}` : "",
  },
  {
    key: "stmt",
    label: "Stmt",
    width: "48px",
    align: "center",
    render: (d) => d.stmtIndex != null ? String(d.stmtIndex) : "",
  },
  {
    key: "message",
    label: "Message",
    render: (d) => d.message,
  },
  {
    key: "help",
    label: "Help",
    render: (d) => d.help ?? "",
  },
];

export const FORMAT_OPTIONS: {value: SchemaFormat; label: string}[] = [
  {value: "simple", label: "Simple list"},
  {value: "ddl", label: "DDL"},
];

export const FORMAT_PLACEHOLDER: Record<SchemaFormat, string> = {
  simple: "table_name: col1, col2\nusers: id, name, email",
  ddl: "CREATE TABLE users (\n  id INTEGER PRIMARY KEY,\n  name TEXT\n);",
};

export const FORMAT_HELP: Record<SchemaFormat, string> = {
  simple: "One table per line: table_name: col1, col2",
  ddl: "Paste CREATE TABLE statements",
};

export function renderValidationBadge(diagnostics: DiagnosticEntry[]): m.Children {
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

export function renderValidationTab(
  diagnostics: DiagnosticEntry[],
  revealDiagnostic?: (d: DiagnosticEntry) => void,
): m.Children {
  const emptyContent = m("div.sq-validation-empty", [
    m("span.sq-validation-empty__icon", "\u2713"),
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

export function renderSchemaTab(app: {
  schemaContext: {
    format: SchemaFormat;
    rawText: string;
    parseError?: string;
    parsedTableCount?: number;
    apply: (engine: Engine, force?: boolean) => boolean;
    configKey: string;
  };
  runtime: Engine;
}): m.Children {
  return m("div.sq-details-panel__body", [
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
      : app.schemaContext.parsedTableCount !== undefined
        ? m("span.sq-details-panel__help",
            `${app.schemaContext.parsedTableCount} table(s) loaded`)
        : m("span.sq-details-panel__help", FORMAT_HELP[app.schemaContext.format]),
  ]);
}
