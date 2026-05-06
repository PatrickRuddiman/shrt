Parent slice: [shrt — cli-surface](../slices/cli-surface.md), [shrt — shim-management](../slices/shim-management.md)
Depends on: 01, 07, 08

# Task 10 — shrt add + remove commands (atomic pair-write/delete) + test helpers

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Implement `shrt add` (atomic pair-write of `.shrt`+`.exe`) and `shrt remove` (pair-delete). Provide the shared integration-test helpers in `tests/common/mod.rs`.

## Tasks
- [x] In `crates/shrt/src/shim.rs` implement `pub fn add(ctx: &Ctx, name: &str, cfg: &SidecarConfig, force: bool) -> anyhow::Result<()>` per `slices/shim-management.md` §4 add-pair-write sequence: `fs::create_dir_all(&ctx.shim_dir)`; collision check on both `<name>.exe` and `<name>.shrt` (skip if `force`); write `<name>.shrt.tmp` via `config::write_sidecar`; write `<name>.exe.tmp` via `fs::write` of `RUNNER_BYTES`; `fs::rename` `.shrt.tmp` → `.shrt`; `fs::rename` `.exe.tmp` → `.exe`. On any failure during the four steps, best-effort `remove_file` of any `.tmp` left behind and any successfully-renamed `.shrt` if `.exe` rename failed.
- [x] Map errors emitted by `add` so the CLI returns exit 73 on collision-without-force or unwritable shim dir, exit 64 on a `write_sidecar` sanitization failure, exit 1 otherwise. Use a custom error type with explicit code-mapping helper rather than ad-hoc string matching.
- [x] In `crates/shrt/src/shim.rs` implement `pub fn remove(ctx: &Ctx, name: &str) -> anyhow::Result<()>` per `slices/shim-management.md` §4: delete `<name>.exe`, then `<name>.shrt`. If both files are absent at the start, return an error mapped to exit 66.
- [x] Replace the stub in `crates/shrt/src/commands/add.rs`: validate `name` via `cli::validate_name`; compute `(target, body) = cli::parse_template_and_target(&template, override_)`; build `SidecarConfig { target, template: body, shell, cwd, description, created: Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)), version: 1 }`; call `shim::add(ctx, &name, &cfg, force)`. Silent on success.
- [x] Replace the stub in `crates/shrt/src/commands/remove.rs`: validate name; call `shim::remove`. Silent on success.
- [x] Create `crates/shrt/tests/common/mod.rs` per `slices/testing-harness.md` §4: `pub fn stub_path() -> &'static Path` runs `cargo build -p argv-stub` once via `OnceLock<PathBuf>` and returns the absolute path (probe both `target/debug/argv-stub.exe` and `target/release/argv-stub.exe` based on which exists most recently); `pub fn make_shim_dir() -> tempfile::TempDir`; `pub fn shrt(shim_dir: &Path) -> Command` returns `Command::new(env!("CARGO_BIN_EXE_shrt"))` with `--shim-dir=<dir>`; `pub fn add_stub_shim(shim_dir, name, template, shell, env_pairs)` constructs and runs the add; `pub fn invoke_shim(shim_dir, name, args, env_pairs) -> std::process::Output`.
- [x] Create `crates/shrt/tests/add_remove.rs` covering: `add_creates_pair` (both files exist after add); `add_collision_without_force_fails` (second add → exit 73); `add_force_overwrites`; `remove_deletes_pair`; `remove_missing_shim_exits_66`; `name_validation_rejects_path_separator` (`shrt add foo/bar "..."` → exit 64).

## Acceptance criteria
- [x] `cargo test -p shrt --test add_remove` passes.
- [x] `test -f crates/shrt/tests/common/mod.rs && test -f crates/shrt/tests/add_remove.rs`.
- [x] `grep -q 'pub fn add' crates/shrt/src/shim.rs && grep -q 'pub fn remove' crates/shrt/src/shim.rs`.
- [x] `grep -q 'pub fn stub_path' crates/shrt/tests/common/mod.rs && grep -q 'pub fn invoke_shim' crates/shrt/tests/common/mod.rs`.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
