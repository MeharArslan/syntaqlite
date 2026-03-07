// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {AstFieldValue, AstJsonNode, AstListNode, AstResult} from "../types";
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

// ── Helpers ──

/** Returns true when a field value should be hidden in "hide empty" mode. */
export function isFieldValueEmpty(value: AstFieldValue): boolean {
  if (value === null || value === undefined) return true;
  if (Array.isArray(value)) return value.length === 0;
  if (typeof value === "boolean") return !value;
  return false;
}

function isListNode(node: AstJsonNode): node is AstListNode {
  return typeof (node as AstListNode).count === "number";
}

/** All field entries of a regular node, excluding the structural keys. */
function nodeFields(node: AstJsonNode): [string, AstFieldValue][] {
  return Object.entries(node).filter(
    ([k]) => k !== "type" && k !== "count" && k !== "children",
  ) as [string, AstFieldValue][];
}

function noneValue(): m.Vnode {
  return m("span.sq-ast-tree__value.sq-ast-tree__value--none", "(none)");
}

function leafRow(label: string, value: m.Children): m.Vnode {
  return m("div.sq-ast-tree__node", [
    m("div.sq-ast-tree__leaf", [m("span.sq-ast-tree__field-label", `${label}:`), " ", value]),
  ]);
}

function renderFieldValue(
  showEmpty: boolean,
  label: string,
  value: AstFieldValue,
): m.Vnode | undefined {
  if (!showEmpty && isFieldValueEmpty(value)) return undefined;

  // Child node — recurse.
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    return m("div.sq-ast-tree__node", [
      m("details", {open: true}, [
        m("summary", [m("span.sq-ast-tree__field-label", `${label}:`)]),
        renderNodes(showEmpty, [value]),
      ]),
    ]);
  }

  // Null / absent.
  if (value === null || value === undefined) {
    return leafRow(label, noneValue());
  }

  // Flags — string[].
  if (Array.isArray(value)) {
    const display = value.length === 0 ? "(none)" : value.join(" | ");
    const cls =
      value.length === 0
        ? "span.sq-ast-tree__value.sq-ast-tree__value--none"
        : "span.sq-ast-tree__value";
    return leafRow(label, m(cls, display));
  }

  // Boolean.
  if (typeof value === "boolean") {
    return leafRow(
      label,
      m("span.sq-ast-tree__value.sq-ast-tree__value--bool", value ? "TRUE" : "FALSE"),
    );
  }

  // Span / enum — string.
  return leafRow(label, m("span.sq-ast-tree__value.sq-ast-tree__value--string", `"${value}"`));
}

function renderNodes(showEmpty: boolean, nodes: AstJsonNode[]): m.Vnode {
  return m(
    "div",
    nodes.map((node) => {
      if (isListNode(node)) {
        return m("div.sq-ast-tree__node", [
          m("details", {open: true}, [
            m("summary", [
              m("span.sq-ast-tree__name", node.type),
              m("span.sq-ast-tree__list-count", `[${node.count ?? 0}]`),
            ]),
            node.children && node.children.length > 0
              ? renderNodes(showEmpty, node.children)
              : undefined,
          ]),
        ]);
      }
      const fields = nodeFields(node);
      return m("div.sq-ast-tree__node", [
        m("details", {open: true}, [
          m("summary", [m("span.sq-ast-tree__name", node.type)]),
          fields.length > 0
            ? m(
                "div",
                fields
                  .map(([label, value]) => renderFieldValue(showEmpty, label, value ?? null))
                  .filter(Boolean),
              )
            : undefined,
        ]),
      ]);
    }),
  );
}
