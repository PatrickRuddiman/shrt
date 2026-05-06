Parent spec: [shrt — Specification](../spec.md)

# shrt — substitution-engine

## §1 Summary
Owns the placeholder language inside the sidecar's `template` field and the post-substitution Windows argv tokenizer. Defines the exact grammar accepted, every resolution rule, and the runner-side exit codes for each failure class. Does not own how the substituted output is launched as a process — that's the `runner` slice.

## §2 Codebase reconnaissance
> Greenfield: no existing system to reconcile. Decisions below are unconstrained.

Sibling slice `sidecar-format.md` defines `template` as a UTF-8 basic-string with `\"`, `\\`, `\n`, `\t` as the only in-string escapes. This slice consumes that already-decoded string.

## §3 Decisions
1. **Placeholder set.** Options: spec set exactly / superset (e.g. `{N}` for any N) / programmable. **Chosen:** exactly `{1}`…`{9}`, `{1?}`…`{9?}`, `{INPUT}`, `{@}`, `{ENV:NAME}`, `{ENV:NAME:default}`, `{{`, `}}`. Rationale: matches spec §4.1 + §13 acceptance.
2. **ENV name grammar.** Options: any non-`}` non-`:` / `[A-Za-z_][A-Za-z0-9_]*` / OS-specific. **Chosen:** `[A-Za-z_][A-Za-z0-9_]*`, length ≥ 1. Rationale: standard env-var conventions; `{ENV:}` rejected at parse time.
3. **Default-value content in `{ENV:NAME:default}`.** Options: literal up to `}` / escaped `}` allowed / no defaults. **Chosen:** literal text from after the second `:` to the first `}`; `:` allowed inside default; `}` not allowed (no escape mechanism inside placeholders). Rationale: keeps the runner scanner one-pass; users wanting `}` can manage it via env.
4. **Whitespace inside placeholders.** Options: stripped / preserved / forbidden. **Chosen:** forbidden — `{ 1 }` is a parse error. Rationale: removes ambiguity; templates are short and authored intentionally.
5. **`{N}` and `{N?}` substitution mode.** Options: raw / shell-quoted / context-aware. **Chosen:** raw text, exactly the user's arg. Rationale: matches spec §4.4 worked example; quoting belongs to the template author or to `{@}`.
6. **`{@}` quoting algorithm.** Options: handcrafted / `winapi`-call / CRT exact. **Chosen:** Microsoft CRT argument-quoting, hand-implemented (no winapi dep). Rationale: spec §4.2 mandates CRT-exact behavior; runner is std-only.
7. **`{INPUT}` join separator.** Options: single space / configurable / no-op. **Chosen:** single ASCII space `0x20`; zero args → empty string. Rationale: matches spec §4.1.
8. **Argv tokenizer post-substitution (shell=false).** Options: hand-rolled CRT-exact / call `CommandLineToArgvW` / split-on-whitespace. **Chosen:** hand-rolled CRT-exact in `crates/shrt-runner/src/argv.rs`. Rationale: spec §4.2 mandates exact CRT match; runner is std-only.
9. **Shell=true assembly.** Options: concatenate target + substituted with auto-quote / concatenate raw / per-arg array. **Chosen:** concatenate `<target> <substituted_line>` and pass as the single string argument after `/c`; no auto-escaping. Rationale: matches spec §4.2 ("user is responsible for safe input").
10. **Code layout.** Options: single file / two files in runner / shared `shrt-core` lib crate. **Chosen:** `crates/shrt-runner/src/substitute.rs` + `crates/shrt-runner/src/argv.rs`. Rationale: only one current callsite (the runner); CLI does not validate templates in v0.1; no shared crate until a second consumer exists.
11. **CLI-side validation at `shrt add`.** Options: parse template at add / accept verbatim / lint-only flag. **Chosen:** accept verbatim, no validation. Rationale: avoids duplicating the parser or premature `shrt-core` extraction; errors surface on first shim invocation.
12. **Non-UTF-8 user arg.** Options: lossy / strict-error / pass through as bytes. **Chosen:** strict-error → exit 64 with the offending arg index. Rationale: Windows argv is UTF-16 and round-trips losslessly to UTF-8 in practice; failure is exceptional.
13. **Non-UTF-8 env-var value.** Options: lossy / strict-error / treat as missing. **Chosen:** strict-error → exit 78 naming the variable. Rationale: same as Decision 12; an unreadable env value is a config error, not an arg error.
14. **`{ENV:NAME}` unset.** Options: empty / error. **Chosen:** error, exit 78 naming the variable. Rationale: matches spec §4.1.
15. **`{N}` missing arg.** Options: empty / error. **Chosen:** error, exit 64 naming the index and arg count. Rationale: matches spec §4.1.
16. **Template parse-error class.** Options: exit 64 / exit 78 / exit 1. **Chosen:** exit 78. Rationale: a malformed template is a sidecar-content (config) defect, identical class to `sidecar-format` parse failures; consistent stderr message shape.
17. **Substitution output shape.** Options: argv vector / single string / either-or by `shell` flag. **Chosen:** single owned `String`. Rationale: shell=true and shell=false branches both consume one string (one as cmd-line, one as tokenizer input); a unified output keeps the substitution module shell-agnostic.
18. **Stderr message format.** Options: prefixed / structured / minimal. **Chosen:** `shrt-runner: <sidecar-path>: <reason>` for sidecar-bound errors and `shrt-runner: <sidecar-path>: template offset <N>: <reason>` for in-template parse errors. Rationale: matches the convention locked in `sidecar-format` slice §4.

