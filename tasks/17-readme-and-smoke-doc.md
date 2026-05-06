Parent slice: [shrt — distribution](../slices/distribution.md), [shrt — testing-harness](../slices/testing-harness.md)
Depends on: 13

# Task 17 — README + manual cross-shell smoke checklist

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Write the user-facing `README.md` (install, quickstart, placeholder reference, troubleshooting) per spec §13 acceptance, plus the manual cross-shell smoke checklist per `slices/testing-harness.md` §3 Decision 10.

## Tasks
- [x] Create `README.md` (repo root) with these sections in order: title `# shrt` + tagline; **Install** covering Scoop (primary), direct download from GitHub Releases, and from-source via `cargo install --git ...` plus a one-line `shrt init` step; **Quickstart** walking through the spec §1.2 example (`shrt add wt "copilot -p '/worktree create a worktree for {1}' --yolo"` → `wt "ado item 37839929"`); **Placeholder reference** as a Markdown table mirroring `slices/substitution-engine.md` §4 (`{1}`–`{9}`, `{1?}`, `{INPUT}`, `{@}`, `{ENV:NAME}`, `{ENV:NAME:default}`, `{{`, `}}`); **Troubleshooting** with sub-sections "Shim dir not on PATH" (link to `shrt init`'s output and the manual one-liners), "After upgrading shrt" (run `shrt sync`), and "Diagnosing a broken shim" (run `shrt doctor`); **License** noting `MIT OR Apache-2.0`.
- [x] In `README.md`, every command shown in fenced code blocks must be a real `shrt` subcommand or flag — no fictional commands. Cross-check by ensuring each command appears in `./target/release/shrt.exe --help` output (verified by an Acceptance check below).
- [x] Create `crates/shrt/tests/manual-smoke.md` per `slices/testing-harness.md` §3 Decision 10. Include a checklist for each shell: PowerShell 7, Windows PowerShell 5.1, cmd.exe, Git Bash, VS Code integrated terminal, Windows Terminal — each with the steps "create a `wt` shim via `shrt add`", "invoke `wt foo` and verify the target receives `foo`", "verify exit code propagates". Plus a section "shim as Git external diff/merge driver" per spec §11.3 with the steps to register a shim with `git config diff.<name>.command`.

## Acceptance criteria
- [x] `test -f README.md && test -f crates/shrt/tests/manual-smoke.md`.
- [x] `grep -q 'scoop install shrt' README.md`.
- [x] `grep -q 'cargo install --git' README.md`.
- [x] `grep -qi 'placeholder' README.md && grep -qi 'troubleshooting' README.md`.
- [x] Every line in the README's quickstart that begins with `shrt ` is a real subcommand: `for cmd in $(grep -oE '^shrt [a-z]+' README.md | sort -u); do ./target/release/shrt.exe --help 2>&1 | grep -q "${cmd#shrt }" || (echo "missing: $cmd"; exit 1); done` exits 0 (PowerShell equivalent acceptable).
- [x] `grep -q 'PowerShell 7' crates/shrt/tests/manual-smoke.md && grep -q 'Git Bash' crates/shrt/tests/manual-smoke.md && grep -q 'cmd.exe' crates/shrt/tests/manual-smoke.md`.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
