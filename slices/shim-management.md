Parent spec: [shrt — Specification](../spec.md)

# shrt — shim-management

## §1 Summary
The filesystem and TOML I/O behind every `shrt` command. Owns sidecar serialization (writer side), atomic write of the `.shrt` + `.exe` pair, shim-dir creation, PATH detection, and the per-check logic that powers `shrt sync` and `shrt doctor`. Provides the function signatures the `cli-surface` slice dispatches into; consumes `RUNNER_BYTES` from `build-pipeline` and the schema from `sidecar-format`.

## §2 Codebase reconnaissance
> Greenfield: no existing system to reconcile. Decisions below are unconstrained.

Sibling slices already locked:
- `slices/cli-surface.md` — names every public fn this slice exports and their callers.
- `slices/sidecar-format.md` — wire-format invariants the writer must satisfy.
- `slices/build-pipeline.md` — `RUNNER_BYTES: &'static [u8]` is available via `crates/shrt/src/shim.rs`.

## §3 Decisions
1. **Atomic write pattern.** Options: direct write / temp + rename / temp + fsync + rename. **Chosen:** temp + rename (`fs::rename` maps to `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` on Windows). Rationale: the only stable atomic-replace primitive Rust gives us; spec §10's threat model doesn't demand fsync (developer machine, not a database).
2. **Temp filename.** Options: random suffix / PID suffix / `.tmp` plain. **Chosen:** `<final>.tmp`. Best-effort `remove_file` on failure paths. Rationale: collisions only matter under concurrent `shrt add` for the same shim, which spec §10.1 excludes.
3. **Pair-write ordering for `add`.** Options: exe-first / sidecar-first / both-temp-then-rename-both. **Chosen:** write `<name>.shrt.tmp` → write `<name>.exe.tmp` → rename `.shrt.tmp` → rename `.exe.tmp`. Rationale: a renamed `.shrt` without `.exe` is harmless; the reverse is a live broken shim that exits 66 at runtime.
4. **Pair-delete ordering for `remove`.** Options: parallel / exe-first / sidecar-first. **Chosen:** delete `<name>.exe`, then `<name>.shrt`. Rationale: an exe-less sidecar is invisible to PATH; surviving a partial remove keeps the user's environment safe.
5. **Windows ACL hardening (sidecar files).** Options: `windows-sys` for `SetSecurityInfo` / `windows-acl` crate / defer. **Chosen:** **defer to v0.2**. Rationale: spec §10.2 says "should have" and explicitly notes "not enforced on read"; std has no API; adding a winapi crate just for ACLs is over-investment for v0.1's threat model.
6. **Shim-dir creation policy.** Options: `init` only / `add` also auto-creates. **Chosen:** both — `init` is explicit; `add` calls `fs::create_dir_all` if the dir is absent. Rationale: spec §5.1 says `init` is idempotent and `add` follows the workflow expectation that you can `shrt add` from a fresh checkout.
7. **PATH detection.** Options: registry probe / env-var split. **Chosen:** read the runner-process `PATH` env var, split on `;`, lowercase-compare each entry's canonicalized absolute path with the shim-dir's. Rationale: matches what the user's current shell sees; registry probe would lie about an unrestarted session.
8. **Sidecar reader.** Options: hand-rolled / `toml` crate with serde-derive. **Chosen:** `toml` crate + `#[derive(Deserialize)]` on `SidecarConfig`. Rationale: spec §7.1 lists `toml`; the runner's hand-rolled reader is a separate concern (per `sidecar-format` slice).
9. **Sidecar writer.** Options: `toml::to_string` / `toml_edit` / hand-rolled. **Chosen:** **hand-rolled** in `crates/shrt/src/config.rs::write_sidecar`. Rationale: `sidecar-format` Decision 2 requires always-basic-string serialization; the `toml` crate's serializer prefers literal-string form when content allows, which violates the round-trip guarantee. ~50 LOC for a closed schema; lower risk than post-processing.
10. **Writer-side string sanitization.** Options: pass-through / restricted set / full TOML range. **Chosen:** restricted set — accept any UTF-8 except control chars below 0x20, with `\n` and `\t` escaped. `\r` is rejected outright (writer normalizes line endings to `\n`). Rationale: the runner's escape set is narrow per `sidecar-format` Decision 1; rejecting at write time keeps the round-trip guarantee enforced by the type system + a single validation pass.
11. **Sync logic.** Options: rewrite all `*.exe` / pair-by-sidecar / hash-based skip. **Chosen:** for each `<name>.shrt`, byte-compare `<name>.exe` against `RUNNER_BYTES`; if different, rewrite atomically. Tally `updated` vs `total`. Rationale: matches `cli-surface` Decision 16; avoids touching unrelated `.exe` files; byte-equal is the sync's authoritative signal.
12. **Doctor: target-resolution check.** Options: spawn-and-fail / `which` crate. **Chosen:** `which::which(&entry.target)` per shim. Rationale: spec §7.1 already lists `which`; matches what the runner does at runtime.
13. **Doctor: parse check.** **Chosen:** re-deserialize each `.shrt` via the same reader path. Rationale: most-conservative test of the round-trip guarantee.
14. **Doctor: byte-equality check.** **Chosen:** read each `<name>.exe` in full and compare against `RUNNER_BYTES`. Mismatch → fail with hint "run `shrt sync`". Rationale: matches `cli-surface` Decision 18 and `runner` Decision 11 (no embedded version string in the runner; bytes are the version).
15. **Doctor: PATH check.** **Chosen:** delegate to the same `is_on_path()` helper used by `init` and `path`. Rationale: single source of truth.
16. **Module layout.** **Chosen:** three files in `crates/shrt/src/`: `config.rs` (`SidecarConfig`, `Entry`, read/write), `shim.rs` (`RUNNER_BYTES` + the high-level `init`/`add`/`remove`/`list`/`show`/`sync`/`path_report`/`doctor` fns), `paths.rs` (`shim_dir()` resolution + `is_on_path()`). Per spec §2.1.
17. **Cross-volume rename guard.** **Chosen:** none — temp lives in the same directory as the final, so `fs::rename` is always intra-volume on Windows. Rationale: structural invariant, not a runtime check.
18. **Concurrent-`shrt`-process safety.** **Chosen:** out of scope. Rationale: spec §10.1 explicitly excludes multi-user / shared-config scenarios; no file locking, no PID tracking.
19. **`Entry` shape.** **Chosen:** `SidecarConfig` plus a `name: String`. See §4. Rationale: every reader command (`list`, `show`, `doctor`) needs the name alongside the parsed config.
20. **`add` collision exit code.** Options: 1 / 73 / 64. **Chosen:** 73 (`EX_CANTCREAT`). Rationale: spec §5.3 maps "cannot create output" to 73; refusing to overwrite without `--force` is exactly that.

