pub fn tokenize(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;

    while i < n {
        while i < n && (chars[i] == ' ' || chars[i] == '\t') {
            i += 1;
        }
        if i >= n {
            break;
        }

        let mut arg = String::new();
        let mut in_quotes = false;

        while i < n {
            let c = chars[i];
            if c == '\\' {
                let mut count = 0;
                while i < n && chars[i] == '\\' {
                    count += 1;
                    i += 1;
                }
                if i < n && chars[i] == '"' {
                    for _ in 0..count / 2 {
                        arg.push('\\');
                    }
                    if count % 2 == 0 {
                        in_quotes = !in_quotes;
                    } else {
                        arg.push('"');
                    }
                    i += 1;
                } else {
                    for _ in 0..count {
                        arg.push('\\');
                    }
                }
            } else if c == '"' {
                in_quotes = !in_quotes;
                i += 1;
            } else if !in_quotes && (c == ' ' || c == '\t') {
                break;
            } else {
                arg.push(c);
                i += 1;
            }
        }

        out.push(arg);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_unquoted_args() {
        assert_eq!(tokenize("a b c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn quoted_groups_args() {
        assert_eq!(tokenize("\"a b c\""), vec!["a b c"]);
    }

    #[test]
    fn escaped_quote_literal() {
        assert_eq!(tokenize("\\\"x"), vec!["\"x"]);
    }

    #[test]
    fn double_backslash_before_quote_in_quoted() {
        // "abc\\" → 1 backslash + close quote
        assert_eq!(tokenize("\"abc\\\\\""), vec!["abc\\"]);
    }

    #[test]
    fn quad_backslash_before_quote_in_quoted() {
        // "abc\\\\" → 2 backslashes + close quote
        assert_eq!(tokenize("\"abc\\\\\\\\\""), vec!["abc\\\\"]);
    }

    #[test]
    fn backslashes_literal_at_end_unquoted() {
        // 2 trailing backslashes, no quote follows → literal
        assert_eq!(tokenize("\\\\"), vec!["\\\\"]);
    }

    #[test]
    fn unterminated_quote_lenient() {
        assert_eq!(tokenize("\"abc"), vec!["abc"]);
    }

    #[test]
    fn empty_input() {
        let expected: Vec<String> = vec![];
        assert_eq!(tokenize(""), expected);
    }

    #[test]
    fn whitespace_only_input() {
        let expected: Vec<String> = vec![];
        assert_eq!(tokenize("   \t   "), expected);
    }

    #[test]
    fn tab_separates_args() {
        assert_eq!(tokenize("a\tb"), vec!["a", "b"]);
    }

    #[test]
    fn multiple_spaces_between_args() {
        assert_eq!(tokenize("a    b"), vec!["a", "b"]);
    }

    #[test]
    fn quoted_concatenates_with_unquoted() {
        // "abc"def becomes one arg `abcdef` because the closing quote toggles
        // in_quotes off but does not split.
        assert_eq!(tokenize("\"abc\"def"), vec!["abcdef"]);
    }

    #[test]
    fn empty_quoted_arg() {
        assert_eq!(tokenize("\"\""), vec![""]);
    }

    #[test]
    fn complex_mixed_arg() {
        // -p "/worktree create a worktree for ado item 37839929" --yolo
        // → 3 args
        let line = "-p \"/worktree create a worktree for ado item 37839929\" --yolo";
        assert_eq!(
            tokenize(line),
            vec![
                "-p",
                "/worktree create a worktree for ado item 37839929",
                "--yolo",
            ]
        );
    }
}
