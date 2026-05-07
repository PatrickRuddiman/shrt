mod common;
use common::*;

#[test]
fn doctor_reports_mixed_state() {
    let tmp = make_shim_dir();

    add_stub_shim(tmp.path(), "good", "{1}", false);

    add_stub_shim(tmp.path(), "badparse", "{1}", false);
    std::fs::write(tmp.path().join("badparse.shrt"), "not valid toml = =").unwrap();

    add_stub_shim(tmp.path(), "missingtarget", "{1}", false);
    std::fs::write(
        tmp.path().join("missingtarget.shrt"),
        "target = \"definitelynotacommand_xyz_42\"\ntemplate = \"\"\nversion = 1\n",
    )
    .unwrap();

    let output = shrt(tmp.path()).arg("doctor").arg("--json").output().unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(report["summary"], "fail");
    assert_eq!(output.status.code(), Some(1));

    let checks = report["checks"].as_array().unwrap();

    let badparse_parse = checks.iter().find(|c| c["name"] == "badparse: parse");
    assert!(badparse_parse.is_some(), "missing badparse: parse");
    assert_eq!(badparse_parse.unwrap()["status"], "fail");

    let missingtarget_target = checks
        .iter()
        .find(|c| c["name"] == "missingtarget: target");
    assert!(missingtarget_target.is_some(), "missing missingtarget: target");
    assert_eq!(missingtarget_target.unwrap()["status"], "fail");

    let good_checks: Vec<_> = checks
        .iter()
        .filter(|c| {
            c["name"]
                .as_str()
                .map(|n| n.starts_with("good:"))
                .unwrap_or(false)
        })
        .collect();
    assert!(
        !good_checks.is_empty(),
        "expected at least one check for 'good'"
    );
    for c in &good_checks {
        assert_eq!(
            c["status"], "ok",
            "good check '{}' should be ok",
            c["name"]
        );
    }

    let acls = checks.iter().find(|c| c["name"] == "acls").unwrap();
    assert_eq!(acls["status"], "warn");
}

#[test]
fn doctor_clean_state_no_fails() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);

    let output = shrt(tmp.path()).arg("doctor").arg("--json").output().unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_ne!(report["summary"], "fail");
    let fails: Vec<_> = report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["status"] == "fail")
        .collect();
    assert!(
        fails.is_empty(),
        "expected no fails, got: {:?}",
        fails
    );
    assert!(output.status.success());
}

#[test]
fn doctor_text_output_uses_status_tags() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);

    let output = shrt(tmp.path()).arg("doctor").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[OK]"));
    assert!(stdout.contains("[WARN]"));
}

#[test]
fn doctor_exit_zero_on_warn_only() {
    let tmp = make_shim_dir();
    // Empty shim dir + ACL warn = at most warns, no fails
    let output = shrt(tmp.path()).arg("doctor").output().unwrap();
    assert!(output.status.success());
}

#[test]
fn doctor_byte_mismatch_reported() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);
    std::fs::write(tmp.path().join("wt0.exe"), b"junk").unwrap();

    let output = shrt(tmp.path()).arg("doctor").arg("--json").output().unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let bytes = report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "wt0: bytes")
        .unwrap();
    assert_eq!(bytes["status"], "fail");
    assert!(
        bytes["message"]
            .as_str()
            .unwrap()
            .contains("shrt sync"),
        "expected sync hint in message"
    );
}
