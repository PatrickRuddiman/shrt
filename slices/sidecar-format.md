Parent spec: [shrt — Specification](../spec.md)

# shrt — sidecar-format

## §1 Summary
Locks the wire contract between the `shrt` CLI (writer, full `toml` crate) and the `shrt-runner` binary (reader, hand-rolled subset). Defines exactly which TOML constructs may appear in a `.shrt` file, how missing/unknown values are handled, and the runner exit code for every parse-time failure. Does not specify *how* shims are written to disk (atomicity, ACLs, paths) — that's `shim-management`.

## §2 Codebase reconnaissance
> Greenfield: no existing system to reconcile. Decisions below are unconstrained.

The spec names the writer-side toolkit (`serde` + `toml` crates) and constrains the reader (`std`-only, hand-rolled). No prior code exists at `crates/shrt/` or `crates/shrt-runner/`.

## §3 Decisions
1. **String escape set the runner must support.** Options: spec's set (`\"` `\\` `\n` `\t`) / spec + `\r` + `\0` / full TOML basic-string escape set. **Chosen:** spec's set only. Rationale: minimum runner parser surface; CLI normalizes on write so the round-trip guarantee holds.
2. **String form the writer emits.** Options: always basic (`"..."`) / let `toml` pick basic vs. literal / always basic with extra `{`/`}` escaping. **Chosen:** always basic. Rationale: literal-string and multi-line forms are forbidden by Decision 1; pinning to basic is the natural follow-through.
3. **File encoding.** Options: UTF-8 no BOM / UTF-8 with optional BOM / any encoding. **Chosen:** UTF-8 no BOM, reader rejects BOM with exit 78. Rationale: simplest invariant; CLI writes UTF-8.
4. **Line endings the reader accepts.** Options: `\n` only / `\n` and `\r\n` / any. **Chosen:** `\n` and `\r\n`. Rationale: Windows-first; a manually edited file in Notepad must round-trip.
5. **Whitespace tolerance.** Options: strict / permissive (any whitespace around `=`, leading whitespace ignored, blank lines ignored). **Chosen:** permissive. Rationale: matches TOML; users will hand-edit.
6. **Comment rules.** Options: full-line + trailing `#` / line-start `#` only / no comments. **Chosen:** full-line and trailing `#` to end-of-line; `#` inside a basic string is content, not a comment. Rationale: matches spec §6.3 verbatim and the example sidecar in §3.1 already uses trailing comments.
7. **Multiple assignments per line.** Options: allowed / rejected. **Chosen:** rejected (one `key = value` per line). Rationale: matches TOML; keeps the runner parser one-pass.
8. **Unknown keys (reader).** Options: ignore silently / warn to stderr and continue / error. **Chosen:** warn + continue. Rationale: matches spec §6.3 verbatim; preserves forward compatibility while flagging schema drift.
9. **Unknown keys (writer).** Options: open / closed via serde struct. **Chosen:** closed via serde-derived struct. Rationale: type system enforces the round-trip guarantee; nothing the runner can't parse can be written.
10. **`version` missing.** Options: default to `1` / error. **Chosen:** default to `1`. Rationale: matches spec §3.1.
11. **`version` unknown (>1) or ≤0.** Options: exit 78 / warn + continue. **Chosen:** exit 78 with message naming the file and value. Rationale: matches spec §3.2 ("runner refuses unknown versions") and §5.3 (config error class).
12. **Bool literal forms.** Options: lowercase only / case-insensitive / accept 0/1. **Chosen:** lowercase `true`/`false` only, case-sensitive. Rationale: matches spec §6.3 and TOML.
13. **Integer literal form for `version`.** Options: decimal only / decimal + hex / decimal + underscores. **Chosen:** decimal digits only, no underscores, no sign. Rationale: matches spec §6.3; only legal values are small positives.
14. **`created` field semantics.** Options: validated ISO-8601 / freeform / runner ignores. **Chosen:** ISO-8601 UTC with `Z` suffix on write (CLI uses `chrono`), runner does NOT validate (informational only). Rationale: spec §3.2 marks it informational; runner spends no parser budget on it.
15. **Optional-field defaults when omitted.** **Chosen:** `shell` → false; `cwd` → empty (inherit parent CWD); `description` → empty; `created` → omitted from any informational output.
16. **`cwd` storage form.** Options: writer expands `~`/`${VAR}` once / writer stores literal, reader expands at runtime. **Chosen:** stored literal, runtime expansion. Rationale: spec §3.3 only makes sense with runtime expansion; sidecars stay portable across user-name changes and environments.
17. **Sidecar absent at runtime.** Options: exit 66 / exit 78 / exit 1. **Chosen:** exit 66 (`EX_NOINPUT`). Rationale: matches spec §5.3.
18. **Required field missing or wrong type.** **Chosen:** exit 78 with a one-line stderr message naming the sidecar absolute path and the offending key. Rationale: matches spec §6.3.
19. **Sidecar filename derivation from the shim's exe path.** **Chosen:** runner reads `current_exe()`, requires the path to end in `.exe` (case-insensitive), and substitutes `.shrt`. Missing `.exe` → exit 78. Rationale: v0.1 is Windows-only per §9; cross-platform changes this rule.

## §4 Contracts & shapes

