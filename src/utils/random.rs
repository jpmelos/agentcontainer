//! Random value generation utilities.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher as _, Hasher as _};

/// Generate a random name suffix in the range `1..=999_999`.
///
/// Uses OS-seeded randomness via `RandomState`.
pub(crate) fn random_name_suffix() -> u32 {
    let hash = RandomState::new().build_hasher().finish();
    // The modulo operation guarantees the value is 0..=999_998, which fits in u32.
    #[expect(
        clippy::as_conversions,
        clippy::integer_division_remainder_used,
        reason = "The modulo operation ensures the value fits in u32."
    )]
    let suffix = (hash % 999_999) as u32 + 1;
    suffix
}

#[cfg(test)]
mod tests {
    use super::random_name_suffix;

    #[test]
    fn suffix_is_in_valid_range() {
        for _ in 0..100 {
            let suffix = random_name_suffix();
            assert!(suffix >= 1, "Suffix {suffix} is below minimum 1.");
            assert!(
                suffix <= 999_999,
                "Suffix {suffix} is above maximum 999_999."
            );
        }
    }
}
