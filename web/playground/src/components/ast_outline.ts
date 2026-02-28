// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {AstField, AstJsonNode, AstResult} from "../types";
import "./ast_outline.css";

export interface AstOutlineAttrs {
  result: AstResult | undefined;
  showEmpty: boolean;
}

export const AstOutline: m.Component<AstOutlineAttrs> = {
  view(vnode) {
    const {result, showEmpty} = vnode.attrs;
    if (!result) return m("div.sq-ast-tree", "(no data)");

    if (!result.ok) {
      return m("div.sq-ast-tree.sq-ast-tree--error", `Error: ${result.error}`);
    }

    if (result.statements.length === 0) {
      return m("div.sq-ast-tree", "(empty)");
    }

    return m("div.sq-ast-tree", [renderNodes(showEmpty, result.statements)]);
  },
};

// ── Render helpers ──

export function isFieldEmpty(f: AstField): boolean {
  switch (f.kind) {
    case "node":
      return f.child === undefined;
    case "span":
      return f.value === undefined;
    case "bool":
      return f.value === false;
    case "enum":
      return f.value === undefined;
    case "flags":
      return f.value.length === 0;
  }
}

function noneValue(): m.Vnode {
  return m("span.sq-ast-tree__value.sq-ast-tree__value--none", "(none)");
}

function leafRow(label: string, value: m.Children): m.Vnode {
  return m("div.sq-ast-tree__node", [
    m("div.sq-ast-tree__leaf", [m("span.sq-ast-tree__field-label", `${label}:`), " ", value]),
  ]);
}

function renderField(showEmpty: boolean, f: AstField): m.Vnode | undefined {
  if (!showEmpty && isFieldEmpty(f)) return undefined;

  if (f.kind === "node") {
    if (f.child === undefined) {
      return leafRow(f.label, noneValue());
    }
    return m("div.sq-ast-tree__node", [
      m("details", {open: true}, [
        m("summary", [m("span.sq-ast-tree__field-label", `${f.label}:`)]),
        renderNodes(showEmpty, [f.child]),
      ]),
    ]);
  }

  if (f.kind === "span") {
    const val =
      f.value === undefined
        ? noneValue()
        : m("span.sq-ast-tree__value.sq-ast-tree__value--string", `"${f.value}"`);
    return leafRow(f.label, val);
  }

  if (f.kind === "bool") {
    return leafRow(
      f.label,
      m("span.sq-ast-tree__value.sq-ast-tree__value--bool", f.value ? "TRUE" : "FALSE"),
    );
  }

  if (f.kind === "enum") {
    const val = f.value === undefined ? noneValue() : m("span.sq-ast-tree__value", String(f.value));
    return leafRow(f.label, val);
  }

  if (f.kind === "flags") {
    const display = f.value.length === 0 ? "(none)" : f.value.join(" | ");
    const cls =
      f.value.length === 0
        ? "span.sq-ast-tree__value.sq-ast-tree__value--none"
        : "span.sq-ast-tree__value";
    return leafRow(f.label, m(cls, display));
  }

  return undefined;
}

function renderNodes(showEmpty: boolean, nodes: AstJsonNode[]): m.Vnode {
  return m(
    "div",
    nodes.map((node) => {
      if (node.type === "list") {
        return m("div.sq-ast-tree__node", [
          m("details", {open: true}, [
            m("summary", [
              m("span.sq-ast-tree__name", node.name),
              m("span.sq-ast-tree__list-count", `[${node.count}]`),
            ]),
            node.children && node.children.length > 0
              ? renderNodes(showEmpty, node.children)
              : undefined,
          ]),
        ]);
      }
      return m("div.sq-ast-tree__node", [
        m("details", {open: true}, [
          m("summary", [m("span.sq-ast-tree__name", node.name)]),
          node.fields && node.fields.length > 0
            ? m("div", node.fields.map((f) => renderField(showEmpty, f)).filter(Boolean))
            : undefined,
        ]),
      ]);
    }),
  );
}
