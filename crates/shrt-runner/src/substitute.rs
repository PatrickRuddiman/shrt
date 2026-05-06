use std::ffi::OsString;
use std::fmt;

#[derive(Debug)]
pub enum SubstError {
    MissingArg(usize),
    ArgNotUtf8(usize),
    TemplateParse { offset: usize, reason: &'static str },
    EnvUnset(String),
    EnvNotUtf8(String),
}

impl SubstError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::MissingArg(_) | Self::ArgNotUtf8(_) => 64,
            _ => 78,
        }
    }
}

impl fmt::Display for SubstError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingArg(n) => write!(f, "template requires {{{}}} but only {} args provided", n, n - 1),
            Self::ArgNotUtf8(n) => write!(f, "argument {} is not valid UTF-8", n),
            Self::TemplateParse { offset, reason } => {
                write!(f, "template offset {}: {}", offset, reason)
            }
            Self::EnvUnset(name) => write!(f, "environment variable '{}' is not set", name),
            Self::EnvNotUtf8(name) => {
                write!(f, "environment variable '{}' is not valid UTF-8", name)
            }
        }
    }
}

pub fn substitute(
    template: &str,
    args: &[OsString],
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Result<String, SubstError> {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let next = bytes[i..]
            .iter()
            .position(|&b| b == b'{' || b == b'}')
            .map(|off| i + off);
        let chunk_end = next.unwrap_or(bytes.len());
        out.push_str(&template[i..chunk_end]);
        if chunk_end == bytes.len() {
            break;
        }
        i = chunk_end;
        let c = bytes[i];

        if c == b'{' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                out.push('{');
                i += 2;
                continue;
            }
            let start = i + 1;
            let close = bytes[start..]
                .iter()
                .position(|&b| b == b'}')
                .map(|p| start + p)
                .ok_or(SubstError::TemplateParse {
                    offset: i,
                    reason: "unmatched '{'",
                })?;
            let inside = &template[start..close];
            if inside.is_empty() {
                return Err(SubstError::TemplateParse {
                    offset: i,
                    reason: "empty placeholder",
                });
            }
            if inside.chars().any(|c| c.is_whitespace()) {
                return Err(SubstError::TemplateParse {
                    offset: i,
                    reason: "whitespace in placeholder",
                });
            }
            let resolved = resolve_placeholder(inside, args, env, i)?;
            out.push_str(&resolved);
            i = close + 1;
        } else {
            // c == b'}'
            if i + 1 < bytes.len() && bytes[i + 1] == b'}' {
                out.push('}');
                i += 2;
                continue;
            }
            return Err(SubstError::TemplateParse {
                offset: i,
                reason: "unmatched '}'",
            });
        }
    }
    Ok(out)
}

fn resolve_placeholder(
    s: &str,
    args: &[OsString],
    env: &dyn Fn(&str) -> Option<OsString>,
    offset: usize,
) -> Result<String, SubstError> {
    if s == "INPUT" {
        return join_args(args, false);
    }
    if s == "@" {
        return join_args(args, true);
    }
    if let Some((n, optional)) = parse_positional(s) {
        return match args.get(n - 1) {
            Some(a) => a
                .to_str()
                .map(|s| s.to_string())
                .ok_or(SubstError::ArgNotUtf8(n)),
            None if optional => Ok(String::new()),
            None => Err(SubstError::MissingArg(n)),
        };
    }
    if let Some(rest) = s.strip_prefix("ENV:") {
        let (name, default) = match rest.find(':') {
            Some(idx) => (&rest[..idx], Some(&rest[idx + 1..])),
            None => (rest, None),
        };
        if !is_valid_env_name(name) {
            return Err(SubstError::TemplateParse {
                offset,
                reason: "invalid ENV name",
            });
        }
        return match env(name) {
            Some(v) => v
                .to_str()
                .map(|s| s.to_string())
                .ok_or_else(|| SubstError::EnvNotUtf8(name.to_string())),
            None => match default {
                Some(d) => Ok(d.to_string()),
                None => Err(SubstError::EnvUnset(name.to_string())),
            },
        };
    }
    Err(SubstError::TemplateParse {
        offset,
        reason: "unrecognized placeholder",
    })
}

fn parse_positional(s: &str) -> Option<(usize, bool)> {
    let (digits, optional) = match s.strip_suffix('?') {
        Some(d) => (d, true),
        None => (s, false),
    };
    if digits.len() != 1 {
        return None;
    }
    let c = digits.chars().next()?;
    if !c.is_ascii_digit() {
        return None;
    }
    let n = (c as u8 - b'0') as usize;
    if n < 1 {
        return None;
    }
    Some((n, optional))
}

fn is_valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn join_args(args: &[OsString], crt_quote: bool) -> Result<String, SubstError> {
    let mut parts: Vec<String> = Vec::with_capacity(args.len());
    for (i, a) in args.iter().enumerate() {
        let s = a.to_str().ok_or(SubstError::ArgNotUtf8(i + 1))?;
        parts.push(if crt_quote {
            crt_quote_arg(s)
        } else {
            s.to_string()
        });
    }
    Ok(parts.join(" "))
}

