Parent slice: [shrt — runner](../slices/runner.md)
Depends on: 01

# Task 04 — runner path module (PATH+PATHEXT search + cwd expansion)

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Implement `crates/shrt-runner/src/path.rs` with target resolution against `PATH`+`PATHEXT` and `cwd` expansion of `~` and `${VAR}`.

## Tasks
- [x] Create `crates/shrt-runner/src/path.rs` with `pub enum PathError` covering `Empty` (exit 78), `EnvUnset(String)` (exit 78), `EnvNotUtf8(String)` (exit 78), `NotFound(String)` (exit 127), `CwdMissing(PathBuf)` (exit 78). Provide a method returning the spec exit code.
- [x] In `crates/shrt-runner/src/path.rs` implement `pub fn resolve_target(target: &str) -> Result<PathBuf, PathError>` per `slices/runner.md` §4: empty → `Empty`; contains `/` or `\\` → treat as path (absolute used as-is, relative joined with current dir, must exist); otherwise PATH+PATHEXT search.
- [x] PATH+PATHEXT search rules in `crates/shrt-runner/src/path.rs`: read `PATHEXT` env (default `.COM;.EXE;.BAT;.CMD` if unset); split on `;` lowercasing for matching; if `target` already ends in a recognized extension probe as-is (extension list `[""]`); else for each PATH directory in order, for each extension in order, probe `<dir>\<target><ext>`; first hit wins; none → `NotFound`.
- [x] In `crates/shrt-runner/src/path.rs` implement `pub fn expand_cwd(cwd: &str) -> Result<Option<PathBuf>, PathError>` per `slices/runner.md` §4: empty → `Ok(None)`; leading `~` followed by end or `\\`/`/` → replace with `USERPROFILE` env value (unset → `EnvUnset`); walk for `${VAR}` and replace each (unset → `EnvUnset`, non-utf8 → `EnvNotUtf8`, unmatched `${` → `EnvUnset` with partial name); verify result exists and is a directory else `CwdMissing`.
- [x] Add `#[cfg(test)] mod tests` in `crates/shrt-runner/src/path.rs` covering: `resolve_target("cmd")` finds `cmd.exe` via system PATH+PATHEXT; `resolve_target("findstr")` works; absolute path passthrough; bare nonexistent target → `NotFound`; `expand_cwd("")` → `Ok(None)`; `~` expansion (set `USERPROFILE` per-test via `Command::env`-equivalent injection on a wrapper helper, or stash the prior value and restore — keep tests serial-friendly); `${SOMEVAR}` expansion with a test-local env injection; unset variable → `EnvUnset`.
- [x] Add `mod path;` to `crates/shrt-runner/src/main.rs`.

## Acceptance criteria
- [x] `cargo build -p shrt-runner` exits 0.
- [x] `cargo test -p shrt-runner path::tests` passes.
- [x] `test -f crates/shrt-runner/src/path.rs`.
- [x] `grep -q 'pub fn resolve_target' crates/shrt-runner/src/path.rs && grep -q 'pub fn expand_cwd' crates/shrt-runner/src/path.rs`.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
