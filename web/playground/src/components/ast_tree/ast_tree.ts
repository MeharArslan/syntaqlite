// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {AstResult, Theme} from "../../types";
import {flattenAst} from "./flatten";
import {AstCanvasRenderer} from "./renderer";
import "./ast_tree.css";

export interface AstGraphAttrs {
  result: AstResult | undefined;
  showEmpty: boolean;
  theme: Theme;
}

export class AstGraph implements m.ClassComponent<AstGraphAttrs> {
  oncreate(vnode: m.VnodeDOM<AstGraphAttrs>) {
    const container = vnode.dom as HTMLElement;
    const canvas = container.querySelector("canvas")!;
    this.renderer = new AstCanvasRenderer(canvas, container);
    this.lastTheme = vnode.attrs.theme;
    this.updateGraph(vnode.attrs);
  }

  onupdate(vnode: m.VnodeDOM<AstGraphAttrs>) {
    if (vnode.attrs.theme !== this.lastTheme) {
      this.lastTheme = vnode.attrs.theme;
      if (this.renderer) this.renderer.refreshColors();
    }
    this.updateGraph(vnode.attrs);
  }

  onremove() {
    if (this.renderer) {
      this.renderer.destroy();
      this.renderer = undefined;
    }
  }

  view() {
    return m("div.sq-ast-graph", [
      m("canvas"),
      m("div.sq-ast-graph-controls", [
        m("button.sq-ast-graph-btn", {onclick: () => this.renderer?.zoomIn()}, "+"),
        m("button.sq-ast-graph-btn", {onclick: () => this.renderer?.zoomOut()}, "−"),
      ]),
    ]);
  }

  private renderer: AstCanvasRenderer | undefined = undefined;
  private lastTheme: Theme | undefined = undefined;
  private lastResult: AstResult | undefined = undefined;
  private lastShowEmpty = false;

  private updateGraph(attrs: AstGraphAttrs) {
    if (!this.renderer) return;
    const {result, showEmpty} = attrs;
    if (!result || !result.ok || result.statements.length === 0) return;
    // Only re-layout (and re-fit) when data actually changes, not on every Mithril redraw.
    if (result === this.lastResult && showEmpty === this.lastShowEmpty) return;
    this.lastResult = result;
    this.lastShowEmpty = showEmpty;
    const roots = flattenAst(result.statements, showEmpty);
    this.renderer.update(roots);
  }
}
