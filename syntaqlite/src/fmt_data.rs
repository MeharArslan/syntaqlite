use std::sync::LazyLock;

use syntaqlite_runtime::fmt::{FmtCtx, LoadedFmt, NodeFmt, StaticFmt};

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
