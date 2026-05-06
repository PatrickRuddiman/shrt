use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let target = env::var("TARGET").expect("TARGET");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let manifest = manifest_dir.join("../shrt-runner/Cargo.toml");
    if !manifest.exists() {
        panic!(
            "shrt-runner manifest not found at {} (workspace member is required)",
            manifest.display()
        );
    }

    let target_dir = out_dir.join("runner-target");

    let status = Command::new(&cargo)
        .arg("build")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--release")
        .arg("--target")
        .arg(&target)
        .arg("--target-dir")
        .arg(&target_dir)
        .status()
        .expect("failed to invoke cargo to build shrt-runner");

    if !status.success() {
        panic!("cargo build of shrt-runner failed with status {}", status);
    }

    let produced = target_dir
        .join(&target)
        .join("release")
        .join("shrt-runner.exe");
    let dest = out_dir.join("shrt-runner.exe");
    std::fs::copy(&produced, &dest).unwrap_or_else(|e| {
        panic!(
            "failed to copy {} to {}: {}",
            produced.display(),
            dest.display(),
            e
        )
    });

    println!("cargo:rerun-if-changed=../shrt-runner/src");
    println!("cargo:rerun-if-changed=../shrt-runner/Cargo.toml");
    println!("cargo:rerun-if-changed=../../Cargo.lock");
}
