Parent slice: [shrt — runner](../slices/runner.md)
Depends on: 02, 03, 04

# Task 05 — runner main orchestration

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Wire the runner's modules together in `crates/shrt-runner/src/main.rs`. After this task, `shrt-runner.exe` is a self-sufficient executable that obeys all locked exit codes.

## Tasks
- [x] Replace the stub in `crates/shrt-runner/src/main.rs` with the full orchestration per `slices/runner.md` §5: `current_exe()` failure → exit 1; `derive_sidecar_path` mapping; `sidecar::parse` mapping (66 / 78); empty `target` → exit 78; collect `args_os().skip(1)`; call `substitute::substitute(&cfg.template, &user_args, &|n| std::env::var_os(n))` mapping errors per `slices/substitution-engine.md` §4 table.
- [x] In `crates/shrt-runner/src/main.rs` implement the spawn branch on `cfg.shell`: false → call `path::resolve_target(&cfg.target)` (78/127), build `Command::new(resolved)` with `.args(argv::tokenize(&substituted))`; true → `Command::new("cmd")` with `.args(["/c", &format!("{} {}", cfg.target, substituted)])`. Apply `current_dir` only if `expand_cwd` returned `Some`. Set all three stdio streams to `Stdio::inherit()`.
- [x] In `crates/shrt-runner/src/main.rs` propagate the child exit code with `std::process::exit(status.code().unwrap_or(1))` per `slices/runner.md` §3 Decision 8 (full i32, not the truncated `ExitCode::from(... as u8)` from spec pseudocode). Spawn failure after target resolution → exit 1.
- [x] All stderr error messages use the format `shrt-runner: <sidecar-abs-path>: <reason>` (or runner self-error when no sidecar yet) per `slices/sidecar-format.md` §4 + `slices/substitution-engine.md` §3 Decision 18. No additional logging beyond the documented messages.
- [x] Confirm `mod sidecar; mod substitute; mod argv; mod path;` are declared in `crates/shrt-runner/src/main.rs`.

## Acceptance criteria
- [x] `cargo build -p shrt-runner --release` exits 0.
- [x] `cargo test -p shrt-runner` passes (every prior unit test still runs).
- [x] `./target/release/shrt-runner.exe; [ $? -eq 66 ]` (PowerShell: `& target/release/shrt-runner.exe; if ($LASTEXITCODE -ne 66) { exit 1 }`) — invoking the runner directly without a sidecar exits 66 because the derived `target/release/shrt-runner.shrt` does not exist.
- [x] Stripped binary size: `(Get-Item target/release/shrt-runner.exe).Length -lt 300KB` should be true on a release build (manual size check — note: this is a budget guidance from spec §6.1, not a hard test bar; the perf assertion lives in task 14 and is `#[ignore]`-tagged).

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
