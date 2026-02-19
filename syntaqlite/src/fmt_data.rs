use std::sync::LazyLock;

use syntaqlite_runtime::fmt::{FmtCtx, LoadedFmt, NodeFmt, NodeInfo, StaticFmt};

use crate::generated::nodes::{FIELD_DESCRIPTORS, NodeTag};

static LOADED: LazyLock<StaticFmt> = LazyLock::new(|| {
    LoadedFmt::load(include_bytes!("generated/fmt.bin"))
        .expect("failed to load embedded fmt bytecode")
        .into_static()
});

pub fn dispatch() -> &'static [Option<NodeFmt>] {
    LOADED.dispatch
}

pub fn ctx() -> &'static FmtCtx<'static> {
    &LOADED.ctx
}

fn is_list_tag(tag: u32) -> bool {
    NodeTag::from_raw(tag).map_or(false, |t| t.is_list())
}

/// Node metadata for the SQLite dialect, used by the runtime formatter.
pub static NODE_INFO: NodeInfo = NodeInfo {
    field_descriptors: FIELD_DESCRIPTORS,
    is_list: is_list_tag,
};
