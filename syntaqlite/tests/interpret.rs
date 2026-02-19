use syntaqlite_runtime::fmt::interpret::interpret;
use syntaqlite_runtime::fmt::ops::FmtOp;
use syntaqlite::NodeId;
use syntaqlite_runtime::FieldVal;
use syntaqlite_runtime::fmt::{render, DocArena, FmtCtx, FormatConfig, NIL_DOC};

fn noop_child(_: NodeId, _: &mut DocArena) -> u32 {
    NIL_DOC
}

fn no_lists(_: NodeId) -> Vec<NodeId> {
    panic!("resolve_list not expected")
}

fn ctx(strings: &[String]) -> FmtCtx<'_> {
    FmtCtx {
        strings,
        enum_display: &[],
    }
}

fn s(strs: &[&str]) -> Vec<String> {
    strs.iter().map(|s| s.to_string()).collect()
}

fn run(ops: &[FmtOp], ctx: &FmtCtx, fields: &[FieldVal], config: &FormatConfig) -> String {
    let mut arena = DocArena::new();
    let doc = interpret(ops, ctx, fields, None, &mut arena, &noop_child, &no_lists, None);
    render(&arena, doc, config)
}

fn run_default(ops: &[FmtOp], ctx: &FmtCtx, fields: &[FieldVal]) -> String {
    run(ops, ctx, fields, &FormatConfig::default())
}

const NULL: NodeId = NodeId::NULL;

// -- Basic ops --

#[test]
fn single_keyword() {
    let strings = s(&["SELECT"]);
    assert_eq!(run_default(&[FmtOp::Keyword(0)], &ctx(&strings), &[]), "SELECT");
}

#[test]
fn group_fits_flat() {
    let strings = s(&["SELECT", "FROM"]);
    let ops = &[
        FmtOp::GroupStart,
        FmtOp::Keyword(0),
        FmtOp::Line,
        FmtOp::Keyword(1),
        FmtOp::GroupEnd,
    ];
    assert_eq!(run_default(ops, &ctx(&strings), &[]), "SELECT FROM");
}

#[test]
fn group_breaks_when_narrow() {
    let strings = s(&["SELECT", "FROM"]);
    let ops = &[
        FmtOp::GroupStart,
        FmtOp::Keyword(0),
        FmtOp::Line,
        FmtOp::Keyword(1),
        FmtOp::GroupEnd,
    ];
    let config = FormatConfig { line_width: 5, ..Default::default() };
    assert_eq!(run(ops, &ctx(&strings), &[], &config), "SELECT\nFROM");
}

#[test]
fn nest_indentation() {
    let strings = s(&["SELECT", "a"]);
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
    assert_eq!(run(ops, &ctx(&strings), &[], &config), "SELECT\n    a");
}

#[test]
fn span_reads_source_text() {
    let strings = s(&[]);
    let fields = &[FieldVal::Span("hello", 0)];
    let ops = &[FmtOp::Span(0)];
    assert_eq!(run_default(ops, &ctx(&strings), fields), "hello");
}

#[test]
fn child_recurses_into_child_node() {
    let strings = s(&[]);
    let fields = &[FieldVal::NodeId(NodeId(42))];
    let ops = &[FmtOp::Child(0)];
    let ctx = ctx(&strings);
    let mut arena = DocArena::new();
    let doc = interpret(
        ops, &ctx, fields, None, &mut arena,
        &|node_id: NodeId, arena: &mut DocArena| {
            assert_eq!(node_id, NodeId(42));
            arena.text("child_result")
        },
        &no_lists,
        None,
    );
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "child_result");
}

#[test]
fn child_skips_null_node() {
    let strings = s(&["a", "b"]);
    let fields = &[FieldVal::NodeId(NULL)];
    let ops = &[FmtOp::Keyword(0), FmtOp::Child(0), FmtOp::Keyword(1)];
    assert_eq!(run_default(ops, &ctx(&strings), fields), "ab");
}

// -- IfSet --

#[test]
fn ifset_executes_then_branch() {
    let strings = s(&["YES", "NO"]);
    let fields = &[FieldVal::NodeId(NodeId(42))];
    let ops = &[
        FmtOp::IfSet(0, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(&strings), fields), "YES");
}

