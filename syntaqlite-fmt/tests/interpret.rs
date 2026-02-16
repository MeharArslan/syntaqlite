use syntaqlite_fmt::interpret::{interpret, FieldVal};
use syntaqlite_fmt::ops::FmtOp;
use syntaqlite_fmt::{render, DocArena, FmtCtx, FormatConfig, NIL_DOC};

fn noop_child(_: u32, _: &mut DocArena) -> u32 {
    NIL_DOC
}

fn no_lists(_: u32) -> Vec<u32> {
    panic!("resolve_list not expected")
}

fn ctx<'a>(strings: &'a [&'a str]) -> FmtCtx<'a> {
    FmtCtx {
        strings,
        enum_display: &[],
    }
}

fn run(ops: &[FmtOp], ctx: &FmtCtx, fields: &[FieldVal], config: &FormatConfig) -> String {
    let mut arena = DocArena::new();
    let doc = interpret(ops, ctx, fields, None, &mut arena, &noop_child, &no_lists);
    render(&arena, doc, config)
}

fn run_default(ops: &[FmtOp], ctx: &FmtCtx, fields: &[FieldVal]) -> String {
    run(ops, ctx, fields, &FormatConfig::default())
}

const NULL: u32 = 0xFFFF_FFFF;

// -- Basic ops --

#[test]
fn single_keyword() {
    let strings = &["SELECT"];
    assert_eq!(run_default(&[FmtOp::Keyword(0)], &ctx(strings), &[]), "SELECT");
}

#[test]
fn group_fits_flat() {
    let strings = &["SELECT", "FROM"];
    let ops = &[
        FmtOp::GroupStart,
        FmtOp::Keyword(0),
        FmtOp::Line,
        FmtOp::Keyword(1),
        FmtOp::GroupEnd,
    ];
    assert_eq!(run_default(ops, &ctx(strings), &[]), "SELECT FROM");
}

#[test]
fn group_breaks_when_narrow() {
    let strings = &["SELECT", "FROM"];
    let ops = &[
        FmtOp::GroupStart,
        FmtOp::Keyword(0),
        FmtOp::Line,
        FmtOp::Keyword(1),
        FmtOp::GroupEnd,
    ];
    let config = FormatConfig { line_width: 5, ..Default::default() };
    assert_eq!(run(ops, &ctx(strings), &[], &config), "SELECT\nFROM");
}

#[test]
fn nest_indentation() {
    let strings = &["SELECT", "a"];
    let ops = &[
        FmtOp::GroupStart,
        FmtOp::Keyword(0),
        FmtOp::NestStart(4),
        FmtOp::Line,
        FmtOp::Keyword(1),
        FmtOp::NestEnd,
        FmtOp::GroupEnd,
    ];
    let config = FormatConfig { line_width: 5, ..Default::default() };
    assert_eq!(run(ops, &ctx(strings), &[], &config), "SELECT\n    a");
}

#[test]
fn span_reads_source_text() {
    let fields = &[FieldVal::Span("hello")];
    let ops = &[FmtOp::Span(0)];
    assert_eq!(run_default(ops, &ctx(&[]), fields), "hello");
}

#[test]
fn child_recurses_into_child_node() {
    let fields = &[FieldVal::NodeId(42)];
    let ops = &[FmtOp::Child(0)];
    let ctx = ctx(&[]);
    let mut arena = DocArena::new();
    let doc = interpret(
        ops, &ctx, fields, None, &mut arena,
        &|node_id, arena| {
            assert_eq!(node_id, 42);
            arena.text("child_result")
        },
        &no_lists,
    );
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "child_result");
}

#[test]
fn child_skips_null_node() {
    let fields = &[FieldVal::NodeId(NULL)];
    let strings = &["a", "b"];
    let ops = &[FmtOp::Keyword(0), FmtOp::Child(0), FmtOp::Keyword(1)];
    assert_eq!(run_default(ops, &ctx(strings), fields), "ab");
}

// -- IfSet --

#[test]
fn ifset_executes_then_branch() {
    let fields = &[FieldVal::NodeId(42)];
    let strings = &["YES", "NO"];
    let ops = &[
        FmtOp::IfSet(0, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(strings), fields), "YES");
}

