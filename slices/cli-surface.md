Parent spec: [shrt — Specification](../spec.md)

# shrt — cli-surface

## §1 Summary
The `shrt` binary's user-facing surface: clap argument grammar, global flags, command dispatch, name validation, output formatting (text + JSON), and the exit-code mapping for every command. Owns argument parsing and how results are presented; delegates filesystem mechanics (sidecar read/write, runner-bytes copy, ACLs, PATH detection's underlying lookup) to `shim-management`.

## §2 Codebase reconnaissance
> Greenfield: no existing system to reconcile. Decisions below are unconstrained.

Sibling slices:
- `slices/build-pipeline.md` — exposes `crates/shrt/src/shim.rs::RUNNER_BYTES`, consumed by `add` and `sync`.
- `slices/sidecar-format.md` — locks the TOML schema this slice's commands serialize.
- `slices/runner.md` — defines the runner-side exit codes; CLI-side exit codes (this slice) are aligned with spec §5.3 and do not duplicate runner exit codes.

## §3 Decisions
1. **clap configuration style.** Options: derive / builder / clap-attributes. **Chosen:** derive (`#[derive(Parser, Subcommand)]`). Rationale: spec §7.1 names `clap` with `derive`; lowest line-count for the command count.
2. **Source layout.** Options: one file with all commands / per-command file / a sub-crate. **Chosen:** `cli.rs` for the top-level enum + `commands/{init,add,remove,list,show,sync,path,doctor}.rs`, each exposing a `run(ctx, args) -> anyhow::Result<i32>`. Rationale: matches spec §2.1 verbatim; each command stays under ~80 lines.
3. **Global flag placement.** Options: per-subcommand / root-only / both. **Chosen:** root-only with `#[arg(global = true)]` on the three flags. Rationale: cleaner UX (`shrt --json list` works); subcommands don't have to redeclare.
4. **`--shim-dir` resolution chain.** Options: code-side fallback / clap-managed env. **Chosen:** clap-managed: `#[arg(long = "shim-dir", env = "SHRT_DIR")]` resolves explicit flag > `SHRT_DIR` > the computed default. Rationale: the precedence is clap's standard; no hand-rolled chain.
5. **Default shim-dir.** Options: `~/.shrt/bin` via `directories` crate / hard-coded `%USERPROFILE%\.shrt\bin`. **Chosen:** `directories::UserDirs::new()?.home_dir().join(".shrt").join("bin")`. Rationale: spec §7.1 lists `directories`; portable to macOS/Linux when §9 is unblocked.
6. **`--quiet` semantics.** Options: silence everything / silence non-errors / silence everything except JSON. **Chosen:** suppresses non-error stdout; errors still go to stderr; `--json` output still prints (machine consumers want it regardless of `--quiet`). Rationale: matches POSIX-tool conventions.
7. **`--json` scope.** Options: every command / informational commands only. **Chosen:** five commands — `init`, `list`, `show`, `path`, `doctor`. `add`/`remove`/`sync` are side-effecting; success is silent (exit 0) and `--json` is a no-op for them. Rationale: those three have nothing meaningful to print; an empty `{}` would be noise.
8. **`shrt add` argument grammar.** Options: combined string / separate fields / template-with-extracted-target. **Chosen:** two positionals `<name> <template>` plus optional `--target`. Default extraction: split `<template>` on first ASCII whitespace; head = target, tail = template body. With `--target=X`, `<template>` is the body verbatim. Rationale: matches both invocation forms in spec §5.1.
9. **Name validation rules.** Options: blocklist / allowlist. **Chosen:** allowlist `[A-Za-z0-9._-]+`, 1–64 chars, no leading/trailing whitespace, no `..` consecutively, not a Windows reserved device name (`con`, `prn`, `aux`, `nul`, `com1`–`com9`, `lpt1`–`lpt9`, case-insensitive). Rationale: simpler to reason about than blocklisting metacharacters; covers the §10.2 hardening requirement.
10. **`shrt remove` on missing shim.** Options: exit 0 with warning / exit 1 / exit 66. **Chosen:** exit 66. Rationale: spec §5.3 maps "cannot open input" to 66; an absent shim is missing input.
11. **`shrt list` default format.** Options: aligned table / one shim per line in compact form. **Chosen:** aligned `<name>  <target>` two-column table, lexicographic by name. `--verbose` adds `template`, `cwd`, `description`, `created`. Rationale: spec §5.1 explicitly distinguishes default vs verbose.
12. **`shrt show` default format.** Options: parsed structured render / file contents verbatim. **Chosen:** print the `.shrt` file's exact bytes. `--json` prints the parsed config plus the absolute path. Rationale: spec §5.1 says "Print the contents of <name>.shrt to stdout."
13. **`shrt path` default format.** Options: one-line absolute path / structured. **Chosen:** one-line absolute path; `--json` adds an `on_path` boolean. Rationale: spec §5.1 example uses it as a substring in shell init.
14. **`shrt init` behavior.** Options: silent if exists / idempotent with status / always print instructions. **Chosen:** idempotent — creates the directory if absent, prints PowerShell, cmd, and Git Bash one-liners for adding it to PATH only when not already on PATH. Rationale: matches spec §5.1 ("Idempotent. If already initialized, print path and PATH status").
15. **PATH detection method.** Options: read PATH env / Windows registry. **Chosen:** read `PATH` env var, split on `;`, case-insensitive normalized comparison. Rationale: env var is what the current process sees; matches what every shell exposes; registry probe would lie about an unrestarted session.
16. **`shrt sync` filtering rule.** Options: rewrite every `*.exe` / pair-by-sidecar. **Chosen:** for each `<shim-dir>/<name>.shrt`, rewrite `<shim-dir>/<name>.exe` with `RUNNER_BYTES`. `*.exe` files without a matching sidecar are left untouched (they aren't ours). Rationale: avoids mass-corrupting unrelated tools that happen to share the directory.
17. **`shrt doctor` checks.** **Chosen:** four checks per spec §5.1: (a) shim-dir on PATH? (b) every `*.exe` bytes equal to `RUNNER_BYTES`? (c) every `*.shrt` parses successfully? (d) every `target` resolves on PATH? Rationale: literally what spec lists.
18. **Doctor runner-version check method.** Options: byte-equality / version string in runner / file mtime. **Chosen:** byte-equality vs. `RUNNER_BYTES`. Rationale: zero runner overhead (runner stays std-only and lean); byte-equal is the strongest correctness signal anyway.
19. **`shrt edit`.** **Chosen:** not implemented in v0.1 per spec §13. | Defer.
20. **Tab-completion command.** **Chosen:** not implemented in v0.1. Rationale: spec §12 question 4 floats it but §13 acceptance does not require it; defer to v0.2.
21. **Color / TTY detection.** **Chosen:** none in v0.1. Plain ASCII output. Rationale: simplicity; no new deps; v0.2 can add `clap`'s built-in color or `anstyle`.
22. **Error reporting.** Options: anyhow chain to stderr / single-line / serde-json error. **Chosen:** `anyhow::Result` propagated to `main()`; stderr prints the full error chain (`{:#}`) unless `--quiet`, in which case only the top message. Exit code mapped per spec §5.3. Rationale: matches spec §7.1 (`anyhow` for the CLI).
23. **CLI context struct.** **Chosen:** `Ctx { shim_dir: PathBuf, quiet: bool, json: bool, runner_bytes: &'static [u8] }` constructed in `main()` and passed by `&Ctx` to each `commands::*::run`. Rationale: a single value carries every cross-cutting concern; nothing else is shared between commands.

## §4 Contracts & shapes

**Top-level CLI grammar (clap derive):**
```
shrt [GLOBAL OPTIONS] <COMMAND>

GLOBAL OPTIONS
  --shim-dir <PATH>   Override default shim directory; env: SHRT_DIR
  --quiet             Suppress non-error stdout; JSON output unaffected
  --json              Machine-readable output where applicable
  -h, --help
  -V, --version

COMMANDS
  init                                      Create shim dir; print PATH status
  add <NAME> <TEMPLATE> [FLAGS]             Create a new shim
  remove <NAME>                             Delete a shim
  list [--verbose]                          Print all shims
  show <NAME>                               Print sidecar contents
  sync                                      Refresh embedded runner in every shim
  path                                      Print shim directory path
  doctor                                    Run diagnostic checks
```

**`shrt add` flags:** `--target <CMD>`, `--shell`, `--cwd <PATH>`, `--desc <TEXT>`, `--force`.

**Name validation regex:** `^[A-Za-z0-9._-]{1,64}$` AND not matching `..` AND not matching `^(con|prn|aux|nul|com[1-9]|lpt[1-9])$` (case-insensitive). Failure → exit 64 with `shrt: invalid shim name '<input>'`.

**JSON shapes:**

| Command | Shape |
|---|---|
| `init --json` | `{"shim_dir": "<abs-path>", "created": <bool>, "on_path": <bool>}` |
| `list --json` | `[{"name", "target", "template", "shell", "cwd", "description", "created"}, ...]` (sorted by `name`) |
| `show <name> --json` | `{"path": "<abs-path>", "config": {<full schema from sidecar-format §4>}}` |
| `path --json` | `{"path": "<abs-path>", "on_path": <bool>}` |
| `doctor --json` | `{"summary": "ok"|"warn"|"fail", "checks": [{"name", "status": "ok"|"warn"|"fail", "message"}, ...]}` |

`add`, `remove`, `sync` produce no stdout on success regardless of `--json`. Errors → stderr text (not JSON).

**Exit-code mapping (CLI):**

| Exit | When |
|---|---|
| 0 | Success. |
| 1 | Generic error (default for `anyhow` errors not classified below). |
| 64 | Usage error: bad/missing arg, name validation failure. |
| 66 | Missing input: `remove`/`show` on a non-existent shim. |
| 73 | Cannot create output: shim-dir not writable (permission, read-only volume). |
| 78 | Config error: a `*.shrt` failed to parse during `list`/`show`/`doctor`. |

**Module boundaries:**

| Module | Owns |
|---|---|
| `crates/shrt/src/main.rs` | Build `Ctx`, dispatch via clap, map errors to exit codes. |
| `crates/shrt/src/cli.rs` | clap `Parser`/`Subcommand` definitions + `validate_name()` + `parse_template_and_target()`. |
| `crates/shrt/src/commands/<cmd>.rs` | One per command; calls into `shim-management`. |
| `crates/shrt/src/shim.rs` | `RUNNER_BYTES` (from `build-pipeline`). |
| (delegated) | Sidecar read/write, shim-dir creation, PATH detection underlying lookup, sync mechanics, doctor's per-check logic — all in `shim-management`. |

**Where each command's logic lives:**

| Command | clap struct | Calls into `shim-management` |
|---|---|---|
| `init` | `InitArgs` (no fields) | `init(ctx) -> InitReport` |
| `add` | `AddArgs { name, template, target, shell, cwd, desc, force }` | `add(ctx, name, config, force)` |
| `remove` | `RemoveArgs { name }` | `remove(ctx, name)` |
| `list` | `ListArgs { verbose }` | `list(ctx) -> Vec<Entry>` |
| `show` | `ShowArgs { name }` | `show(ctx, name) -> Entry` |
| `sync` | `SyncArgs` (no fields) | `sync(ctx) -> SyncReport` |
| `path` | `PathArgs` (no fields) | `path(ctx) -> PathReport` |
| `doctor` | `DoctorArgs` (no fields) | `doctor(ctx) -> DoctorReport` |

The exact shape of `*Report` types is `shim-management`'s responsibility; this slice consumes them and renders.

## §5 Sequence

**`shrt list --json` (representative read-only flow):**
1. `main()` parses argv via clap. Builds `Ctx { shim_dir, quiet, json: true, runner_bytes }`.
2. `main()` dispatches to `commands::list::run(&ctx, ListArgs { verbose: false })`.
3. `list::run` calls `shim_management::list(&ctx)` → `Vec<Entry>` (or surface a sidecar parse error → exit 78).
4. `list::run` serializes the vec to JSON via `serde_json::to_string_pretty` and prints to stdout.
5. Returns `Ok(0)`.

**`shrt add wt "copilot -p '/worktree create...' --yolo"`:**
1. clap parses `name = "wt"`, `template = "copilot -p '/worktree create...' --yolo"`. `--target` absent.
2. `cli::validate_name("wt")` succeeds.
3. `cli::parse_template_and_target` splits on first whitespace → `target = "copilot"`, body = `"-p '/worktree create...' --yolo"`.
4. `commands::add::run` builds the in-memory config (`SidecarConfig`) and calls `shim_management::add(&ctx, "wt", config, force=false)`.
5. shim-management writes both `<shim-dir>/wt.exe` (from `RUNNER_BYTES`) and `<shim-dir>/wt.shrt`.
6. On success: silent, exit 0. On collision without `--force`: exit 73 with a clear message.

**`shrt doctor`:**
1. `commands::doctor::run` calls `shim_management::doctor(&ctx)` → `DoctorReport`.
2. If `--json`: serialize and print.
3. Else: render an aligned per-check list to stdout (or stderr for `fail`).
4. Exit code: 0 if all `ok`, 1 if any `fail`, 0 if only `warn`.

## §6 Out of scope
- Filesystem operations (sidecar read/write, atomicity, ACLs, shim-dir mkdir, byte-comparison loop in sync, target-resolution in doctor). All `shim-management`.
- The exact shape of `Entry`, `InitReport`, `SyncReport`, `DoctorReport`. Defined in `shim-management`.
- `shrt edit`. Spec §13 v0.2.
- Shell completion script generation (`shrt completion <shell>` or written by `init`). Spec §12 question 4; defer to v0.2.
- TTY/color/theming. Defer to v0.2.
- CLI-side template syntax validation at `add` time. Locked out by `substitution-engine` §3 Decision 11; the runner discovers template parse errors at first invocation.

> If the parent spec is ambiguous on anything this slice depends on, stop and update the spec. Do not invent behavior here.
