Parent slice: [shrt — distribution](../slices/distribution.md)
Depends on: 15

# Task 16 — release workflow + bundle-runner-src script + Scoop manifest job

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Author the publish-time bundling script, the GitHub Actions release workflow that uploads x64+ARM64 binaries on `v*` tags, and the cross-repo Scoop manifest update.

## Tasks
- [x] Create `scripts/bundle-runner-src.ps1` per `slices/distribution.md` §4: `Remove-Item -Recurse -Force` `crates/shrt/runner-src` if present; `New-Item -ItemType Directory crates/shrt/runner-src/src`; `Copy-Item crates/shrt-runner/Cargo.toml crates/shrt/runner-src/Cargo.toml`; `Copy-Item crates/shrt-runner/src/* crates/shrt/runner-src/src/ -Recurse`. Idempotent. Run from repo root.
- [x] Create `.github/workflows/release.yml` with `name: release`. Trigger: `on: push: tags: ['v*']`.
- [x] Job `build-x64` runs on `windows-latest`: checkout → setup-rust → `cargo build -p shrt --release --target=x86_64-pc-windows-msvc --locked` → upload artifact `shrt-x86_64.exe` from `target/x86_64-pc-windows-msvc/release/shrt.exe`.
- [x] Job `build-arm64` runs on `windows-11-arm`: as above with `--target=aarch64-pc-windows-msvc`, artifact `shrt-aarch64.exe`.
- [x] Job `release` runs on `ubuntu-latest`, `needs: [build-x64, build-arm64]`: download both artifacts; rename to `shrt-${{ github.ref_name }}-x86_64-pc-windows-msvc.exe` and `shrt-${{ github.ref_name }}-aarch64-pc-windows-msvc.exe`; compute `sha256sums.txt` covering both; `gh release create ${{ github.ref_name }}` with all three assets attached. Use `${{ secrets.GITHUB_TOKEN }}`.
- [x] Job `publish-crates` runs on `windows-latest`, `needs: release`: checkout → setup-rust → `cargo publish -p shrt-runner --token ${{ secrets.CARGO_REGISTRY_TOKEN }}` → `pwsh scripts/bundle-runner-src.ps1` → `cargo publish -p shrt --token ${{ secrets.CARGO_REGISTRY_TOKEN }}`.
- [x] Job `update-scoop` runs on `ubuntu-latest`, `needs: release`: clone `PatrickRuddiman/PersonalScoopBucket` via deploy key (or fine-grained PAT in `secrets.SCOOP_BUCKET_TOKEN`); render `bucket/shrt.json` per `slices/distribution.md` §4 manifest shape (version, description, homepage, license, architecture.64bit/arm64 url+hash, bin, post_install); commit and push.
- [x] Test the bundling script locally: from the repo root, run `pwsh scripts/bundle-runner-src.ps1`. Verify `crates/shrt/runner-src/Cargo.toml` and `crates/shrt/runner-src/src/main.rs` (and any other runner source files) exist after the run. Then run `cargo build -p shrt --release` and verify it still succeeds (proves `build.rs` correctly probes `runner-src/` first when present).
- [x] Add `crates/shrt/runner-src/` to `.gitignore` (already done in task 01; verify it is present).

## Acceptance criteria
- [x] `test -f scripts/bundle-runner-src.ps1 && test -f .github/workflows/release.yml`.
- [x] `pwsh scripts/bundle-runner-src.ps1` exits 0.
- [x] After running the script: `test -f crates/shrt/runner-src/Cargo.toml && test -f crates/shrt/runner-src/src/main.rs`.
- [x] After running the script: `cargo build -p shrt --release --locked` exits 0.
- [x] `grep -q 'cargo publish' .github/workflows/release.yml && grep -q 'aarch64-pc-windows-msvc' .github/workflows/release.yml && grep -q 'bundle-runner-src.ps1' .github/workflows/release.yml`.
- [x] After the bundling script + build, `cargo test --workspace --locked` (the same gate as CI) still passes.

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