fn crt_quote_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }
    if !arg
        .chars()
        .any(|c| c == ' ' || c == '\t' || c == '"' || c == '\\')
    {
        return arg.to_string();
    }
    let chars: Vec<char> = arg.chars().collect();
    let mut out = String::from("\"");
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' {
            let mut count = 0;
            while i < chars.len() && chars[i] == '\\' {
                count += 1;
                i += 1;
            }
            if i == chars.len() {
                for _ in 0..count * 2 {
                    out.push('\\');
                }
            } else if chars[i] == '"' {
                for _ in 0..count * 2 {
                    out.push('\\');
                }
                out.push_str("\\\"");
                i += 1;
            } else {
                for _ in 0..count {
                    out.push('\\');
                }
            }
        } else if c == '"' {
            out.push_str("\\\"");
            i += 1;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_env(_: &str) -> Option<OsString> {
        None
    }

    fn args(strs: &[&str]) -> Vec<OsString> {
        strs.iter().map(|s| OsString::from(s)).collect()
    }

    #[test]
    fn positional_arg() {
        let r = substitute("hi {1}", &args(&["world"]), &no_env).unwrap();
        assert_eq!(r, "hi world");
    }

    #[test]
    fn multiple_positional_args() {
        let r = substitute("{1}-{2}-{3}", &args(&["a", "b", "c"]), &no_env).unwrap();
        assert_eq!(r, "a-b-c");
    }

    #[test]
    fn missing_required_arg_64() {
        let err = substitute("{1}", &args(&[]), &no_env).unwrap_err();
        assert!(matches!(err, SubstError::MissingArg(1)));
        assert_eq!(err.exit_code(), 64);
    }

    #[test]
    fn optional_arg_empty_when_missing() {
        let r = substitute("[{1?}]", &args(&[]), &no_env).unwrap();
        assert_eq!(r, "[]");
    }

    #[test]
    fn optional_arg_present() {
        let r = substitute("[{1?}]", &args(&["x"]), &no_env).unwrap();
        assert_eq!(r, "[x]");
    }

    #[test]
    fn input_joins_with_single_space() {
        let r = substitute("{INPUT}", &args(&["a", "b", "c"]), &no_env).unwrap();
        assert_eq!(r, "a b c");
    }

    #[test]
    fn input_empty_when_no_args() {
        let r = substitute("{INPUT}", &args(&[]), &no_env).unwrap();
        assert_eq!(r, "");
    }

    #[test]
    fn at_quotes_each_arg() {
        let r = substitute("{@}", &args(&["a b", "c"]), &no_env).unwrap();
        assert_eq!(r, "\"a b\" c");
    }

    #[test]
    fn at_quotes_arg_with_internal_quote() {
        let r = substitute("{@}", &args(&["he said \"hi\""]), &no_env).unwrap();
        assert_eq!(r, "\"he said \\\"hi\\\"\"");
    }

    #[test]
    fn at_empty_when_no_args() {
        let r = substitute("{@}", &args(&[]), &no_env).unwrap();
        assert_eq!(r, "");
    }

    #[test]
    fn env_resolves() {
        let env = |k: &str| {
            if k == "FOO" {
                Some(OsString::from("bar"))
            } else {
                None
            }
        };
        let r = substitute("{ENV:FOO}", &[], &env).unwrap();
        assert_eq!(r, "bar");
    }

    #[test]
    fn env_unset_errors_78() {
        let err = substitute("{ENV:NEVERSET}", &[], &no_env).unwrap_err();
        assert!(matches!(err, SubstError::EnvUnset(_)));
        assert_eq!(err.exit_code(), 78);
    }

    #[test]
    fn env_default_used_when_unset() {
        let r = substitute("{ENV:UNSET:fallback}", &[], &no_env).unwrap();
        assert_eq!(r, "fallback");
    }

    #[test]
    fn env_default_can_contain_colon() {
        let r = substitute("{ENV:UNSET:/usr/bin}", &[], &no_env).unwrap();
        assert_eq!(r, "/usr/bin");
    }

    #[test]
    fn empty_env_name_rejected() {
        let err = substitute("{ENV:}", &[], &no_env).unwrap_err();
        assert!(matches!(err, SubstError::TemplateParse { .. }));
    }

    #[test]
    fn double_brace_literal() {
        let r = substitute("{{1}}", &args(&[]), &no_env).unwrap();
        assert_eq!(r, "{1}");
    }

    #[test]
    fn double_brace_around_text() {
        let r = substitute("a {{ b }} c", &args(&[]), &no_env).unwrap();
        assert_eq!(r, "a { b } c");
    }

    #[test]
    fn whitespace_in_placeholder_rejected() {
        let err = substitute("{ 1 }", &args(&["x"]), &no_env).unwrap_err();
        assert!(matches!(err, SubstError::TemplateParse { .. }));
    }

    #[test]
    fn unmatched_open_brace_rejected() {
        let err = substitute("foo {bar", &args(&[]), &no_env).unwrap_err();
        assert!(matches!(err, SubstError::TemplateParse { .. }));
    }

    #[test]
    fn unmatched_close_brace_rejected() {
        let err = substitute("foo }bar", &args(&[]), &no_env).unwrap_err();
        assert!(matches!(err, SubstError::TemplateParse { .. }));
    }

    #[test]
    fn unrecognized_placeholder_rejected() {
        let err = substitute("{XYZ}", &[], &no_env).unwrap_err();
        assert!(matches!(err, SubstError::TemplateParse { .. }));
    }

    #[test]
    fn template_passes_through_literal() {
        let r = substitute("hello world", &[], &no_env).unwrap();
        assert_eq!(r, "hello world");
    }

    #[test]
    fn template_with_unicode() {
        let r = substitute("café {1}", &args(&["☕"]), &no_env).unwrap();
        assert_eq!(r, "café ☕");
    }

    #[test]
    fn empty_placeholder_rejected() {
        let err = substitute("{}", &[], &no_env).unwrap_err();
        assert!(matches!(err, SubstError::TemplateParse { .. }));
    }
}
