// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum FunctionCategory {
    Scalar,
    Aggregate,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AritySpec {
    Exact(usize),
    AtLeast(usize),
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FunctionOverload {
    pub category: FunctionCategory,
    pub arity: AritySpec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FunctionSet {
    pub name: String,
    pub overloads: Vec<FunctionOverload>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RelationEntry {
    pub name: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TableFunctionSet {
    pub name: String,
    pub overloads: Vec<FunctionOverload>,
    pub output_columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FunctionCheckResult {
    Ok,
    Unknown,
    WrongArity { expected: Vec<usize> },
}

#[derive(Debug, Default, Clone)]
pub(crate) struct CatalogLayer {
    relations: HashMap<String, RelationEntry>,
    functions: HashMap<String, FunctionSet>,
    table_functions: HashMap<String, TableFunctionSet>,
}

impl CatalogLayer {
    pub(crate) fn insert_relation(&mut self, name: impl Into<String>, columns: Vec<String>) {
        let name = name.into();
        self.relations
            .insert(canonical_name(&name), RelationEntry { name, columns });
    }

    pub(crate) fn insert_function_overload(
        &mut self,
        name: impl Into<String>,
        category: FunctionCategory,
        arity: AritySpec,
    ) {
        let name = name.into();
        let key = canonical_name(&name);
        self.functions
            .entry(key)
            .and_modify(|set| {
                set.overloads.push(FunctionOverload { category, arity });
            })
            .or_insert_with(|| FunctionSet {
                name,
                overloads: vec![FunctionOverload { category, arity }],
            });
    }

    pub(crate) fn insert_function_arities(
        &mut self,
        name: impl Into<String>,
        category: FunctionCategory,
        arities: &[i16],
    ) {
        let name = name.into();
        if arities.is_empty() {
            self.insert_function_overload(name, category, AritySpec::Any);
            return;
        }

        for &a in arities {
            let arity = if a == -1 {
                AritySpec::Any
            } else if a < -1 {
                AritySpec::AtLeast(
                    usize::try_from(-i32::from(a) - 1).expect("negative arity encodes minimum"),
                )
            } else {
                AritySpec::Exact(
                    usize::try_from(i32::from(a)).expect("fixed arity must be non-negative"),
                )
            };
            self.insert_function_overload(name.clone(), category, arity);
        }
    }

    pub(crate) fn insert_table_function_overload(
        &mut self,
        name: impl Into<String>,
        arity: AritySpec,
        output_columns: Vec<String>,
    ) {
        let name = name.into();
        let key = canonical_name(&name);
        self.table_functions
            .entry(key)
            .and_modify(|set| {
                set.overloads.push(FunctionOverload {
                    category: FunctionCategory::Scalar,
                    arity,
                });
            })
            .or_insert_with(|| TableFunctionSet {
                name,
                overloads: vec![FunctionOverload {
                    category: FunctionCategory::Scalar,
                    arity,
                }],
                output_columns,
            });
    }

    pub(crate) fn relation(&self, name: &str) -> Option<&RelationEntry> {
        self.relations.get(&canonical_name(name))
    }

    pub(crate) fn function(&self, name: &str) -> Option<&FunctionSet> {
        self.functions.get(&canonical_name(name))
    }

    pub(crate) fn table_function(&self, name: &str) -> Option<&TableFunctionSet> {
        self.table_functions.get(&canonical_name(name))
    }
}

pub(crate) struct CatalogChain<'a> {
    layers: Vec<&'a CatalogLayer>,
}

impl<'a> CatalogChain<'a> {
    pub(crate) fn new() -> Self {
        CatalogChain { layers: Vec::new() }
    }

    pub(crate) fn push_layer(&mut self, layer: &'a CatalogLayer) {
        self.layers.push(layer);
    }

    pub(crate) fn with_layers(layers: Vec<&'a CatalogLayer>) -> Self {
        CatalogChain { layers }
    }

    pub(crate) fn check_function(&self, name: &str, arg_count: usize) -> FunctionCheckResult {
        let Some(set) = self.lookup_first(|layer| layer.function(name)) else {
            return FunctionCheckResult::Unknown;
        };

        if set
            .overloads
            .iter()
            .copied()
            .any(|ov| overload_accepts(ov, arg_count))
        {
            return FunctionCheckResult::Ok;
        }

        FunctionCheckResult::WrongArity {
            expected: expected_fixed_arities(set),
        }
    }

    pub(crate) fn relation(&self, name: &str) -> Option<&'a RelationEntry> {
        self.lookup_first(|layer| layer.relation(name))
    }

    pub(crate) fn table_function(&self, name: &str) -> Option<&'a TableFunctionSet> {
        self.lookup_first(|layer| layer.table_function(name))
    }

    pub(crate) fn all_relation_names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for layer in &self.layers {
            for relation in layer.relations.values() {
                push_unique_name(&mut seen, &mut out, &relation.name);
            }
        }
        out.sort_unstable_by_key(|name| canonical_name(name));
        out
    }

    pub(crate) fn all_function_names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for layer in &self.layers {
            for function in layer.functions.values() {
                push_unique_name(&mut seen, &mut out, &function.name);
            }
        }
        out.sort_unstable_by_key(|name| canonical_name(name));
        out
    }

    pub(crate) fn all_table_function_names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for layer in &self.layers {
            for function in layer.table_functions.values() {
                push_unique_name(&mut seen, &mut out, &function.name);
            }
        }
        out.sort_unstable_by_key(|name| canonical_name(name));
        out
    }

    fn lookup_first<T>(
        &self,
        mut get: impl FnMut(&'a CatalogLayer) -> Option<&'a T>,
    ) -> Option<&'a T> {
        for layer in &self.layers {
            if let Some(value) = get(layer) {
                return Some(value);
            }
        }
        None
    }
}

