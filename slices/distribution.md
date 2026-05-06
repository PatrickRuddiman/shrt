Parent spec: [shrt — Specification](../spec.md)

# shrt — distribution

## §1 Summary
Owns publish channels (GitHub Releases prebuilt binaries + a personal Scoop bucket), the release artifact matrix, the Scoop autoupdate hookup, and license + repo metadata. crates.io publishing is explicitly out of scope (Decision 1).

## §2 Codebase reconnaissance
> Greenfield: no existing system to reconcile. Decisions below are unconstrained.

Sibling slices this slice fixes or feeds:
- `slices/build-pipeline.md` — `build.rs` invocation form and the workspace `Cargo.toml` profile knobs. This slice retroactively updates that slice's §3 Decision 2 (`--manifest-path` instead of `-p`) and §3 Decision 7 (profile declared in both workspace AND runner crate).
- `slices/runner.md` — runner binary size budget enforced via the release profile.

## §3 Decisions
1. **Distribution channels.** Options: crates.io + GitHub Releases + Scoop / GitHub Releases + Scoop only. **Chosen:** GitHub Releases + Scoop only. Rationale: spec §8.1's crates.io channel was dropped — author doesn't want crates.io maintenance overhead (ownership, versioning lifecycle, deprecations); prebuilt binaries in Releases plus a personal Scoop bucket cover every install path the spec actually cares about.
2. **`build.rs` invocation form.** **Chosen:** `cargo build --manifest-path=../shrt-runner/Cargo.toml --release --target=$TARGET --target-dir=$OUT_DIR/runner-target`. Rationale: with crates.io off the table, the runner is always a workspace sibling; no two-context probe needed.
3. **Profile-knob redundancy.** **Chosen:** declare `[profile.release]` (with the size-stripping cocktail from `build-pipeline`) in BOTH the workspace `Cargo.toml` AND `crates/shrt-runner/Cargo.toml`. Rationale: harmless redundancy; protects against accidental standalone runner builds.
4. **GitHub Releases asset matrix.** Options: x64 only / x64 + ARM64 / x64 + ARM64 + standalone-runner. **Chosen:** x64 + ARM64 `shrt.exe` plus a `sha256sums.txt`. Rationale: matches spec §8.1; standalone runner has no consumer for v0.1.
5. **Asset naming.** **Chosen:** `shrt-vX.Y.Z-x86_64-pc-windows-msvc.exe`, `shrt-vX.Y.Z-aarch64-pc-windows-msvc.exe`, `sha256sums.txt`. Rationale: triple-suffixed naming is conventional and survives Scoop's `architecture` mapping cleanly.
6. **Scoop bucket repo.** **Chosen:** existing repo `PatrickRuddiman/PersonalScoopBucket`, manifest at `bucket/shrt.json`. Rationale: author maintains one personal Scoop bucket for all their tools; no point in a per-project bucket.
7. **Scoop manifest auto-update.** Options: manual edits in the bucket / cross-repo push from this workflow / bucket-side `scoop checkver` autoupdate. **Chosen:** **bucket-side autoupdate** — `bucket/shrt.json` in `PatrickRuddiman/PersonalScoopBucket` declares `checkver` + `autoupdate` blocks so `scoop checkver -u shrt` (run periodically by a workflow in the bucket repo, or on demand) bumps the version and resolves the per-arch hashes from `sha256sums.txt` in the latest GitHub Release. Rationale: keeps the bucket self-contained; this repo's release workflow only has to publish the release artifacts; no cross-repo PAT to manage here.
8. **winget.** **Chosen:** out of scope per spec §8.1 (post-1.0). Defer.
9. **Versioning policy.** **Chosen:** SemVer; tag format `vX.Y.Z`; one workspace-level `[workspace.package].version` shared by both crates; sidecar `version` field is independent (owned by `sidecar-format`). Rationale: matches spec §8.2.
10. **License.** Options: MIT / Apache-2.0 / `MIT OR Apache-2.0`. **Chosen:** `MIT OR Apache-2.0`. Rationale: Rust community default; broadest compatibility.
11. **Repo URL.** **Chosen:** `https://github.com/PatrickRuddiman/shrt`. Rationale: matches the canonical repo location; CI workflow and Cargo manifests reference the same URL.
12. **README.md location.** **Chosen:** repo root, single file. Required by spec §13. Rationale: standard.
13. **Code-signing.** **Chosen:** out of scope. Rationale: not in spec.
14. **From-source install.** **Chosen:** advertise `cargo install --git https://github.com/PatrickRuddiman/shrt --locked shrt` in the README for users who want a source build without crates.io. Rationale: zero infrastructure cost; cargo handles git fetch + workspace build natively.

## §4 Contracts & shapes

**Repo layout additions for distribution:**
```
shrt/
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── .github/workflows/
│   ├── ci.yml                 # owned by build-pipeline
│   └── release.yml            # owned by this slice
└── crates/
    ├── shrt/
    │   └── Cargo.toml
    └── shrt-runner/
        └── Cargo.toml         # has its own [profile.release]
```