**On-disk sidecar invariants (writer + reader contract):**
- Encoding: UTF-8, no BOM. Reader exits 78 if a BOM (`EF BB BF`) is the first three bytes.
- Line endings: `\n` or `\r\n`. Reader treats `\r` only as a line terminator when immediately followed by `\n`; a bare `\r` inside a string is decoded literally if escaped (`\n` and `\t` only — bare control characters in a string are illegal and produce exit 78).
- One `key = value` assignment per line. Whitespace around `=` is unrestricted. Leading whitespace on a line is ignored. Blank lines are ignored.
- Comments: `#` outside a basic string introduces a comment that runs to the line end. `#` inside a basic string is a literal character.
- String form: always basic (`"..."`). The only escapes the reader recognizes inside a basic string are `\"`, `\\`, `\n`, `\t`. Any other backslash escape is a parse error → exit 78. Literal-string form (`'...'`), multi-line basic (`"""..."""`), and multi-line literal (`'''...'''`) are not accepted.
- Boolean form: bare `true` or `false` (lowercase, no quotes). Anything else is a parse error.
- Integer form: one or more decimal digits, no sign, no underscores, no leading `+`/`-`. Used only for `version`.

**Sidecar schema (closed):**

| Key | Type | Required | Reader behavior on absence | Notes |
|---|---|---|---|---|
| `target` | string | yes | exit 78 | Resolved per `runner` slice; opaque to format. |
| `template` | string | yes | exit 78 | Substitution grammar owned by `substitution-engine` slice. |
| `shell` | bool | no | default false | |
| `cwd` | string | no | default `""` (inherit) | Stored literal; runtime expands `~` and `${VAR}`. |
| `description` | string | no | default `""` | Informational. |
| `created` | string | no | omitted | Not validated by runner. |
| `version` | integer | no | default 1 | Reader exits 78 if `version > 1` or `version <= 0`. |

Any other top-level key: warn to stderr (`shrt-runner: <path>: ignoring unknown key '<name>'`) and continue.

**Sidecar filename derivation:**
- Input: shim path from `current_exe()`.
- Rule: case-insensitive trailing `.exe` is replaced with `.shrt`. Path must keep the same parent directory.
- Failure: shim path does not end in `.exe` → exit 78 (`shrt-runner: <path>: shim must end in .exe`).

**Round-trip guarantee:**
- The CLI emits sidecars exclusively through a `serde::Serialize`-derived struct (the closed schema above) and a `toml::ser` configuration that always emits basic strings. Anything the CLI writes is parseable by the runner's reader. Anything the runner cannot parse is rejected with the specified exit code, and the spec §11.1 unit-test suite asserts both directions.

**Reader exit codes (sidecar-related):**

| Exit | Meaning | Triggers |
|---|---|---|
| 66 | Sidecar file does not exist at the derived path. | `current_exe()` succeeded but `.shrt` is absent. |
| 78 | Format violation. | BOM; bad escape; bad bool; bad integer; multi-line or literal string; multiple assignments on one line; missing required key; wrong value type; `version > 1` or `version <= 0`; non-`.exe` shim filename. |

Stderr message shape on parse error: `shrt-runner: <absolute-path>: line <N>: <reason>`. On schema error (post-parse): `shrt-runner: <absolute-path>: <reason>`.

## §5 Sequence

**Write path (CLI, `shrt add`):**
1. CLI builds an in-memory `SidecarConfig` struct (closed schema) with user-supplied `target`, `template`, optional `shell` / `cwd` / `description`, current-time `created`, `version = 1`.
2. CLI serializes the struct via the `toml` crate, configured to emit basic strings only. Result is a UTF-8 string with `\n` line endings and no BOM.
3. CLI writes to `<shim-dir>/<name>.shrt`. Atomicity, parent-dir creation, and ACLs belong to the `shim-management` slice.

**Read path (runner, every shim invocation):**
1. Runner calls `current_exe()` and validates the path ends in `.exe` (case-insensitive). Failure → exit 78.
2. Runner derives the sidecar path by replacing `.exe` with `.shrt`. Reads the file; ENOENT → exit 66; other I/O error → exit 1.
3. Runner verifies no leading BOM. Failure → exit 78.
4. Runner parses line by line: skips blank lines and full-line comments, splits on the first `=`, trims whitespace, decodes the basic-string / bool / integer value per §4. Any violation → exit 78 with the line number.
5. Runner applies the schema: rejects missing `target`/`template` → exit 78; warns on unknown keys; checks `version` → exit 78 if out of range; fills defaults for omitted optional fields.
6. Runner hands the populated config to the substitution stage (owned by the `substitution-engine` slice).

## §6 Out of scope
- How the sidecar is written to disk (atomic rename, ACLs, parent-dir creation, force-overwrite). Belongs to `shim-management`.
- Substitution grammar inside `template`. Belongs to `substitution-engine`.
- Resolution of `target` against PATH at runtime. Belongs to `runner`.
- Expansion semantics of `~` and `${VAR}` in `cwd` at runtime (which env / which home). Belongs to `runner`.
- Validation of shim *names* (path separators, metacharacters). Belongs to `cli-surface` / `shim-management`.
- Cross-platform sidecar filename rules (no `.exe` extension on macOS/Linux). Out per spec §9, deferred.

> If the parent spec is ambiguous on anything this slice depends on, stop and update the spec. Do not invent behavior here.