#[test]
fn ifset_executes_else_branch() {
    let strings = s(&["YES", "NO"]);
    let fields = &[FieldVal::NodeId(NULL)];
    let ops = &[
        FmtOp::IfSet(0, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(&strings), fields), "NO");
}

#[test]
fn ifset_without_else() {
    let strings = s(&["a", "b", "c"]);
    let fields = &[FieldVal::NodeId(NULL)];
    let ops = &[
        FmtOp::Keyword(0),
        FmtOp::IfSet(0, 2),
        FmtOp::Line,
        FmtOp::Keyword(1),
        FmtOp::EndIf,
        FmtOp::Keyword(2),
    ];
    assert_eq!(run_default(ops, &ctx(&strings), fields), "ac");
}

// -- ForEach --

#[test]
fn foreach_comma_separated() {
    let strings = s(&[", "]);
    let fields = &[FieldVal::NodeId(NodeId(99))];
    let ops = &[
        FmtOp::GroupStart,
        FmtOp::ForEachStart(0),
        FmtOp::ChildItem,
        FmtOp::ForEachSep(0),
        FmtOp::ForEachEnd,
        FmtOp::GroupEnd,
    ];
    let ctx = ctx(&strings);
    let mut arena = DocArena::new();
    let doc = interpret(
        ops, &ctx, fields, None, &mut arena,
        &|id: NodeId, arena: &mut DocArena| match id.0 {
            10 => arena.text("a"),
            20 => arena.text("b"),
            30 => arena.text("c"),
            _ => panic!("unexpected"),
        },
        &|id: NodeId| match id.0 {
            99 => vec![NodeId(10), NodeId(20), NodeId(30)],
            _ => panic!("unexpected list"),
        },
        None,
    );
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "a, b, c");
}

#[test]
fn foreach_with_line_breaks() {
    let strings = s(&[","]);
    let fields = &[FieldVal::NodeId(NodeId(99))];
    let ops = &[
        FmtOp::GroupStart,
        FmtOp::ForEachStart(0),
        FmtOp::ChildItem,
        FmtOp::ForEachSep(0),
        FmtOp::Line,
        FmtOp::ForEachEnd,
        FmtOp::GroupEnd,
    ];
    let ctx = ctx(&strings);
    let mut arena = DocArena::new();
    let doc = interpret(
        ops, &ctx, fields, None, &mut arena,
        &|id: NodeId, arena: &mut DocArena| match id.0 {
            10 => arena.text("aaaa"),
            20 => arena.text("bbbb"),
            _ => panic!("unexpected"),
        },
        &|id: NodeId| match id.0 {
            99 => vec![NodeId(10), NodeId(20)],
            _ => panic!("unexpected"),
        },
        None,
    );
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "aaaa, bbbb");
    let narrow = FormatConfig { line_width: 5, ..Default::default() };
    assert_eq!(render(&arena, doc, &narrow), "aaaa,\nbbbb");
}

#[test]
fn foreach_empty_list() {
    let strings = s(&["a", ", ", "b"]);
    let fields = &[FieldVal::NodeId(NodeId(99))];
    let ops = &[
        FmtOp::Keyword(0),
        FmtOp::ForEachStart(0),
        FmtOp::ChildItem,
        FmtOp::ForEachSep(1),
        FmtOp::ForEachEnd,
        FmtOp::Keyword(2),
    ];
    let ctx = ctx(&strings);
    let mut arena = DocArena::new();
    let doc = interpret(
        ops, &ctx, fields, None, &mut arena,
        &noop_child,
        &|id: NodeId| match id.0 {
            99 => vec![],
            _ => panic!("unexpected"),
        },
        None,
    );
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "ab");
}

// -- IfBool --

#[test]
fn ifbool_true() {
    let strings = s(&["YES", "NO"]);
    let fields = &[FieldVal::Bool(true)];
    let ops = &[
        FmtOp::IfBool(0, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(&strings), fields), "YES");
}

#[test]
fn ifbool_false() {
    let strings = s(&["YES", "NO"]);
    let fields = &[FieldVal::Bool(false)];
    let ops = &[
        FmtOp::IfBool(0, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(&strings), fields), "NO");
}

// -- IfFlag --

