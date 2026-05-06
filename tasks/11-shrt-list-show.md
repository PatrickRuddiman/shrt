Parent slice: [shrt — cli-surface](../slices/cli-surface.md), [shrt — shim-management](../slices/shim-management.md)
Depends on: 10

# Task 11 — shrt list + show commands

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Implement `shrt list` (default + `--verbose` + `--json`) and `shrt show` (default raw-content + `--json` parsed shape).

## Tasks
- [x] In `crates/shrt/src/shim.rs` implement `pub fn list(ctx: &Ctx) -> anyhow::Result<Vec<Entry>>` per `slices/shim-management.md` §4: enumerate `<shim-dir>/*.shrt`; for each, read sidecar via `config::read_sidecar`; build `Entry`. Sort by name lexicographically. Sidecar parse errors propagate as `anyhow::Error` mapped to exit 78.
- [x] In `crates/shrt/src/shim.rs` implement `pub fn show(ctx: &Ctx, name: &str) -> anyhow::Result<(PathBuf, String, Entry)>` returning the absolute sidecar path, raw file content, and parsed Entry. Missing shim → error mapped to exit 66.
- [x] Replace the stub in `crates/shrt/src/commands/list.rs`: default mode prints aligned `<name>  <target>` two-column table sorted by name (compute max-name-width from the vec); `--verbose` adds template, cwd, description, created on subsequent lines per shim; `--json` prints `serde_json::to_string_pretty(&entries)`.
- [x] Replace the stub in `crates/shrt/src/commands/show.rs`: validate name; call `shim::show`. Default mode prints the raw content; `--json` mode prints `{"path": <abs>, "config": <Entry>}`.
- [x] Create `crates/shrt/tests/list_show.rs` covering: `list_empty` (zero shims → empty table or `[]`); `list_default_columns` (two shims listed alphabetically); `list_json_shape` (each entry has `name`/`target`/`template`/`shell`/`cwd`/`description`/`created`/`version`); `show_default_prints_raw_contents`; `show_json_includes_path_and_config`; `show_missing_exits_66`. Use `tests/common/mod.rs` helpers.

## Acceptance criteria
- [x] `cargo test -p shrt --test list_show` passes.
- [x] `test -f crates/shrt/tests/list_show.rs`.
- [x] `grep -q 'pub fn list' crates/shrt/src/shim.rs && grep -q 'pub fn show' crates/shrt/src/shim.rs`.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
