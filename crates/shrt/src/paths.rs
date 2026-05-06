use std::path::{Path, PathBuf};

pub fn shim_dir(override_: Option<&Path>) -> anyhow::Result<PathBuf> {
    if let Some(p) = override_ {
        return Ok(p.to_path_buf());
    }
    let user_dirs = directories::UserDirs::new()
        .ok_or_else(|| anyhow::anyhow!("could not determine user home directory"))?;
    Ok(user_dirs.home_dir().join(".shrt").join("bin"))
}

pub fn is_on_path(shim_dir: &Path) -> bool {
    let path_var = std::env::var("PATH").unwrap_or_default();
    is_on_path_in(&path_var, shim_dir)
}

pub fn is_on_path_in(path_var: &str, shim_dir: &Path) -> bool {
    let needle = normalize_for_compare(shim_dir);
    path_var
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .any(|entry| normalize_for_compare(Path::new(entry)) == needle)
}

pub(crate) fn normalize_for_compare(p: &Path) -> String {
    let mut s = p.to_string_lossy().replace('/', "\\");
    while s.len() > 3 && s.ends_with('\\') {
        s.pop();
    }
    s.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shim_dir_uses_override() {
        let custom = Path::new("C:/foo/bar");
        let r = shim_dir(Some(custom)).unwrap();
        assert_eq!(r, custom);
    }

    #[test]
    fn shim_dir_default_under_home() {
        let r = shim_dir(None).unwrap();
        let s = r.to_string_lossy();
        assert!(s.contains(".shrt"));
        assert!(s.ends_with("bin"));
    }

    #[test]
    fn is_on_path_in_finds_dir() {
        let path_var = "C:\\foo;C:\\Users\\bob\\.shrt\\bin;C:\\bar";
        let dir = Path::new("C:\\Users\\bob\\.shrt\\bin");
        assert!(is_on_path_in(path_var, dir));
    }

    #[test]
    fn is_on_path_in_case_insensitive() {
        let path_var = "C:\\FOO;c:\\Users\\BOB\\.shrt\\bin";
        let dir = Path::new("C:\\users\\bob\\.shrt\\bin");
        assert!(is_on_path_in(path_var, dir));
    }

    #[test]
    fn is_on_path_in_returns_false_when_absent() {
        let path_var = "C:\\foo;C:\\bar";
        let dir = Path::new("C:\\baz");
        assert!(!is_on_path_in(path_var, dir));
    }

    #[test]
    fn is_on_path_in_tolerates_trailing_semicolon() {
        let path_var = "C:\\foo;C:\\Users\\bob\\.shrt\\bin;";
        let dir = Path::new("C:\\Users\\bob\\.shrt\\bin");
        assert!(is_on_path_in(path_var, dir));
    }

    #[test]
    fn is_on_path_in_empty_path_var_false() {
        let dir = Path::new("C:\\Users\\bob\\.shrt\\bin");
        assert!(!is_on_path_in("", dir));
    }

    #[test]
    fn is_on_path_in_normalizes_slashes() {
        let path_var = "C:/Users/bob/.shrt/bin";
        let dir = Path::new("C:\\Users\\bob\\.shrt\\bin");
        assert!(is_on_path_in(path_var, dir));
    }

    #[test]
    fn is_on_path_in_tolerates_trailing_backslash() {
        let path_var = "C:\\Users\\bob\\.shrt\\bin\\";
        let dir = Path::new("C:\\Users\\bob\\.shrt\\bin");
        assert!(is_on_path_in(path_var, dir));
    }

    #[test]
    fn is_on_path_in_skips_empty_entries() {
        let path_var = ";;C:\\Users\\bob\\.shrt\\bin;;";
        let dir = Path::new("C:\\Users\\bob\\.shrt\\bin");
        assert!(is_on_path_in(path_var, dir));
    }

    #[test]
    fn is_on_path_in_partial_substring_does_not_match() {
        let path_var = "C:\\Users\\bob\\.shrt\\binary";
        let dir = Path::new("C:\\Users\\bob\\.shrt\\bin");
        assert!(!is_on_path_in(path_var, dir));
    }
}
