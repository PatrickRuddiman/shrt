use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct SidecarConfig {
    pub target: String,
    pub template: String,
    pub shell: bool,
    pub cwd: String,
    pub description: String,
    pub created: Option<String>,
    pub version: u32,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            target: String::new(),
            template: String::new(),
            shell: false,
            cwd: String::new(),
            description: String::new(),
            created: None,
            version: 1,
        }
    }
}

#[derive(Debug)]
pub enum SidecarError {
    NotFound,
    Io(std::io::Error),
    Bom,
    InvalidUtf8,
    BadEscape { line: usize },
    BadValue { line: usize, reason: &'static str },
    MissingRequired { key: &'static str },
    WrongType { line: usize, key: String, expected: &'static str },
    MultipleAssignments { line: usize },
    BadVersion { value: i64 },
    BadShimSuffix,
    UnclosedString { line: usize },
}

impl SidecarError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NotFound => 66,
            Self::Io(_) => 1,
            _ => 78,
        }
    }
}

impl fmt::Display for SidecarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "sidecar file not found"),
            Self::Io(e) => write!(f, "i/o error: {}", e),
            Self::Bom => write!(f, "UTF-8 BOM not allowed"),
            Self::InvalidUtf8 => write!(f, "file is not valid UTF-8"),
            Self::BadEscape { line } => write!(f, "line {}: bad string escape", line),
            Self::BadValue { line, reason } => write!(f, "line {}: {}", line, reason),
            Self::MissingRequired { key } => write!(f, "missing required key '{}'", key),
            Self::WrongType { line, key, expected } => {
                write!(f, "line {}: key '{}' must be {}", line, key, expected)
            }
            Self::MultipleAssignments { line } => {
                write!(f, "line {}: trailing content after value", line)
            }
            Self::BadVersion { value } => write!(f, "unsupported version: {}", value),
            Self::BadShimSuffix => write!(f, "shim must end in .exe"),
            Self::UnclosedString { line } => write!(f, "line {}: unclosed string", line),
        }
    }
}

pub fn parse(path: &Path) -> Result<SidecarConfig, SidecarError> {
    let bytes = std::fs::read(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => SidecarError::NotFound,
        _ => SidecarError::Io(e),
    })?;
    parse_bytes(&bytes)
}

pub fn derive_sidecar_path(exe: &Path) -> Result<PathBuf, SidecarError> {
    let ext = exe.extension().and_then(|e| e.to_str());
    match ext {
        Some(e) if e.eq_ignore_ascii_case("exe") => {
            let mut p = exe.to_path_buf();
            p.set_extension("shrt");
            Ok(p)
        }
        _ => Err(SidecarError::BadShimSuffix),
    }
}

fn parse_bytes(bytes: &[u8]) -> Result<SidecarConfig, SidecarError> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Err(SidecarError::Bom);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| SidecarError::InvalidUtf8)?;
    parse_str(text)
}

fn parse_str(text: &str) -> Result<SidecarConfig, SidecarError> {
    let mut cfg = SidecarConfig::default();
    let mut have_target = false;
    let mut have_template = false;

    for (idx, raw) in text.split('\n').enumerate() {
        let line_no = idx + 1;
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let eq = trimmed.find('=').ok_or(SidecarError::BadValue {
            line: line_no,
            reason: "expected '='",
        })?;
        let key = trimmed[..eq].trim();
        if key.is_empty() {
            return Err(SidecarError::BadValue {
                line: line_no,
                reason: "empty key",
            });
        }
        let after = trimmed[eq + 1..].trim_start();
        let (value, rest) = parse_value(after, line_no)?;
        let trailing = rest.trim();
        if !trailing.is_empty() && !trailing.starts_with('#') {
            return Err(SidecarError::MultipleAssignments { line: line_no });
        }

        match key {
            "target" => {
                cfg.target = value.into_string(line_no, "target")?;
                have_target = true;
            }
            "template" => {
                cfg.template = value.into_string(line_no, "template")?;
                have_template = true;
            }
            "shell" => cfg.shell = value.into_bool(line_no, "shell")?,
            "cwd" => cfg.cwd = value.into_string(line_no, "cwd")?,
            "description" => cfg.description = value.into_string(line_no, "description")?,
            "created" => cfg.created = Some(value.into_string(line_no, "created")?),
            "version" => {
                let v = value.into_int(line_no, "version")?;
                if v <= 0 || v > 1 {
                    return Err(SidecarError::BadVersion { value: v });
                }
                cfg.version = v as u32;
            }
            _ => {
                eprintln!(
                    "shrt-runner: ignoring unknown key '{}' (line {})",
                    key, line_no
                );
            }
        }
    }

    if !have_target {
        return Err(SidecarError::MissingRequired { key: "target" });
    }
    if !have_template {
        return Err(SidecarError::MissingRequired { key: "template" });
    }
    Ok(cfg)
}