**Workspace `Cargo.toml` shared metadata** (added by this slice, alongside `[profile.release]` from build-pipeline):
```
[workspace.package]
version       = "0.1.0"
edition       = "2021"
authors       = ["Patrick Ruddiman"]
license       = "MIT OR Apache-2.0"
repository    = "https://github.com/PatrickRuddiman/shrt"
homepage      = "https://github.com/PatrickRuddiman/shrt"
description   = "Parameterized command shortcuts for Windows."
keywords      = ["windows", "cli", "shim", "alias", "shortcut"]
categories    = ["command-line-utilities", "development-tools"]
```

Each crate's `Cargo.toml` inherits via `version.workspace = true`, etc.

**`.github/workflows/release.yml` shape (separate from `ci.yml`):**

Triggers: `push` on tags matching `v*`.

Jobs:

| Job | Runs on | Steps |
|---|---|---|
| `build-x64` | `windows-latest` | checkout → setup-rust stable → `cargo build -p shrt --release --target x86_64-pc-windows-msvc` → upload artifact `shrt-x86_64.exe` |
| `build-arm64` | `windows-11-arm` | as above with `--target aarch64-pc-windows-msvc` |
| `release` | `ubuntu-latest`, `needs: [build-x64, build-arm64]` | gather artifacts → rename to release scheme → compute `sha256sums.txt` → `gh release create vX.Y.Z --title ... --notes ... <assets>` |

Scoop bucket update is out-of-band: the bucket repo's own `scoop checkver -u shrt` workflow (manual or scheduled) discovers the new release via `checkver`+`autoupdate` blocks and resolves hashes from `sha256sums.txt`. crates.io publishing is dropped entirely.

**Scoop manifest shape (`bucket/shrt.json`, generated):**
```
{
  "version": "X.Y.Z",
  "description": "Parameterized command shortcuts for Windows.",
  "homepage": "https://github.com/PatrickRuddiman/shrt",
  "license": "MIT OR Apache-2.0",
  "architecture": {
    "64bit": {
      "url": "https://github.com/PatrickRuddiman/shrt/releases/download/vX.Y.Z/shrt-vX.Y.Z-x86_64-pc-windows-msvc.exe#/shrt.exe",
      "hash": "<sha256>"
    },
    "arm64": {
      "url": "https://github.com/PatrickRuddiman/shrt/releases/download/vX.Y.Z/shrt-vX.Y.Z-aarch64-pc-windows-msvc.exe#/shrt.exe",
      "hash": "<sha256>"
    }
  },
  "bin": "shrt.exe",
  "post_install": "Write-Host 'Run `shrt init` to set up the shim directory.'"
}
```

The `#/shrt.exe` URL fragment renames the downloaded file to `shrt.exe` on disk per Scoop convention.

## §5 Sequence

**Tag a release (`git tag vX.Y.Z && git push --tags`):**
1. `release.yml` triggers.
2. `build-x64` and `build-arm64` jobs run in parallel; each produces `shrt.exe` for its target. Artifacts uploaded.
3. `release` job downloads both artifacts, renames per the asset-naming scheme, generates `sha256sums.txt`, creates the GitHub Release with all three assets attached.
4. (Out-of-band) The `PatrickRuddiman/PersonalScoopBucket` repo's `scoop checkver -u shrt` workflow picks up the new GitHub Release on its next run and bumps `bucket/shrt.json` (version + per-arch hashes from `sha256sums.txt`).

**End-user `scoop install shrt`** (after the bucket is added):
1. Scoop fetches the prebuilt `shrt-vX.Y.Z-<arch>.exe` from the GitHub Release for the user's arch, verifies SHA256, places at `~/scoop/apps/shrt/<version>/shrt.exe`, shims into `~/scoop/shims/shrt.exe`.
2. `post_install` prints the `shrt init` reminder.

**End-user from-source install** (`cargo install --git https://github.com/PatrickRuddiman/shrt --locked shrt`):
1. Cargo clones the repo, builds `shrt` in workspace context (build.rs probes `../shrt-runner/Cargo.toml`, builds the runner per-target, embeds the bytes via `include_bytes!`).
2. Resulting `shrt.exe` is installed to `~/.cargo/bin/`.

**End-user upgrade flow** (matches spec §8.3):
1. `scoop update shrt` (or re-download the binary from Releases, or re-run `cargo install --git ... --force`).
2. `shrt sync` rewrites every existing shim's `.exe` with the fresh `RUNNER_BYTES`.
3. Optional: `shrt doctor` confirms all shims now byte-match.

## §6 Out of scope
- Workspace topology, build.rs subprocess mechanics, profile knob list. Owned by `build-pipeline`.
- Filesystem mechanics of `shrt sync` rewriting shim `.exe` files. Owned by `shim-management`.
- The actual content of the README's quickstart/troubleshooting sections. Outside this slice; produced as a v0.1 deliverable.
- `winget` manifest. Spec §8.1 post-1.0.
- Code-signing certificates. Not in spec.
- Cross-platform release matrix (macOS/Linux). Spec §9 defers cross-platform entirely.
- Rolling back a botched release (e.g. `cargo yank`). Manual ops, not a slice concern.

> If the parent spec is ambiguous on anything this slice depends on, stop and update the spec. Do not invent behavior here.
