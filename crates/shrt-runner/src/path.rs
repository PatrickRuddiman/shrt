use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum PathError {
    Empty,
    EnvUnset(String),
    EnvNotUtf8(String),
    NotFound(String),
    CwdMissing(PathBuf),
}

impl PathError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NotFound(_) => 127,
            _ => 78,
        }
    }
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "target is empty"),
            Self::EnvUnset(name) => write!(f, "environment variable '{}' is not set", name),
            Self::EnvNotUtf8(name) => {
                write!(f, "environment variable '{}' is not valid UTF-8", name)
            }
            Self::NotFound(t) => write!(f, "target '{}' not found on PATH", t),
            Self::CwdMissing(p) => write!(f, "cwd does not exist: {}", p.display()),
        }
    }
}

pub fn resolve_target(target: &str) -> Result<PathBuf, PathError> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let pathext_var =
        std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
    resolve_target_in(target, &path_var, &pathext_var)
}

pub fn expand_cwd(cwd: &str) -> Result<Option<PathBuf>, PathError> {
    expand_cwd_with(cwd, &|n| std::env::var_os(n))
}

fn resolve_target_in(
    target: &str,
    path_var: &str,
    pathext_var: &str,
) -> Result<PathBuf, PathError> {
    if target.is_empty() {
        return Err(PathError::Empty);
    }

    if target.contains('/') || target.contains('\\') {
        let path = PathBuf::from(target);
        let abs = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .map(|d| d.join(&path))
                .unwrap_or(path)
        };
        if abs.exists() {
            return Ok(abs);
        }
        return Err(PathError::NotFound(target.to_string()));
    }

    let extensions: Vec<String> = pathext_var
        .split(';')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let target_lower = target.to_lowercase();
    let target_has_known_ext = extensions.iter().any(|ext| target_lower.ends_with(ext));

    let probe_exts: Vec<&str> = if target_has_known_ext {
        vec![""]
    } else {
        extensions.iter().map(|s| s.as_str()).collect()
    };

    for dir in path_var.split(';') {
        let dir = dir.trim();
        if dir.is_empty() {
            continue;
        }
        let dir_path = Path::new(dir);
        for ext in &probe_exts {
            let candidate = dir_path.join(format!("{}{}", target, ext));
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    Err(PathError::NotFound(target.to_string()))
}

fn expand_cwd_with(
    cwd: &str,
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Result<Option<PathBuf>, PathError> {
    if cwd.is_empty() {
        return Ok(None);
    }

    let mut out = String::with_capacity(cwd.len());
    let mut chars = cwd.chars().peekable();

    if chars.peek() == Some(&'~') {
        let mut iter = chars.clone();
        iter.next();
        match iter.peek() {
            None | Some('/') | Some('\\') => {
                let home = match env("USERPROFILE") {
                    Some(v) => v
                        .to_str()
                        .map(|s| s.to_string())
                        .ok_or_else(|| PathError::EnvNotUtf8("USERPROFILE".to_string()))?,
                    None => return Err(PathError::EnvUnset("USERPROFILE".to_string())),
                };
                out.push_str(&home);
                chars.next();
            }
            _ => {}
        }
    }

    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            chars.next();
            let mut name = String::new();
            let mut closed = false;
            for c2 in chars.by_ref() {
                if c2 == '}' {
                    closed = true;
                    break;
                }
                name.push(c2);
            }
            if !closed || name.is_empty() {
                return Err(PathError::EnvUnset(name));
            }
            match env(&name) {
                Some(v) => match v.to_str() {
                    Some(s) => out.push_str(s),
                    None => return Err(PathError::EnvNotUtf8(name)),
                },
                None => return Err(PathError::EnvUnset(name)),
            }
        } else {
            out.push(c);
        }
    }

    let path = PathBuf::from(&out);
    if !path.is_dir() {
        return Err(PathError::CwdMissing(path));
    }
    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_unique(label: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let name = format!(
            "shrt_runner_path_{}_{}_{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        p.push(name);
        p
    }

    #[test]
    fn resolve_target_empty_errors() {
        let err = resolve_target("").unwrap_err();
        assert!(matches!(err, PathError::Empty));
        assert_eq!(err.exit_code(), 78);
    }

    #[test]
    fn resolve_target_finds_cmd_via_system_path() {
        let r = resolve_target("cmd").unwrap();
        assert!(r.is_file());
        assert!(r.to_string_lossy().to_lowercase().ends_with("cmd.exe"));
    }

    #[test]
    fn resolve_target_finds_findstr_via_system_path() {
        let r = resolve_target("findstr").unwrap();
        assert!(r.is_file());
    }

    #[test]
    fn resolve_target_bare_not_found_127() {
        let err = resolve_target("definitely_not_a_real_command_xyz_42").unwrap_err();
        assert!(matches!(err, PathError::NotFound(_)));
        assert_eq!(err.exit_code(), 127);
    }

    #[test]
    fn resolve_target_path_style_passthrough() {
        let p = temp_unique("absolute_passthrough");
        std::fs::write(&p, b"x").unwrap();
        let s = p.to_str().unwrap();
        let r = resolve_target(s).unwrap();
        assert_eq!(r, p);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn resolve_target_path_style_not_found() {
        let err = resolve_target("C:/nonexistent_xyz_42/foo.exe").unwrap_err();
        assert!(matches!(err, PathError::NotFound(_)));
    }

    #[test]
    fn resolve_target_in_pathext_search() {
        let dir = std::env::temp_dir();
        let stem = format!(
            "shrt_test_pathext_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let bin = dir.join(format!("{}.bat", stem));
        std::fs::write(&bin, b"@echo off").unwrap();
        let result = resolve_target_in(&stem, dir.to_str().unwrap(), ".BAT;.EXE").unwrap();
        assert_eq!(result, bin);
        let _ = std::fs::remove_file(&bin);
    }

    #[test]
    fn resolve_target_in_skips_pathext_when_target_has_extension() {
        let dir = std::env::temp_dir();
        let name = format!(
            "shrt_test_pathext_has_{}_{}.exe",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let bin = dir.join(&name);
        std::fs::write(&bin, b"x").unwrap();
        let result = resolve_target_in(&name, dir.to_str().unwrap(), ".EXE;.BAT").unwrap();
        assert_eq!(result, bin);
        let _ = std::fs::remove_file(&bin);
    }

    #[test]
    fn resolve_target_in_default_pathext_when_unset() {
        let r = resolve_target_in(
            "cmd",
            &std::env::var("PATH").unwrap_or_default(),
            ".COM;.EXE;.BAT;.CMD",
        )
        .unwrap();
        assert!(r.is_file());
    }

    #[test]
    fn expand_cwd_empty_returns_none() {
        let r = expand_cwd("").unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn expand_cwd_tilde_uses_userprofile() {
        let temp = std::env::temp_dir();
        let temp_str = temp.to_str().unwrap().to_string();
        let env = move |k: &str| -> Option<OsString> {
            if k == "USERPROFILE" {
                Some(OsString::from(&temp_str))
            } else {
                None
            }
        };
        let r = expand_cwd_with("~", &env).unwrap();
        assert!(r.is_some());
        assert_eq!(r.unwrap(), temp);
    }

    #[test]
    fn expand_cwd_tilde_with_subpath() {
        let temp = std::env::temp_dir();
        let sub = temp_unique("expand_cwd_subdir");
        std::fs::create_dir_all(&sub).unwrap();
        let temp_str = temp.to_str().unwrap().to_string();
        let env = move |k: &str| -> Option<OsString> {
            if k == "USERPROFILE" {
                Some(OsString::from(&temp_str))
            } else {
                None
            }
        };
        let sub_name = sub.file_name().unwrap().to_str().unwrap().to_string();
        let cwd = format!("~/{}", sub_name);
        let r = expand_cwd_with(&cwd, &env).unwrap().unwrap();
        assert!(r.is_dir());
        let _ = std::fs::remove_dir(&sub);
    }

    #[test]
    fn expand_cwd_tilde_unset_userprofile_errors() {
        let env = |_: &str| -> Option<OsString> { None };
        let err = expand_cwd_with("~", &env).unwrap_err();
        assert!(matches!(err, PathError::EnvUnset(_)));
        assert_eq!(err.exit_code(), 78);
    }

    #[test]
    fn expand_cwd_dollar_var_expansion() {
        let temp = std::env::temp_dir();
        let temp_str = temp.to_str().unwrap().to_string();
        let env = move |k: &str| -> Option<OsString> {
            if k == "SHRT_TEST_DIR" {
                Some(OsString::from(&temp_str))
            } else {
                None
            }
        };
        let r = expand_cwd_with("${SHRT_TEST_DIR}", &env).unwrap();
        assert_eq!(r.unwrap(), temp);
    }

    #[test]
    fn expand_cwd_dollar_var_unset_errors() {
        let env = |_: &str| -> Option<OsString> { None };
        let err = expand_cwd_with("${NEVERSET}", &env).unwrap_err();
        assert!(matches!(err, PathError::EnvUnset(_)));
    }

    #[test]
    fn expand_cwd_unmatched_dollar_brace_errors() {
        let env = |_: &str| -> Option<OsString> { None };
        let err = expand_cwd_with("${OPEN", &env).unwrap_err();
        assert!(matches!(err, PathError::EnvUnset(_)));
    }

    #[test]
    fn expand_cwd_missing_directory_errors() {
        let err = expand_cwd("C:/definitely_not_a_directory_xyz_42/sub").unwrap_err();
        assert!(matches!(err, PathError::CwdMissing(_)));
        assert_eq!(err.exit_code(), 78);
    }
}
