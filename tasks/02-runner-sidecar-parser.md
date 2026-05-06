Parent slice: [shrt — sidecar-format](../slices/sidecar-format.md), [shrt — runner](../slices/runner.md)
Depends on: 01

# Task 02 — runner sidecar parser

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Bootstrap `crates/shrt-runner/` and implement the hand-rolled, std-only TOML reader that consumes the locked sidecar schema, plus the `.exe` → `.shrt` filename derivation.

## Tasks
- [x] Create `crates/shrt-runner/Cargo.toml`: `[package] name = "shrt-runner"`, version/edition/license/repository/description/authors inherited via `*.workspace = true`. No `[dependencies]`. `[[bin]] name = "shrt-runner"`. Declare a per-crate `[profile.release]` with the same five knobs as the workspace (per `slices/build-pipeline.md` §3 Decision 7 + `slices/distribution.md` §3 Decision 5; needed for the published-crate path that runs without a workspace).
- [x] Create `crates/shrt-runner/src/main.rs` as a stub: `fn main() {}`. Wiring lands in task 05.
- [x] Create `crates/shrt-runner/src/sidecar.rs` declaring `pub struct SidecarConfig { target, template, shell, cwd, description, created: Option<String>, version: u32 }` with `impl Default` matching `slices/sidecar-format.md` §4.
- [x] In `crates/shrt-runner/src/sidecar.rs` add `pub enum SidecarError` covering `NotFound` (exit 66), `Io` (exit 1), `Bom`, `BadEscape`, `BadValue`, `MissingRequired`, `WrongType`, `MultipleAssignments`, `BadVersion`, `BadShimSuffix` (all exit 78). Implement a method returning the spec exit code and a stderr `Display` matching `slices/sidecar-format.md` §4 message shape (`shrt-runner: <abs-path>: <reason>` or `... line <N>: <reason>`).
- [x] Implement `pub fn parse(path: &Path) -> Result<SidecarConfig, SidecarError>` in `crates/shrt-runner/src/sidecar.rs`: read bytes; reject leading BOM `EF BB BF`; UTF-8 decode (invalid → `BadValue`); walk lines splitting on `\n` after trimming optional trailing `\r`; skip blank/comment-only lines; trim leading whitespace; split on first unquoted `=`; multiple `=` outside strings → `MultipleAssignments`; parse value as basic-string (escapes only `\"` `\\` `\n` `\t`), bool (lowercase only), or non-negative decimal int; reject literal-string `'...'` and any `"""` form; honor schema (`target` and `template` required, others optional with defaults); unknown keys: warn to stderr `shrt-runner: <path>: ignoring unknown key '<name>'`; reject `version > 1` or `version <= 0` → `BadVersion`.
- [x] Add `pub fn derive_sidecar_path(exe: &Path) -> Result<PathBuf, SidecarError>` in `crates/shrt-runner/src/sidecar.rs`: require the path to end in `.exe` (case-insensitive); substitute `.shrt`; mismatch → `BadShimSuffix`.
- [x] Add `#[cfg(test)] mod tests` in `crates/shrt-runner/src/sidecar.rs` covering: round-trip of every field; BOM rejection; `\r\n` accepted; unknown key warn-and-continue; missing `target` → 78; `version = 99` → 78; literal-string form rejected; `derive_sidecar_path` substitutes case-insensitively.
- [x] Add `mod sidecar;` to `crates/shrt-runner/src/main.rs` so the module compiles.

## Acceptance criteria
- [x] `cargo build -p shrt-runner --release` exits 0.
- [x] `cargo test -p shrt-runner sidecar::tests` passes.
- [x] `test -f crates/shrt-runner/Cargo.toml && test -f crates/shrt-runner/src/main.rs && test -f crates/shrt-runner/src/sidecar.rs`.
- [x] `grep -q 'opt-level = "z"' crates/shrt-runner/Cargo.toml`.
- [x] `grep -q 'pub fn parse' crates/shrt-runner/src/sidecar.rs`.
- [x] `grep -q 'pub fn derive_sidecar_path' crates/shrt-runner/src/sidecar.rs`.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
