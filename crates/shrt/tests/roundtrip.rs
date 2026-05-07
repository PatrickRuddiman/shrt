mod common;
use common::*;

#[test]
fn emoji_in_description_round_trips() {
    let tmp = make_shim_dir();
    let stub = stub_path().to_string_lossy().into_owned();

    let status = shrt(tmp.path())
        .arg("add")
        .arg("wt0")
        .arg(format!("{} t", stub))
        .arg("--desc")
        .arg("🎉 hello")
        .status()
        .unwrap();
    assert!(status.success());

    let output = shrt(tmp.path()).arg("list").arg("--json").output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["description"], "🎉 hello");
}

#[test]
fn newline_in_description_round_trips() {
    let tmp = make_shim_dir();
    let stub = stub_path().to_string_lossy().into_owned();

    let status = shrt(tmp.path())
        .arg("add")
        .arg("wt0")
        .arg(format!("{} t", stub))
        .arg("--desc")
        .arg("line1\nline2")
        .status()
        .unwrap();
    assert!(status.success());

    let output = shrt(tmp.path()).arg("list").arg("--json").output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["description"], "line1\nline2");
}

#[test]
fn unknown_key_warns_but_shim_runs() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt0", "{1}", false);

    let sidecar = tmp.path().join("wt0.shrt");
    let original = std::fs::read_to_string(&sidecar).unwrap();
    let with_unknown = format!("mystery = \"x\"\n{}", original);
    std::fs::write(&sidecar, with_unknown).unwrap();

    let output = invoke_shim(tmp.path(), "wt0", &["arg1"], &[]);
    assert!(
        output.status.success(),
        "expected shim to run despite unknown key; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ignoring unknown key"),
        "stderr did not mention warning: {}",
        stderr
    );
    assert!(stderr.contains("mystery"), "stderr: {}", stderr);
}
