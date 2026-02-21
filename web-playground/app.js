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
    this.astJsonRaw = this.resolveRuntimeFn("wasm_ast_json");
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

  runAstJson(sql) {
    const status = this.withInput(sql, (ptr, len) => this.astJsonRaw(ptr, len));
    const text = this.readAndClearResult();
    if (status !== 0) {
      return { ok: false, error: text };
    }
    try {
      return { ok: true, statements: JSON.parse(text) };
    } catch (e) {
      return { ok: false, error: `JSON parse error: ${e.message}` };
    }
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
  engineStatus: document.querySelector("#engine-status"),
  sqlInput: document.querySelector("#sql-input"),
  lineWidth: document.querySelector("#line-width"),
  keywordCase: document.querySelector("#keyword-case"),
  semicolons: document.querySelector("#semicolons"),
  formatOutput: document.querySelector("#format-output"),
  astOutput: document.querySelector("#ast-output"),
  astShowNulls: document.querySelector("#ast-show-nulls"),
  astViewMode: document.querySelector("#ast-view-mode"),
  astCanvas: document.querySelector("#ast-canvas"),
  astCanvasContainer: document.querySelector("#ast-canvas-container"),
  astPanel: document.querySelector('[data-panel="ast"]'),
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

// ── AST tree rendering ──

/**
 * Render an array of AST node objects (from JSON) into a DOM tree.
 *
 * Node shapes (from wasm_ast_json):
 *   List:  { type:"list", name, count, children:[] }
 *   Node:  { type:"node", name, fields:[] }
 *   Field: { label, kind:"node"|"span"|"bool"|"enum"|"flags", value|child }
 */
function renderTree(nodes) {
  const frag = document.createDocumentFragment();

  for (const node of nodes) {
    const wrapper = document.createElement("div");
    wrapper.className = "ast-node";

    if (node.type === "list") {
      const details = document.createElement("details");
      details.open = true;
      const summary = document.createElement("summary");
      summary.innerHTML =
        `<span class="ast-node-name">${esc(node.name)}</span>` +
        `<span class="ast-list-count">[${node.count}]</span>`;
      details.appendChild(summary);
      if (node.children && node.children.length > 0) {
        details.appendChild(renderTree(node.children));
      }
      wrapper.appendChild(details);
    } else if (node.type === "node") {
      const details = document.createElement("details");
      details.open = true;
      const summary = document.createElement("summary");
      summary.innerHTML =
        `<span class="ast-node-name">${esc(node.name)}</span>`;
      details.appendChild(summary);
      if (node.fields && node.fields.length > 0) {
        details.appendChild(renderFields(node.fields));
      }
      wrapper.appendChild(details);
    }

    frag.appendChild(wrapper);
  }

  const container = document.createElement("div");
  container.appendChild(frag);
  return container;
}

function isFieldEmpty(f) {
  if (f.kind === "node") return f.child === null;
  if (f.kind === "span") return f.value === null;
  if (f.kind === "bool") return f.value === false;
  if (f.kind === "enum") return f.value === null;
  if (f.kind === "flags") return f.value.length === 0;
  return false;
}

function renderFields(fields) {
  const showNulls = ui.astShowNulls.checked;
  const frag = document.createDocumentFragment();

  for (const f of fields) {
    if (!showNulls && isFieldEmpty(f)) continue;
    const wrapper = document.createElement("div");
    wrapper.className = "ast-node";

    if (f.kind === "node") {
      if (f.child === null) {
        // Null child — render as leaf.
        const div = document.createElement("div");
        div.className = "ast-leaf";
        div.innerHTML =
          `<span class="ast-field-label">${esc(f.label)}:</span> ` +
          `<span class="ast-leaf-value null-val">(none)</span>`;
        wrapper.appendChild(div);
      } else {
        // Non-null child — collapsible field label wrapping the child node.
        const details = document.createElement("details");
        details.open = true;
        const summary = document.createElement("summary");
        summary.innerHTML =
          `<span class="ast-field-label">${esc(f.label)}:</span>`;
        details.appendChild(summary);
        details.appendChild(renderTree([f.child]));
        wrapper.appendChild(details);
      }
    } else if (f.kind === "span") {
      const div = document.createElement("div");
      div.className = "ast-leaf";
      if (f.value === null) {
        div.innerHTML =
          `<span class="ast-field-label">${esc(f.label)}:</span> ` +
          `<span class="ast-leaf-value null-val">(none)</span>`;
      } else {
        div.innerHTML =
          `<span class="ast-field-label">${esc(f.label)}:</span> ` +
          `<span class="ast-leaf-value string-val">"${esc(f.value)}"</span>`;
      }
      wrapper.appendChild(div);
    } else if (f.kind === "bool") {
      const div = document.createElement("div");
      div.className = "ast-leaf";
      const display = f.value ? "TRUE" : "FALSE";
      div.innerHTML =
        `<span class="ast-field-label">${esc(f.label)}:</span> ` +
        `<span class="ast-leaf-value bool-val">${display}</span>`;
      wrapper.appendChild(div);
    } else if (f.kind === "enum") {
      const div = document.createElement("div");
      div.className = "ast-leaf";
      if (f.value === null) {
        div.innerHTML =
          `<span class="ast-field-label">${esc(f.label)}:</span> ` +
          `<span class="ast-leaf-value null-val">(none)</span>`;
      } else {
        div.innerHTML =
          `<span class="ast-field-label">${esc(f.label)}:</span> ` +
          `<span class="ast-leaf-value">${esc(String(f.value))}</span>`;
      }
      wrapper.appendChild(div);
    } else if (f.kind === "flags") {
      const div = document.createElement("div");
      div.className = "ast-leaf";
      const display = f.value.length === 0 ? "(none)" : f.value.join(" | ");
      const cls = f.value.length === 0 ? "null-val" : "";
      div.innerHTML =
        `<span class="ast-field-label">${esc(f.label)}:</span> ` +
        `<span class="ast-leaf-value ${cls}">${esc(display)}</span>`;
      wrapper.appendChild(div);
    }

    frag.appendChild(wrapper);
  }

  const container = document.createElement("div");
  container.appendChild(frag);
  return container;
}

function esc(s) {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

// ── AST graph visualization ──

/**
 * Flatten the JSON AST into a uniform VisNode tree for canvas rendering.
 * Returns an array of root VisNodes (one per statement).
 */
function flattenAst(statements, showNulls) {
  function flattenNode(node) {
    if (node.type === "list") {
      const children = (node.children || []).map(flattenNode);
      return { label: node.name, kind: "list", leafText: `[${node.count}]`, children, collapsed: false, x: 0, y: 0, w: 0, h: 0 };
    }
    // node.type === "node"
    const children = [];
    const leafLines = [];
    for (const f of (node.fields || [])) {
      if (!showNulls && isFieldEmpty(f)) continue;
      if (f.kind === "node") {
        if (f.child === null) {
          leafLines.push(`${f.label}: (none)`);
        } else {
          const child = flattenNode(f.child);
          child.fieldLabel = f.label;
          children.push(child);
        }
      } else if (f.kind === "span") {
        leafLines.push(f.value === null ? `${f.label}: (none)` : `${f.label}: "${f.value}"`);
      } else if (f.kind === "bool") {
        leafLines.push(`${f.label}: ${f.value ? "TRUE" : "FALSE"}`);
      } else if (f.kind === "enum") {
        leafLines.push(f.value === null ? `${f.label}: (none)` : `${f.label}: ${f.value}`);
      } else if (f.kind === "flags") {
        const display = f.value.length === 0 ? "(none)" : f.value.join(" | ");
        leafLines.push(`${f.label}: ${display}`);
      }
    }
    return { label: node.name, kind: "node", leafText: leafLines.join("\n"), children, collapsed: false, x: 0, y: 0, w: 0, h: 0 };
  }
  return statements.map(flattenNode);
}

/**
 * Layout a tree using a simplified Reingold-Tilford algorithm.
 * Mutates nodes in-place with x, y, w, h.
 */
function layoutTree(roots, ctx) {
  const FONT_LABEL = "bold 12px 'JetBrains Mono', monospace";
  const FONT_LEAF = "10px 'JetBrains Mono', monospace";
  const FONT_FIELD = "10px 'JetBrains Mono', monospace";
  const PAD_X = 10;
  const PAD_Y = 6;
  const V_GAP = 50;
  const H_GAP = 16;
  const MIN_W = 60;
  const LINE_H = 14;

  // Measure badge width once
  ctx.font = "bold 10px 'JetBrains Mono', monospace";
  const BADGE_W = ctx.measureText("[+]").width + 12; // badge text + gap

  function measure(node) {
    // Measure label line
    ctx.font = FONT_LABEL;
    let displayLabel = node.label;
    if (node.fieldLabel) displayLabel = `${node.fieldLabel}: ${node.label}`;
    let labelW = ctx.measureText(displayLabel).width;
    // Reserve space for collapse badge next to label
    if (node.children.length > 0) labelW += BADGE_W;
    let maxW = labelW;

    // Measure leaf text lines
    let lineCount = 1; // label line
    if (node.kind === "list") {
      ctx.font = FONT_LEAF;
      maxW = Math.max(maxW, ctx.measureText(node.leafText).width);
      lineCount = 1; // label includes count
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

    // Recursively measure children
    if (!node.collapsed) {
      for (const child of node.children) {
        measure(child);
      }
    }
  }

  // Assign y positions based on depth and compute subtree widths bottom-up
  function computeSubtreeWidth(node) {
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

  function assignPositions(node, left, depth) {
    node.y = depth * (node.h + V_GAP);
    // Center this node within its subtree allocation
    node.x = left + node._subtreeW / 2 - node.w / 2;

    if (!node.collapsed && node.children.length > 0) {
      // Distribute children centered under the subtree width
      let totalChildW = 0;
      for (let i = 0; i < node.children.length; i++) {
        if (i > 0) totalChildW += H_GAP;
        totalChildW += node.children[i]._subtreeW;
      }
      let childLeft = left + (node._subtreeW - totalChildW) / 2;
      for (const child of node.children) {
        assignPositions(child, childLeft, depth + 1);
        childLeft += child._subtreeW + H_GAP;
      }
    }
  }

  // Measure all roots
  for (const root of roots) measure(root);

  // Layout multiple roots side by side
  let totalW = 0;
  for (const root of roots) {
    computeSubtreeWidth(root);
    totalW += root._subtreeW;
  }
  totalW += (roots.length - 1) * H_GAP * 2;

  let curX = 0;
  for (let i = 0; i < roots.length; i++) {
    assignPositions(roots[i], curX, 0);
    curX += roots[i]._subtreeW + H_GAP * 2;
  }

  // Compute total bounding box
  let maxX = 0, maxY = 0;
  function bounds(node) {
    maxX = Math.max(maxX, node.x + node.w);
    maxY = Math.max(maxY, node.y + node.h);
    if (!node.collapsed) {
      for (const child of node.children) bounds(child);
    }
  }
  for (const root of roots) bounds(root);

  return { roots, width: maxX, height: maxY };
}

class AstCanvasRenderer {
  constructor(canvas, container) {
    this.canvas = canvas;
    this.container = container;
    this.ctx = canvas.getContext("2d");
    this.tree = null;
    this.treeWidth = 0;
    this.treeHeight = 0;
    this.transform = { panX: 0, panY: 0, zoom: 1.0 };
    this.hoverNode = null;
    this.dragging = false;
    this.dragStart = { x: 0, y: 0 };
    this.dragPanStart = { x: 0, y: 0 };
    this.dragMoved = false;

    // Read CSS colors once
    const cs = getComputedStyle(document.documentElement);
    this.colors = {
      accent: cs.getPropertyValue("--accent").trim(),
      muted: cs.getPropertyValue("--muted").trim(),
      ink: cs.getPropertyValue("--ink").trim(),
      line: cs.getPropertyValue("--line").trim(),
      lineStrong: cs.getPropertyValue("--line-strong").trim(),
      surface: cs.getPropertyValue("--surface").trim(),
      codeBg: cs.getPropertyValue("--code-bg").trim(),
      accentSoft: cs.getPropertyValue("--accent-soft").trim(),
    };

    this._bindEvents();
    this._initResizeObserver();
  }

  _bindEvents() {
    this.canvas.addEventListener("mousedown", (e) => this._onMouseDown(e));
    this.canvas.addEventListener("mousemove", (e) => this._onMouseMove(e));
    this.canvas.addEventListener("mouseup", (e) => this._onMouseUp(e));
    this.canvas.addEventListener("mouseleave", () => this._onMouseLeave());
    this.canvas.addEventListener("wheel", (e) => this._onWheel(e), { passive: false });
  }

  _initResizeObserver() {
    this._resizeObserver = new ResizeObserver(() => {
      this._updateCanvasSize();
      this.render();
    });
    this._resizeObserver.observe(this.container);
  }

  _updateCanvasSize() {
    const rect = this.container.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    this.canvas.width = rect.width * dpr;
    this.canvas.height = rect.height * dpr;
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    this._displayWidth = rect.width;
    this._displayHeight = rect.height;
  }

  update(roots, showNulls) {
    this._updateCanvasSize();
    const layout = layoutTree(roots, this.ctx);
    this.tree = layout.roots;
    this.treeWidth = layout.width;
    this.treeHeight = layout.height;
    this.fitToView();
    this.render();
  }

  fitToView() {
    if (!this.tree || this.tree.length === 0) return;
    const pad = 40;
    const availW = this._displayWidth - pad * 2;
    const availH = this._displayHeight - pad * 2;
    if (availW <= 0 || availH <= 0) return;
    const scaleX = availW / this.treeWidth;
    const scaleY = availH / this.treeHeight;
    this.transform.zoom = Math.min(scaleX, scaleY, 2.0);
    this.transform.panX = (this._displayWidth - this.treeWidth * this.transform.zoom) / 2;
    this.transform.panY = pad;
  }

  render() {
    const ctx = this.ctx;
    const w = this._displayWidth;
    const h = this._displayHeight;
    if (!w || !h) return;

    ctx.save();
    ctx.clearRect(0, 0, w, h);
    ctx.translate(this.transform.panX, this.transform.panY);
    ctx.scale(this.transform.zoom, this.transform.zoom);

    if (this.tree) {
      // Draw edges first
      for (const root of this.tree) this._drawEdges(root);
      // Draw nodes on top
      for (const root of this.tree) this._drawNodes(root);
    }

    ctx.restore();
  }

  _drawEdges(node) {
    if (node.collapsed) return;
    const ctx = this.ctx;
    ctx.strokeStyle = this.colors.lineStrong;
    ctx.lineWidth = 1;
    for (const child of node.children) {
      const fromX = node.x + node.w / 2;
      const fromY = node.y + node.h;
      const toX = child.x + child.w / 2;
      const toY = child.y;
      ctx.beginPath();
      ctx.moveTo(fromX, fromY);
      // Bezier curve for smoother edges
      const midY = (fromY + toY) / 2;
      ctx.bezierCurveTo(fromX, midY, toX, midY, toX, toY);
      ctx.stroke();
      this._drawEdges(child);
    }
  }

  _drawNodes(node) {
    const ctx = this.ctx;
    const isHover = node === this.hoverNode;
    const r = 6;

    // Rounded rect
    ctx.beginPath();
    ctx.roundRect(node.x, node.y, node.w, node.h, r);
    ctx.fillStyle = isHover ? this.colors.accentSoft : this.colors.surface;
    ctx.fill();
    ctx.strokeStyle = node.collapsed ? this.colors.muted : this.colors.line;
    ctx.lineWidth = 1;
    if (node.collapsed) {
      ctx.setLineDash([4, 3]);
    }
    ctx.stroke();
    ctx.setLineDash([]);

    // Label
    let displayLabel = node.label;
    if (node.fieldLabel) displayLabel = `${node.fieldLabel}: ${node.label}`;
    ctx.font = "bold 12px 'JetBrains Mono', monospace";
    ctx.fillStyle = this.colors.accent;
    ctx.textBaseline = "top";
    const textX = node.x + 10;
    let textY = node.y + 6;
    ctx.fillText(displayLabel, textX, textY);
    textY += 14;

    // Leaf text
    if (node.leafText && !node.collapsed) {
      ctx.font = "10px 'JetBrains Mono', monospace";
      ctx.fillStyle = this.colors.muted;
      const lines = node.kind === "list" ? [node.leafText] : node.leafText.split("\n");
      for (const line of lines) {
        if (node.kind === "list") {
          // For lists, draw count inline with label (skip — already in label area conceptually)
        } else {
          ctx.fillText(line, textX, textY);
          textY += 14;
        }
      }
    }

    // Collapse badge
    if (node.children.length > 0) {
      ctx.font = "bold 10px 'JetBrains Mono', monospace";
      ctx.fillStyle = this.colors.muted;
      const badge = node.collapsed ? "[+]" : "[-]";
      const badgeW = ctx.measureText(badge).width;
      ctx.fillText(badge, node.x + node.w - badgeW - 6, node.y + 6);
    }

    // Recurse
    if (!node.collapsed) {
      for (const child of node.children) this._drawNodes(child);
    }
  }

  _canvasToTree(e) {
    const rect = this.canvas.getBoundingClientRect();
    const cx = e.clientX - rect.left;
    const cy = e.clientY - rect.top;
    const tx = (cx - this.transform.panX) / this.transform.zoom;
    const ty = (cy - this.transform.panY) / this.transform.zoom;
    return { cx, cy, tx, ty };
  }

  _hitTest(tx, ty) {
    if (!this.tree) return null;
    function check(node) {
      if (tx >= node.x && tx <= node.x + node.w && ty >= node.y && ty <= node.y + node.h) {
        return node;
      }
      if (!node.collapsed) {
        for (const child of node.children) {
          const hit = check(child);
          if (hit) return hit;
        }
      }
      return null;
    }
    for (const root of this.tree) {
      const hit = check(root);
      if (hit) return hit;
    }
    return null;
  }

  _onMouseDown(e) {
    this.dragging = true;
    this.dragMoved = false;
    this.dragStart = { x: e.clientX, y: e.clientY };
    this.dragPanStart = { x: this.transform.panX, y: this.transform.panY };
  }

  _onMouseMove(e) {
    if (this.dragging) {
      const dx = e.clientX - this.dragStart.x;
      const dy = e.clientY - this.dragStart.y;
      if (Math.abs(dx) > 2 || Math.abs(dy) > 2) this.dragMoved = true;
      this.transform.panX = this.dragPanStart.x + dx;
      this.transform.panY = this.dragPanStart.y + dy;
      this.render();
    } else {
      const { tx, ty } = this._canvasToTree(e);
      const node = this._hitTest(tx, ty);
      if (node !== this.hoverNode) {
        this.hoverNode = node;
        this.canvas.style.cursor = node ? "pointer" : "grab";
        this.render();
      }
    }
  }

  _onMouseUp(e) {
    if (this.dragging && !this.dragMoved) {
      // Click — toggle collapse
      const { tx, ty } = this._canvasToTree(e);
      const node = this._hitTest(tx, ty);
      if (node && node.children.length > 0) {
        // Remember node's screen position before re-layout
        const oldScreenX = node.x * this.transform.zoom + this.transform.panX;
        const oldScreenY = node.y * this.transform.zoom + this.transform.panY;

        node.collapsed = !node.collapsed;
        // Re-layout
        const layout = layoutTree(this.tree, this.ctx);
        this.tree = layout.roots;
        this.treeWidth = layout.width;
        this.treeHeight = layout.height;

        // Anchor viewport so clicked node stays at its screen position
        this.transform.panX = oldScreenX - node.x * this.transform.zoom;
        this.transform.panY = oldScreenY - node.y * this.transform.zoom;
        this.render();
      }
    }
    this.dragging = false;
    this.canvas.style.cursor = this.hoverNode ? "pointer" : "grab";
  }

  _onMouseLeave() {
    this.dragging = false;
    if (this.hoverNode) {
      this.hoverNode = null;
      this.render();
    }
    this.canvas.style.cursor = "grab";
  }

  _onWheel(e) {
    e.preventDefault();
    const { cx, cy } = this._canvasToTree(e);
    const oldZoom = this.transform.zoom;
    const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
    const newZoom = Math.max(0.15, Math.min(4.0, oldZoom * factor));
    // Zoom centered on cursor position
    this.transform.panX = cx - (cx - this.transform.panX) * (newZoom / oldZoom);
    this.transform.panY = cy - (cy - this.transform.panY) * (newZoom / oldZoom);
    this.transform.zoom = newZoom;
    this.render();
  }
}

let astCanvasRenderer = null;

function renderAstOutput(result) {
  const isGraph = ui.astViewMode.value === "graph";
  ui.astPanel.classList.toggle("graph-mode", isGraph);

  if (!isGraph) {
    // Outline mode
    ui.astOutput.innerHTML = "";
    ui.astOutput.classList.remove("error-text");

    if (!result.ok) {
      ui.astOutput.classList.add("error-text");
      ui.astOutput.textContent = `Error: ${result.error}`;
      return;
    }

    if (result.statements.length === 0) {
      ui.astOutput.textContent = "(empty)";
      return;
    }

    ui.astOutput.appendChild(renderTree(result.statements));
    return;
  }

  // Graph mode
  if (!result.ok) {
    // Fall back to outline for errors
    ui.astPanel.classList.remove("graph-mode");
    ui.astOutput.innerHTML = "";
    ui.astOutput.classList.add("error-text");
    ui.astOutput.textContent = `Error: ${result.error}`;
    return;
  }

  if (result.statements.length === 0) {
    ui.astPanel.classList.remove("graph-mode");
    ui.astOutput.innerHTML = "";
    ui.astOutput.textContent = "(empty)";
    return;
  }

  if (!astCanvasRenderer) {
    astCanvasRenderer = new AstCanvasRenderer(ui.astCanvas, ui.astCanvasContainer);
  }

  const roots = flattenAst(result.statements, ui.astShowNulls.checked);
  astCanvasRenderer.update(roots, ui.astShowNulls.checked);
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
    const result = state.runtime.runAstJson(sql);
    renderAstOutput(result);
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

  // Format options changes trigger re-run
  ui.lineWidth.addEventListener("input", scheduleAutoRun);
  ui.keywordCase.addEventListener("change", scheduleAutoRun);
  ui.semicolons.addEventListener("change", scheduleAutoRun);

  // AST options
  ui.astShowNulls.addEventListener("change", scheduleAutoRun);
  ui.astViewMode.addEventListener("change", scheduleAutoRun);

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