enum Value {
    Str(String),
    Bool(bool),
    Int(i64),
}

impl Value {
    fn into_string(self, line: usize, key: &str) -> Result<String, SidecarError> {
        match self {
            Value::Str(s) => Ok(s),
            _ => Err(SidecarError::WrongType {
                line,
                key: key.to_string(),
                expected: "string",
            }),
        }
    }
    fn into_bool(self, line: usize, key: &str) -> Result<bool, SidecarError> {
        match self {
            Value::Bool(b) => Ok(b),
            _ => Err(SidecarError::WrongType {
                line,
                key: key.to_string(),
                expected: "bool",
            }),
        }
    }
    fn into_int(self, line: usize, key: &str) -> Result<i64, SidecarError> {
        match self {
            Value::Int(n) => Ok(n),
            _ => Err(SidecarError::WrongType {
                line,
                key: key.to_string(),
                expected: "integer",
            }),
        }
    }
}

fn parse_value<'a>(input: &'a str, line: usize) -> Result<(Value, &'a str), SidecarError> {
    match input.as_bytes().first().copied() {
        Some(b'"') => parse_basic_string(input, line),
        Some(b'\'') => Err(SidecarError::BadValue {
            line,
            reason: "literal-string form not allowed",
        }),
        Some(b't') | Some(b'f') => parse_bool(input, line),
        Some(b) if b.is_ascii_digit() => parse_int(input, line),
        Some(b'-') | Some(b'+') => Err(SidecarError::BadValue {
            line,
            reason: "signed integer not allowed",
        }),
        _ => Err(SidecarError::BadValue {
            line,
            reason: "unrecognized value",
        }),
    }
}

fn parse_basic_string<'a>(input: &'a str, line: usize) -> Result<(Value, &'a str), SidecarError> {
    if input.starts_with("\"\"\"") {
        return Err(SidecarError::BadValue {
            line,
            reason: "multi-line basic string not allowed",
        });
    }
    let mut chars = input.char_indices();
    chars.next();
    let mut out = String::new();
    while let Some((idx, c)) = chars.next() {
        match c {
            '"' => {
                let rest = &input[idx + '"'.len_utf8()..];
                return Ok((Value::Str(out), rest));
            }
            '\\' => match chars.next() {
                Some((_, '"')) => out.push('"'),
                Some((_, '\\')) => out.push('\\'),
                Some((_, 'n')) => out.push('\n'),
                Some((_, 't')) => out.push('\t'),
                _ => return Err(SidecarError::BadEscape { line }),
            },
            c if (c as u32) < 0x20 => {
                return Err(SidecarError::BadValue {
                    line,
                    reason: "bare control character in string",
                });
            }
            c => out.push(c),
        }
    }
    Err(SidecarError::UnclosedString { line })
}