#[test]
fn ifflag_set() {
    let strings = s(&["DISTINCT", "ALL"]);
    let fields = &[FieldVal::Flags(0b0000_0001)];
    let ops = &[
        FmtOp::IfFlag(0, 1, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(&strings), fields), "DISTINCT");
}

#[test]
fn ifflag_clear() {
    let strings = s(&["DISTINCT", "ALL"]);
    let fields = &[FieldVal::Flags(0b0000_0000)];
    let ops = &[
        FmtOp::IfFlag(0, 1, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(&strings), fields), "ALL");
}

// -- IfEnum --

#[test]
fn ifenum_match() {
    let strings = s(&[" DESC", ""]);
    let fields = &[FieldVal::Enum(1)];
    let ops = &[
        FmtOp::IfEnum(0, 1, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(&strings), fields), " DESC");
}

#[test]
fn ifenum_no_match() {
    let strings = s(&[" DESC", ""]);
    let fields = &[FieldVal::Enum(0)];
    let ops = &[
        FmtOp::IfEnum(0, 1, 2),
        FmtOp::Keyword(0),
        FmtOp::Else(1),
        FmtOp::Keyword(1),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(&strings), fields), "");
}

// -- IfSpan --

#[test]
fn ifspan_set() {
    let strings = s(&["HAS_SPAN"]);
    let fields = &[FieldVal::Span("hello", 0)];
    let ops = &[
        FmtOp::IfSpan(0, 1),
        FmtOp::Keyword(0),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(&strings), fields), "HAS_SPAN");
}

#[test]
fn ifspan_empty() {
    let strings = s(&["HAS_SPAN"]);
    let fields = &[FieldVal::Span("", 0)];
    let ops = &[
        FmtOp::IfSpan(0, 1),
        FmtOp::Keyword(0),
        FmtOp::EndIf,
    ];
    assert_eq!(run_default(ops, &ctx(&strings), fields), "");
}

// -- EnumDisplay --

#[test]
fn enum_display_maps_ordinal() {
    let strings = s(&["+", "-", "*"]);
    let fields = &[FieldVal::Enum(2)];
    let enum_display: &[u16] = &[0, 1, 2];
    let ctx = FmtCtx { strings: &strings, enum_display };
    let ops = &[FmtOp::EnumDisplay(0, 0)];
    let mut arena = DocArena::new();
    let doc = interpret(ops, &ctx, fields, None, &mut arena, &noop_child, &no_lists, None);
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "*");
}

#[test]
fn enum_display_with_nonzero_base() {
    let strings = s(&["AND", "OR"]);
    let enum_display: &[u16] = &[10, 11, 0, 1];
    let ctx = FmtCtx { strings: &strings, enum_display };
    let fields = &[FieldVal::Enum(1)];
    let ops = &[FmtOp::EnumDisplay(0, 2)];
    let mut arena = DocArena::new();
    let doc = interpret(ops, &ctx, fields, None, &mut arena, &noop_child, &no_lists, None);
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "OR");
}

// -- ForEachSelfStart --

#[test]
fn foreach_self_start() {
    let strings = s(&[", "]);
    let children = &[NodeId(10), NodeId(20), NodeId(30)];
    let ops = &[
        FmtOp::ForEachSelfStart,
        FmtOp::ChildItem,
        FmtOp::ForEachSep(0),
        FmtOp::ForEachEnd,
    ];
    let ctx = ctx(&strings);
    let mut arena = DocArena::new();
    let doc = interpret(
        ops, &ctx, &[], Some(children), &mut arena,
        &|id: NodeId, arena: &mut DocArena| match id.0 {
            10 => arena.text("x"),
            20 => arena.text("y"),
            30 => arena.text("z"),
            _ => panic!("unexpected"),
        },
        &no_lists,
        None,
    );
    assert_eq!(render(&arena, doc, &FormatConfig::default()), "x, y, z");
}

#[test]
fn foreach_self_empty() {
    let strings = s(&["[", ", ", "]"]);
    let children: &[NodeId] = &[];
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
            let ctx = ctx(&strings);
            let mut arena = DocArena::new();
            let doc = interpret(ops, &ctx, &[], Some(children), &mut arena, &noop_child, &no_lists, None);
            render(&arena, doc, &FormatConfig::default())
        },
        "[]"
    );
}
