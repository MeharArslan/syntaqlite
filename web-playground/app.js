const RUNTIME_JS_PATH = "./syntaqlite-runtime.js";
const RUNTIME_WASM_PATH = "./syntaqlite-runtime.wasm";
const BUILTIN_DIALECT_WASM_PATH = "./syntaqlite-sqlite.wasm";
const BUILTIN_DIALECT_SYMBOL = "syntaqlite_sqlite_dialect";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

let runtimeModulePromise = null;

class RuntimeEngine {
  constructor(module) {
    this.module = module;

    this.setDialectRaw = this.resolveRuntimeFn("wasm_set_dialect");
    this.clearDialectRaw = this.resolveRuntimeFn("wasm_clear_dialect");
    this.allocRaw = this.resolveRuntimeFn("wasm_alloc");
    this.freeRaw = this.resolveRuntimeFn("wasm_free");
    this.astRaw = this.resolveRuntimeFn("wasm_ast");
    this.fmtRaw = this.resolveRuntimeFn("wasm_fmt");
    this.resultPtrRaw = this.resolveRuntimeFn("wasm_result_ptr");
    this.resultLenRaw = this.resolveRuntimeFn("wasm_result_len");
    this.resultFreeRaw = this.resolveRuntimeFn("wasm_result_free");
  }

  static async load() {
    const module = await loadRuntimeModule();
    return new RuntimeEngine(module);
  }

  resolveRuntimeFn(symbol) {
    const fn = this.module[`_${symbol}`];
    if (typeof fn !== "function") {
      throw new Error(`missing runtime function: _${symbol}`);
    }
    return fn;
  }

  resolveDialectFn(symbol, localScope = null) {
    if (localScope && typeof localScope[symbol] === "function") {
      return localScope[symbol];
    }
    if (localScope && typeof localScope[`_${symbol}`] === "function") {
      return localScope[`_${symbol}`];
    }

    const direct = this.module[`_${symbol}`];
    if (typeof direct === "function") {
      return direct;
    }

    if (typeof this.module.cwrap === "function") {
      try {
        return this.module.cwrap(symbol, "number", []);
      } catch {
        // Fall through to explicit error below.
      }
    }

    throw new Error(`missing dialect symbol: ${symbol}`);
  }

  heapU8() {
    const heap = this.module.HEAPU8 || globalThis.HEAPU8;
    if (!heap) {
      throw new Error("runtime HEAPU8 is not available");
    }
    return heap;
  }

  async loadDialectFromUrl(url, symbol) {
    if (typeof this.module.loadDynamicLibrary !== "function") {
      throw new Error("runtime module does not expose loadDynamicLibrary");
    }

    const localScope = {};
    const maybePromise = this.module.loadDynamicLibrary(url, {
      loadAsync: true,
      global: false,
      nodelete: true,
    }, localScope);

    if (maybePromise && typeof maybePromise.then === "function") {
      await maybePromise;
    }

    const fn = this.resolveDialectFn(symbol, localScope);
    const ptr = fn() >>> 0;
    if (ptr === 0) {
      throw new Error(`${symbol} returned null`);
    }

    this.setDialectPointer(ptr);
    return ptr;
  }

  withInput(sql, fn) {
    const input = encoder.encode(sql);
    const ptr = this.allocRaw(input.length);
    if (input.length > 0 && ptr === 0) {
      throw new Error("allocation failed");
    }

    if (input.length > 0) {
      this.heapU8().set(input, ptr);
    }

    try {
      return fn(ptr, input.length);
    } finally {
      this.freeRaw(ptr, input.length);
    }
  }

  readAndClearResult() {
    const ptr = this.resultPtrRaw();
    const len = this.resultLenRaw();
    const text = len === 0 ? "" : decoder.decode(this.heapU8().subarray(ptr, ptr + len));
    this.resultFreeRaw();
    return text;
  }

  setDialectPointer(ptr) {
    const status = this.setDialectRaw(ptr >>> 0);
    const detail = this.readAndClearResult();
    if (status !== 0) {
      throw new Error(detail || `wasm_set_dialect failed with status ${status}`);
    }
  }

  clearDialectPointer() {
    this.clearDialectRaw();
    this.readAndClearResult();
  }

  runAst(sql) {
    const status = this.withInput(sql, (ptr, len) => this.astRaw(ptr, len));
    const text = this.readAndClearResult();
    return { ok: status === 0, text };
  }

  runFmt(sql, opts) {
    const status = this.withInput(sql, (ptr, len) =>
      this.fmtRaw(ptr, len, opts.lineWidth, opts.keywordCase, opts.semicolons ? 1 : 0)
    );
    const text = this.readAndClearResult();
    return { ok: status === 0, text };
  }
}

function loadRuntimeModule() {
  if (runtimeModulePromise) {
    return runtimeModulePromise;
  }

  runtimeModulePromise = new Promise((resolve, reject) => {
    const moduleConfig = {
      noInitialRun: true,
      locateFile(path) {
        if (path === "syntaqlite_wasm.wasm" || path === "syntaqlite-wasm.wasm") {
          return RUNTIME_WASM_PATH;
        }
        return path;
      },
      onRuntimeInitialized() {
        resolve(moduleConfig);
      },
      onAbort(reason) {
        reject(new Error(`runtime aborted: ${reason}`));
      },
    };

    window.Module = moduleConfig;

    const script = document.createElement("script");
    script.src = RUNTIME_JS_PATH;
    script.async = true;
    script.onerror = () => {
      reject(new Error(`failed to load ${RUNTIME_JS_PATH}`));
    };
    document.head.appendChild(script);
  });

  return runtimeModulePromise;
}