## §4 Contracts & shapes

**`crates/shrt/src/config.rs`:**

| Item | Shape |
|---|---|
| `SidecarConfig` | `{ target: String, template: String, shell: bool, cwd: String, description: String, created: Option<String>, version: u32 }` with `Serialize` + `Deserialize` derived. Default values: `shell=false`, `cwd=""`, `description=""`, `created=None`, `version=1`. |
| `Entry` | `{ name: String, ..SidecarConfig fields }`. Built by reading a `.shrt` and joining with the file stem. |
| `read_sidecar(path: &Path) -> anyhow::Result<SidecarConfig>` | `toml::from_str` on a UTF-8-decoded read; surfaces parse errors with file path context. |
| `write_sidecar(path: &Path, cfg: &SidecarConfig) -> anyhow::Result<()>` | Hand-rolled writer (see below). Atomic via temp + rename. |

**Hand-rolled writer rules (matches `sidecar-format` slice §4):**
- UTF-8, no BOM, `\n` line endings, one `key = value` per line.
- Required fields always written (`target`, `template`, `version`).
- Optional fields written only when non-default (`shell` if `true`, `cwd`/`description` if non-empty, `created` if `Some`).
- Strings emitted as basic strings (`"..."`); the writer escapes `"`, `\`, `\n`, `\t`. Any byte < 0x20 except `\n`/`\t` causes `write_sidecar` to return `Err(...)` → CLI maps to exit 64.
- No comments are emitted (the writer never produces them; manual `#` comments survive a read but get dropped on rewrite).

**`crates/shrt/src/paths.rs`:**

| Item | Shape |
|---|---|
| `shim_dir(override_flag: Option<&Path>) -> anyhow::Result<PathBuf>` | Returns `override_flag` if `Some`, else `directories::UserDirs::new()?.home_dir().join(".shrt").join("bin")`. Caller passes the resolved value to `Ctx`. |
| `is_on_path(shim_dir: &Path) -> bool` | Reads `PATH`, splits on `;`, lowercase-compares canonicalized absolute paths. |

**`crates/shrt/src/shim.rs` exports:**

| Function | Returns | Notes |
|---|---|---|
| `pub const RUNNER_BYTES: &'static [u8]` | from `build-pipeline` | |
| `init(ctx) -> anyhow::Result<InitReport>` | `InitReport { shim_dir, created, on_path }` | Creates the dir if absent; sets `created` accordingly. |
| `add(ctx, name, cfg, force) -> anyhow::Result<()>` | — | Pair-write per Decision 3; collision without force → exit 73. |
| `remove(ctx, name) -> anyhow::Result<()>` | — | Pair-delete per Decision 4; missing shim → exit 66. |
| `list(ctx) -> anyhow::Result<Vec<Entry>>` | sorted by name | Reads every `*.shrt`; sidecar parse errors propagate as exit 78. |
| `show(ctx, name) -> anyhow::Result<(PathBuf, String, Entry)>` | path + raw content + parsed | Caller (cli) chooses which to print. |
| `sync(ctx) -> anyhow::Result<SyncReport>` | counts | Per Decision 11. |
| `path_report(ctx) -> PathReport` | `{ path, on_path }` | Pure; no I/O beyond env read. |
| `doctor(ctx) -> anyhow::Result<DoctorReport>` | structured | Four checks per Decisions 12–15. |

