/// Slugify a raw string.
pub(crate) fn slugify(raw: &str) -> String {
    let lowercased = raw.to_lowercase();
    let replaced: String = lowercased
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();

    // Collapse consecutive underscores into one.
    let mut collapsed = String::with_capacity(replaced.len());
    let mut last_was_underscore = false;
    for character in replaced.chars() {
        if character == '_' {
            if !last_was_underscore {
                collapsed.push(character);
            }
            last_was_underscore = true;
        } else {
            collapsed.push(character);
            last_was_underscore = false;
        }
    }

    // Trim leading and trailing underscores.
    collapsed.trim_matches('_').to_owned()
}

#[cfg(test)]
mod tests {
    use super::slugify;

    mod slugify {
        use super::*;

        #[test]
        fn slugify_lowercases_and_replaces_spaces() {
            assert_eq!(slugify("Hello World"), "hello_world");
        }

        #[test]
        fn slugify_handles_special_chars() {
            assert_eq!(slugify("foo@bar!baz"), "foo_bar_baz");
        }

        #[test]
        fn slugify_collapses_consecutive_underscores() {
            assert_eq!(slugify("foo__bar"), "foo_bar");
        }

        #[test]
        fn slugify_collapses_consecutive_special_chars() {
            assert_eq!(slugify("foo  bar"), "foo_bar");
        }

        #[test]
        fn slugify_trims_leading_trailing_underscores() {
            assert_eq!(slugify("_foo_bar_"), "foo_bar");
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
}
