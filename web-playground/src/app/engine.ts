// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import type {
  AstResult,
  DiagnosticEntry,
  DiagnosticsResult,
  DialectBinding,
  EmscriptenModule,
  EmscriptenModuleConfig,
  FormatOptions,
  FormatResult,
} from "../types";

const RUNTIME_JS_PATH = "./syntaqlite-runtime.js";
const RUNTIME_WASM_PATH = "./syntaqlite-runtime.wasm";

type WasmFn = (...args: number[]) => number;

export class Engine {
  status = "Loading...";
  statusError = false;

  private module: EmscriptenModule | null = null;
  private encoder = new TextEncoder();
  private decoder = new TextDecoder();

  private setDialectRaw: WasmFn | null = null;
  private clearDialectRaw: WasmFn | null = null;
  private allocRaw: WasmFn | null = null;
  private freeRaw: WasmFn | null = null;
  private astRaw: WasmFn | null = null;
  private astJsonRaw: WasmFn | null = null;
  private fmtRaw: WasmFn | null = null;
  private diagnosticsRaw: WasmFn | null = null;
  private semanticTokensRaw: WasmFn | null = null;
  private resultPtrRaw: WasmFn | null = null;
  private resultLenRaw: WasmFn | null = null;
  private resultFreeRaw: WasmFn | null = null;

  get ready(): boolean {
    return this.module !== null;
  }

  updateStatus(text: string, isError = false): void {
    this.status = text;
    this.statusError = isError;
  }

  async load(): Promise<void> {
    const module = await loadRuntimeModule();
    this.module = module;
    this.setDialectRaw = this.resolveRuntimeFn("wasm_set_dialect");
    this.clearDialectRaw = this.resolveRuntimeFn("wasm_clear_dialect");
    this.allocRaw = this.resolveRuntimeFn("wasm_alloc");
    this.freeRaw = this.resolveRuntimeFn("wasm_free");
    this.astRaw = this.resolveRuntimeFn("wasm_ast");
    this.astJsonRaw = this.resolveRuntimeFn("wasm_ast_json");
    this.fmtRaw = this.resolveRuntimeFn("wasm_fmt");
    this.diagnosticsRaw = this.tryResolveRuntimeFn("wasm_diagnostics");
    this.semanticTokensRaw = this.tryResolveRuntimeFn("wasm_semantic_tokens");
    this.resultPtrRaw = this.resolveRuntimeFn("wasm_result_ptr");
    this.resultLenRaw = this.resolveRuntimeFn("wasm_result_len");
    this.resultFreeRaw = this.resolveRuntimeFn("wasm_result_free");
  }

  private resolveRuntimeFn(symbol: string): WasmFn {
    const fn = this.module![`_${symbol}`];
    if (typeof fn !== "function") {
      throw new Error(`missing runtime function: _${symbol}`);
    }
    return fn;
  }

  /** Like resolveRuntimeFn but returns null if not found. */
  private tryResolveRuntimeFn(symbol: string): WasmFn | null {
    const fn = this.module![`_${symbol}`];
    return typeof fn === "function" ? fn : null;
  }

  private resolveDialectFn(
    symbol: string,
    localScope: Record<string, unknown> | null = null,
  ): WasmFn {
    if (localScope && typeof localScope[symbol] === "function") {
      return localScope[symbol] as WasmFn;
    }
    if (localScope && typeof localScope[`_${symbol}`] === "function") {
      return localScope[`_${symbol}`] as WasmFn;
    }
    const direct = this.module![`_${symbol}`];
    if (typeof direct === "function") {
      return direct;
    }
    if (typeof this.module!.cwrap === "function") {
      try {
        return this.module!.cwrap(symbol, "number", []);
      } catch {
        // Fall through to explicit error below.
      }
    }
    throw new Error(`missing dialect symbol: ${symbol}`);
  }

  private heapU8(): Uint8Array {
    const heap = this.module!.HEAPU8 || window.HEAPU8;
    if (!heap) throw new Error("runtime HEAPU8 is not available");
    return heap;
  }

  async loadDialectFromUrl(url: string, symbol: string): Promise<DialectBinding> {
    if (typeof this.module!.loadDynamicLibrary !== "function") {
      throw new Error("runtime module does not expose loadDynamicLibrary");
    }
    const localScope: Record<string, unknown> = {};
    const maybePromise = this.module!.loadDynamicLibrary(
      url,
      {loadAsync: true, global: false, nodelete: true},
      localScope,
    );
    if (maybePromise && typeof (maybePromise as Promise<void>).then === "function") {
      await maybePromise;
    }
    let ptr: number;
    try {
      const fn = this.resolveDialectFn(symbol, localScope);
      ptr = fn() >>> 0;
    } catch {
      throw new Error(`Symbol "${symbol}" not found in the WASM module.`);
    }
    if (ptr === 0) throw new Error(`Symbol "${symbol}" returned null.`);
    this.setDialectPointer(ptr);
    return {symbol, ptr, label: symbol};
  }

