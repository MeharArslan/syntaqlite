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

// ── UI references ──

const ui = {
  extensionFile: document.querySelector("#extension-file"),
  extensionName: document.querySelector("#extension-name"),
  clearExtension: document.querySelector("#clear-extension"),
  toggleFmtOptions: document.querySelector("#toggle-fmt-options"),
  fmtPopover: document.querySelector("#fmt-popover"),
  engineStatus: document.querySelector("#engine-status"),
  sqlInput: document.querySelector("#sql-input"),
  lineWidth: document.querySelector("#line-width"),
  keywordCase: document.querySelector("#keyword-case"),
  semicolons: document.querySelector("#semicolons"),
  formatOutput: document.querySelector("#format-output"),
  astOutput: document.querySelector("#ast-output"),
  tabs: document.querySelectorAll(".tab"),
  tabPanels: document.querySelectorAll(".tab-panel"),
};

const state = {
  runtime: null,
  builtinDialect: null,
  uploadedDialect: null,
  activeDialect: null,
  activeTab: "format",
  debounceTimer: null,
};

// ── Status ──

function updateStatus(text, isError = false) {
  ui.engineStatus.textContent = text;
  ui.engineStatus.classList.toggle("error", isError);
}

function ensureReady() {
  if (!state.runtime) {
    throw new Error("runtime module is not loaded");
  }
  if (!state.activeDialect) {
    throw new Error("no active dialect loaded");
  }
}

// ── Dialect ──

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
  updateStatus("Ready.");
}

async function onDialectFileChanged() {
  if (!state.runtime) {
    updateStatus("Runtime is not initialized yet.", true);
    return;
  }

  const file = ui.extensionFile.files?.[0];
  if (!file) {
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
    updateStatus(`Dialect: ${file.name}`);
    scheduleAutoRun();
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
  ui.extensionFile.value = "";

  if (state.builtinDialect) {
    try {
      activateDialect(state.builtinDialect);
      updateStatus("Using built-in SQLite dialect.");
      scheduleAutoRun();
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

// ── Tabs ──

function switchTab(tabName) {
  state.activeTab = tabName;
  ui.tabs.forEach((t) => t.classList.toggle("active", t.dataset.tab === tabName));
  ui.tabPanels.forEach((p) => p.classList.toggle("active", p.dataset.panel === tabName));
  scheduleAutoRun();
}

// ── Format options popover ──

function toggleFmtPopover() {
  ui.fmtPopover.classList.toggle("hidden");
}

function closeFmtPopoverOnClickOutside(e) {
  if (
    !ui.fmtPopover.classList.contains("hidden") &&
    !ui.fmtPopover.contains(e.target) &&
    e.target !== ui.toggleFmtOptions
  ) {
    ui.fmtPopover.classList.add("hidden");
  }
}

// ── Running ──

function collectFormatOptions() {
  return {
    lineWidth: Math.max(20, Number(ui.lineWidth.value || 80)),
    keywordCase: Number(ui.keywordCase.value || 0),
    semicolons: ui.semicolons.checked,
  };
}

function runActiveTab() {
  try {
    ensureReady();
  } catch {
    return;
  }

  const sql = ui.sqlInput.value;

  if (state.activeTab === "format") {
    const result = state.runtime.runFmt(sql, collectFormatOptions());
    ui.formatOutput.textContent = result.ok ? result.text : `Error: ${result.text}`;
  } else {
    const result = state.runtime.runAst(sql);
    ui.astOutput.textContent = result.ok ? result.text : `Error: ${result.text}`;
  }
}

function scheduleAutoRun() {
  clearTimeout(state.debounceTimer);
  state.debounceTimer = setTimeout(runActiveTab, 150);
}

// ── Events ──

function bindEvents() {
  // Dialect: auto-load on file select
  ui.extensionFile.addEventListener("change", onDialectFileChanged);
  ui.clearExtension.addEventListener("click", onClearExtension);

  // Format options popover
  ui.toggleFmtOptions.addEventListener("click", toggleFmtPopover);
  document.addEventListener("click", closeFmtPopoverOnClickOutside);

  // Format options changes trigger re-run
  ui.lineWidth.addEventListener("input", scheduleAutoRun);
  ui.keywordCase.addEventListener("change", scheduleAutoRun);
  ui.semicolons.addEventListener("change", scheduleAutoRun);

  // Tabs
  ui.tabs.forEach((tab) => {
    tab.addEventListener("click", () => switchTab(tab.dataset.tab));
  });

  // Auto-run on SQL input change
  ui.sqlInput.addEventListener("input", scheduleAutoRun);
}

async function main() {
  bindEvents();
  try {
    await initRuntime();
    await initBuiltinDialect();
    // Run once with initial content
    runActiveTab();
  } catch (err) {
    updateStatus(`Failed to initialize: ${err.message}`, true);
  }
}

main();
