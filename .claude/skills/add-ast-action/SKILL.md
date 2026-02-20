---
name: add-ast-action
description: Add new CST-to-AST folding actions for SQLite grammar rules. Use when implementing new AST node types, adding grammar action rules, or extending the parser to handle additional SQL syntax. Covers the full workflow from node definition through testing, including debugging techniques.
---

# Add AST Action

Implement new CST-to-AST folding for SQLite grammar rules. This skill covers defining AST nodes, writing grammar action rules, code generation, and debugging.

## Step-by-step workflow

### 1. Identify upstream rules to implement

Check the upstream grammar for rules to implement:

```bash
grep -n 'your_nonterminal' third_party/src/sqlite/src/parse.y
```

**Critical**: Rule signatures in action files MUST match upstream parse.y exactly (same tokens, same alternations like `ID|INDEXED|JOIN_KW`).

### 2. Define AST nodes

Create or edit a `.synq` node definition file in `syntaqlite/parser-nodes/`.

See `syntaqlite/parser-nodes/SYNTAX.md` for the `.synq` file format.

### 3. Write grammar action rules

Create a `.y` file in `syntaqlite/parser-actions/`:

```c
// syntaqlite/parser-actions/your_actions.y

// Conventions:
// - pCtx: SyntaqliteParseContext*
// - pCtx->astCtx: AST context for builder calls
// - pCtx->zSql: Original SQL text
// - Terminals: SyntaqliteToken with .z (pointer), .n (length), .type
// - Non-terminals: uint32_t node IDs

your_rule(A) ::= SOME_TOKEN(B) expr(C). {
    A = ast_your_node(pCtx->astCtx, syntaqlite_span(pCtx, B), C);
}
```

**Key patterns**:
- Token to source span: `syntaqlite_span(pCtx, B)` converts SyntaqliteToken to SyntaqliteSourceSpan
- No span: `SYNTAQLITE_NO_SPAN` for empty/missing spans
- Null node: `SYNTAQLITE_NULL_NODE` (0xFFFFFFFF) for nullable index fields
- Folding away: `expr(A) ::= LP expr(B) RP. { A = B; }` — discard wrapper
- Multi-token dispatch: `expr(A) ::= expr(L) PLUS|MINUS(OP) expr(R). { ... switch(OP.type) ... }`
- List building: `list_append(ctx, list_id, child)` — pass `SYNTAQLITE_NULL_NODE` as list_id for first element
- Enum casts: `(SyntaqliteYourEnum)R` to cast integer nonterminals to enum types
- Bool values: `SYNTAQLITE_BOOL_FALSE`, `SYNTAQLITE_BOOL_TRUE`, or `(SyntaqliteBool)X` for cast
- Enum defaults: Use named constants like `SYNTAQLITE_CONFLICT_ACTION_DEFAULT`, `SYNTAQLITE_SORT_ORDER_ASC`
- Flags: `(SyntaqliteYourFlags){.raw = (uint8_t)F}` for flags compound literals, `{.raw = 0}` for empty

**Precedence annotations**: If the upstream rule has a precedence marker like `[BITNOT]` or `[IN]`, include it in the action file between the `.` and `{`:
```c
expr(A) ::= expr(B) in_op(C) LP exprlist(D) RP. [IN] {
    ...
}
```

### 4. Regenerate, build, and test

```bash
tools/dev/run-codegen    # Regenerate all code
cargo build              # Build
cargo test               # Run tests
```

## Debugging techniques

### Parser tracing

When a rule produces unexpected results or a syntax error, use the `--trace` flag on the CLI to enable Lemon's built-in parser trace:

```bash
echo "SELECT 1 UNION SELECT 2;" | cargo run -p syntaqlite-cli -- ast --trace
```

### Cross-referencing upstream parse.y

When debugging why a rule doesn't match, compare your action signature against the ACTUAL upstream rule:

```bash
grep -A5 'your_nonterminal(A) ::=' third_party/src/sqlite/src/parse.y
```

Watch for:
- Token alternations: upstream may combine `ID|INDEXED|JOIN_KW` where we have separate rules
- The `idj` nonterminal: upstream uses `idj(X)` for function names, which lemon -g expands to token alternations
- `%ifdef`/`%ifndef` blocks: upstream grammar has conditional compilation that affects which rules exist

### Common failure modes

1. **Syntax error on valid SQL**: Your action rule signature doesn't match upstream exactly. Check token names, alternation order.

2. **Rule silently not applied (bare rule used instead)**: Signature mismatch due to precedence markers or label differences.

3. **Fallback tokens breaking things**: Tokens like UNION/EXCEPT/INTERSECT can fall back to ID if `%ifdef SQLITE_OMIT_COMPOUND_SELECT` blocks are incorrectly included. We don't define OMIT macros, so `%ifdef` blocks should be excluded.

## Key files reference

| File | Purpose |
|------|---------|
| `syntaqlite/parser-nodes/*.synq` | AST node definitions |
| `syntaqlite/parser-actions/*.y` | Grammar action rules |
| `syntaqlite-codegen/` | Rust code generator |
| `third_party/src/sqlite/src/parse.y` | Upstream SQLite grammar (ground truth) |
