// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! [`TypedDialect<'d, N>`]: a dialect handle tagged with a [`NodeFamily`].

use std::marker::PhantomData;

use syntaqlite_syntax::dialect_traits::NodeFamily;

use super::handle::Dialect;

/// A dialect handle tagged with a [`NodeFamily`], carrying both the semantic
/// dialect data and the knowledge of which node/token types it produces.
///
/// Use this at construction boundaries (`Parser::new`, etc.) so the
/// node type parameter `N` can be inferred automatically.
///
/// Call [`raw()`](Self::raw) to downgrade to an untyped [`Dialect`] for
/// passing into untyped infrastructure (formatter, validator).
#[derive(Clone, Copy)]
pub struct TypedDialect<'d, N: NodeFamily> {
    inner: Dialect<'d>,
    _marker: PhantomData<N>,
}

// SAFETY: same reasoning as Dialect — wraps immutable static data.
unsafe impl<N: NodeFamily> Send for TypedDialect<'_, N> {}
unsafe impl<N: NodeFamily> Sync for TypedDialect<'_, N> {}

impl<'d, N: NodeFamily> TypedDialect<'d, N> {
    /// Build a `TypedDialect` from a [`Dialect<'d>`] handle.
    pub fn new(dialect: Dialect<'d>) -> Self {
        TypedDialect {
            inner: dialect,
            _marker: PhantomData,
        }
    }

    /// Return the untagged [`Dialect`] handle.
    pub fn raw(&self) -> Dialect<'d> {
        self.inner
    }
}

impl<'d, N: NodeFamily> From<TypedDialect<'d, N>> for Dialect<'d> {
    fn from(d: TypedDialect<'d, N>) -> Self {
        d.inner
    }
}