  private withInput<T>(sql: string, fn: (ptr: number, len: number) => T): T {
    const input = this.encoder.encode(sql);
    const ptr = this.allocRaw!(input.length);
    if (input.length > 0 && ptr === 0) throw new Error("allocation failed");
    if (input.length > 0) this.heapU8().set(input, ptr);
    try {
      return fn(ptr, input.length);
    } finally {
      this.freeRaw!(ptr, input.length);
    }
  }

  private readAndClearResult(): string {
    const ptr = this.resultPtrRaw!();
    const len = this.resultLenRaw!();
    const text = len === 0 ? "" : this.decoder.decode(this.heapU8().subarray(ptr, ptr + len));
    this.resultFreeRaw!();
    return text;
  }

  setDialectPointer(ptr: number): void {
    const status = this.setDialectRaw!(ptr >>> 0);
    const detail = this.readAndClearResult();
    if (status !== 0) {
      throw new Error(detail || `wasm_set_dialect failed with status ${status}`);
    }
  }

  clearDialectPointer(): void {
    this.clearDialectRaw!();
    this.readAndClearResult();
  }

  runAst(sql: string): FormatResult {
    const status = this.withInput(sql, (ptr, len) => this.astRaw!(ptr, len));
    const text = this.readAndClearResult();
    return {ok: status === 0, text};
  }

  runAstJson(sql: string): AstResult {
    const status = this.withInput(sql, (ptr, len) => this.astJsonRaw!(ptr, len));
    const text = this.readAndClearResult();
    if (status !== 0) return {ok: false, error: text};
    try {
      return {ok: true, statements: JSON.parse(text)};
    } catch (e) {
      return {ok: false, error: `JSON parse error: ${(e as Error).message}`};
    }
  }

  runFmt(sql: string, opts: FormatOptions): FormatResult {
    const status = this.withInput(sql, (ptr, len) =>
      this.fmtRaw!(ptr, len, opts.lineWidth, opts.keywordCase, opts.semicolons ? 1 : 0),
    );
    const text = this.readAndClearResult();
    return {ok: status === 0, text};
  }

  /** Run semantic token analysis over a byte range. Returns a pre-encoded
   *  Uint32Array (5 u32s per token: deltaLine, deltaStartChar, length,
   *  legendIndex, 0) ready for Monaco, or null on failure.
   *  Pass rangeStart=0 and rangeEnd=0xFFFFFFFF for the full document. */
  runSemanticTokens(
    sql: string,
    rangeStart = 0,
    rangeEnd = 0xffffffff,
    version = 1,
  ): Uint32Array | null {
    if (!this.semanticTokensRaw) return null;
    try {
      const count = this.withInput(sql, (ptr, len) =>
        this.semanticTokensRaw!(ptr, len, rangeStart, rangeEnd, version),
      );
      if (count <= 0) {
        this.resultFreeRaw!();
        return count === 0 ? new Uint32Array(0) : null;
      }
      // Read raw bytes from RESULT_BUF as a Uint32Array (5 u32s per token).
      const rptr = this.resultPtrRaw!();
      const rlen = this.resultLenRaw!();
      const bytes = this.heapU8().slice(rptr, rptr + rlen);
      this.resultFreeRaw!();
      return new Uint32Array(bytes.buffer, bytes.byteOffset, bytes.byteLength / 4);
    } catch (e) {
      console.warn("wasm_semantic_tokens failed:", e);
      return null;
    }
  }

  runDiagnostics(sql: string, version = 1): DiagnosticsResult {
    if (!this.diagnosticsRaw) return {ok: false, diagnostics: []};
    try {
      const count = this.withInput(sql, (ptr, len) => this.diagnosticsRaw!(ptr, len, version));
      const text = this.readAndClearResult();
      if (count < 0) return {ok: false, diagnostics: []};
      if (count === 0) return {ok: true, diagnostics: []};
      const diagnostics: DiagnosticEntry[] = JSON.parse(text);
      return {ok: true, diagnostics};
    } catch (e) {
      console.warn("wasm_diagnostics failed:", e);
      return {ok: false, diagnostics: []};
    }
  }
}

function loadRuntimeModule(): Promise<EmscriptenModule> {
  return new Promise<EmscriptenModule>((resolve, reject) => {
    const moduleConfig: EmscriptenModuleConfig = {
      noInitialRun: true,
      locateFile(path: string) {
        if (path === "syntaqlite_wasm.wasm" || path === "syntaqlite-wasm.wasm") {
          return RUNTIME_WASM_PATH;
        }
        return path;
      },
      onRuntimeInitialized() {
        resolve(moduleConfig as unknown as EmscriptenModule);
      },
      onAbort(reason: string) {
        reject(new Error(`runtime aborted: ${reason}`));
      },
    };

    window.Module = moduleConfig;

    const script = document.createElement("script");
    script.src = RUNTIME_JS_PATH;
    script.async = true;
    script.onerror = () => reject(new Error(`failed to load ${RUNTIME_JS_PATH}`));
    document.head.appendChild(script);
  });
}
