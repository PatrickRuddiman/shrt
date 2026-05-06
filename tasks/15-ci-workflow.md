Parent slice: [shrt — build-pipeline](../slices/build-pipeline.md)
Depends on: 01

# Task 15 — CI workflow (.github/workflows/ci.yml)

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Author the GitHub Actions CI workflow that builds and tests the workspace on PR/push for both Windows architectures per `slices/build-pipeline.md` §4.

## Tasks
- [ ] Create `.github/workflows/ci.yml` with `name: ci`. Triggers: `on: { push: { branches: [main] }, pull_request: {} }`.
- [ ] Add a single job `test` with `strategy.fail-fast: false` and `matrix.os: [windows-latest, windows-11-arm]`. `runs-on: ${{ matrix.os }}`.
- [ ] Steps in the `test` job: `actions/checkout@v4`; install stable Rust via `dtolnay/rust-toolchain@stable` (or equivalent action that respects `rust-toolchain.toml`); cache cargo via `Swatinem/rust-cache@v2`; run `cargo test --workspace --locked`.
- [ ] Do NOT include any release-asset upload, crates.io publish, or Scoop manifest steps — those belong to task 16's release workflow.
- [ ] Verify the workflow's local equivalent passes by running `cargo test --workspace --locked` from the repo root before marking this task done.

## Acceptance criteria
- [ ] `test -f .github/workflows/ci.yml`.
- [ ] `grep -q 'cargo test --workspace' .github/workflows/ci.yml`.
- [ ] `grep -q 'windows-latest' .github/workflows/ci.yml && grep -q 'windows-11-arm' .github/workflows/ci.yml`.
- [ ] `grep -q 'actions/checkout' .github/workflows/ci.yml`.
- [ ] `cargo test --workspace --locked` (the same command CI will run) exits 0 locally.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
