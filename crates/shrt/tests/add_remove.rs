mod common;
use common::*;

#[test]
fn add_creates_pair() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1}", false);
    assert!(tmp.path().join("wt.exe").is_file());
    assert!(tmp.path().join("wt.shrt").is_file());
}

#[test]
fn add_collision_without_force_exits_73() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1}", false);

    let template = stub_template("{1}");
    let output = shrt(tmp.path())
        .arg("add")
        .arg("wt")
        .arg(&template)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(73));
}

#[test]
fn add_force_overwrites() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1}", false);

    let template = stub_template("{1}");
    let output = shrt(tmp.path())
        .arg("add")
        .arg("wt")
        .arg(&template)
        .arg("--force")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "force-overwrite failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn add_writes_atomically_no_tmp_files_remain() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1}", false);
    assert!(!tmp.path().join("wt.exe.tmp").exists());
    assert!(!tmp.path().join("wt.shrt.tmp").exists());
}

#[test]
fn add_with_target_override() {
    let tmp = make_shim_dir();
    let stub = stub_path().to_string_lossy().into_owned();

    let output = shrt(tmp.path())
        .arg("add")
        .arg("wt")
        .arg("{1}")
        .arg("--target")
        .arg(&stub)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(tmp.path().join("wt.exe").is_file());
}

#[test]
fn remove_deletes_pair() {
    let tmp = make_shim_dir();
    add_stub_shim(tmp.path(), "wt", "{1}", false);

    let status = shrt(tmp.path())
        .arg("remove")
        .arg("wt")
        .status()
        .unwrap();
    assert!(status.success());

    assert!(!tmp.path().join("wt.exe").exists());
    assert!(!tmp.path().join("wt.shrt").exists());
}

#[test]
fn remove_missing_shim_exits_66() {
    let tmp = make_shim_dir();

    let output = shrt(tmp.path())
        .arg("remove")
        .arg("doesnotexist")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(66));
}

#[test]
fn name_validation_rejects_path_separator_exits_64() {
    let tmp = make_shim_dir();
    let template = stub_template("");

    let output = shrt(tmp.path())
        .arg("add")
        .arg("foo/bar")
        .arg(&template)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(64));
}

#[test]
fn name_validation_rejects_reserved_device_exits_64() {
    let tmp = make_shim_dir();
    let template = stub_template("");

    let output = shrt(tmp.path())
        .arg("add")
        .arg("con")
        .arg(&template)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(64));
}

#[test]
fn add_warns_when_target_not_on_path() {
    let tmp = make_shim_dir();

    let output = shrt(tmp.path())
        .arg("add")
        .arg("ghost")
        .arg("definitelynotacommand_xyz_42 {1}")
        .output()
        .unwrap();
    assert!(output.status.success(), "add should succeed despite bad target");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found on PATH"),
        "stderr missing target warning: {}",
        stderr
    );
    assert!(
        stderr.contains("--shell"),
        "stderr missing --shell hint: {}",
        stderr
    );
}

#[test]
fn add_with_shell_flag_skips_target_warning() {
    let tmp = make_shim_dir();

    let output = shrt(tmp.path())
        .arg("add")
        .arg("piper")
        .arg("echo hello")
        .arg("--shell")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("not found on PATH"),
        "unexpected target warning under --shell: {}",
        stderr
    );
}
