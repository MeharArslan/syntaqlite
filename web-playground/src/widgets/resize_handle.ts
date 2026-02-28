// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import "./resize_handle.css";

export type ResizeAxis = "horizontal" | "vertical";

export interface ResizeHandleAttrs {
  axis: ResizeAxis;
  onResize: (delta: number) => void;
  onResizeEnd?: () => void;
}

export class ResizeHandle implements m.ClassComponent<ResizeHandleAttrs> {
  private moveListener: ((e: MouseEvent) => void) | undefined = undefined;
  private upListener: (() => void) | undefined = undefined;

  onremove() {
    this.cleanup();
  }

  view(vnode: m.Vnode<ResizeHandleAttrs>) {
    const {axis} = vnode.attrs;
    return m("div.sq-resize-handle", {
      class: `sq-resize-handle--${axis}`,
      onmousedown: (e: MouseEvent) => this.start(e, vnode.attrs),
    });
  }

  private start(e: MouseEvent, attrs: ResizeHandleAttrs) {
    e.preventDefault();
    const {axis, onResize, onResizeEnd} = attrs;
    const isVertical = axis === "vertical";
    const cursor = isVertical ? "ew-resize" : "ns-resize";
    let lastPos = isVertical ? e.clientX : e.clientY;

    this.moveListener = (ev: MouseEvent) => {
      const pos = isVertical ? ev.clientX : ev.clientY;
      const delta = pos - lastPos;
      if (delta !== 0) {
        lastPos = pos;
        onResize(delta);
        m.redraw();
      }
    };

    this.upListener = () => {
      this.cleanup();
      onResizeEnd?.();
      m.redraw();
    };

    document.body.style.cursor = cursor;
    document.body.style.userSelect = "none";
    document.addEventListener("mousemove", this.moveListener);
    document.addEventListener("mouseup", this.upListener);
  }

  private cleanup() {
    if (this.moveListener) document.removeEventListener("mousemove", this.moveListener);
    if (this.upListener) document.removeEventListener("mouseup", this.upListener);
    this.moveListener = undefined;
    this.upListener = undefined;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }
}
