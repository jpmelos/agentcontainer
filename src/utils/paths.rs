//! Path utilities.

/// Whether a path uses the unsupported `~user` syntax.
///
/// Returns `true` when the path starts with `~` followed by something other than `/`
/// (e.g., `~alice/data`). Bare `~` and `~/…` are not matched because they are handled by
/// `expand_tilde`.
pub(crate) fn has_tilde_user_prefix(path: &str) -> bool {
    path.starts_with('~') && path != "~" && !path.starts_with("~/")
}

/// Whether a non-absolute path looks like a filesystem path rather than a Docker volume name.
///
/// A path is considered filesystem-like if it starts with `.` (e.g., `./data`, `../parent`) or
/// contains `/` (e.g., `data/subdir`, `~alice/data`). Plain names without `.` prefix or `/`
/// (e.g., `my_volume`) are treated as Docker volume names.
pub(crate) fn is_relative_filesystem_path(path: &str) -> bool {
    path.starts_with('.') || path.contains('/')
}

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

/// Expand tilde, then resolve any remaining relative path to an absolute path.
///
/// Steps:
/// 1. Expand a leading `~` to `home_dir` (only `~` and `~/…`; `~user/…` is left untouched).
/// 2. If the result is already absolute, normalize it (resolve `.`, `..`, consecutive slashes).
/// 3. Otherwise, resolve it against `cwd` (which also normalizes).
///
/// Callers that want to warn about unsupported `~user` syntax should check for it before calling
/// this function (see `has_tilde_user_prefix`).
pub(crate) fn expand_and_resolve_path(path: &str, home_dir: &str, cwd: &str) -> String {
    let expanded = expand_tilde(path, home_dir);
    if expanded.starts_with('/') {
        normalize_absolute_path(&expanded)
    } else {
        resolve_relative_path(&expanded, cwd)
    }
}

/// Resolve a relative path against a base directory, producing a normalized absolute path.
///
/// The path is joined to `base_dir` and then normalized lexically: consecutive slashes are
/// collapsed, `.` components are removed, `..` components pop the preceding directory, and `..`
/// at the root is silently discarded.
fn resolve_relative_path(path: &str, base_dir: &str) -> String {
    normalize_absolute_path(&format!("{base_dir}/{path}"))
}

