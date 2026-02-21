// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import type {AstField, AstJsonNode} from "../../types";
import type {VisNode} from "./types";

function isFieldEmpty(f: AstField): boolean {
  switch (f.kind) {
    case "node":
      return f.child === null;
    case "span":
      return f.value === null;
    case "bool":
      return f.value === false;
    case "enum":
      return f.value === null;
    case "flags":
      return f.value.length === 0;
  }
}

function flattenNode(node: AstJsonNode, showEmpty: boolean): VisNode {
  if (node.type === "list") {
    const children = (node.children || []).map((c) => flattenNode(c, showEmpty));
    return {
      label: node.name,
      kind: "list",
      leafText: `[${node.count}]`,
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

  for (const f of node.fields || []) {
    if (!showEmpty && isFieldEmpty(f)) continue;
    if (f.kind === "node") {
      if (f.child === null) {
        leafLines.push(`${f.label}: (none)`);
      } else {
        const child = flattenNode(f.child, showEmpty);
        child.fieldLabel = f.label;
        children.push(child);
      }
    } else if (f.kind === "span") {
      leafLines.push(f.value === null ? `${f.label}: (none)` : `${f.label}: "${f.value}"`);
    } else if (f.kind === "bool") {
      leafLines.push(`${f.label}: ${f.value ? "TRUE" : "FALSE"}`);
    } else if (f.kind === "enum") {
      leafLines.push(f.value === null ? `${f.label}: (none)` : `${f.label}: ${f.value}`);
    } else if (f.kind === "flags") {
      const display = f.value.length === 0 ? "(none)" : f.value.join(" | ");
      leafLines.push(`${f.label}: ${display}`);
    }
  }

  return {
    label: node.name,
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
