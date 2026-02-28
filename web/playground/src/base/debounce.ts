// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// biome-ignore lint/suspicious/noExplicitAny: generic debounce requires flexible signature
export function debounce<T extends (...args: any[]) => void>(fn: T, ms: number): T {
  let timer: ReturnType<typeof setTimeout> | undefined = undefined;
  // biome-ignore lint/suspicious/noExplicitAny: generic debounce requires flexible signature
  return ((...args: any[]) => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  }) as unknown as T;
}
