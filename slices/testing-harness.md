Parent spec: [shrt — Specification](../spec.md)

# shrt — testing-harness

## §1 Summary
Defines the test layout (unit + integration), the stub target binary used to capture argv, the helper that builds and locates it, the per-test shim-dir isolation pattern, and the enumerated test cases that must pass for v0.1 acceptance. Owns nothing about runtime behavior — only how it's verified.

## §2 Codebase reconnaissance
> Greenfield: no existing system to reconcile. Decisions below are unconstrained.

This slice integrates against every prior slice:
- `slices/sidecar-format.md` — every reader exit code (66, 78) is covered by a negative test.
- `slices/substitution-engine.md` — every placeholder form has a unit test; the missing-arg path (64) and missing-env path (78) have integration tests.
- `slices/runner.md` — exit codes 1, 64, 66, 78, 127 each have at least one integration test.
- `slices/cli-surface.md` — name validation, JSON shapes, exit codes mapped per spec §5.3.
- `slices/shim-management.md` — pair-write atomicity, sync byte-restoration, doctor's four checks.
- `slices/build-pipeline.md` — CI invocation `cargo test --workspace`.
- `slices/distribution.md` — release mode tests on tag only.

## §3 Decisions
1. **Test framework.** Options: stock `cargo test` / `cucumber-rs` / `assert_cmd`. **Chosen:** stock. Rationale: zero extra build-time deps; standard.
2. **Unit-test placement.** Options: `#[cfg(test)] mod tests` in-module / a sibling `tests/unit/` dir / per-crate `tests/`. **Chosen:** in-module per spec §11.1; one block per source file. Rationale: tests live next to the function; `cargo test` discovers automatically.
3. **Integration-test placement.** Options: monolithic / per-topic file / per-test file. **Chosen:** `crates/shrt/tests/{roundtrip,invocation,exit_codes,sync_doctor,name_validation,perf}.rs`, one file per topic, plus a `crates/shrt/tests/common/mod.rs` for shared helpers. Rationale: balances cohesion (related cases together) with isolation (cargo runs each file as a separate test binary).
4. **Stub target binary.** Options: `[[bin]]` in shrt with `required-features` / shared workspace member / a Rust file in `tests/`. **Chosen:** a separate workspace member `crates/argv-stub/` with `publish = false`. Rationale: `[[bin]]` would either ship the stub to end users or require feature-gating gymnastics; a sibling crate is cleanest.
5. **How tests find argv-stub.** Options: `escargot` crate / hardcoded `target/debug/argv-stub.exe` / build-and-cache helper. **Chosen:** `crates/shrt/tests/common/mod.rs::stub_path()` runs `cargo build -p argv-stub` once (cached in a `OnceLock<PathBuf>`) and returns the absolute path. Rationale: robust against `cargo test -p shrt` (which by default doesn't build sibling crates); no extra deps.
6. **How tests invoke `shrt` itself.** Options: hand-rolled `Command::new(env!("CARGO_BIN_EXE_shrt"))` / `assert_cmd`. **Chosen:** hand-rolled. Rationale: cargo sets `CARGO_BIN_EXE_shrt` automatically for integration tests in the shrt crate; an extra dep buys nothing.
7. **Per-test shim-dir isolation.** Options: shared dir + name-prefix / unique tempdir / global mutex. **Chosen:** unique tempdir via `tempfile::tempdir()`; tests pass it to shrt as `--shim-dir` or via `SHRT_DIR=<path>` env, never globally. Rationale: lets `cargo test` use full default parallelism.
8. **Dev-dependencies for `shrt` crate.** **Chosen:** `tempfile` + `serde_json`, that's it. Rationale: the JSON-output assertions need `serde_json`; tempdirs need `tempfile`; everything else fits in `std`.
9. **argv-stub behavior.** **Chosen:** prints `{"argv": [...]}` (excluding argv[0]) JSON to stdout; if `READ_STDIN=1` env is set, slurps stdin and adds `"stdin": "..."`; exits `EXIT_CODE` env if set, else 0. Rationale: covers every integration assertion (argv, stdin passthrough, exit-code propagation) in one tool.
10. **Manual cross-shell smoke tests** (spec §11.3 — PowerShell 7, Windows PowerShell 5.1, cmd, Git Bash, VS Code, Windows Terminal). **Chosen:** documented as a checklist in `tests/manual-smoke.md`; not run by CI. Rationale: those shells don't exist on GH-hosted Windows runners in usable form (no Git Bash by default, no VS Code).
11. **CI test invocation.** **Chosen:** `cargo test --workspace` on PR/push (debug). Release-mode test pass on tag only. Rationale: PR feedback should be fast; release builds run rarely.
12. **Round-trip parser test approach.** Options: hit the runner's reader directly via a lib API / behavioral via shim invocation. **Chosen:** behavioral — write a sidecar via shrt's writer, invoke the shim, observe argv passed to argv-stub. Rationale: avoids exposing the runner's parser as a lib (it's a binary by design); end-to-end coverage of the round-trip guarantee.
13. **Property-based testing.** **Chosen:** out of scope. Rationale: table-based suffices for the schema's small surface; v0.2 can add `proptest`.
14. **Cold-start perf test.** Options: assert hard threshold / report only / skip. **Chosen:** report only — `crates/shrt/tests/perf.rs` is `#[ignore]`-tagged; on tag builds CI runs `cargo test --release -- --ignored --nocapture` to print timings. Rationale: spec §6.1 is a target; CI variance makes hard thresholds brittle.
15. **Cross-platform test scope.** **Chosen:** Windows only for v0.1 (matches spec §9). Rationale: tests use `\r\n` line ending tolerance, `cmd.exe`, and PATHEXT — all Windows-specific.
16. **`.exe`-suffix invariant test.** **Chosen:** lock — rename a shim's `.exe` to `.bin`, invoke, assert exit 78. Validates `sidecar-format` Decision 19.
17. **`shrt sync` byte-restoration test.** **Chosen:** lock — overwrite `<name>.exe` with garbage bytes, run `shrt sync`, assert the file is byte-equal to `RUNNER_BYTES` afterward. Validates `shim-management` Decision 11.
18. **`shrt doctor` mixed-state test.** **Chosen:** lock — set up three shims (good / bad-sidecar / missing-target), run `shrt doctor --json`, parse output, assert exact failing-check identities. Validates `cli-surface` Decision 18 + `shim-management` doctor flow.
19. **External-tool exposure in tests.** **Chosen:** only `argv-stub`, `cmd.exe`, and `findstr` (a Windows built-in) appear as targets. No network, no third-party binaries. Rationale: hermetic test runs.
20. **Test parallelism.** **Chosen:** default cargo parallelism. Per-test tempdirs and `Command::env` (process-local env, never `std::env::set_var`) make tests independent. Rationale: simplest path that's correct.