function dialectSymbolFromName(name) {
  if (!name || !name.trim()) {
    return "syntaqlite_dialect";
  }
  return `syntaqlite_${name.trim()}_dialect`;
}

const ui = {
  extensionFile: document.querySelector("#extension-file"),
  extensionName: document.querySelector("#extension-name"),
  loadExtension: document.querySelector("#load-extension"),
  clearExtension: document.querySelector("#clear-extension"),
  engineStatus: document.querySelector("#engine-status"),
  sqlInput: document.querySelector("#sql-input"),
  lineWidth: document.querySelector("#line-width"),
  keywordCase: document.querySelector("#keyword-case"),
  semicolons: document.querySelector("#semicolons"),
  runFormat: document.querySelector("#run-format"),
  runAst: document.querySelector("#run-ast"),
  runBoth: document.querySelector("#run-both"),
  formatOutput: document.querySelector("#format-output"),
  astOutput: document.querySelector("#ast-output"),
};

const state = {
  runtime: null,
  builtinDialect: null,
  uploadedDialect: null,
  activeDialect: null,
};

function updateStatus(text, isError = false) {
  ui.engineStatus.textContent = text;
  ui.engineStatus.style.color = isError ? "#b00020" : "";
}

function ensureReady() {
  if (!state.runtime) {
    throw new Error("runtime module is not loaded");
  }
  if (!state.activeDialect) {
    throw new Error("no active dialect loaded");
  }
}

function activateDialect(binding) {
  state.runtime.setDialectPointer(binding.ptr);
  state.activeDialect = binding;
}

async function initRuntime() {
  state.runtime = await RuntimeEngine.load();
}

async function initBuiltinDialect() {
  const ptr = await state.runtime.loadDialectFromUrl(
    BUILTIN_DIALECT_WASM_PATH,
    BUILTIN_DIALECT_SYMBOL
  );
  const binding = {
    symbol: BUILTIN_DIALECT_SYMBOL,
    ptr,
    label: "Built-in SQLite",
  };
  activateDialect(binding);
  state.builtinDialect = binding;
  updateStatus("Runtime and built-in SQLite dialect ready.");
}

async function onLoadExtension() {
  if (!state.runtime) {
    updateStatus("Runtime is not initialized yet.", true);
    return;
  }

  const file = ui.extensionFile.files?.[0];
  if (!file) {
    updateStatus("Select a dialect .wasm file first.", true);
    return;
  }

  const symbol = dialectSymbolFromName(ui.extensionName.value);
  const url = URL.createObjectURL(file);

  try {
    const ptr = await state.runtime.loadDialectFromUrl(url, symbol);
    const binding = {
      symbol,
      ptr,
      label: file.name,
    };
    activateDialect(binding);
    state.uploadedDialect = binding;
    updateStatus(`Loaded dialect ${file.name} via ${symbol} (ptr=${ptr}).`);
  } catch (err) {
    updateStatus(`Failed to load dialect: ${err.message}`, true);
  } finally {
    URL.revokeObjectURL(url);
  }
}

function onClearExtension() {
  if (!state.runtime) {
    updateStatus("Runtime is not initialized yet.", true);
    return;
  }

  state.uploadedDialect = null;

  if (state.builtinDialect) {
    try {
      activateDialect(state.builtinDialect);
      updateStatus("Uploaded dialect removed. Using built-in SQLite dialect.");
      return;
    } catch (err) {
      updateStatus(`Failed to restore built-in dialect: ${err.message}`, true);
      return;
    }
  }

  state.runtime.clearDialectPointer();
  state.activeDialect = null;
  updateStatus("Dialect cleared.");
}

function collectFormatOptions() {
  return {
    lineWidth: Math.max(20, Number(ui.lineWidth.value || 80)),
    keywordCase: Number(ui.keywordCase.value || 0),
    semicolons: ui.semicolons.checked,
  };
}

function runFormat() {
  try {
    ensureReady();
  } catch (err) {
    updateStatus(err.message, true);
    return;
  }

  const sql = ui.sqlInput.value;
  const result = state.runtime.runFmt(sql, collectFormatOptions());
  if (!result.ok) {
    ui.formatOutput.textContent = "";
    updateStatus(`Format error: ${result.text}`, true);
    return;
  }

  ui.formatOutput.textContent = result.text;
  updateStatus(`Format completed via ${state.activeDialect.label}.`);
}

function runAst() {
  try {
    ensureReady();
  } catch (err) {
    updateStatus(err.message, true);
    return;
  }

  const sql = ui.sqlInput.value;
  const result = state.runtime.runAst(sql);
  if (!result.ok) {
    ui.astOutput.textContent = "";
    updateStatus(`AST error: ${result.text}`, true);
    return;
  }

  ui.astOutput.textContent = result.text;
  updateStatus(`AST completed via ${state.activeDialect.label}.`);
}

function bindEvents() {
  ui.loadExtension.addEventListener("click", onLoadExtension);
  ui.clearExtension.addEventListener("click", onClearExtension);
  ui.runFormat.addEventListener("click", runFormat);
  ui.runAst.addEventListener("click", runAst);
  ui.runBoth.addEventListener("click", () => {
    runFormat();
    runAst();
  });
}

async function main() {
  bindEvents();
  try {
    await initRuntime();
    await initBuiltinDialect();
  } catch (err) {
    updateStatus(`Failed to initialize playground: ${err.message}`, true);
  }
}

main();
