Parent slice: [shrt — cli-surface](../slices/cli-surface.md), [shrt — shim-management](../slices/shim-management.md)
Depends on: 10, 12

# Task 13 — shrt doctor command (4-check diagnostic)

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Implement `shrt doctor` running the four checks from `slices/cli-surface.md` §3 Decision 17 plus the ACL-deferral warning, with text + JSON output and the right exit code on summary.

## Tasks
- [x] In `crates/shrt/src/shim.rs` declare `pub enum Status { Ok, Warn, Fail }` with `#[derive(Serialize)] #[serde(rename_all = "lowercase")]`. Declare `pub struct Check { name: String, status: Status, message: String }` and `pub struct DoctorReport { summary: Status, checks: Vec<Check> }` with `#[derive(Serialize)]`.
- [x] In `crates/shrt/src/shim.rs` implement `pub fn doctor(ctx: &Ctx) -> anyhow::Result<DoctorReport>` per `slices/shim-management.md` §5 doctor flow: (a) PATH check via `paths::is_on_path`; (b) for each shim, parse via `config::read_sidecar` (`Check { name: format!("{name}: parse"), ... }`); (c) for each shim, byte-compare `<name>.exe` against `ctx.runner_bytes` (`Check { name: format!("{name}: bytes"), ... }`, fail message suggests `shrt sync`); (d) for each shim, `which::which(&entry.target)` (`Check { name: format!("{name}: target"), ... }`). Add a `Warn`-status check `{ name: "acls", message: "Windows user-only ACLs deferred to v0.2" }` per `slices/shim-management.md` §3 Decision 5. Aggregate summary: `Fail` if any check Fails; `Warn` if any Warn but none Fail; else `Ok`.
- [x] Replace the stub in `crates/shrt/src/commands/doctor.rs`: call `shim::doctor`. JSON mode prints the `DoctorReport`. Text mode prints `[OK] ...`, `[WARN] ...`, `[FAIL] ...` lines. Exit code: 0 on `Ok`, 0 on `Warn`, 1 on `Fail`.
- [x] Create `crates/shrt/tests/doctor.rs` test `doctor_reports_mixed_state`: in a tempdir, set up three shims via `add_stub_shim` — (i) a good shim pointing at the stub binary; (ii) a shim with a deliberately corrupted sidecar (overwrite `<name>.shrt` with `not toml`); (iii) a shim whose sidecar names a non-existent target. Run `shrt doctor --shim-dir <temp> --json`. Parse the report. Assert `summary == "fail"`. Assert the Vec of checks contains exactly the expected `parse`/`target` failures and that the good shim's checks are all `ok`. Assert the `acls` warn check is present.

## Acceptance criteria
- [x] `cargo test -p shrt --test doctor` passes.
- [x] `test -f crates/shrt/tests/doctor.rs`.
- [x] `grep -q 'pub fn doctor' crates/shrt/src/shim.rs && grep -q 'pub struct DoctorReport' crates/shrt/src/shim.rs`.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
