// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

export interface VisNode {
  label: string;
  kind: "list" | "node";
  leafText: string;
  fieldLabel?: string;
  children: VisNode[];
  collapsed: boolean;
  x: number;
  y: number;
  w: number;
  h: number;
  /** Internal: subtree width computed during layout. */
  _subtreeW?: number;
}

export interface TreeLayout {
  roots: VisNode[];
  width: number;
  height: number;
}

export interface Transform {
  panX: number;
  panY: number;
  zoom: number;
}
