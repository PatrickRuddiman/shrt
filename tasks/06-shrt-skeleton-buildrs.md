Parent slice: [shrt — build-pipeline](../slices/build-pipeline.md), [shrt — cli-surface](../slices/cli-surface.md), [shrt — distribution](../slices/distribution.md)
Depends on: 01, 05

# Task 06 — shrt CLI skeleton + build.rs embedding

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Bootstrap `crates/shrt/` with the build.rs that embeds `shrt-runner.exe` bytes via `include_bytes!`, plus the clap argument grammar skeleton. After this task, `shrt --help` prints the full subcommand list (commands are stubs `unimplemented!()`).

## Tasks
- [x] Create `crates/shrt/Cargo.toml`: `[package] name = "shrt"`, version/edition/license/description/repository inherited via `*.workspace = true`. `[[bin]] name = "shrt"`. `[dependencies]`: `clap = { version = "4", features = ["derive", "env"] }`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `toml = "0.8"`, `directories = "5"`, `which = "6"`, `anyhow = "1"`, `chrono = { version = "0.4", default-features = false, features = ["clock"] }`. `[dev-dependencies]`: `tempfile = "3"`.
- [x] Create `crates/shrt/build.rs`: read `OUT_DIR`, `TARGET`, `CARGO`, `CARGO_MANIFEST_DIR` from env; resolve runner manifest at `<MANIFEST_DIR>/../shrt-runner/Cargo.toml`, panic if missing; spawn `${CARGO} build --manifest-path=<runner> --release --target=${TARGET} --target-dir=${OUT_DIR}/runner-target`; on nonzero exit, propagate stderr and panic; copy `${OUT_DIR}/runner-target/${TARGET}/release/shrt-runner.exe` to `${OUT_DIR}/shrt-runner.exe`; emit `cargo:rerun-if-changed=` for `../shrt-runner/src`, `../shrt-runner/Cargo.toml`, `../../Cargo.lock`.
- [x] Create `crates/shrt/src/shim.rs` with `pub const RUNNER_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shrt-runner.exe"));`. (High-level functions land in tasks 09–13.)
- [x] Create `crates/shrt/src/cli.rs` with the clap derive types per `slices/cli-surface.md` §4: `pub struct Cli` (with global `--shim-dir` carrying `env = "SHRT_DIR"`, `--quiet`, `--json`); `pub enum Commands` with variants `Init`, `Add(AddArgs)`, `Remove(RemoveArgs)`, `List(ListArgs)`, `Show(ShowArgs)`, `Sync`, `Path`, `Doctor`. Define `pub struct Ctx { shim_dir: PathBuf, quiet: bool, json: bool, runner_bytes: &'static [u8] }`. Implement `pub fn validate_name(name: &str) -> anyhow::Result<()>` per `slices/cli-surface.md` §4 (`^[A-Za-z0-9._-]{1,64}$`, no `..`, no Windows reserved device names). Implement `pub fn parse_template_and_target(template: &str, override_: Option<&str>) -> (String, String)` returning `(target, body)` — split on first ASCII whitespace when override is None; otherwise `(override.to_owned(), template.to_owned())`.
- [x] Create `crates/shrt/src/main.rs`: parse `Cli` via clap, build `Ctx { shim_dir: paths::shim_dir(...)?, quiet, json, runner_bytes: RUNNER_BYTES }` (paths module is task 08; for now stub `shim_dir` to the override + a hard-coded fallback if needed — but better: include `mod paths;` and stub `paths.rs` here returning override-or-`directories`-default; task 08 fleshes out tests). Dispatch each command to `commands::<name>::run(&ctx, args)`. Map `anyhow::Error` to exit codes per `slices/cli-surface.md` §4 (1/64/66/73/78). On error: print full chain to stderr unless `quiet` (then top message only).
- [x] Create stub files `crates/shrt/src/commands/{init,add,remove,list,show,sync,path,doctor}.rs` each with `pub fn run(_ctx: &crate::cli::Ctx, _args: ...) -> anyhow::Result<i32> { unimplemented!() }`. Add `mod commands;` and a `pub mod {init,add,remove,list,show,sync,path,doctor};` in `crates/shrt/src/commands/mod.rs`.
- [x] Create stub `crates/shrt/src/paths.rs` with `pub fn shim_dir(override_: Option<&Path>) -> anyhow::Result<PathBuf>` returning the override or the `directories::UserDirs` default. (Tests + `is_on_path` land in task 08.) Declare `mod paths;` in `main.rs`.
- [x] Add `#[cfg(test)] mod tests` in `crates/shrt/src/cli.rs` covering `validate_name` (allowlist accepts `wt`, rejects `foo/bar`, rejects `..`, rejects `con`, rejects 65-char strings, rejects empty) and `parse_template_and_target` (default extraction + override behavior).

## Acceptance criteria
- [x] `cargo build -p shrt --release` exits 0 (proves build.rs probes correctly via workspace context AND embeds runner bytes).
- [x] `cargo test -p shrt cli::tests` passes.
- [x] `./target/release/shrt.exe --help | grep -E '\b(init|add|remove|list|show|sync|path|doctor)\b' | wc -l` ≥ 8 (PowerShell equivalent: pipe through `Select-String` and assert count).
- [x] `test -f crates/shrt/Cargo.toml && test -f crates/shrt/build.rs && test -f crates/shrt/src/main.rs && test -f crates/shrt/src/cli.rs && test -f crates/shrt/src/shim.rs`.
- [x] `grep -q 'include_bytes!' crates/shrt/src/shim.rs && grep -q 'env!("OUT_DIR")' crates/shrt/src/shim.rs`.
- [x] `grep -q 'pub fn validate_name' crates/shrt/src/cli.rs && grep -q 'pub fn parse_template_and_target' crates/shrt/src/cli.rs`.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
