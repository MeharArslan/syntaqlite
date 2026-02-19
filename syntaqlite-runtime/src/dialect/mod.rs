//! Dialect types: the opaque handle and C ABI mirror structs.

pub mod ffi;

// ── Opaque dialect handle ──────────────────────────────────────────────

/// An opaque dialect handle. Dialect crates (e.g. `syntaqlite`) provide a
/// function that returns a `&'static Dialect<'static>` for their grammar.
#[derive(Clone, Copy)]
pub struct Dialect<'d> {
    pub(crate) raw: &'d ffi::Dialect,
}

impl<'d> Dialect<'d> {
    /// Create a `Dialect` from a raw C pointer returned by a dialect's
    /// FFI function (e.g. `syntaqlite_sqlite_dialect`).
    ///
    /// # Safety
    /// The pointer must point to a valid `ffi::Dialect` whose data lives
    /// at least as long as `'d`.
    pub unsafe fn from_raw(raw: *const ffi::Dialect) -> Self {
        unsafe { Dialect { raw: &*raw } }
    }
}

// SAFETY: The dialect wraps a reference to a C struct with no mutable state.
// The raw pointers inside ffi::Dialect all point to immutable static data.
unsafe impl Send for Dialect<'_> {}
unsafe impl Sync for Dialect<'_> {}
