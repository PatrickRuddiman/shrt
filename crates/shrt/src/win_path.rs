use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct PathChange {
    pub added: bool,
    pub already_present: bool,
}

#[cfg(windows)]
pub fn ensure_on_user_path(dir: &Path) -> anyhow::Result<PathChange> {
    use winreg::enums::*;
    use winreg::{RegKey, RegValue};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .map_err(|e| anyhow::anyhow!("opening HKCU\\Environment: {}", e))?;

    let (current_str, vtype) = match env.get_raw_value("Path") {
        Ok(rv) => (decode_utf16le(&rv.bytes), rv.vtype),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            (String::new(), REG_EXPAND_SZ)
        }
        Err(e) => anyhow::bail!("reading HKCU\\Environment\\Path: {}", e),
    };

    if path_contains(&current_str, dir) {
        return Ok(PathChange {
            added: false,
            already_present: true,
        });
    }

    let new_str = append_path(&current_str, dir);

    let rv = RegValue {
        bytes: encode_utf16le_nul(&new_str),
        vtype,
    };
    env.set_raw_value("Path", &rv)
        .map_err(|e| anyhow::anyhow!("writing HKCU\\Environment\\Path: {}", e))?;

    broadcast_environment_change();

    Ok(PathChange {
        added: true,
        already_present: false,
    })
}

#[cfg(not(windows))]
pub fn ensure_on_user_path(_dir: &Path) -> anyhow::Result<PathChange> {
    anyhow::bail!("auto PATH-add only implemented on Windows");
}

fn path_contains(path_var: &str, dir: &Path) -> bool {
    let needle = crate::paths::normalize_for_compare(dir);
    path_var
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .any(|entry| crate::paths::normalize_for_compare(Path::new(entry)) == needle)
}

fn append_path(current: &str, dir: &Path) -> String {
    let trimmed = current.trim_end_matches(';');
    let dir_str = dir.display().to_string();
    if trimmed.is_empty() {
        dir_str
    } else {
        format!("{};{}", trimmed, dir_str)
    }
}

fn decode_utf16le(bytes: &[u8]) -> String {
    let mut units: Vec<u16> = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        units.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    while units.last() == Some(&0) {
        units.pop();
    }
    String::from_utf16_lossy(&units)
}

fn encode_utf16le_nul(s: &str) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(s.encode_utf16().count() * 2 + 2);
    for unit in s.encode_utf16() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out.extend_from_slice(&[0, 0]);
    out
}

#[cfg(windows)]
fn broadcast_environment_change() {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };

    let env_wide: Vec<u16> = "Environment\0".encode_utf16().collect();
    let mut result: usize = 0;
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST as HWND,
            WM_SETTINGCHANGE,
            0 as WPARAM,
            env_wide.as_ptr() as LPARAM,
            SMTO_ABORTIFHUNG,
            5000,
            &mut result,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_contains_finds_dir() {
        assert!(path_contains(
            "C:\\foo;C:\\Users\\bob\\.shrt\\bin;C:\\bar",
            Path::new("C:\\Users\\bob\\.shrt\\bin")
        ));
    }

    #[test]
    fn path_contains_case_insensitive() {
        assert!(path_contains(
            "C:\\Users\\BOB\\.shrt\\bin",
            Path::new("c:\\users\\bob\\.shrt\\BIN")
        ));
    }

    #[test]
    fn path_contains_misses_partial() {
        assert!(!path_contains(
            "C:\\Users\\bob\\.shrt\\binary",
            Path::new("C:\\Users\\bob\\.shrt\\bin")
        ));
    }

    #[test]
    fn path_contains_handles_empty() {
        assert!(!path_contains("", Path::new("C:\\foo")));
    }

    #[test]
    fn append_path_to_empty() {
        assert_eq!(append_path("", Path::new("C:\\foo")), "C:\\foo");
    }

    #[test]
    fn append_path_to_existing() {
        assert_eq!(
            append_path("C:\\a;C:\\b", Path::new("C:\\foo")),
            "C:\\a;C:\\b;C:\\foo"
        );
    }

    #[test]
    fn append_path_strips_trailing_semicolons() {
        assert_eq!(
            append_path("C:\\a;C:\\b;", Path::new("C:\\foo")),
            "C:\\a;C:\\b;C:\\foo"
        );
    }

    #[test]
    fn utf16le_round_trip() {
        let original = "C:\\Users\\bob;%USERPROFILE%\\.shrt\\bin";
        let encoded = encode_utf16le_nul(original);
        // Last two bytes should be the trailing nul.
        assert_eq!(&encoded[encoded.len() - 2..], &[0, 0]);
        let decoded = decode_utf16le(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_strips_trailing_nuls() {
        let mut bytes = encode_utf16le_nul("hello");
        bytes.extend_from_slice(&[0, 0, 0, 0]); // extra nuls
        assert_eq!(decode_utf16le(&bytes), "hello");
    }
}
