// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import type {Theme} from "../types";

export class ThemeManager {
  current: Theme;
  private listeners: Array<(theme: Theme) => void> = [];

  constructor(initial: Theme = "dark") {
    this.current = initial;
    this.apply(initial);
  }

  toggle(): void {
    this.current = this.current === "dark" ? "light" : "dark";
    this.apply(this.current);
  }

  apply(theme: Theme): void {
    document.documentElement.setAttribute("data-theme", theme);
    for (const fn of this.listeners) fn(theme);
  }

  onChange(fn: (theme: Theme) => void): () => void {
    this.listeners.push(fn);
    return () => {
      const idx = this.listeners.indexOf(fn);
      if (idx >= 0) this.listeners.splice(idx, 1);
    };
  }
}