fn canonical_name(name: &str) -> String {
    name.to_ascii_lowercase()
}

fn overload_accepts(overload: FunctionOverload, arg_count: usize) -> bool {
    match overload.arity {
        AritySpec::Exact(n) => n == arg_count,
        AritySpec::AtLeast(min) => arg_count >= min,
        AritySpec::Any => true,
    }
}

fn expected_fixed_arities(set: &FunctionSet) -> Vec<usize> {
    let mut expected: Vec<usize> = set
        .overloads
        .iter()
        .filter_map(|ov| match ov.arity {
            AritySpec::Exact(n) => Some(n),
            AritySpec::AtLeast(_) | AritySpec::Any => None,
        })
        .collect();
    expected.sort_unstable();
    expected.dedup();
    expected
}

fn push_unique_name(seen: &mut HashSet<String>, out: &mut Vec<String>, name: &str) {
    let lower = canonical_name(name);
    if seen.insert(lower) {
        out.push(name.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_shadowing_uses_first_layer_only() {
        let mut high = CatalogLayer::default();
        high.insert_function_overload("abs", FunctionCategory::Scalar, AritySpec::Exact(2));

        let mut low = CatalogLayer::default();
        low.insert_function_overload("abs", FunctionCategory::Scalar, AritySpec::Exact(1));

        let chain = CatalogChain::with_layers(vec![&high, &low]);

        assert_eq!(chain.check_function("abs", 2), FunctionCheckResult::Ok);
        assert_eq!(
            chain.check_function("abs", 1),
            FunctionCheckResult::WrongArity { expected: vec![2] }
        );
    }

    #[test]
    fn function_lookup_supports_more_than_three_layers() {
        let empty_a = CatalogLayer::default();
        let empty_b = CatalogLayer::default();
        let empty_c = CatalogLayer::default();
        let mut fourth = CatalogLayer::default();
        fourth.insert_function_overload("sum", FunctionCategory::Aggregate, AritySpec::Any);
        fourth.insert_function_overload("rank", FunctionCategory::Window, AritySpec::Exact(0));

        let mut chain = CatalogChain::new();
        chain.push_layer(&empty_a);
        chain.push_layer(&empty_b);
        chain.push_layer(&empty_c);
        chain.push_layer(&fourth);
        assert_eq!(chain.check_function("sum", 0), FunctionCheckResult::Ok);
        assert_eq!(chain.check_function("sum", 12), FunctionCheckResult::Ok);
        assert_eq!(
            chain.all_function_names(),
            vec!["rank".to_string(), "sum".to_string()]
        );
    }

    #[test]
    fn encoded_arities_handle_at_least_correctly() {
        let mut layer = CatalogLayer::default();
        layer.insert_function_arities("printf", FunctionCategory::Scalar, &[-2]);
        let chain = CatalogChain::with_layers(vec![&layer]);

        assert_eq!(
            chain.check_function("printf", 0),
            FunctionCheckResult::WrongArity {
                expected: Vec::new()
            }
        );
        assert_eq!(chain.check_function("printf", 1), FunctionCheckResult::Ok);
    }

    #[test]
    fn relation_lookup_uses_layer_precedence() {
        let mut high = CatalogLayer::default();
        high.insert_relation("users", vec!["id".into()]);
        let mut low = CatalogLayer::default();
        low.insert_relation("users", vec!["id".into(), "email".into()]);

        let chain = CatalogChain::with_layers(vec![&high, &low]);
        let users = chain
            .relation("users")
            .expect("users relation should exist");
        assert_eq!(users.columns, vec!["id"]);
    }

    #[test]
    fn relation_names_deduplicate_in_precedence_order() {
        let mut high = CatalogLayer::default();
        high.insert_relation("Users", vec![]);
        high.insert_relation("Orders", vec![]);
        let mut low = CatalogLayer::default();
        low.insert_relation("users", vec![]);
        low.insert_relation("items", vec![]);

        let chain = CatalogChain::with_layers(vec![&high, &low]);
        assert_eq!(
            chain.all_relation_names(),
            vec![
                "items".to_string(),
                "Orders".to_string(),
                "Users".to_string()
            ]
        );
    }

    #[test]
    fn table_functions_are_first_class_layered_objects() {
        let mut high = CatalogLayer::default();
        high.insert_table_function_overload("json_each", AritySpec::Exact(1), vec!["value".into()]);
        let mut low = CatalogLayer::default();
        low.insert_table_function_overload(
            "json_each",
            AritySpec::Exact(2),
            vec!["value".into(), "type".into()],
        );

        let chain = CatalogChain::with_layers(vec![&high, &low]);
        let tf = chain
            .table_function("json_each")
            .expect("table function should resolve");
        assert_eq!(tf.output_columns, vec!["value"]);
        assert_eq!(
            chain.all_table_function_names(),
            vec!["json_each".to_string()]
        );
    }
}
