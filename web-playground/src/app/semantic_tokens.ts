// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import * as monaco from "monaco-editor";
import type {Engine} from "./engine";

// Legend order must match the LSP legend in lsp.rs.
const TOKEN_LEGEND: monaco.languages.SemanticTokensLegend = {
  tokenTypes: [
    "keyword",      // 0
    "variable",     // 1
    "string",       // 2
    "number",       // 3
    "operator",     // 4
    "comment",      // 5
    "punctuation",  // 6
    "type",         // 7 (identifier)
    "function",     // 8
  ],
  tokenModifiers: [],
};

// Map TokenCategory enum value → legend index.
// Category values: 1=keyword, 2=identifier, 3=string, 4=number,
// 5=operator, 6=punctuation, 7=comment, 8=variable, 9=function.
function categoryToLegendIndex(cat: number): number {
  switch (cat) {
    case 1: return 0; // keyword
    case 2: return 7; // identifier → type
    case 3: return 2; // string
    case 4: return 3; // number
    case 5: return 4; // operator
    case 6: return 6; // punctuation
    case 7: return 5; // comment
    case 8: return 1; // variable
    case 9: return 8; // function
    default: return 0;
  }
}

/** Convert a byte offset to a 0-based line and character position. */
function offsetToLineChar(source: string, offset: number): {line: number; char: number} {
  const clamped = Math.min(offset, source.length);
  let line = 0;
  let char = 0;
  for (let i = 0; i < clamped; i++) {
    if (source[i] === "\n") {
      line++;
      char = 0;
    } else {
      char++;
    }
  }
  return {line, char};
}

let disposable: monaco.IDisposable | null = null;

export function registerSemanticTokensProvider(engine: Engine): void {
  if (disposable) {
    disposable.dispose();
  }

  disposable = monaco.languages.registerDocumentSemanticTokensProvider("sql", {
    getLegend(): monaco.languages.SemanticTokensLegend {
      return TOKEN_LEGEND;
    },

    provideDocumentSemanticTokens(
      model: monaco.editor.ITextModel,
    ): monaco.languages.ProviderResult<monaco.languages.SemanticTokens> {
      if (!engine.ready) return {data: new Uint32Array(0)};

      const source = model.getValue();
      const result = engine.runSemanticTokens(source);
      if (!result.ok || result.tokens.length === 0) {
        return {data: new Uint32Array(0)};
      }

      // Encode into Monaco's delta format: [deltaLine, deltaStartChar, length, tokenTypeIndex, modifiersBitset]
      const data = new Uint32Array(result.tokens.length * 5);
      let prevLine = 0;
      let prevStart = 0;

      for (let i = 0; i < result.tokens.length; i++) {
        const tok = result.tokens[i];
        const pos = offsetToLineChar(source, tok.o);
        const deltaLine = pos.line - prevLine;
        const deltaStart = deltaLine === 0 ? pos.char - prevStart : pos.char;

        const idx = i * 5;
        data[idx] = deltaLine;
        data[idx + 1] = deltaStart;
        data[idx + 2] = tok.l;
        data[idx + 3] = categoryToLegendIndex(tok.t);
        data[idx + 4] = 0;

        prevLine = pos.line;
        prevStart = pos.char;
      }

      return {data};
    },

    releaseDocumentSemanticTokens(): void {
      // Nothing to release.
    },
  });
}
