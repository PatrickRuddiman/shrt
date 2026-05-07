mod common;
use common::*;

#[test]
fn list_empty_when_no_shims() {
    let tmp = make_shim_dir();
    let output = shrt(tmp.path()).arg("list").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "");
}

#[test]
fn list_default_sorted_alphabetically() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);
    add_stub_shim(tmp.path(), "abc", "test", false);

    let output = shrt(tmp.path()).arg("list").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let abc_pos = stdout.find("abc").expect("abc not in output");
    let wt_pos = stdout.find("wt0").expect("wt not in output");
    assert!(
        abc_pos < wt_pos,
        "expected alphabetical order, got: {}",
        stdout
    );
}

#[test]
fn list_verbose_includes_template_and_target() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);

    let output = shrt(tmp.path()).arg("list").arg("--verbose").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("target:"));
    assert!(stdout.contains("template:"));
}

#[test]
fn list_json_shape_includes_all_fields() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);

    let output = shrt(tmp.path()).arg("list").arg("--json").output().unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.is_array());
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    let entry = &arr[0];
    assert_eq!(entry["name"], "wt0");
    for field in &[
        "name",
        "target",
        "template",
        "shell",
        "cwd",
        "description",
        "created",
        "version",
    ] {
        assert!(
            entry.get(*field).is_some(),
            "missing field '{}' in: {}",
            field,
            entry
        );
    }
}

#[test]
fn list_empty_json_is_empty_array() {
    let tmp = make_shim_dir();
    let output = shrt(tmp.path()).arg("list").arg("--json").output().unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[test]
fn show_default_prints_raw_sidecar_contents() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);

    let output = shrt(tmp.path()).arg("show").arg("wt0").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("target = "));
    assert!(stdout.contains("template = "));
    assert!(stdout.contains("version = 1"));
}

#[test]
fn show_json_shape_has_path_and_config() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);

    let output = shrt(tmp.path())
        .arg("show")
        .arg("wt0")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.get("path").is_some());
    let config = json.get("config").expect("missing config");
    assert!(config.get("target").is_some());
    assert!(config.get("template").is_some());
    assert!(config.get("version").is_some());
}

#[test]
fn show_missing_exits_66() {
    let tmp = make_shim_dir();
    let output = shrt(tmp.path())
        .arg("show")
        .arg("doesnotexist")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(66));
}

#[test]
fn list_bad_sidecar_exits_78() {
    let tmp = make_shim_dir();
    // Create a file that looks like a sidecar but is malformed.
    std::fs::write(tmp.path().join("bad.shrt"), "this is not valid toml = =").unwrap();

    let output = shrt(tmp.path()).arg("list").output().unwrap();
    assert_eq!(output.status.code(), Some(78));
}
