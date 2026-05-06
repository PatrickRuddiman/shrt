Parent slice: [shrt — testing-harness](../slices/testing-harness.md)
Depends on: 10, 13

# Task 14 — integration tests: invocation, exit codes, round-trip, name validation, perf

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Author the remaining integration test files enumerated in `slices/testing-harness.md` §4. Together with tasks 09–13's per-command tests, this completes the v0.1 acceptance test surface.

## Tasks
- [x] Create `crates/shrt/tests/invocation.rs` with: `add_then_invoke_passes_argv_correctly` (template `"{1} {2}"`, invoke `shim foo bar`, parse stub JSON, assert argv `["foo","bar"]`); `placeholder_input_joins_with_single_space` (template `"{INPUT}"`, invoke with three args, assert single space joining); `placeholder_at_quotes_each_arg_per_crt_rules` (template `"{@}"`, invoke with `["a b", "c"]`, assert argv preserves boundaries); `placeholder_env_substitutes_env_value` (template `"{ENV:GREETING}"`, set GREETING via `Command::env`, assert stub argv); `placeholder_env_with_default_uses_default_when_unset` (template `"{ENV:NEVERSET:fallback}"`, assert argv `["fallback"]`); `add_with_shell_true_supports_pipes` (target `cmd`, template `"/c echo hello | findstr h"`, `--shell`, assert stdout contains `hello`); `child_exit_code_propagates` (set `EXIT_CODE=42` in stub env, assert shim exits 42); `stdin_passthrough_works` (set `READ_STDIN=1`, pipe a known string into the shim, parse JSON, assert `stdin` field matches); `stdout_passthrough_works` (assert captured stdout contains the stub's JSON); `stderr_passthrough_works` (have stub write to stderr — extend `argv-stub` if needed via a `WRITE_STDERR` env honored in task 01's stub OR have the shim use `cmd /c` to echo to stderr).
- [x] Create `crates/shrt/tests/exit_codes.rs` with: `missing_positional_arg_exits_64` (template `"{1}"`, invoke with no args, assert exit 64); `missing_env_var_in_template_exits_78` (template `"{ENV:NEVERSET_KEYS}"`, assert exit 78); `target_not_found_exits_127` (sidecar target `definitelynotacommand_xyz`, assert exit 127); `missing_sidecar_exits_66` (delete `<name>.shrt` after add, invoke `<name>.exe` directly, assert exit 66); `bad_sidecar_exits_78` (corrupt `<name>.shrt` after add, invoke, assert exit 78); `version_mismatch_exits_78` (overwrite sidecar with `version = 99`, assert exit 78); `shim_renamed_off_exe_exits_78` (rename `<name>.exe` → `<name>.bin`, invoke `<name>.bin`, assert exit 78).
- [x] Create `crates/shrt/tests/roundtrip.rs` with: `roundtrip_emoji_in_description_round_trips` (`--desc "🎉 hello"`, list, assert `description` field equals); `roundtrip_newline_in_template_escapes_correctly` (template containing `\n` and `\t`; verify writer escapes; verify runner reader decodes by invoking the shim and checking argv); `roundtrip_unknown_key_warns_but_runs` (manually inject `mystery = "x"` into a written sidecar, invoke shim, assert it still runs, assert stderr contains `ignoring unknown key 'mystery'`).
- [x] Create `crates/shrt/tests/name_validation.rs` with: `name_validation_rejects_path_separator` (`shrt add foo/bar "..."` → exit 64); `name_validation_rejects_reserved_device` (`shrt add con "..."` → exit 64); `name_validation_accepts_alphanumeric` (`shrt add wt0_0 "echo"` → exit 0); `name_validation_rejects_double_dot` (`shrt add a..b "..."` → exit 64); `name_validation_rejects_long_name` (65-char name → exit 64).
- [x] Create `crates/shrt/tests/perf.rs` with `#[ignore] #[test] fn cold_start_under_50ms()` per `slices/testing-harness.md` §3 Decision 14: build a stub shim, warm-up invoke, then time 10 invocations (discard first), `eprintln!("cold-start avg: {} ms", avg_ms)`. No assertion — reporting only.
- [x] If task 01's `argv-stub` lacks a `WRITE_STDERR` mode needed by `stderr_passthrough_works`, extend `crates/argv-stub/src/main.rs` to honor a `WRITE_STDERR` env (when set, write its value to stderr before printing the JSON). Update task 01's argv-stub file in place and update the unit tests.

## Acceptance criteria
- [x] `cargo test -p shrt --test invocation` passes.
- [x] `cargo test -p shrt --test exit_codes` passes.
- [x] `cargo test -p shrt --test roundtrip` passes.
- [x] `cargo test -p shrt --test name_validation` passes.
- [x] `cargo test -p shrt --tests` passes (all integration files together).
- [x] `cargo test -p shrt --test perf -- --ignored --nocapture 2>&1 | grep -q 'cold-start avg'`.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
