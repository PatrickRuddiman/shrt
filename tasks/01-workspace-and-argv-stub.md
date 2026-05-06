Parent slice: [shrt — distribution](../slices/distribution.md), [shrt — testing-harness](../slices/testing-harness.md), [shrt — build-pipeline](../slices/build-pipeline.md)
Depends on: none

# Task 01 — workspace skeleton + argv-stub crate

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Establish the Cargo workspace (manifest, profile, toolchain pin, licenses, gitignore) and the `argv-stub` test stub crate. After this task, `cargo test --workspace` runs and at least one binary builds.

## Tasks
- [x] Create `Cargo.toml` (repo root): `[workspace]` with `members = ["crates/*"]` and `resolver = "2"`. `[workspace.package]` with `version = "0.1.0"`, `edition = "2021"`, `authors = ["Patrick Ruddiman"]`, `license = "MIT OR Apache-2.0"`, `repository = "https://github.com/PatrickRuddiman/shrt"`, `homepage` (same), `description = "Parameterized command shortcuts for Windows."`, `keywords` and `categories` per `slices/distribution.md` §4.
- [x] In the same `Cargo.toml`, add `[profile.release]` with `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = "symbols"` per `slices/build-pipeline.md` §3 Decision 6.
- [x] Create `rust-toolchain.toml` (repo root) with `[toolchain]` `channel = "stable"`.
- [x] Create `LICENSE-MIT` and `LICENSE-APACHE` (repo root) with the standard texts.
- [x] Create `.gitignore` (repo root) listing at least `target/` and `crates/shrt/runner-src/` (the publish-time bundle directory must not be committed per `slices/distribution.md` §3 Decision 2).
- [x] Create `crates/argv-stub/Cargo.toml`: `[package] name = "argv-stub"`, `version.workspace = true`, `edition.workspace = true`, `license.workspace = true`, `publish = false`. `[[bin]] name = "argv-stub"`. No `[dependencies]`.
- [x] Create `crates/argv-stub/src/main.rs`: read `EXIT_CODE` env (parse i32, default 0); read `READ_STDIN` env, when `1` slurp stdin to a `String`; emit JSON with `argv` from `args().skip(1)` and optional `stdin` field; hand-roll the JSON output (no `serde_json`); call `std::process::exit(code)`.
- [x] Add `#[cfg(test)] mod tests` in `crates/argv-stub/src/main.rs` with at least: a JSON-escape test for special characters (`"`, `\`, `\n`), a test that empty args produces `{"argv":[]}`.

## Acceptance criteria
- [x] `cargo build --workspace --release` exits 0.
- [x] `cargo test --workspace` passes (argv-stub unit tests run).
- [x] `test -f Cargo.toml && test -f rust-toolchain.toml && test -f LICENSE-MIT && test -f LICENSE-APACHE && test -f .gitignore && test -f crates/argv-stub/Cargo.toml && test -f crates/argv-stub/src/main.rs`.
- [x] `grep -q 'opt-level = "z"' Cargo.toml`.
- [x] `grep -q 'panic = "abort"' Cargo.toml`.
- [x] `grep -q 'publish = false' crates/argv-stub/Cargo.toml`.
- [x] `./target/release/argv-stub.exe alpha "two words"` prints a single JSON line whose `argv` field equals `["alpha","two words"]` (verify with: `./target/release/argv-stub.exe alpha "two words" | grep -q '"argv":\["alpha","two words"\]'`).
- [x] `EXIT_CODE=7 ./target/release/argv-stub.exe; [ $? -eq 7 ]` (Unix-style exit-code check; on PowerShell: `$env:EXIT_CODE=7; ./target/release/argv-stub.exe; if ($LASTEXITCODE -ne 7) { exit 1 }`).

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