#[test]
fn ifset_executes_else_branch() {
    let fields = &[FieldVal::NodeId(NULL)];
    let strings = &["YES", "NO"];
    let ops = &[
        FmtOp::IfSet(0, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(strings), fields), "NO");
}

#[test]
fn ifset_without_else() {
    let fields = &[FieldVal::NodeId(NULL)];
    let strings = &["a", "b", "c"];
    let ops = &[
        FmtOp::Keyword(0),
        FmtOp::IfSet(0, 2),
        FmtOp::Line,
        FmtOp::Keyword(1),
        FmtOp::EndIf,
        FmtOp::Keyword(2),
    ];
    assert_eq!(run_default(ops, &ctx(strings), fields), "ac");
}

// -- ForEach --

#[test]
fn foreach_comma_separated() {
    let fields = &[FieldVal::NodeId(99)];
    let strings = &[", "];
    let ops = &[
        FmtOp::GroupStart,
        FmtOp::ForEachStart(0),
        FmtOp::ChildItem,
        FmtOp::ForEachSep(0),
        FmtOp::ForEachEnd,
        FmtOp::GroupEnd,
    ];
    let ctx = ctx(strings);
    let mut arena = DocArena::new();
    let doc = interpret(
        ops, &ctx, fields, None, &mut arena,
        &|id: u32, arena: &mut DocArena| match id {
            10 => arena.text("a"),
            20 => arena.text("b"),
            30 => arena.text("c"),
            _ => panic!("unexpected: {id}"),
        },
        &|id| match id {
            99 => vec![10, 20, 30],
            _ => panic!("unexpected list: {id}"),
        },
    );
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "a, b, c");
}

#[test]
fn foreach_with_line_breaks() {
    let fields = &[FieldVal::NodeId(99)];
    let strings = &[","];
    let ops = &[
        FmtOp::GroupStart,
        FmtOp::ForEachStart(0),
        FmtOp::ChildItem,
        FmtOp::ForEachSep(0),
        FmtOp::Line,
        FmtOp::ForEachEnd,
        FmtOp::GroupEnd,
    ];
    let ctx = ctx(strings);
    let mut arena = DocArena::new();
    let doc = interpret(
        ops, &ctx, fields, None, &mut arena,
        &|id: u32, arena: &mut DocArena| match id {
            10 => arena.text("aaaa"),
            20 => arena.text("bbbb"),
            _ => panic!("unexpected"),
        },
        &|id| match id {
            99 => vec![10, 20],
            _ => panic!("unexpected"),
        },
    );
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "aaaa, bbbb");
    let narrow = FormatConfig { line_width: 5, ..Default::default() };
    assert_eq!(render(&arena, doc, &narrow), "aaaa,\nbbbb");
}

#[test]
fn foreach_empty_list() {
    let fields = &[FieldVal::NodeId(99)];
    let strings = &["a", ", ", "b"];
    let ops = &[
        FmtOp::Keyword(0),
        FmtOp::ForEachStart(0),
        FmtOp::ChildItem,
        FmtOp::ForEachSep(1),
        FmtOp::ForEachEnd,
        FmtOp::Keyword(2),
    ];
    let ctx = ctx(strings);
    let mut arena = DocArena::new();
    let doc = interpret(
        ops, &ctx, fields, None, &mut arena,
        &noop_child,
        &|id| match id {
            99 => vec![],
            _ => panic!("unexpected"),
        },
    );
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "ab");
}

// -- IfBool --

#[test]
fn ifbool_true() {
    let fields = &[FieldVal::Bool(true)];
    let strings = &["YES", "NO"];
    let ops = &[
        FmtOp::IfBool(0, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(strings), fields), "YES");
}

#[test]
fn ifbool_false() {
    let fields = &[FieldVal::Bool(false)];
    let strings = &["YES", "NO"];
    let ops = &[
        FmtOp::IfBool(0, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(strings), fields), "NO");
}

// -- IfFlag --

#[test]
fn ifflag_set() {
    let fields = &[FieldVal::Flags(0b0000_0001)];
    let strings = &["DISTINCT", "ALL"];
    let ops = &[
        FmtOp::IfFlag(0, 1, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(strings), fields), "DISTINCT");
}

