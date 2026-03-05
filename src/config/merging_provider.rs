//! A custom figment `Provider` that wraps multiple providers and applies controlled-depth merging.
//!
//! Controlled-depth merging means:
//! - Top-level keys: dicts are unioned, scalars are replaced by the later provider.
//! - Second-level keys: replaced atomically, no recursive merging into inner structs.
//!
//! This is essential for `environment_variables`, where the inner fields of two entries for the
//! same variable name across sources must not be merged together.

use figment::{
    Error, Metadata, Profile, Provider,
    value::{Dict, Map, Value},
};

/// A figment provider that merges multiple providers in priority order using controlled-depth
/// merging.
pub(crate) struct MergingProvider {
    providers: Vec<Box<dyn Provider>>,
}

impl MergingProvider {
    /// Create a new `MergingProvider` from a list of providers in priority order (lowest first).
    pub(crate) fn new(providers: Vec<Box<dyn Provider>>) -> Self {
        Self { providers }
    }
}

impl Provider for MergingProvider {
    fn metadata(&self) -> Metadata {
        Metadata::named("merging provider")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, Error> {
        let mut merged = Dict::new();
        for provider in &self.providers {
            // Propagate errors from any individual provider.
            let provider_data = provider.data()?;
            for (_profile, dict) in provider_data {
                merge_dicts(&mut merged, &dict);
            }
        }
        Ok(Profile::Default.collect(merged))
    }
}

/// Merge `incoming` into `base` using controlled-depth semantics.
///
/// - If both `base` and `incoming` have a dict at the same key, the inner entries of `incoming`
///   are inserted into `base`'s dict atomically (no further recursion).
/// - For all other cases (scalar vs. anything, or key only in `incoming`), `incoming` replaces
///   `base`.
fn merge_dicts(base: &mut Dict, incoming: &Dict) {
    for (key, incoming_value) in incoming {
        match (base.get_mut(key), incoming_value) {
            (
                Some(&mut Value::Dict(_, ref mut base_inner)),
                &Value::Dict(_, ref incoming_inner),
            ) => {
                // Union inner keys, but replace each entry atomically (no deeper recursion).
                for (inner_key, inner_value) in incoming_inner {
                    base_inner.insert(inner_key.clone(), inner_value.clone());
                }
            }
            _ => {
                base.insert(key.clone(), incoming_value.clone());
            }
        }
    }
}
