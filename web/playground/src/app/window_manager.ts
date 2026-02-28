// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";

const MOBILE_BREAKPOINT = 700;

export class WindowManager {
  isMobile: boolean;

  constructor() {
    const mql = window.matchMedia(`(max-width: ${MOBILE_BREAKPOINT}px)`);
    this.isMobile = mql.matches;
    mql.addEventListener("change", (e) => {
      this.isMobile = e.matches;
      m.redraw();
    });
  }
}
