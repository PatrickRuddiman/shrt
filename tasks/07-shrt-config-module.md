Parent slice: [shrt — shim-management](../slices/shim-management.md), [shrt — sidecar-format](../slices/sidecar-format.md)
Depends on: 06

# Task 07 — shrt config module (writer + reader)

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Implement `crates/shrt/src/config.rs` with the serde reader (full `toml` crate) and a hand-rolled basic-string-only writer with sanitization. Round-trip is bit-perfect across the type system.

## Tasks
- [x] Create `crates/shrt/src/config.rs` defining `pub struct SidecarConfig` with `#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]` matching `slices/sidecar-format.md` §4 schema: `target: String`, `template: String`, `shell: bool` (default false), `cwd: String` (default ""), `description: String` (default ""), `created: Option<String>` (default None), `version: u32` (default 1). Use `#[serde(default)]` on optional fields so omitted keys deserialize to defaults.
- [x] In `crates/shrt/src/config.rs` define `pub struct Entry { name: String, ..SidecarConfig fields }` (flatten via `#[serde(flatten)]` on a `config` field, OR duplicate fields plus a `From<(String, SidecarConfig)>` impl — pick whichever produces the JSON shape spec'd in `slices/cli-surface.md` §4 for `list --json`).
- [x] Implement `pub fn read_sidecar(path: &Path) -> anyhow::Result<SidecarConfig>` in `crates/shrt/src/config.rs` using `toml::from_str` after `fs::read_to_string`. Surface parse errors with file path context via `anyhow::Context`.
- [x] Implement `pub fn write_sidecar(path: &Path, cfg: &SidecarConfig) -> anyhow::Result<()>` in `crates/shrt/src/config.rs` per `slices/shim-management.md` §4 hand-rolled writer rules: UTF-8 + `\n` line endings + no BOM; always emit `target`, `template`, `version`; emit `shell = true` only when true; emit `cwd`/`description` only when non-empty; emit `created` only when `Some`; basic-string form `"..."` always with escapes for `"`, `\`, `\n`, `\t`; reject any byte < 0x20 except `\n`/`\t`; reject `\r`. Atomicity: write to `<path>.tmp`, then `fs::rename` to `<path>`.
- [x] Add a private helper `escape_basic(s: &str) -> Result<String>` in `crates/shrt/src/config.rs` that encodes per the rules above. Bytes 0x00–0x1F (except 0x09 and 0x0A) cause an error variant the caller maps to exit 64.
- [x] Add `#[cfg(test)] mod tests` in `crates/shrt/src/config.rs` covering: `write_sidecar` produces `"..."` form even when no escaping needed; `escape_basic` rejects `\u{0001}` in any string field; `escape_basic` correctly escapes `"`, `\`, embedded `\n`, embedded `\t`; `read_sidecar` accepts the writer's exact output; round-trip equality on a fully-populated `SidecarConfig`; default values applied when reading a minimal sidecar; `\r` in any input rejected.
- [x] Add `mod config;` to `crates/shrt/src/main.rs`.

## Acceptance criteria
- [x] `cargo test -p shrt config::tests` passes.
- [x] `test -f crates/shrt/src/config.rs`.
- [x] `grep -q 'pub fn read_sidecar' crates/shrt/src/config.rs && grep -q 'pub fn write_sidecar' crates/shrt/src/config.rs`.
- [x] `grep -q 'pub struct SidecarConfig' crates/shrt/src/config.rs`.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