**Report shapes:**

```
InitReport { shim_dir: PathBuf, created: bool, on_path: bool }
SyncReport { updated: usize, total: usize, errors: Vec<(String /*name*/, String /*reason*/)> }
PathReport { path: PathBuf, on_path: bool }
DoctorReport { summary: Status, checks: Vec<Check> }
Check       { name: String, status: Status, message: String }
Status      { Ok, Warn, Fail }   // serializable as lowercase string in JSON
```

**Exit-code mapping (this slice's failures, propagated to CLI):**

| Failure | Exit | Trigger |
|---|---|---|
| Shim-dir not writable | 73 | Permission denied / read-only volume on `init`/`add`/`remove`/`sync`. |
| Add collision without force | 73 | Either `<name>.exe` or `<name>.shrt` exists. |
| Remove missing shim | 66 | Neither file exists. |
| Sidecar parse error during list/show/doctor | 78 | `toml::from_str` failure on any read. |
| Generic I/O error | 1 | Anything else (`anyhow` fallback). |

**`add` pair-write sequence (atomic-as-possible):**
1. `mkdir_p(shim_dir)`.
2. If `<shim_dir>/<name>.exe` or `<shim_dir>/<name>.shrt` exists and `!force` → return `Error("collision")` mapped to exit 73.
3. `write_atomically(<shim_dir>/<name>.shrt.tmp, sidecar_bytes)` via the hand-rolled writer.
4. `write_atomically(<shim_dir>/<name>.exe.tmp, RUNNER_BYTES)`.
5. `fs::rename(.shrt.tmp -> .shrt)`.
6. `fs::rename(.exe.tmp -> .exe)` — at this point the shim is live.
7. On any failure during 3–6: best-effort `remove_file` of any `.tmp` left behind; if step 5 succeeded but step 6 failed, also remove the renamed `.shrt`.

**`sync` rewrite step (per shim, when bytes differ):**
1. `write_atomically(<shim_dir>/<name>.exe.tmp, RUNNER_BYTES)`.
2. `fs::rename(.exe.tmp -> .exe)`.

## §5 Sequence

**`shrt add wt "copilot -p ..."`:**
1. `cli-surface` builds `SidecarConfig { target, template, shell, cwd, description, created: Some(now_utc_iso()), version: 1 }` and calls `shim::add(&ctx, "wt", cfg, force)`.
2. `shim::add` runs the sequence above. Hand-rolled writer in `config::write_sidecar` produces the TOML body; sanitization rejects forbidden control chars.
3. On success: returns `Ok(())`; cli emits no stdout.

**`shrt sync`:**
1. `cli-surface` calls `shim::sync(&ctx)`.
2. `shim::sync` enumerates `<shim_dir>/*.shrt`. For each:
   - Read the matching `<name>.exe` (or skip with an error entry if absent).
   - Compare bytes to `RUNNER_BYTES`.
   - If different: rewrite atomically; increment `updated`.
3. Return `SyncReport`.

**`shrt doctor`:**
1. `shim::doctor` runs four checks in order:
   - `path` check — calls `is_on_path(shim_dir)`.
   - For each shim:
     - `parse` — `read_sidecar` and capture any error.
     - `bytes` — read `<name>.exe`, compare to `RUNNER_BYTES`.
     - `target` — `which(entry.target)`.
2. Aggregate per-check results into `Vec<Check>`. Summary is `Fail` if any fail; `Warn` if any warn; else `Ok`. ACL-deferral note appears as a `Warn`-status `Check { name: "acls", message: "deferred to v0.2" }` so the user sees it in every doctor run.

## §6 Out of scope
- Argument parsing, output formatting, JSON shape rendering. Owned by `cli-surface`.
- The runner's reader code, escape decoding, and version-mismatch handling. Owned by `runner` and `sidecar-format`.
- Generation of the embedded runner binary. Owned by `build-pipeline`.
- Publishing logistics (crates.io single-crate fetch, Scoop manifest, GitHub Releases). Owned by `distribution`.
- Tab-completion script writing during `init`. Spec §12 question 4; deferred to v0.2.
- Concurrent-process file locking. Spec §10.1 excludes the use case.
- Windows ACL setting on sidecar files. Decision 5; deferred to v0.2.
- Test harness for `add → run → assert` integration coverage. Owned by `testing-harness`.

> If the parent spec is ambiguous on anything this slice depends on, stop and update the spec. Do not invent behavior here.
