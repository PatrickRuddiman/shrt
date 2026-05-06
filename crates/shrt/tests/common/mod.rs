use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

static STUB: OnceLock<PathBuf> = OnceLock::new();

pub fn stub_path() -> &'static Path {
    let pb = STUB.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root from crates/shrt");

        let stub_bin = workspace_root
            .join("target")
            .join("debug")
            .join("argv-stub.exe");

        // Always invoke cargo build — cargo's incremental compilation
        // makes this a near-no-op when sources are unchanged, and avoids
        // serving a stale binary if argv-stub source changes.
        let status = Command::new(&cargo)
            .arg("build")
            .arg("-p")
            .arg("argv-stub")
            .current_dir(workspace_root)
            .status()
            .expect("failed to spawn cargo to build argv-stub");
        assert!(status.success(), "argv-stub build failed");

        assert!(
            stub_bin.exists(),
            "argv-stub.exe still missing at {}",
            stub_bin.display()
        );
        stub_bin
    });
    pb.as_path()
}

pub fn make_shim_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("creating tempdir")
}

pub fn shrt(shim_dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_shrt"));
    cmd.arg("--shim-dir").arg(shim_dir);
    cmd
}

pub fn stub_template(extra: &str) -> String {
    let stub = stub_path().to_string_lossy().into_owned();
    if extra.is_empty() {
        stub
    } else {
        format!("{} {}", stub, extra)
    }
}

pub fn add_stub_shim(
    shim_dir: &Path,
    name: &str,
    template_body: &str,
    shell: bool,
) {
    let template = stub_template(template_body);
    let mut cmd = shrt(shim_dir);
    cmd.arg("add").arg(name).arg(&template);
    if shell {
        cmd.arg("--shell");
    }
    let output = cmd.output().expect("running shrt add");
    assert!(
        output.status.success(),
        "shrt add failed (exit={:?}): stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn invoke_shim(
    shim_dir: &Path,
    name: &str,
    args: &[&str],
    envs: &[(&str, &str)],
) -> std::process::Output {
    let exe = shim_dir.join(format!("{}.exe", name));
    let mut cmd = Command::new(&exe);
    cmd.args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("running shim")
}
