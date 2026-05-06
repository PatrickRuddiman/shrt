Parent spec: [shrt — Specification](../spec.md)

# shrt — runner

## §1 Summary
The runtime path of every shim invocation: `current_exe()` → sidecar discovery and parse → placeholder substitution → target resolution against PATH+PATHEXT → process spawn with stdio inheritance → exit-code propagation. Integrates the contracts owned by `sidecar-format` and `substitution-engine`; owns target resolution, `cwd` expansion at runtime, the spawn call, and the exit-code-mapping module.

## §2 Codebase reconnaissance
> Greenfield: no existing system to reconcile. Decisions below are unconstrained.

Sibling slices:
- `slices/sidecar-format.md` — fixes file encoding, parser strictness, schema, and reader exit codes 66/78.
- `slices/substitution-engine.md` — fixes placeholder grammar, argv tokenizer, and substitution exit codes 64/78.

This slice picks up after substitution returns a `String` and routes execution.

## §3 Decisions
1. **Spawn API.** Options: `std::process::Command` / direct `winapi` `CreateProcessW` / spawning crate. **Chosen:** `std::process::Command`. Rationale: std-only mandate (spec §6.1); `Command` already wraps `CreateProcessW` with arg-array semantics matching our needs.
2. **Target resolution.** Options: rely on `CreateProcessW`'s implicit PATH search / hand-implemented PATH+PATHEXT search / `which` crate. **Chosen:** hand-implemented PATH+PATHEXT search. Rationale: `CreateProcessW` does not apply `PATHEXT` so bare `npm` would not find `npm.cmd`; `which` violates std-only.
3. **Path-vs-bare classification of `target`.** Options: heuristic on first byte / path-separator test / always-search-PATH. **Chosen:** if `target` contains `/` or `\`, treat as a path (absolute → use as-is; relative → resolve against runner CWD). Otherwise, PATH+PATHEXT search. Rationale: matches spec §3.3 ("Resolved against PATH if not absolute") and handles the realistic relative-path case.
4. **Target-not-found exit.** Options: 1 / 127 / 78. **Chosen:** 127. Rationale: matches spec §5.3 verbatim.
5. **Spawn failure after resolution** (e.g. ACCESS_DENIED, executable corrupt). Options: 1 / 127 / 126. **Chosen:** 1. Rationale: 127 is reserved for "not found"; spec §5.3 lacks a "found-but-cannot-exec" code, and a generic-error fallback is the only honest mapping.
6. **`cwd` expansion rules.** **Chosen:** leading `~` → `USERPROFILE` env value; `${VAR}` anywhere → env value; unset `${VAR}` → exit 78 with the variable name; Windows-style `%VAR%` is NOT expanded. Empty `cwd` → don't call `Command::current_dir` (inherit). Rationale: spec §3.3 documents `~` and `${VAR}`; sticking to that surface keeps the runner small.
7. **Stdio.** **Chosen:** `Stdio::inherit()` for all three streams. Rationale: matches spec §6.2 verbatim.
8. **Exit-code propagation precision.** Options: `ExitCode::from(code as u8)` (8-bit truncation, per spec pseudocode) / `std::process::exit(code as i32)` (full i32). **Chosen:** `std::process::exit(code as i32)`. Rationale: tools like Cargo and MSBuild emit exit codes outside 0–255; truncating loses signal. Spec pseudocode is illustrative; precision wins.
9. **Signal handling.** Options: install handler / forward / none. **Chosen:** none — rely on Windows console default (CTRL+C reaches both runner and child; both terminate). Rationale: spec is silent; OS default already does the right thing for the documented threat model.
10. **Empty `target` after parse.** Options: pass through to PATH search (which will fail with 127) / specific exit. **Chosen:** exit 78 with `shrt-runner: <sidecar>: target is empty`. Rationale: defense-in-depth; classifies as config error before spawn pipeline.
11. **Magic-flag for runner version (`<shim>.exe --shrt-runner-version`).** **Chosen:** not implemented in v0.1. Rationale: spec §12 question 5 leans defer; runner-version detection moves to the `shim-management` slice via stored-in-sidecar version or file-byte hash.
12. **`--help` / other reserved-looking args.** **Chosen:** pass through as regular args to the target. No magic flags in the runner. Rationale: any reserved flag would steal namespace from real targets that accept the same flag.
13. **Code layout.** **Chosen:** `crates/shrt-runner/src/` with `main.rs` (orchestration + exit-code mapping), `sidecar.rs` (per `sidecar-format`), `substitute.rs` + `argv.rs` (per `substitution-engine`), `path.rs` (PATH+PATHEXT search and `cwd` expansion). Five flat modules. Rationale: smallest layout that keeps each contract in one file.
14. **Stderr message convention.** **Chosen:** `shrt-runner: <sidecar-absolute-path>: <reason>` for sidecar-bound errors; same prefix for PATH/spawn errors so the user can locate the misconfigured shim. Rationale: matches `sidecar-format` slice §4 convention.
15. **Release-profile knobs.** **Chosen:** out of scope; owned by `build-pipeline`. This slice asserts only the spec §6.1 budgets (<300 KB stripped, <10 ms cold start) as acceptance constraints. Rationale: profile config is build-system territory.

## §4 Contracts & shapes

**Public entry of the runner binary:** `crates/shrt-runner/src/main.rs::main()` returning a process exit via `std::process::exit`.

**Module boundaries (within `crates/shrt-runner/src/`):**

| Module | Responsibility | Public surface |
|---|---|---|
| `main` | Orchestrate the eight steps in §5; map every error variant to an exit code; emit stderr in the locked format. | `fn main()` |
| `sidecar` | Implements `sidecar-format` slice §5 read path. | `fn parse(path: &Path) -> Result<SidecarConfig, SidecarError>` |
| `substitute` | Implements `substitution-engine` slice. | `fn substitute(template: &str, args: &[OsString], env: &dyn Fn(&str)->Option<OsString>) -> Result<String, SubstError>` |
| `argv` | Implements `substitution-engine` slice §4 CRT tokenizer. | `fn tokenize(line: &str) -> Vec<String>` |
| `path` | Target resolution + `cwd` expansion. | `fn resolve_target(target: &str) -> Result<PathBuf, PathError>`, `fn expand_cwd(cwd: &str) -> Result<Option<PathBuf>, PathError>` |

**`SidecarConfig` shape (consumed from the sidecar parser):**
- `target: String`
- `template: String`
- `shell: bool` (default false)
- `cwd: String` (default `""`)
- `description: String` (informational)
- `created: Option<String>` (informational; runner ignores)
- `version: u32` (validated 1; reader rejects others per `sidecar-format` slice §3 Decision 11)

**`PathError` variants and exit codes:**

| Variant | Trigger | Exit |
|---|---|---|
| `Empty` | `target` is empty after parse. | 78 |
| `EnvUnset(name)` | `${VAR}` referenced in `cwd` but unset. | 78 |
| `EnvNotUtf8(name)` | env value is non-UTF-8 (rare on Windows). | 78 |
| `NotFound(target)` | PATH+PATHEXT search exhausted; or absolute/relative path doesn't exist. | 127 |
| `CwdMissing(path)` | expanded `cwd` doesn't exist or isn't a directory. | 78 |

**PATH+PATHEXT search algorithm (when `target` has no path separator):**
1. Read `PATHEXT` env var; split on `;`; if absent, default to `.COM;.EXE;.BAT;.CMD`. Lowercase comparison.
2. If `target` already has a recognized extension (one of the `PATHEXT` entries, case-insensitive), the extension list is `[""]` (try as-is, no probing).
3. Read `PATH` env var; split on `;`. Skip empty entries. Don't de-dup.
4. For each PATH directory, in order: for each extension in the list above, in order: probe `<dir>\<target><ext>`. First hit wins.
5. None hit → `PathError::NotFound`.

**`cwd` expansion algorithm:**
1. Empty input → return `None` (caller skips `current_dir`).
2. If first char is `~` and the next char is end-of-string or a path separator: replace the leading `~` with `USERPROFILE`. Unset `USERPROFILE` → `PathError::EnvUnset("USERPROFILE")`.
3. Walk the remaining string scanning for `${`; for each `${VAR}` literal: replace with env value, error per the table above on unset/non-utf8. `${` without a closing `}` is an error → `PathError::EnvUnset` with the partial name (or a dedicated variant — pick the existing one for fewer error types).
4. Verify the resulting path exists and is a directory; failure → `PathError::CwdMissing`.

**Exit-code summary (runner-level, comprehensive):**

| Exit | Source | Trigger |
|---|---|---|
| 0 | child | child exited 0 |
| any i32 | child | child's actual exit code |
| 1 | runner | spawn failed after target resolution; child terminated abnormally with no code; uncategorized I/O error |
| 64 | substitution-engine | required `{N}` missing; user-arg non-UTF-8 |
| 66 | sidecar-format | sidecar file does not exist |
| 78 | sidecar-format / substitution-engine / path | sidecar parse error; template parse error; ENV resolution error; empty target; cwd expansion error |
| 127 | path | target not found |

**Process spawn shape (final):**
- shell=false: `Command::new(resolved_target).args(tokenize(substituted))`. Note `target` is **not** repeated in `args`; `Command::new` provides argv[0] from the target path.
- shell=true: `Command::new("cmd").args(["/c", &format!("{} {}", config.target, substituted)])`. The literal string `config.target` (not the resolved path) is what's passed to `cmd /c`, so PATH resolution happens inside cmd.exe; `cmd /c` is found via the OS default.
- All variants: `current_dir(expand_cwd(...)?)` only if expansion returned `Some`; `stdin/stdout/stderr` inherited.

## §5 Sequence

1. `main()` calls `current_exe()`. Failure → exit 1, stderr `shrt-runner: cannot determine own path: <io-error>`.
2. `main()` validates the exe path ends in `.exe` (case-insensitive) and derives the sidecar path by replacing the suffix with `.shrt`. Non-`.exe` exe → exit 78 (per `sidecar-format` slice).
3. `main()` calls `sidecar::parse(&sidecar_path)`. Errors map to 66 (absent) or 78 (parse / schema). Stderr per the locked convention.
4. `main()` collects user args via `args_os().skip(1)`. Calls `substitute::substitute(&config.template, &user_args, &|n| std::env::var_os(n))`. Errors map to 64 / 78.
5. `main()` calls `path::expand_cwd(&config.cwd)`. Errors map to 78 / 127.
6. **Branch on `config.shell`:**
   - false: call `path::resolve_target(&config.target)` (78 / 127); build `Command::new(resolved)`; `args(argv::tokenize(&substituted))`.
   - true: build `Command::new("cmd").args(["/c", &format!("{} {}", config.target, substituted)])`. No prior call to `path::resolve_target`.
7. Apply `current_dir` if expansion returned `Some`; set all three stdios to inherit; call `.status()`.
8. On `Ok(status)`: `std::process::exit(status.code().unwrap_or(1))`. On `Err(_)`: exit 1 (post-resolution spawn failure).

## §6 Out of scope
- Sidecar grammar, schema, escapes, encoding. Owned by `sidecar-format`.
- Placeholder grammar, CRT quoting, argv tokenizer. Owned by `substitution-engine`.
- Build / release profile flags that gate the binary-size budget. Owned by `build-pipeline`.
- Runner version reporting / cross-shim drift detection. Owned by `shim-management` (`shrt doctor`, `shrt sync`).
- Cross-platform behavior: shim has no `.exe` suffix on macOS/Linux, `cmd /c` becomes `sh -c`. Deferred per spec §9.
- Signal forwarding beyond OS default; killing orphaned children if runner is itself terminated. Documented as a known limitation, not engineered around.

> If the parent spec is ambiguous on anything this slice depends on, stop and update the spec. Do not invent behavior here.
