mod common;
use common::*;
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn add_then_invoke_passes_argv_correctly() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1} {2}", false);

    let output = invoke_shim(tmp.path(), "wt", &["foo", "bar"], &[]);
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["argv"], serde_json::json!(["foo", "bar"]));
}

#[test]
fn placeholder_input_joins_with_single_space() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{INPUT}", false);

    let output = invoke_shim(tmp.path(), "wt", &["a", "b", "c"], &[]);
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    // {INPUT} produces "a b c"; tokenizer then splits on whitespace.
    assert_eq!(json["argv"], serde_json::json!(["a", "b", "c"]));
}

#[test]
fn placeholder_at_preserves_arg_boundaries() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{@}", false);

    let output = invoke_shim(tmp.path(), "wt", &["a b", "c"], &[]);
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["argv"], serde_json::json!(["a b", "c"]));
}

#[test]
fn placeholder_env_substitutes_env_value() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{ENV:SHRT_TEST_GREETING}", false);

    let output = invoke_shim(
        tmp.path(),
        "wt",
        &[],
        &[("SHRT_TEST_GREETING", "hello")],
    );
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["argv"], serde_json::json!(["hello"]));
}

#[test]
fn placeholder_env_default_used_when_unset() {
    let tmp = make_shim_dir();
    add_stub_shim(
        tmp.path(),
        "wt",
        "{ENV:SHRT_NEVERSET_VAR_XYZ:fallback}",
        false,
    );

    let output = invoke_shim(tmp.path(), "wt", &[], &[]);
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["argv"], serde_json::json!(["fallback"]));
}

#[test]
fn shell_true_supports_pipes() {
    let tmp = make_shim_dir();

    let status = shrt(tmp.path())
        .arg("add")
        .arg("piper")
        .arg("echo hello | findstr h")
        .arg("--shell")
        .status()
        .unwrap();
    assert!(status.success());

    let output = invoke_shim(tmp.path(), "piper", &[], &[]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello"),
        "expected 'hello' in stdout, got: {}",
        stdout
    );
}

#[test]
fn child_exit_code_propagates() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "", false);

    let output = invoke_shim(tmp.path(), "wt", &[], &[("EXIT_CODE", "42")]);
    assert_eq!(output.status.code(), Some(42));
}

#[test]
fn stdin_passthrough_works() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "", false);

    let exe = tmp.path().join("wt.exe");
    let mut cmd = Command::new(&exe);
    cmd.env("READ_STDIN", "1");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"hello stdin")
        .unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["stdin"], "hello stdin");
}

#[test]
fn stdout_passthrough_works() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1}", false);

    let output = invoke_shim(tmp.path(), "wt", &["x"], &[]);
    assert!(output.status.success());
    assert!(
        output.stdout.starts_with(b"{"),
        "expected JSON on stdout, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn stderr_passthrough_works() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1}", false);

    let output = invoke_shim(tmp.path(), "wt", &["x"], &[("WRITE_STDERR", "uh oh")]);
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("uh oh"), "stderr: {}", stderr);
}