## §4 Contracts & shapes

**Placeholder grammar (post-TOML-decode, applied to the `template` string):**

| Form | Matches | Resolves to |
|---|---|---|
| `{N}` | N is a single digit `1`–`9` | `args[N-1]` as raw text; missing → exit 64 |
| `{N?}` | N is a single digit `1`–`9` | `args[N-1]` as raw text; missing → empty string |
| `{INPUT}` | literal | `args` joined by single ASCII space `0x20`; zero args → empty string |
| `{@}` | literal | each `args[i]` CRT-quoted, joined by single space; zero args → empty string |
| `{ENV:NAME}` | `NAME` matches `[A-Za-z_][A-Za-z0-9_]*` | env value; unset → exit 78; non-UTF-8 → exit 78 |
| `{ENV:NAME:default}` | as above plus literal default | env value if set+UTF-8; otherwise the default text |
| `{{` | two literal `{` | one literal `{` |
| `}}` | two literal `}` | one literal `}` |

Anything else of the form `{...}` (or an unmatched `{` or `}`) is a template parse error → exit 78.

**Reserved invariants of the scanner:**
- Single forward pass over the template; no lookbehind beyond two characters.
- A `{` not followed by `{` begins a placeholder; the placeholder ends at the first `}`. If the content between does not match the table above, exit 78.
- A `}` not preceded by `}` outside a placeholder is a parse error → exit 78.
- Whitespace inside a placeholder is a parse error.

**CRT argv-quoting algorithm for `{@}` (each arg, then space-join):**
1. If the arg is empty, emit `""`.
2. If the arg contains no space, tab, `"`, or `\`, emit it unquoted.
3. Otherwise: emit `"`, then walk the arg producing output:
   - On a run of `\` followed by `"` or end-of-arg: emit `2N` backslashes, then either `\"` (for the quote) or end the loop; the closing `"` then follows.
   - On `\` not followed by `"`: emit one `\`.
   - On `"`: emit `\"`.
   - On any other byte: emit it.
4. Append closing `"`.

**CRT argv-tokenizer (post-substitution, shell=false):**
1. Skip leading whitespace.
2. Begin a new arg. State: `in_quotes = false`, output buffer empty.
3. Walk byte-by-byte:
   - On `"`: if the previous run of `\` had length 2N, those 2N produce N `\` and the `"` toggles `in_quotes`. If 2N+1, those 2N+1 produce N `\` followed by a literal `"` in the arg.
   - On `\`: defer; counted as part of a `\`-run.
   - On whitespace when `in_quotes = false`: emit the arg (with any pending backslashes flushed as literals), reset, skip further whitespace, return to step 2.
   - On any other byte: flush pending backslashes as literals, emit the byte.
4. End-of-input: flush pending backslashes; if a current arg is non-empty or any quote was seen, emit it. Unterminated `"` is **not** an error (matches CRT lenient behavior); the open quote simply runs to end-of-string.

Argv produced by the tokenizer is the runner's process-launch argv (with `target` prepended). The substitution module returns only the pre-tokenize string.

**Cross-module call sites:**
- `crates/shrt-runner/src/substitute.rs::substitute(template, args, env_lookup) -> Result<String, SubstError>` — the only public entry from this slice's substitution side.
- `crates/shrt-runner/src/argv.rs::tokenize(line) -> Vec<String>` — pure function; total (no failure mode).

**Error mapping:**

| Cause | Exit |
|---|---|
| Template parse failure (malformed placeholder, bad ENV name, unmatched `{` or `}`, whitespace in placeholder) | 78 |
| `{N}` references arg index N but `args.len() < N` | 64 |
| `{ENV:NAME}` and the var is unset | 78 |
| Env-var or user-arg contains non-UTF-8 | 78 / 64 (see Decisions 12, 13, 14) |

## §5 Sequence

**Substitution path (every shim invocation, after sidecar parse):**
1. Runner has the decoded `template: String`, `args: &[OsString]`, and a closure for env lookup.
2. `substitute()` walks `template` once, emitting literal segments and resolving each placeholder per §4. On any resolution failure, returns the appropriate exit-code variant; runner converts to process exit immediately.
3. `substitute()` returns a single `String`: the pre-tokenized command-line.
4. Runner branches on `config.shell`:
   - `shell = false`: passes the string to `argv::tokenize()`; prepends `config.target`; result is the spawn argv.
   - `shell = true`: concatenates `format!("{target} {substituted}")`; result is the single string argument to `cmd /c`.
5. Runner hands off to the process-spawn stage (owned by `runner` slice).

## §6 Out of scope
- Process spawn, stdio inheritance, exit-code propagation. Belongs to `runner`.
- Resolution of `target` against PATH. Belongs to `runner`.
- The on-disk encoding of `template` (escapes, line-end, encoding). Belongs to `sidecar-format`.
- Any CLI-side template parsing or `shrt add` lint. Deferred to v0.2 if a second consumer materializes (per Decision 11).

> If the parent spec is ambiguous on anything this slice depends on, stop and update the spec. Do not invent behavior here.
