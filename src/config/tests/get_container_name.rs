use super::*;

#[test]
fn normal_name() {
    let config = make_config();
    assert_eq!(config.get_container_name(42), "agentcontainer_myproject_42");
}

#[test]
fn slugifies_name_with_underscores() {
    let mut config = make_config();
    config.project_name = String::from("My Project");
    assert_eq!(config.get_container_name(1), "agentcontainer_my_project_1");
}

#[test]
fn truncates_at_41_chars() {
    let mut config = make_config();
    config.project_name = "a".repeat(50);
    let expected = format!("agentcontainer_{}_1", "a".repeat(41));
    assert_eq!(config.get_container_name(1), expected);
}

#[test]
fn trims_trailing_underscore_after_truncation() {
    // "a" * 40 + " " + "b" → slugified = "a" * 40 + "_b" (42 chars).
    // Truncated to 41 = "a" * 40 + "_" → trailing underscore trimmed → "a" * 40.
    let mut config = make_config();
    config.project_name = format!("{} b", "a".repeat(40));
    let expected = format!("agentcontainer_{}_1", "a".repeat(40));
    assert_eq!(config.get_container_name(1), expected);
}
