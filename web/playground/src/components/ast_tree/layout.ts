// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import type {TreeLayout, VisNode} from "./types";

const FONT_LABEL = "bold 12px 'JetBrains Mono', monospace";
const FONT_LEAF = "10px 'JetBrains Mono', monospace";
const PAD_X = 10;
const PAD_Y = 6;
const V_GAP = 50;
const H_GAP = 16;
const MIN_W = 60;
const LINE_H = 14;

function measure(node: VisNode, ctx: CanvasRenderingContext2D, badgeW: number) {
  ctx.font = FONT_LABEL;
  let displayLabel = node.label;
  if (node.fieldLabel) displayLabel = `${node.fieldLabel}: ${node.label}`;
  let labelW = ctx.measureText(displayLabel).width;
  if (node.children.length > 0) labelW += badgeW;
  let maxW = labelW;

  let lineCount = 1;
  if (node.kind === "list") {
    ctx.font = FONT_LEAF;
    maxW = Math.max(maxW, ctx.measureText(node.leafText).width);
  } else if (node.leafText) {
    const lines = node.leafText.split("\n");
    ctx.font = FONT_LEAF;
    for (const line of lines) {
      maxW = Math.max(maxW, ctx.measureText(line).width);
    }
    lineCount += lines.length;
  }

  node.w = Math.max(MIN_W, maxW + PAD_X * 2);
  node.h = lineCount * LINE_H + PAD_Y * 2;

  if (!node.collapsed) {
    for (const child of node.children) measure(child, ctx, badgeW);
  }
}

function computeSubtreeWidth(node: VisNode): number {
  if (node.collapsed || node.children.length === 0) {
    node._subtreeW = node.w;
    return node._subtreeW;
  }
  let total = 0;
  for (let i = 0; i < node.children.length; i++) {
    if (i > 0) total += H_GAP;
    total += computeSubtreeWidth(node.children[i]);
  }
  node._subtreeW = Math.max(node.w, total);
  return node._subtreeW;
}

function assignPositions(node: VisNode, left: number, top: number) {
  node.y = top;
  node.x = left + (node._subtreeW ?? node.w) / 2 - node.w / 2;

  if (!node.collapsed && node.children.length > 0) {
    let totalChildW = 0;
    for (let i = 0; i < node.children.length; i++) {
      if (i > 0) totalChildW += H_GAP;
      totalChildW += node.children[i]._subtreeW ?? node.children[i].w;
    }
    let childLeft = left + ((node._subtreeW ?? node.w) - totalChildW) / 2;
    const childTop = top + node.h + V_GAP;
    for (const child of node.children) {
      assignPositions(child, childLeft, childTop);
      childLeft += (child._subtreeW ?? child.w) + H_GAP;
    }
  }
}

function computeBounds(node: VisNode, bounds: {maxX: number; maxY: number}) {
  bounds.maxX = Math.max(bounds.maxX, node.x + node.w);
  bounds.maxY = Math.max(bounds.maxY, node.y + node.h);
  if (!node.collapsed) {
    for (const child of node.children) computeBounds(child, bounds);
  }
}

export function layoutTree(roots: VisNode[], ctx: CanvasRenderingContext2D): TreeLayout {
  ctx.font = "bold 10px 'JetBrains Mono', monospace";
  const badgeW = ctx.measureText("[+]").width + 12;

  for (const root of roots) measure(root, ctx, badgeW);

  let totalW = 0;
  for (const root of roots) {
    computeSubtreeWidth(root);
    totalW += root._subtreeW ?? root.w;
  }
  totalW += (roots.length - 1) * H_GAP * 2;

  let curX = 0;
  for (const root of roots) {
    assignPositions(root, curX, 0);
    curX += (root._subtreeW ?? root.w) + H_GAP * 2;
  }

  const bounds = {maxX: 0, maxY: 0};
  for (const root of roots) computeBounds(root, bounds);

  return {roots, width: bounds.maxX, height: bounds.maxY};
}
