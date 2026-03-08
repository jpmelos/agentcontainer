//! Path utilities.

/// Replace a leading `~` in a path with the given home directory.
///
/// Only `~` alone or `~/…` is expanded; `~user/…` and embedded tildes are left untouched.
/// A trailing slash on `home_dir` is stripped to avoid producing double slashes.
pub(crate) fn expand_tilde(path: &str, home_dir: &str) -> String {
    let home_dir = home_dir.trim_end_matches('/');
    if path == "~" {
        String::from(home_dir)
    } else if let Some(rest) = path.strip_prefix("~/") {
        format!("{home_dir}/{rest}")
    } else {
        String::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::expand_tilde;

    #[test]
    fn bare_tilde_expands_to_home_dir() {
        assert_eq!(expand_tilde("~", "/home/alice"), "/home/alice");
    }

    #[test]
    fn tilde_slash_prefix_expands_to_home_dir() {
        assert_eq!(
            expand_tilde("~/projects/foo", "/home/alice"),
            "/home/alice/projects/foo"
        );
    }

    #[test]
    fn trailing_slash_on_home_dir_does_not_produce_double_slash() {
        assert_eq!(expand_tilde("~/data", "/home/alice/"), "/home/alice/data");
    }

    #[test]
    fn bare_tilde_with_trailing_slash_on_home_dir() {
        assert_eq!(expand_tilde("~", "/home/alice/"), "/home/alice");
    }

    #[test]
    fn tilde_user_syntax_is_not_expanded() {
        assert_eq!(expand_tilde("~alice/data", "/home/bob"), "~alice/data");
    }

    #[test]
    fn embedded_tilde_is_not_expanded() {
        assert_eq!(expand_tilde("/host/~data", "/home/alice"), "/host/~data");
    }

    #[test]
    fn absolute_path_is_unchanged() {
        assert_eq!(
            expand_tilde("/usr/local/bin/hook", "/home/alice"),
            "/usr/local/bin/hook"
        );
    }

    #[test]
    fn relative_path_is_unchanged() {
        assert_eq!(
            expand_tilde("scripts/hook.sh", "/home/alice"),
            "scripts/hook.sh"
        );
    }
}
