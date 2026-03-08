//! A custom figment `Provider` that wraps multiple providers and applies controlled-depth merging.
//!
//! Controlled-depth merging means:
//! - Top-level keys: dicts are unioned, scalars are replaced by the later provider.
//! - Second-level keys: replaced atomically, no recursive merging into inner structs.
//!
//! This is essential for `environment_variables`, where the inner fields of two entries for the
//! same variable name across sources must not be merged together.
//!
//! Before merging, each provider's volume keys and values undergo tilde expansion so that
//! `~/.ssh` and `/home/alice/.ssh` from different sources are recognized as the same key during
//! priority resolution.

use crate::utils::paths::expand_tilde;
use figment::{
    Error, Metadata, Profile, Provider,
    value::{Dict, Map, Value},
};

/// A figment provider that merges multiple providers in priority order using controlled-depth
/// merging.
///
/// Volume paths are tilde-expanded per-provider before merging so that priority resolution
/// operates on canonical absolute paths.
pub(crate) struct MergingProvider {
    providers: Vec<Box<dyn Provider>>,
    home_dir: String,
}

impl MergingProvider {
    /// Create a new `MergingProvider` from a list of providers in priority order (lowest first).
    pub(crate) fn new(providers: Vec<Box<dyn Provider>>, home_dir: String) -> Self {
        Self {
            providers,
            home_dir,
        }
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
            for (_profile, mut dict) in provider_data {
                expand_tildes_in_volumes(&mut dict, &self.home_dir);
                merge_dicts(&mut merged, &dict);
            }
        }
        Ok(Profile::Default.collect(merged))
    }
}

/// Expand leading `~` to `home_dir` in all volume keys and host-path values.
///
/// Operates on the raw figment `Dict` so that expansion happens before merging. This ensures
/// that `~/.ssh` and `/home/alice/.ssh` from different config sources are treated as the same
/// volume during priority resolution.
fn expand_tildes_in_volumes(dict: &mut Dict, home_dir: &str) {
    let Some(Value::Dict(tag, volumes)) = dict.remove("volumes") else {
        return;
    };
    let expanded: Dict = volumes
        .into_iter()
        .map(|(key, value)| {
            let expanded_key = expand_tilde(&key, home_dir);
            let expanded_value = match value {
                Value::String(t, s) => Value::String(t, expand_tilde(&s, home_dir)),
                other => other,
            };
            (expanded_key, expanded_value)
        })
        .collect();
    dict.insert(String::from("volumes"), Value::Dict(tag, expanded));
}

/// Merge `incoming` into `base` using controlled-depth semantics.
///
/// - If both `base` and `incoming` have a dict at the same key, the inner entries of `incoming`'s
///   dict are inserted into `base`'s dict atomically (no further recursion).
/// - If both `base` and `incoming` have an array at the same key, the arrays are concatenated
///   (`base` first, then `incoming`).
/// - For all other cases (scalar vs. anything, or a key that only exists in `incoming`),
///   `incoming` replaces `base`.
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
            (
                Some(&mut Value::Array(_, ref mut base_items)),
                &Value::Array(_, ref incoming_items),
            ) => {
                // Accumulate: lower-priority items first, higher-priority items appended.
                base_items.extend(incoming_items.iter().cloned());
            }
            _ => {
                base.insert(key.clone(), incoming_value.clone());
            }
        }
    }
}
