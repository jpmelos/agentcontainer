use super::parse_pre_run_output;

mod parse_pre_run_output_tests {
    use super::*;

    #[test]
    fn parses_empty_array() {
        let output = "[]";
        let result = parse_pre_run_output(output).expect("Should parse successfully");
        assert!(result.is_empty());
    }

    #[test]
    fn parses_array() {
        let output = r#"["--network", "host"]"#;
        let result = parse_pre_run_output(output).expect("Should parse successfully");
        assert_eq!(result, vec!["--network", "host"]);
    }

    #[test]
    fn handles_trailing_newline() {
        let output = "[\"--network\", \"host\"]\n";
        let result = parse_pre_run_output(output).expect("Should parse successfully");
        assert_eq!(result, vec!["--network", "host"]);
    }

    #[test]
    fn parses_multiline_array() {
        let output = "[\n  \"--mount\",\n  \"/host:/container\",\n]";
        let result = parse_pre_run_output(output).expect("Should parse successfully");
        assert_eq!(result, vec!["--mount", "/host:/container"]);
    }

    #[test]
    fn handles_surrounding_whitespace() {
        let output = "  [\"--network\", \"host\"]  \n";
        let result = parse_pre_run_output(output).expect("Should parse successfully");
        assert_eq!(result, vec!["--network", "host"]);
    }

    #[test]
    fn rejects_empty_string() {
        let output = "";
        let result = parse_pre_run_output(output);
        result.unwrap_err();
    }

    #[test]
    fn rejects_non_array_output() {
        let output = r#""not an array""#;
        let result = parse_pre_run_output(output);
        result.unwrap_err();
    }

    #[test]
    fn rejects_array_of_non_strings() {
        let output = "[1, 2, 3]";
        let result = parse_pre_run_output(output);
        result.unwrap_err();
    }
}