#[test]
fn ifflag_clear() {
    let fields = &[FieldVal::Flags(0b0000_0000)];
    let strings = &["DISTINCT", "ALL"];
    let ops = &[
        FmtOp::IfFlag(0, 1, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(strings), fields), "ALL");
}

// -- IfEnum --

#[test]
fn ifenum_match() {
    let fields = &[FieldVal::Enum(1)]; // ordinal 1 = DESC
    let strings = &[" DESC", ""];
    let ops = &[
        FmtOp::IfEnum(0, 1, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(strings), fields), " DESC");
}

#[test]
fn ifenum_no_match() {
    let fields = &[FieldVal::Enum(0)]; // ordinal 0 = ASC
    let strings = &[" DESC", ""];
    let ops = &[
        FmtOp::IfEnum(0, 1, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(strings), fields), "");
}

// -- IfSpan --

#[test]
fn ifspan_set() {
    let fields = &[FieldVal::Span("hello")];
    let strings = &["HAS_SPAN"];
    let ops = &[
        FmtOp::IfSpan(0, 1),
        FmtOp::Keyword(0),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(strings), fields), "HAS_SPAN");
}

#[test]
fn ifspan_empty() {
    let fields = &[FieldVal::Span("")];
    let strings = &["HAS_SPAN"];
    let ops = &[
        FmtOp::IfSpan(0, 1),
        FmtOp::Keyword(0),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(strings), fields), "");
}

// -- EnumDisplay --

#[test]
fn enum_display_maps_ordinal() {
    let fields = &[FieldVal::Enum(2)]; // ordinal 2 → "*"
    let strings = &["+", "-", "*"];
    let enum_display: &[u16] = &[0, 1, 2]; // ordinal → string_id
    let ctx = FmtCtx { strings, enum_display };
    let ops = &[FmtOp::EnumDisplay(0, 0)]; // field 0, base 0
    let mut arena = DocArena::new();
    let doc = interpret(ops, &ctx, fields, None, &mut arena, &noop_child, &no_lists);
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "*");
}

#[test]
fn enum_display_with_nonzero_base() {
    let strings = &["AND", "OR"];
    let enum_display: &[u16] = &[10, 11, 0, 1]; // first enum at base 0, ours at base 2
    let ctx = FmtCtx { strings, enum_display };
    let fields = &[FieldVal::Enum(1)]; // ordinal 1 → base[2+1]=1 → "OR"
    let ops = &[FmtOp::EnumDisplay(0, 2)];
    let mut arena = DocArena::new();
    let doc = interpret(ops, &ctx, fields, None, &mut arena, &noop_child, &no_lists);
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "OR");
}

// -- ForEachSelfStart --

#[test]
fn foreach_self_start() {
    let children = &[10u32, 20, 30];
    let strings = &[", "];
    let ops = &[
        FmtOp::ForEachSelfStart,
        FmtOp::ChildItem,
        FmtOp::ForEachSep(0),
        FmtOp::ForEachEnd,
    ];
    let ctx = ctx(strings);
    let mut arena = DocArena::new();
    let doc = interpret(
        ops, &ctx, &[], Some(children), &mut arena,
        &|id: u32, arena: &mut DocArena| match id {
            10 => arena.text("x"),
            20 => arena.text("y"),
            30 => arena.text("z"),
            _ => panic!("unexpected"),
        },
        &no_lists,
    );
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "x, y, z");
}

#[test]
fn foreach_self_empty() {
    let children: &[u32] = &[];
    let strings = &["[", ", ", "]"];
    let ops = &[
        FmtOp::Keyword(0),
        FmtOp::ForEachSelfStart,
        FmtOp::ChildItem,
        FmtOp::ForEachSep(1),
        FmtOp::ForEachEnd,
        FmtOp::Keyword(2),
    ];
    assert_eq!(
        {
            let ctx = ctx(strings);
            let mut arena = DocArena::new();
            let doc = interpret(ops, &ctx, &[], Some(children), &mut arena, &noop_child, &no_lists);
            render(&arena, doc, &FormatConfig::default())
        },
        "[]"
    );
}
