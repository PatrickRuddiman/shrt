Parent slice: [shrt — distribution](../slices/distribution.md)
Depends on: 15

# Task 16 — release workflow (GitHub Releases only)

_Tick `[x]` on each Tasks item as you finish it, and on each Acceptance item as it passes. The unticked state is what tells the next planning run that this task is still safe to edit in place._

## Goal
Author the GitHub Actions release workflow that builds x64 + ARM64 binaries on `v*` tags and uploads them as a GitHub Release with `sha256sums.txt`. Scoop bucket update is bucket-side via `scoop checkver` (no cross-repo push from here). crates.io publishing is dropped per `slices/distribution.md` §3 Decision 1.

## Tasks
- [x] Create `.github/workflows/release.yml` with `name: release`. Trigger: `on: push: tags: ['v*']`.
- [x] Job `build-x64` runs on `windows-latest`: checkout → setup-rust → `cargo build -p shrt --release --target=x86_64-pc-windows-msvc --locked` → upload artifact `shrt-x86_64.exe` from `target/x86_64-pc-windows-msvc/release/shrt.exe`.
- [x] Job `build-arm64` runs on `windows-11-arm`: as above with `--target=aarch64-pc-windows-msvc`, artifact `shrt-aarch64.exe`.
- [x] Job `release` runs on `ubuntu-latest`, `needs: [build-x64, build-arm64]`, `permissions: contents: write`: download both artifacts; rename to `shrt-${{ github.ref_name }}-x86_64-pc-windows-msvc.exe` and `shrt-${{ github.ref_name }}-aarch64-pc-windows-msvc.exe`; compute `sha256sums.txt` covering both; `gh release create ${{ github.ref_name }}` with all three assets attached. Use `${{ secrets.GITHUB_TOKEN }}`.
- [x] (Not implemented) Cross-repo `update-scoop` job. Bucket-side autoupdate is the chosen mechanism — `PatrickRuddiman/PersonalScoopBucket/bucket/shrt.json` declares `checkver` + `autoupdate` blocks pointing at this repo's GitHub Releases and `sha256sums.txt`, and the bucket repo runs `scoop checkver -u shrt` on its own schedule.
- [x] (Not implemented) `publish-crates` job. crates.io publishing is dropped — installs are via Scoop, prebuilt binary download, or `cargo install --git`.

## Acceptance criteria
- [x] `test -f .github/workflows/release.yml`.
- [x] `grep -q 'aarch64-pc-windows-msvc' .github/workflows/release.yml && grep -q 'gh release create' .github/workflows/release.yml`.
- [x] `cargo test --workspace --locked` passes (same gate CI runs).

> If a `## Tasks` checkbox can't be completed without changing what the parent slice specifies, stop and update the slice. Do not redesign here.