/// Normalize an absolute path by resolving `.`, `..`, and consecutive slashes lexically.
///
/// This is a purely lexical operation that does not access the filesystem. Consecutive slashes are
/// collapsed, `.` components are removed, `..` components pop the preceding directory, and `..`
/// that would go above the root is silently discarded.
fn normalize_absolute_path(path: &str) -> String {
    let mut components: Vec<&str> = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                components.pop();
            }
            other => components.push(other),
        }
    }
    if components.is_empty() {
        String::from("/")
    } else {
        format!("/{}", components.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod has_tilde_user_prefix {
        use super::*;

        #[test]
        fn tilde_user_without_slash_is_detected() {
            assert!(has_tilde_user_prefix("~alice"));
        }

        #[test]
        fn tilde_user_with_slash_is_detected() {
            assert!(has_tilde_user_prefix("~alice/data"));
        }

        #[test]
        fn bare_tilde_is_not_tilde_user() {
            assert!(!has_tilde_user_prefix("~"));
        }

        #[test]
        fn tilde_slash_is_not_tilde_user() {
            assert!(!has_tilde_user_prefix("~/data"));
        }

        #[test]
        fn absolute_path_is_not_tilde_user() {
            assert!(!has_tilde_user_prefix("/usr/bin"));
        }

        #[test]
        fn relative_path_is_not_tilde_user() {
            assert!(!has_tilde_user_prefix("scripts/hook.sh"));
        }
    }

    mod is_relative_filesystem_path {
        use super::*;

        #[test]
        fn dot_slash_prefix_is_filesystem_like() {
            assert!(is_relative_filesystem_path("./data"));
        }

        #[test]
        fn dot_dot_prefix_is_filesystem_like() {
            assert!(is_relative_filesystem_path("../parent"));
        }

        #[test]
        fn bare_dot_is_filesystem_like() {
            assert!(is_relative_filesystem_path("."));
        }

        #[test]
        fn path_with_slash_is_filesystem_like() {
            assert!(is_relative_filesystem_path("data/subdir"));
        }

        #[test]
        fn tilde_user_with_slash_is_filesystem_like() {
            assert!(is_relative_filesystem_path("~alice/data"));
        }

        #[test]
        fn plain_name_is_not_filesystem_like() {
            assert!(!is_relative_filesystem_path("my_volume"));
        }

        #[test]
        fn name_with_hyphens_is_not_filesystem_like() {
            assert!(!is_relative_filesystem_path("my-volume"));
        }
    }

    mod expand_tilde {
        use super::*;

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

    mod expand_and_resolve_path {
        use super::*;

        #[test]
        fn tilde_is_expanded_to_home_dir() {
            assert_eq!(
                expand_and_resolve_path("~/data", "/home/alice", "/work"),
                "/home/alice/data"
            );
        }

        #[test]
        fn absolute_path_is_unchanged() {
            assert_eq!(
                expand_and_resolve_path("/usr/bin/hook", "/home/alice", "/work"),
                "/usr/bin/hook"
            );
        }

        #[test]
        fn relative_path_is_resolved_to_cwd() {
            assert_eq!(
                expand_and_resolve_path("scripts/hook.sh", "/home/alice", "/work"),
                "/work/scripts/hook.sh"
            );
        }

        #[test]
        fn dot_slash_relative_path_is_resolved_to_cwd() {
            assert_eq!(
                expand_and_resolve_path("./scripts/hook.sh", "/home/alice", "/work"),
                "/work/scripts/hook.sh"
            );
        }

        #[test]
        fn tilde_user_is_resolved_to_cwd() {
            assert_eq!(
                expand_and_resolve_path("~alice/data", "/home/bob", "/work"),
                "/work/~alice/data"
            );
        }

        #[test]
        fn tilde_expanded_path_with_dot_dot_is_normalized() {
            assert_eq!(
                expand_and_resolve_path("~/data/../other", "/home/alice", "/work"),
                "/home/alice/other"
            );
        }

        #[test]
        fn absolute_path_with_dot_dot_is_normalized() {
            assert_eq!(
                expand_and_resolve_path("/usr/local/../bin/hook", "/home/alice", "/work"),
                "/usr/bin/hook"
            );
        }

        #[test]
        fn absolute_path_with_dot_is_normalized() {
            assert_eq!(
                expand_and_resolve_path("/usr/./bin/hook", "/home/alice", "/work"),
                "/usr/bin/hook"
            );
        }
    }

    mod resolve_relative_path {
        use super::*;

        #[test]
        fn dot_resolves_to_base_dir() {
            assert_eq!(resolve_relative_path(".", "/home/alice"), "/home/alice");
        }

        #[test]
        fn bare_dot_slash_resolves_to_base_dir() {
            assert_eq!(resolve_relative_path("./", "/home/alice"), "/home/alice");
        }

        #[test]
        fn dot_slash_prefix_resolves_relative_to_base_dir() {
            assert_eq!(
                resolve_relative_path("./data", "/home/alice"),
                "/home/alice/data"
            );
        }

        #[test]
        fn relative_path_without_dot_resolves_relative_to_base_dir() {
            assert_eq!(
                resolve_relative_path("scripts/hook.sh", "/home/alice"),
                "/home/alice/scripts/hook.sh"
            );
        }

        #[test]
        fn dot_dot_pops_one_directory() {
            assert_eq!(
                resolve_relative_path("../data", "/home/alice"),
                "/home/data"
            );
        }

        #[test]
        fn double_dot_dot_pops_two_directories() {
            assert_eq!(resolve_relative_path("../../data", "/home/alice"), "/data");
        }

        #[test]
        fn dot_dot_beyond_root_is_discarded() {
            assert_eq!(
                resolve_relative_path("../../../data", "/home/alice"),
                "/data"
            );
        }

        #[test]
        fn dot_dot_to_root() {
            assert_eq!(resolve_relative_path("../../..", "/home/alice"), "/");
        }

        #[test]
        fn embedded_dot_dot_pops_preceding_component() {
            assert_eq!(
                resolve_relative_path("foo/../bar", "/home/alice"),
                "/home/alice/bar"
            );
        }

        #[test]
        fn embedded_dot_is_removed() {
            assert_eq!(
                resolve_relative_path("foo/./bar", "/home/alice"),
                "/home/alice/foo/bar"
            );
        }

        #[test]
        fn trailing_slash_on_base_dir_does_not_produce_double_slash() {
            assert_eq!(
                resolve_relative_path("data", "/home/alice/"),
                "/home/alice/data"
            );
        }

        #[test]
        fn bare_dot_with_trailing_slash_on_base_dir() {
            assert_eq!(resolve_relative_path(".", "/home/alice/"), "/home/alice");
        }
    }
}
