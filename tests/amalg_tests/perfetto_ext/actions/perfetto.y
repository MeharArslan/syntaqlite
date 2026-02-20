// Perfetto dialect extension: CREATE PERFETTO TABLE

// Allow PERFETTO to be used as a regular identifier in non-keyword positions.
%fallback ID PERFETTO.

cmd(A) ::= CREATE PERFETTO TABLE nm(X) AS select(S). {
    A = synq_parse_create_perfetto_table_stmt(pCtx,
        synq_span(pCtx, X),
        S);
}
