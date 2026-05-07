mod common;
use common::*;

fn try_add(shim_dir: &std::path::Path, name: &str) -> std::process::Output {
    let stub = stub_path().to_string_lossy().into_owned();
    shrt(shim_dir)
        .arg("add")
        .arg(name)
        .arg(format!("{} t", stub))
        .output()
        .unwrap()
}

#[test]
fn rejects_path_separator() {
    let tmp = make_shim_dir();
    assert_eq!(try_add(tmp.path(), "foo/bar").status.code(), Some(64));
    assert_eq!(try_add(tmp.path(), "foo\\bar").status.code(), Some(64));
}

#[test]
fn rejects_reserved_devices() {
    let tmp = make_shim_dir();
    for name in &["con", "PRN", "aux", "nul", "com1", "LPT9"] {
        let out = try_add(tmp.path(), name);
        assert_eq!(
            out.status.code(),
            Some(64),
            "expected 64 for '{}'",
            name
        );
    }
}

#[test]
fn accepts_alphanumeric_with_underscore_hyphen_dot() {
    let tmp = make_shim_dir();
    for name in &["wt0", "wt0_0", "foo-bar", "a.b.c"] {
        let out = try_add(tmp.path(), name);
        assert!(
            out.status.success(),
            "expected '{}' to be accepted; stderr: {}",
            name,
            String::from_utf8_lossy(&out.stderr)
        );
        // remove between iterations to avoid 73 collision
        let _ = std::fs::remove_file(tmp.path().join(format!("{}.exe", name)));
        let _ = std::fs::remove_file(tmp.path().join(format!("{}.shrt", name)));
    }
}

#[test]
fn rejects_double_dot() {
    let tmp = make_shim_dir();
    assert_eq!(try_add(tmp.path(), "a..b").status.code(), Some(64));
}

#[test]
fn rejects_too_long_name() {
    let tmp = make_shim_dir();
    let long = "a".repeat(65);
    assert_eq!(try_add(tmp.path(), &long).status.code(), Some(64));
}
