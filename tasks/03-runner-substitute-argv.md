Parent slice: [shrt — substitution-engine](../slices/substitution-engine.md)
Depends on: 01

# Task 03 — runner substitution engine + CRT argv tokenizer

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Implement the placeholder grammar in `crates/shrt-runner/src/substitute.rs` and the Windows CRT argv tokenizer in `crates/shrt-runner/src/argv.rs`. Both std-only.

## Tasks
- [ ] Create `crates/shrt-runner/src/substitute.rs` with `pub enum SubstError` covering the cases in `slices/substitution-engine.md` §4 error mapping: `MissingArg(usize)` (exit 64), `ArgNotUtf8(usize)` (exit 64), `TemplateParse(usize /*offset*/, &'static str /*reason*/)` (exit 78), `EnvUnset(String)` (exit 78), `EnvNotUtf8(String)` (exit 78). Provide a method returning the spec exit code.
- [ ] In `crates/shrt-runner/src/substitute.rs` implement `pub fn substitute(template: &str, args: &[OsString], env: &dyn Fn(&str) -> Option<OsString>) -> Result<String, SubstError>` per `slices/substitution-engine.md` §4 grammar table: one-pass scanner, support `{1}`–`{9}`, `{1?}`–`{9?}`, `{INPUT}`, `{@}`, `{ENV:NAME}`, `{ENV:NAME:default}`, `{{`, `}}`. ENV name regex `[A-Za-z_][A-Za-z0-9_]*` length ≥ 1. Default value is literal up to first `}` and may contain `:`. Whitespace inside `{...}` is a parse error.
- [ ] In `crates/shrt-runner/src/substitute.rs` implement an internal helper `crt_quote_arg(arg: &str) -> String` per `slices/substitution-engine.md` §4 CRT-quoting algorithm (the 4-step procedure): empty → `""`; no special chars → unquoted; otherwise wrap in `"..."` with backslash-doubling rule for runs preceding `"` or end-of-arg, and `"` escaped as `\"`. Used only by `{@}`.
- [ ] Create `crates/shrt-runner/src/argv.rs` implementing `pub fn tokenize(line: &str) -> Vec<String>` per the CRT tokenizer rules in `slices/substitution-engine.md` §4: whitespace splits unquoted args; `"..."` toggles quote state with the 2N / 2N+1 backslash-reduction rule; lenient on unterminated quote (open quote runs to end-of-string).
- [ ] Add `#[cfg(test)] mod tests` in both files. `substitute.rs` tests must cover every placeholder form, missing `{1}` exits 64, missing `{ENV:UNSET}` exits 78, default fallback works, `{{`/`}}` literals, `{ENV:}` rejected, whitespace inside `{...}` rejected, `{@}` quoting on args containing spaces and quotes. `argv.rs` tests must cover: `a b c` → 3 args; `"a b c"` → 1 arg; `\\\"x` → `"x`; `\\\\` → `\\`; trailing unterminated `"` (lenient); empty input.
- [ ] Add `mod substitute;` and `mod argv;` to `crates/shrt-runner/src/main.rs`.

## Acceptance criteria
- [ ] `cargo build -p shrt-runner` exits 0.
- [ ] `cargo test -p shrt-runner substitute::tests` passes.
- [ ] `cargo test -p shrt-runner argv::tests` passes.
- [ ] `test -f crates/shrt-runner/src/substitute.rs && test -f crates/shrt-runner/src/argv.rs`.
- [ ] `grep -q 'pub fn substitute' crates/shrt-runner/src/substitute.rs && grep -q 'pub fn tokenize' crates/shrt-runner/src/argv.rs`.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
