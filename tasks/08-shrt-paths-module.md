Parent slice: [shrt — shim-management](../slices/shim-management.md), [shrt — cli-surface](../slices/cli-surface.md)
Depends on: 06

# Task 08 — shrt paths module (shim_dir + is_on_path)

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Replace the stub `crates/shrt/src/paths.rs` with the full implementation: shim-dir resolution (override + `directories` default) and PATH membership detection.

## Tasks
- [ ] Update `crates/shrt/src/paths.rs` so `pub fn shim_dir(override_: Option<&Path>) -> anyhow::Result<PathBuf>` returns the override when `Some`, else `directories::UserDirs::new().context(...)?.home_dir().join(".shrt").join("bin")`. Surface a clear error if the home dir cannot be resolved.
- [ ] In `crates/shrt/src/paths.rs` add `pub fn is_on_path_in(path_var: &str, shim_dir: &Path) -> bool` (testable internal helper) and `pub fn is_on_path(shim_dir: &Path) -> bool` (calls `is_on_path_in` with `std::env::var("PATH").unwrap_or_default()`). Splitting rule: split on `;`; trim each entry; skip empty; canonicalize each entry via `dunce`-style logic (or just `PathBuf::from(...)` followed by `to_string_lossy()` lowercase comparison) since we only need string equality after normalization. Match must be case-insensitive; trailing `;` tolerated (empty-string entries skipped).
- [ ] Add `#[cfg(test)] mod tests` in `crates/shrt/src/paths.rs` covering: `is_on_path_in` returns true when shim_dir is in the PATH string (mixed case); returns false when absent; case-insensitive match; trailing `;` tolerated; empty PATH string returns false. Tests target `is_on_path_in` directly with synthetic strings to avoid mutating process-global env.

## Acceptance criteria
- [ ] `cargo test -p shrt paths::tests` passes.
- [ ] `cargo build -p shrt --release` exits 0.
- [ ] `grep -q 'pub fn shim_dir' crates/shrt/src/paths.rs && grep -q 'pub fn is_on_path' crates/shrt/src/paths.rs && grep -q 'pub fn is_on_path_in' crates/shrt/src/paths.rs`.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