## §4 Contracts & shapes

**Layout additions:**
```
shrt/
├── crates/
│   ├── argv-stub/                 # new workspace member, publish = false
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   └── shrt/
│       └── tests/
│           ├── common/mod.rs      # shared helpers: stub_path(), make_ctx()
│           ├── roundtrip.rs       # writer→reader fidelity
│           ├── invocation.rs      # add → invoke → argv assertion
│           ├── exit_codes.rs      # 1, 64, 66, 78, 127
│           ├── sync_doctor.rs     # sync + doctor flows
│           ├── name_validation.rs # add rejects bad names
│           ├── perf.rs            # cold-start timing (#[ignore])
│           └── manual-smoke.md    # human-only checklist
```

**`argv-stub` (crates/argv-stub/src/main.rs) contract:**

- Reads `EXIT_CODE` env; on parse-failure, defaults to 0.
- Reads `READ_STDIN` env; if `1`, slurps stdin to a `String`.
- Builds a JSON object: `{"argv": std::env::args().skip(1).collect::<Vec<_>>()}`. If stdin was read, adds `"stdin": <captured>`.
- Writes the JSON + `\n` to stdout.
- Exits with the captured exit code.
- Implementation surface: ~30 lines, std + `serde_json` (already a workspace dev-dep) — actually argv-stub is a workspace member, not a test crate, so it has its own deps. **Lock:** argv-stub uses hand-rolled JSON emission (it's just two known fields) to keep it std-only and fast.

**`crates/shrt/tests/common/mod.rs` helpers:**

| Helper | Returns | Notes |
|---|---|---|
| `stub_path() -> &'static Path` | absolute path to `argv-stub.exe` | Builds once per test process via `cargo build -p argv-stub`, caches in `OnceLock`. |
| `make_shim_dir() -> tempfile::TempDir` | new tempdir | Holds the dir owned by the test; auto-cleans on drop. |
| `shrt(shim_dir) -> Command` | preconfigured `Command` | `Command::new(env!("CARGO_BIN_EXE_shrt"))` with `--shim-dir=<dir>`. |
| `add_stub_shim(dir, name, template, shell, env) -> ()` | success | Wraps the `add` call for the most common case (target = stub path). |
| `invoke_shim(dir, name, args) -> Output` | child output | Locates `<dir>/<name>.exe`, sets up env (`EXIT_CODE`, `READ_STDIN`), runs and captures stdout/stderr/exit. |

**Required integration test set** (each is a `#[test]` somewhere in the files above):

| Test ID | File | Asserts |
|---|---|---|
| `add_then_invoke_passes_argv_correctly` | invocation | argv-stub captures the substituted argv exactly. |
| `placeholder_input_joins_with_single_space` | invocation | `{INPUT}` semantics. |
| `placeholder_at_quotes_each_arg_per_crt_rules` | invocation | `{@}` produces argv with original boundaries. |
| `placeholder_env_substitutes_env_value` | invocation | `{ENV:NAME}` resolves; sets env in the shim invocation. |
| `placeholder_env_with_default_uses_default_when_unset` | invocation | `{ENV:NAME:default}` fallback. |
| `add_with_shell_true_supports_pipes` | invocation | `template = "echo hello \| findstr h"`, `shell=true`; stdout contains "hello". |
| `missing_positional_arg_exits_64` | exit_codes | `{1}` template, invoke with zero args, exit 64. |
| `missing_env_var_in_template_exits_78` | exit_codes | `{ENV:NEVERSET}` exits 78. |
| `target_not_found_exits_127` | exit_codes | sidecar `target = "definitelynotacommand"` → exit 127. |
| `missing_sidecar_exits_66` | exit_codes | delete the `.shrt` then invoke `.exe` → exit 66. |
| `bad_sidecar_exits_78` | exit_codes | corrupt the sidecar → exit 78. |
| `version_mismatch_exits_78` | exit_codes | sidecar with `version = 99` → exit 78. |
| `shim_renamed_off_exe_exits_78` | exit_codes | rename `wt.exe` → `wt.bin`; invoke → exit 78. |
| `child_exit_code_propagates` | exit_codes | invoke a stub-shim with `EXIT_CODE=42`; assert shim exits 42. |
| `stdin_passthrough_works` | invocation | pipe a string to shim stdin; stub captures it. |
| `stdout_passthrough_works` | invocation | stub prints something; test process captures it. |
| `stderr_passthrough_works` | invocation | stub writes to stderr; test process captures it on stderr. |
| `sync_restores_modified_shim_bytes` | sync_doctor | overwrite `<name>.exe` with `b"junk"`; run `shrt sync`; re-read; assert byte-equal to `RUNNER_BYTES`. |
| `doctor_reports_mixed_state` | sync_doctor | three shims (good/bad-sidecar/missing-target); `--json` summary == `fail`; exact failing checks listed. |
| `name_validation_rejects_path_separator` | name_validation | `shrt add foo/bar "..."` exits 64. |
| `name_validation_rejects_reserved_device` | name_validation | `shrt add con "..."` exits 64. |
| `name_validation_accepts_alphanumeric` | name_validation | `shrt add wt0_0 "..."` exits 0. |
| `roundtrip_emoji_in_description_round_trips` | roundtrip | non-ASCII Unicode in `--desc` survives write→read. |
| `roundtrip_newline_in_template_escapes_correctly` | roundtrip | `\n` and `\t` survive the writer + runner reader. |
| `roundtrip_unknown_key_warns_but_runs` | roundtrip | manually inject an unknown TOML key into a sidecar; shim still runs; runner emits a warning to stderr. |

**Required unit test set:**

| Module | Coverage |
|---|---|
| `crates/shrt-runner/src/substitute.rs` | every placeholder form (`{N}`, `{N?}`, `{INPUT}`, `{@}`, `{ENV:NAME}`, `{ENV:NAME:default}`, `{{`, `}}`); unmatched `{`; whitespace-in-placeholder; bad ENV name; missing required; missing optional. |
| `crates/shrt-runner/src/argv.rs` | CRT examples from Microsoft docs: `a b c` → 3 args; `"a b c"` → 1 arg; `\\\"x` → `"x`; `\\\\` → `\\`; trailing unterminated quote; empty input. |
| `crates/shrt-runner/src/sidecar.rs` | UTF-8-no-BOM accepted; BOM rejected; `\r\n` and `\n` both accepted; unknown key warned; missing `target` rejected; `version=2` rejected; basic vs literal string forms (literal rejected). |
| `crates/shrt-runner/src/path.rs` | `~/.bin` expansion; `${VAR}` expansion; unset `${VAR}` errors; `cwd` empty returns None; PATH+PATHEXT search finds `cmd.exe` and `findstr.exe`; bare nonexistent target returns NotFound. |
| `crates/shrt/src/config.rs` | writer escapes `"`, `\`, `\n`, `\t`; rejects bytes < 0x20 except `\n`/`\t`; reader+writer round-trip preserves every field; default values honored on read. |
| `crates/shrt/src/cli.rs` | `validate_name` allowlist; reserved-device names; length cap; empty rejected. `parse_template_and_target` splits on first whitespace; respects `--target` override. |
| `crates/shrt/src/paths.rs` | `is_on_path` case-insensitive; semicolons split correctly; trailing `;` tolerated. |

**Cold-start perf test (`crates/shrt/tests/perf.rs`):**
- One `#[ignore] #[test] fn cold_start_under_50ms()` (or however lenient).
- Setup: build a stub-shim. Loop 10 invocations (discard first, average 9). Print timings via `eprintln!`.
- Does NOT assert; failure is invisible to default `cargo test`. CI tag runs use `--ignored --nocapture` to surface the numbers.

## §5 Sequence

**Local dev: `cargo test --workspace`:**
1. Cargo builds every workspace member (incl. `argv-stub` because it's a workspace member).
2. Cargo runs unit tests inside each crate's `#[cfg(test)]` blocks.
3. Cargo runs integration tests in `crates/shrt/tests/*.rs`. Each file is a separate test binary.
4. Within `tests/common/mod.rs`, `stub_path()` invocations are no-ops (binary already built).
5. Each `#[test]` creates its own tempdir, configures `Command`s via `--shim-dir=<temp>`, runs assertions, drops the tempdir.

**Local dev: `cargo test -p shrt` (subset):**
1. Cargo builds only `shrt` and its deps.
2. `argv-stub` is NOT auto-built. Tests fall back on `stub_path()` which runs `cargo build -p argv-stub` explicitly. Slow first run, fast subsequent.

**CI on PR/push:**
1. Native runner on `windows-latest` (and `windows-11-arm` per `build-pipeline`): `cargo test --workspace` (debug).

**CI on tag (`v*`):**
1. After release artifact build, run `cargo test --workspace --release` to catch profile-dependent regressions.
2. Run `cargo test --release -- --ignored --nocapture` to print perf numbers; do not fail on them.

## §6 Out of scope
- Production behavior of any module — owned by its respective slice.
- Test-result reporting / coverage tooling. Not in spec.
- Mutation testing, fuzz testing, property-based testing. Defer to v0.2.
- Network/integration with a real `copilot.exe` or other third-party tool.
- Cross-platform tests (macOS/Linux). Spec §9 defers.
- Manual smoke automation (PowerShell 5.1, Git Bash, VS Code terminal). Documented checklist only.

> If the parent spec is ambiguous on anything this slice depends on, stop and update the spec. Do not invent behavior here.
