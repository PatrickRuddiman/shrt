mod argv;
mod path;
mod sidecar;
mod substitute;

use std::ffi::OsString;
use std::process::{Command, Stdio};

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("shrt-runner: cannot determine own path: {}", e);
            return 1;
        }
    };

    let sidecar_path = match sidecar::derive_sidecar_path(&exe) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("shrt-runner: {}: {}", exe.display(), e);
            return e.exit_code();
        }
    };

    let cfg = match sidecar::parse(&sidecar_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("shrt-runner: {}: {}", sidecar_path.display(), e);
            return e.exit_code();
        }
    };

    if cfg.target.is_empty() {
        eprintln!("shrt-runner: {}: target is empty", sidecar_path.display());
        return 78;
    }

    let user_args: Vec<OsString> = std::env::args_os().skip(1).collect();

    let substituted = match substitute::substitute(&cfg.template, &user_args, &|n| {
        std::env::var_os(n)
    }) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("shrt-runner: {}: {}", sidecar_path.display(), e);
            return e.exit_code();
        }
    };

    let cwd = match path::expand_cwd(&cfg.cwd) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("shrt-runner: {}: {}", sidecar_path.display(), e);
            return e.exit_code();
        }
    };

    let mut cmd = if cfg.shell {
        let mut c = Command::new("cmd");
        c.arg("/c");
        c.arg(format!("{} {}", cfg.target, substituted));
        c
    } else {
        let resolved = match path::resolve_target(&cfg.target) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("shrt-runner: {}: {}", sidecar_path.display(), e);
                return e.exit_code();
            }
        };
        let mut c = Command::new(resolved);
        c.args(argv::tokenize(&substituted));
        c
    };

    if let Some(d) = cwd {
        cmd.current_dir(d);
    }

    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!(
                "shrt-runner: {}: spawn failed: {}",
                sidecar_path.display(),
                e
            );
            1
        }
    }
}
