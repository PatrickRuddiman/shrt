mod common;
use common::*;

fn write_sidecar(shim_dir: &std::path::Path, name: &str, content: &str) {
    std::fs::write(shim_dir.join(format!("{}.shrt", name)), content).unwrap();
}

#[test]
fn missing_positional_arg_exits_64() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1}", false);

    let output = invoke_shim(tmp.path(), "wt", &[], &[]);
    assert_eq!(output.status.code(), Some(64));
}

#[test]
fn missing_env_var_in_template_exits_78() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{ENV:SHRT_NEVERSET_QQ}", false);

    let output = invoke_shim(tmp.path(), "wt", &[], &[]);
    assert_eq!(output.status.code(), Some(78));
}

#[test]
fn target_not_found_exits_127_with_shell_hint() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1}", false);
    write_sidecar(
        tmp.path(),
        "wt",
        "target = \"definitelynotacommand_xyz_42\"\ntemplate = \"\"\nversion = 1\n",
    );

    let output = invoke_shim(tmp.path(), "wt", &[], &[]);
    assert_eq!(output.status.code(), Some(127));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--shell"),
        "expected --shell hint in stderr, got: {}",
        stderr
    );
}

#[test]
fn missing_sidecar_exits_66() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1}", false);
    std::fs::remove_file(tmp.path().join("wt.shrt")).unwrap();

    let output = invoke_shim(tmp.path(), "wt", &[], &[]);
    assert_eq!(output.status.code(), Some(66));
}

#[test]
fn bad_sidecar_exits_78() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1}", false);
    write_sidecar(tmp.path(), "wt", "not even close to toml = = =");

    let output = invoke_shim(tmp.path(), "wt", &[], &[]);
    assert_eq!(output.status.code(), Some(78));
}

#[test]
fn version_mismatch_exits_78() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1}", false);
    let stub = stub_path().to_string_lossy().replace('\\', "\\\\");
    write_sidecar(
        tmp.path(),
        "wt",
        &format!(
            "target = \"{}\"\ntemplate = \"\"\nversion = 99\n",
            stub
        ),
    );

    let output = invoke_shim(tmp.path(), "wt", &[], &[]);
    assert_eq!(output.status.code(), Some(78));
}

#[test]
fn shim_renamed_off_exe_exits_78() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1}", false);
    let exe = tmp.path().join("wt.exe");
    let bin = tmp.path().join("wt.bin");
    std::fs::rename(&exe, &bin).unwrap();

    let output = std::process::Command::new(&bin).output().unwrap();
    assert_eq!(output.status.code(), Some(78));
}