fn parse_bool<'a>(input: &'a str, line: usize) -> Result<(Value, &'a str), SidecarError> {
    let (val, len) = if input.starts_with("true") {
        (true, 4)
    } else if input.starts_with("false") {
        (false, 5)
    } else {
        return Err(SidecarError::BadValue {
            line,
            reason: "expected bool",
        });
    };
    let rest = &input[len..];
    if rest
        .chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric() || c == '_')
    {
        return Err(SidecarError::BadValue {
            line,
            reason: "expected bool",
        });
    }
    Ok((Value::Bool(val), rest))
}

fn parse_int<'a>(input: &'a str, line: usize) -> Result<(Value, &'a str), SidecarError> {
    let mut end = 0;
    for (i, c) in input.char_indices() {
        if c.is_ascii_digit() {
            end = i + 1;
        } else {
            break;
        }
    }
    if end == 0 {
        return Err(SidecarError::BadValue {
            line,
            reason: "expected integer",
        });
    }
    let n: i64 = input[..end].parse().map_err(|_| SidecarError::BadValue {
        line,
        reason: "integer overflow",
    })?;
    let rest = &input[end..];
    if rest
        .chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
    {
        return Err(SidecarError::BadValue {
            line,
            reason: "unexpected character after integer",
        });
    }
    Ok((Value::Int(n), rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> &'static str {
        "target = \"copilot\"\n\
         template = \"-p \\\"hello {1}\\\" --yolo\"\n\
         shell = false\n\
         cwd = \"~\"\n\
         description = \"test shim\"\n\
         created = \"2026-05-06T10:00:00Z\"\n\
         version = 1\n"
    }

    #[test]
    fn parse_full_config() {
        let cfg = parse_str(good()).unwrap();
        assert_eq!(cfg.target, "copilot");
        assert_eq!(cfg.template, "-p \"hello {1}\" --yolo");
        assert!(!cfg.shell);
        assert_eq!(cfg.cwd, "~");
        assert_eq!(cfg.description, "test shim");
        assert_eq!(cfg.created.as_deref(), Some("2026-05-06T10:00:00Z"));
        assert_eq!(cfg.version, 1);
    }

    #[test]
    fn rejects_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(good().as_bytes());
        assert!(matches!(parse_bytes(&bytes), Err(SidecarError::Bom)));
    }

    #[test]
    fn accepts_crlf_line_endings() {
        let crlf = good().replace('\n', "\r\n");
        let cfg = parse_str(&crlf).unwrap();
        assert_eq!(cfg.target, "copilot");
    }

    #[test]
    fn unknown_key_warns_but_parses() {
        let text = "target = \"x\"\ntemplate = \"y\"\nmystery = \"ignored\"\n";
        let cfg = parse_str(text).unwrap();
        assert_eq!(cfg.target, "x");
        assert_eq!(cfg.template, "y");
    }

    #[test]
    fn missing_target_errors_78() {
        let err = parse_str("template = \"x\"\n").unwrap_err();
        assert!(matches!(
            err,
            SidecarError::MissingRequired { key: "target" }
        ));
        assert_eq!(err.exit_code(), 78);
    }

    #[test]
    fn missing_template_errors_78() {
        let err = parse_str("target = \"x\"\n").unwrap_err();
        assert!(matches!(
            err,
            SidecarError::MissingRequired { key: "template" }
        ));
    }

    #[test]
    fn version_99_rejected() {
        let text = "target = \"x\"\ntemplate = \"y\"\nversion = 99\n";
        let err = parse_str(text).unwrap_err();
        assert!(matches!(err, SidecarError::BadVersion { value: 99 }));
        assert_eq!(err.exit_code(), 78);
    }

    #[test]
    fn version_zero_rejected() {
        let text = "target = \"x\"\ntemplate = \"y\"\nversion = 0\n";
        let err = parse_str(text).unwrap_err();
        assert!(matches!(err, SidecarError::BadVersion { value: 0 }));
    }

    #[test]
    fn version_missing_defaults_to_one() {
        let cfg = parse_str("target = \"x\"\ntemplate = \"y\"\n").unwrap();
        assert_eq!(cfg.version, 1);
    }

    #[test]
    fn literal_string_form_rejected() {
        let err = parse_str("target = 'literal'\ntemplate = \"y\"\n").unwrap_err();
        assert_eq!(err.exit_code(), 78);
    }

    #[test]
    fn multiline_string_form_rejected() {
        let err = parse_str("target = \"\"\"multi\"\"\"\ntemplate = \"y\"\n").unwrap_err();
        assert_eq!(err.exit_code(), 78);
    }

    #[test]
    fn unknown_escape_rejected() {
        let err = parse_str("target = \"x\"\ntemplate = \"bad\\xescape\"\n").unwrap_err();
        assert!(matches!(err, SidecarError::BadEscape { .. }));
    }

    #[test]
    fn comment_lines_ignored() {
        let text =
            "# leading\ntarget = \"x\"\n# middle\ntemplate = \"y\"\n# trailing\n";
        let cfg = parse_str(text).unwrap();
        assert_eq!(cfg.target, "x");
        assert_eq!(cfg.template, "y");
    }

    #[test]
    fn trailing_inline_comment_ignored() {
        let text = "target = \"x\" # named target\ntemplate = \"y\"\n";
        let cfg = parse_str(text).unwrap();
        assert_eq!(cfg.target, "x");
    }

    #[test]
    fn multiple_assignments_rejected() {
        let text = "target = \"x\" template = \"y\"\n";
        let err = parse_str(text).unwrap_err();
        assert!(matches!(err, SidecarError::MultipleAssignments { .. }));
    }

    #[test]
    fn bare_control_char_in_string_rejected() {
        let text = "target = \"x\"\ntemplate = \"a\u{0001}b\"\n";
        let err = parse_str(text).unwrap_err();
        assert_eq!(err.exit_code(), 78);
    }

    #[test]
    fn empty_string_value_accepted() {
        let text = "target = \"x\"\ntemplate = \"y\"\ncwd = \"\"\ndescription = \"\"\n";
        let cfg = parse_str(text).unwrap();
        assert_eq!(cfg.cwd, "");
        assert_eq!(cfg.description, "");
    }

    #[test]
    fn escape_sequences_decoded() {
        let text =
            "target = \"x\"\ntemplate = \"tab\\there\\nnewline\\\"quote\\\\backslash\"\n";
        let cfg = parse_str(text).unwrap();
        assert_eq!(cfg.template, "tab\there\nnewline\"quote\\backslash");
    }

    #[test]
    fn whitespace_around_eq_tolerated() {
        let cfg = parse_str("target=\"x\"\ntemplate    =    \"y\"\n").unwrap();
        assert_eq!(cfg.target, "x");
        assert_eq!(cfg.template, "y");
    }

    #[test]
    fn blank_lines_skipped() {
        let cfg = parse_str("\n\ntarget = \"x\"\n\n\ntemplate = \"y\"\n\n").unwrap();
        assert_eq!(cfg.target, "x");
    }

    #[test]
    fn derive_sidecar_substitutes_exe() {
        let p = Path::new("C:/foo/wt.exe");
        let r = derive_sidecar_path(p).unwrap();
        assert_eq!(r, Path::new("C:/foo/wt.shrt"));
    }

    #[test]
    fn derive_sidecar_case_insensitive() {
        let p = Path::new("C:/foo/wt.EXE");
        let r = derive_sidecar_path(p).unwrap();
        assert_eq!(r.extension().unwrap(), "shrt");
    }

    #[test]
    fn derive_sidecar_rejects_non_exe() {
        let p = Path::new("C:/foo/wt.bin");
        assert!(matches!(
            derive_sidecar_path(p),
            Err(SidecarError::BadShimSuffix)
        ));
    }

    #[test]
    fn derive_sidecar_rejects_no_extension() {
        let p = Path::new("C:/foo/wt");
        assert!(matches!(
            derive_sidecar_path(p),
            Err(SidecarError::BadShimSuffix)
        ));
    }
}
