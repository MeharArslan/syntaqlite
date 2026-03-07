// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import type {AstFieldValue, AstJsonNode, AstListNode} from "../../types";
import {isFieldValueEmpty} from "../ast_outline";
import type {VisNode} from "./types";

function isListNode(node: AstJsonNode): node is AstListNode {
  return typeof (node as AstListNode).count === "number";
}

function nodeFields(node: AstJsonNode): [string, AstFieldValue][] {
  return Object.entries(node).filter(
    ([k]) => k !== "type" && k !== "count" && k !== "children",
  ) as [string, AstFieldValue][];
}

function flattenNode(node: AstJsonNode, showEmpty: boolean): VisNode {
  if (isListNode(node)) {
    const children = (node.children ?? []).map((c) => flattenNode(c, showEmpty));
    return {
      label: node.type,
      kind: "list",
      leafText: `[${node.count ?? 0}]`,
      children,
      collapsed: false,
      x: 0,
      y: 0,
      w: 0,
      h: 0,
    };
  }

  const children: VisNode[] = [];
  const leafLines: string[] = [];

  for (const [label, value] of nodeFields(node)) {
    const v = value ?? null;
    if (!showEmpty && isFieldValueEmpty(v)) continue;

    // Child node — recurse.
    if (v !== null && typeof v === "object" && !Array.isArray(v)) {
      const child = flattenNode(v, showEmpty);
      child.fieldLabel = label;
      children.push(child);
    } else if (v === null) {
      leafLines.push(`${label}: (none)`);
    } else if (Array.isArray(v)) {
      const display = v.length === 0 ? "(none)" : v.join(" | ");
      leafLines.push(`${label}: ${display}`);
    } else if (typeof v === "boolean") {
      leafLines.push(`${label}: ${v ? "TRUE" : "FALSE"}`);
    } else {
      // string (span or enum display name)
      leafLines.push(`${label}: "${v}"`);
    }
  }

  return {
    label: node.type,
    kind: "node",
    leafText: leafLines.join("\n"),
    children,
    collapsed: false,
    x: 0,
    y: 0,
    w: 0,
    h: 0,
  };
}

export function flattenAst(statements: AstJsonNode[], showEmpty: boolean): VisNode[] {
  return statements.map((s) => flattenNode(s, showEmpty));
}
