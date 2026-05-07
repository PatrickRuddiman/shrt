mod common;
use common::*;

#[test]
fn sync_skips_unchanged_shims() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);

    let output = shrt(tmp.path()).arg("sync").arg("--json").output().unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["total"], 1);
    assert_eq!(report["updated"], 0);
    assert_eq!(report["errors"].as_array().unwrap().len(), 0);
}

#[test]
fn sync_restores_modified_shim_bytes() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);
    let exe = tmp.path().join("wt0.exe");
    let original = std::fs::read(&exe).unwrap();

    std::fs::write(&exe, b"junk-bytes-not-runner").unwrap();
    let corrupted = std::fs::read(&exe).unwrap();
    assert_ne!(corrupted, original);

    let output = shrt(tmp.path()).arg("sync").arg("--json").output().unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["updated"], 1);
    assert_eq!(report["total"], 1);

    let restored = std::fs::read(&exe).unwrap();
    assert_eq!(restored, original, "bytes should match the original RUNNER_BYTES");
}

#[test]
fn sync_json_shape() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);

    let output = shrt(tmp.path()).arg("sync").arg("--json").output().unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(report.get("updated").is_some());
    assert!(report.get("total").is_some());
    assert!(report.get("errors").is_some());
    assert!(report["errors"].is_array());
}

#[test]
fn sync_handles_missing_exe() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);
    std::fs::remove_file(tmp.path().join("wt0.exe")).unwrap();

    let output = shrt(tmp.path()).arg("sync").arg("--json").output().unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let errors = report["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0][0], "wt0");
    // Every shim failed -> exit 1
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn sync_text_mode_prints_summary() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);

    let output = shrt(tmp.path()).arg("sync").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("updated:"));
    assert!(stdout.contains("total:"));
}

#[test]
fn sync_empty_dir_returns_zero_total() {
    let tmp = make_shim_dir();

    let output = shrt(tmp.path()).arg("sync").arg("--json").output().unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["total"], 0);
    assert_eq!(report["updated"], 0);
}
