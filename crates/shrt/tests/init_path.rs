use std::process::Command;
use tempfile::tempdir;

fn shrt_bin() -> &'static str {
    env!("CARGO_BIN_EXE_shrt")
}

#[test]
fn init_creates_shim_dir() {
    let tmp = tempdir().unwrap();
    let shim_dir = tmp.path().join("bin");

    let output = Command::new(shrt_bin())
        .arg("init")
        .arg("--shim-dir")
        .arg(&shim_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(shim_dir.is_dir());
}

#[test]
fn init_idempotent() {
    let tmp = tempdir().unwrap();
    let shim_dir = tmp.path().join("bin");

    let first = Command::new(shrt_bin())
        .arg("init")
        .arg("--shim-dir")
        .arg(&shim_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert!(first.status.success());
    let first_json: serde_json::Value =
        serde_json::from_slice(&first.stdout).expect("first init produced valid JSON");
    assert_eq!(first_json["created"], true);

    let second = Command::new(shrt_bin())
        .arg("init")
        .arg("--shim-dir")
        .arg(&shim_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert!(second.status.success());
    let second_json: serde_json::Value =
        serde_json::from_slice(&second.stdout).expect("second init produced valid JSON");
    assert_eq!(second_json["created"], false);
}

#[test]
fn init_json_includes_required_fields() {
    let tmp = tempdir().unwrap();
    let shim_dir = tmp.path().join("bin");

    let output = Command::new(shrt_bin())
        .arg("init")
        .arg("--shim-dir")
        .arg(&shim_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    for field in &[
        "shim_dir",
        "created",
        "on_path",
        "path_added",
        "path_already_present",
        "path_error",
    ] {
        assert!(json.get(*field).is_some(), "missing {}", field);
    }
    assert!(json["on_path"].is_boolean());
    assert!(json["path_added"].is_boolean());
    assert!(json["path_already_present"].is_boolean());
}

#[cfg(windows)]
#[test]
fn init_adds_to_user_path_then_idempotent() {
    use winreg::enums::*;
    use winreg::{RegKey, RegValue};

    // Snapshot HKCU\Environment\Path so we can restore it after the test
    // mutates the user's PATH.
    struct PathGuard {
        original: Option<RegValue>,
    }
    impl Drop for PathGuard {
        fn drop(&mut self) {
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(env) = hkcu.open_subkey_with_flags("Environment", KEY_WRITE) {
                match self.original.take() {
                    Some(rv) => {
                        let _ = env.set_raw_value("Path", &rv);
                    }
                    None => {
                        let _ = env.delete_value("Path");
                    }
                }
            }
        }
    }

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ)
        .expect("open Environment");
    let _guard = PathGuard {
        original: env.get_raw_value("Path").ok(),
    };

    let tmp = tempdir().unwrap();
    let shim_dir = tmp.path().join("shrt-test-bin");

    // First run: path_added should be true (assuming registry write works).
    let first = Command::new(shrt_bin())
        .arg("init")
        .arg("--shim-dir")
        .arg(&shim_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert!(first.status.success());
    let first_json: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert!(
        first_json["path_error"].is_null(),
        "registry write failed: {:?}",
        first_json["path_error"]
    );
    assert_eq!(first_json["path_added"], true);
    assert_eq!(first_json["path_already_present"], false);

    // Second run: path_already_present should be true.
    let second = Command::new(shrt_bin())
        .arg("init")
        .arg("--shim-dir")
        .arg(&shim_dir)
        .arg("--json")
        .output()
        .unwrap();
    assert!(second.status.success());
    let second_json: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(second_json["path_added"], false);
    assert_eq!(second_json["path_already_present"], true);
}

#[test]
fn init_quiet_suppresses_text_output() {
    let tmp = tempdir().unwrap();
    let shim_dir = tmp.path().join("bin");

    let output = Command::new(shrt_bin())
        .arg("init")
        .arg("--shim-dir")
        .arg(&shim_dir)
        .arg("--quiet")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "expected no stdout in quiet mode");
}

#[test]
fn path_prints_shim_dir() {
    let tmp = tempdir().unwrap();
    let shim_dir = tmp.path().join("bin");

    let output = Command::new(shrt_bin())
        .arg("path")
        .arg("--shim-dir")
        .arg(&shim_dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected_str = shim_dir.to_string_lossy();
    assert!(
        stdout.trim().contains(&*expected_str)
            || stdout.trim().contains(&shim_dir.display().to_string()),
        "stdout '{}' did not contain '{}'",
        stdout.trim(),
        expected_str
    );
}

#[test]
fn path_json_shape() {
    let tmp = tempdir().unwrap();
    let shim_dir = tmp.path().join("bin");

    let output = Command::new(shrt_bin())
        .arg("path")
        .arg("--shim-dir")
        .arg(&shim_dir)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.get("path").is_some());
    assert!(json.get("on_path").is_some());
    assert!(json["on_path"].is_boolean());
}
