/// Slugify a raw string.
pub(crate) fn slugify(raw: &str) -> String {
    let lowercased = raw.to_lowercase();
    let replaced: String = lowercased
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect();

    // Collapse consecutive dashes into one.
    let mut collapsed = String::with_capacity(replaced.len());
    let mut last_was_dash = false;
    for character in replaced.chars() {
        if character == '-' {
            if !last_was_dash {
                collapsed.push(character);
            }
            last_was_dash = true;
        } else {
            collapsed.push(character);
            last_was_dash = false;
        }
    }

    // Trim leading and trailing dashes.
    collapsed.trim_matches('-').to_owned()
}

/// Slugify a raw string, returning `"unknown"` if the result would be empty.
pub(crate) fn slugify_or_unknown(raw: &str) -> String {
    let slug = slugify(raw);
    if slug.is_empty() {
        String::from("unknown")
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::{slugify, slugify_or_unknown};

    mod slugify {
        use super::*;

        #[test]
        fn slugify_lowercases_and_replaces_spaces() {
            assert_eq!(slugify("Hello World"), "hello-world");
        }

        #[test]
        fn slugify_handles_special_chars() {
            assert_eq!(slugify("foo@bar!baz"), "foo-bar-baz");
        }

        #[test]
        fn slugify_collapses_consecutive_dashes() {
            assert_eq!(slugify("foo--bar"), "foo-bar");
        }

        #[test]
        fn slugify_collapses_consecutive_special_chars() {
            assert_eq!(slugify("foo  bar"), "foo-bar");
        }

        #[test]
        fn slugify_trims_leading_trailing_dashes() {
            assert_eq!(slugify("-foo-bar-"), "foo-bar");
        }

        #[test]
        fn slugify_trims_leading_trailing_special_chars() {
            assert_eq!(slugify("@foo@"), "foo");
        }

        #[test]
        fn slugify_uppercase_input() {
            assert_eq!(slugify("MyProject"), "myproject");
        }

        #[test]
        fn slugify_all_special_chars_returns_empty_string() {
            assert_eq!(slugify("@@@"), "");
        }
    }

    mod slugify_or_unknown {
        use super::*;

        #[test]
        fn slugify_or_unknown_returns_slug_when_non_empty() {
            assert_eq!(slugify_or_unknown("Hello World"), "hello-world");
        }

        #[test]
        fn slugify_or_unknown_returns_unknown_when_slug_is_empty() {
            assert_eq!(slugify_or_unknown("@@@"), "unknown");
        }
    }
}
