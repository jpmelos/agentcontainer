use super::{parse_hook_output, serialize_hook_input};

mod serialize_hook_input {
    use super::*;

    #[test]
    fn serializes_empty_args() {
        let result = serialize_hook_input(&[]);
        let parsed = parse_hook_output(&result).expect("Round-trip should parse successfully");
        assert!(parsed.is_empty());
    }

    #[test]
    fn serializes_single_flag_value_pair() {
        let args = vec![String::from("--build-arg"), String::from("FOO=bar")];
        let result = serialize_hook_input(&args);
        let parsed = parse_hook_output(&result).expect("Round-trip should parse successfully");
        assert_eq!(parsed, args);
    }

    #[test]
    fn serializes_multiple_flag_value_pairs() {
        let args = vec![
            String::from("--volume"),
            String::from("/host:/container"),
            String::from("--env"),
            String::from("MY_VAR=value"),
        ];
        let result = serialize_hook_input(&args);
        let parsed = parse_hook_output(&result).expect("Round-trip should parse successfully");
        assert_eq!(parsed, args);
    }

    #[test]
    fn output_contains_args_key() {
        let args = vec![String::from("--network"), String::from("host")];
        let result = serialize_hook_input(&args);
        assert!(
            result.contains("args"),
            "Serialized output should contain `args` key: {result:?}"
        );
    }
}

mod parse_hook_output {
    use super::*;

    #[test]
    fn parses_empty_array() {
        let output = "args = []";
        let result = parse_hook_output(output).expect("Should parse successfully");
        assert!(result.is_empty());
    }

    #[test]
    fn parses_array() {
        let output = r#"args = ["--network", "host"]"#;
        let result = parse_hook_output(output).expect("Should parse successfully");
        assert_eq!(result, vec!["--network", "host"]);
    }

    #[test]
    fn handles_trailing_newline() {
        let output = "args = [\"--network\", \"host\"]\n";
        let result = parse_hook_output(output).expect("Should parse successfully");
        assert_eq!(result, vec!["--network", "host"]);
    }

    #[test]
    fn parses_multiline_array() {
        let output = "args = [\n  \"--volume\",\n  \"/host:/container\",\n]";
        let result = parse_hook_output(output).expect("Should parse successfully");
        assert_eq!(result, vec!["--volume", "/host:/container"]);
    }

    #[test]
    fn handles_surrounding_whitespace() {
        let output = "  args = [\"--network\", \"host\"]  \n";
        let result = parse_hook_output(output).expect("Should parse successfully");
        assert_eq!(result, vec!["--network", "host"]);
    }

    #[test]
    fn rejects_empty_string() {
        let output = "";
        let result = parse_hook_output(output);
        result.unwrap_err();
    }

    #[test]
    fn rejects_bare_array_without_key() {
        let output = r#"["--network", "host"]"#;
        let result = parse_hook_output(output);
        result.unwrap_err();
    }

    #[test]
    fn rejects_wrong_key_name() {
        let output = r#"flags = ["--network", "host"]"#;
        let result = parse_hook_output(output);
        result.unwrap_err();
    }

    #[test]
    fn rejects_array_of_non_strings() {
        let output = "args = [1, 2, 3]";
        let result = parse_hook_output(output);
        result.unwrap_err();
    }
}
