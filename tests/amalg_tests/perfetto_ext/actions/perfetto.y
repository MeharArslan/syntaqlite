// Perfetto dialect extension: CREATE PERFETTO TABLE

cmd(A) ::= CREATE PERFETTO TABLE nm(X). {
    A = synq_parse_create_perfetto_table_stmt(pCtx,
        synq_span(pCtx, X));
}
