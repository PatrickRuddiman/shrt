Parent slice: [shrt — cli-surface](../slices/cli-surface.md), [shrt — shim-management](../slices/shim-management.md)
Depends on: 10

# Task 12 — shrt sync command (byte-equality + atomic rewrite)

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Implement `shrt sync` per `slices/shim-management.md` §3 Decision 11: rewrite each shim's `.exe` whose bytes differ from `RUNNER_BYTES`, leaving unrelated `.exe` files alone.

## Tasks
- [ ] In `crates/shrt/src/shim.rs` declare `pub struct SyncReport { updated: usize, total: usize, errors: Vec<(String, String)> }` with `#[derive(Serialize)]`.
- [ ] In `crates/shrt/src/shim.rs` implement `pub fn sync(ctx: &Ctx) -> anyhow::Result<SyncReport>` per `slices/shim-management.md` §4 sync rewrite step: enumerate `<shim-dir>/*.shrt`; for each, derive `<name>.exe`; if `.exe` is absent, push `(name, "missing exe")` into `errors` and continue; read the file fully and compare bytes to `ctx.runner_bytes`; if equal, `total += 1` and continue; else write `<name>.exe.tmp` with `RUNNER_BYTES` and `fs::rename` to `.exe`, increment `updated` and `total`. Per-shim I/O failures are captured into `errors` (`(name, message)`); the function does NOT short-circuit on first error.
- [ ] Replace the stub in `crates/shrt/src/commands/sync.rs`: call `shim::sync`. JSON mode prints the `SyncReport`. Text mode prints `updated: <n> / total: <n>` and one line per error. Always returns 0 (errors are reported, not promoted to a nonzero exit) — exception: if `errors.len() == total` (every shim failed), exit 1.
- [ ] Create `crates/shrt/tests/sync.rs` covering: `sync_skips_unchanged_shims` (after a fresh `add`, `updated == 0` and `total == 1`); `sync_restores_modified_shim_bytes` (overwrite `<name>.exe` with `b"junk"`, run sync, re-read, assert byte-equal to a freshly-added second shim's `.exe`); `sync_json_shape` (parse JSON, assert keys present); `sync_handles_missing_exe` (delete `<name>.exe`, run sync, assert `errors` contains an entry naming the shim).

## Acceptance criteria
- [ ] `cargo test -p shrt --test sync` passes.
- [ ] `test -f crates/shrt/tests/sync.rs`.
- [ ] `grep -q 'pub fn sync' crates/shrt/src/shim.rs && grep -q 'pub struct SyncReport' crates/shrt/src/shim.rs`.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
