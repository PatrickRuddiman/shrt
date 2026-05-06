Parent slice: [shrt — cli-surface](../slices/cli-surface.md), [shrt — shim-management](../slices/shim-management.md)
Depends on: 01, 07, 08

# Task 09 — shrt init + path commands

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Implement `shrt init` and `shrt path` end-to-end. Both are read-only-ish commands; `init` may create the shim dir. JSON output supported on both.

## Tasks
- [x] In `crates/shrt/src/shim.rs` declare the report types per `slices/shim-management.md` §4: `pub struct InitReport { shim_dir: PathBuf, created: bool, on_path: bool }` and `pub struct PathReport { path: PathBuf, on_path: bool }`. Add `#[derive(Serialize)]` on both so `--json` works (use `#[serde(rename_all = "snake_case")]`).
- [x] In `crates/shrt/src/shim.rs` implement `pub fn init(ctx: &Ctx) -> anyhow::Result<InitReport>` per `slices/shim-management.md` §3 Decision 6: if `ctx.shim_dir` does not exist, `fs::create_dir_all` it and set `created = true`; else `created = false`. Compute `on_path = paths::is_on_path(&ctx.shim_dir)`.
- [x] In `crates/shrt/src/shim.rs` implement `pub fn path_report(ctx: &Ctx) -> PathReport`.
- [x] Replace the stub in `crates/shrt/src/commands/init.rs` so `pub fn run(ctx: &Ctx) -> anyhow::Result<i32>` calls `shim::init`. JSON path: `serde_json::to_string_pretty(&report)?` then println. Text path: print `shim dir: <path>`, `created: <bool>`, `on PATH: <bool>`. When `!report.on_path && !ctx.quiet`, print three one-liners — PowerShell (`$env:PATH += ';' + (shrt path)` form), cmd.exe (`set PATH=%PATH%;...`), Git Bash (`export PATH="$PATH:..."`) — instructing the user to add the dir to PATH. Return 0.
- [x] Replace the stub in `crates/shrt/src/commands/path.rs`: text mode prints the absolute path on one line; JSON mode prints `PathReport`.
- [x] Add `mod shim;` to `crates/shrt/src/main.rs` if not already present.
- [x] Create `crates/shrt/tests/init_path.rs` integration test: build a tempdir, run `shrt init --shim-dir <temp> --json`, parse JSON via `serde_json::Value`, assert `created == true`, `shim_dir` matches, `on_path` is a bool. Run a second time and assert `created == false`. Run `shrt path --shim-dir <temp> --json` and assert the path field equals the tempdir.

## Acceptance criteria
- [x] `cargo test -p shrt --test init_path` passes.
- [x] `cargo build -p shrt --release` exits 0.
- [x] `test -f crates/shrt/tests/init_path.rs`.
- [x] `grep -q 'pub fn init' crates/shrt/src/shim.rs && grep -q 'pub fn path_report' crates/shrt/src/shim.rs`.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
