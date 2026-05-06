Parent spec: [shrt — Specification](../spec.md)

# shrt — distribution

## §1 Summary
Owns publish channels (crates.io, GitHub Releases, Scoop), the publish-time bundling of runner sources into the `shrt` crate so `cargo install shrt` works from a single-crate fetch, the release artifact matrix, the Scoop manifest auto-update, and license + repo metadata. Resolves the wrinkle deferred from `build-pipeline` §3 Decision 13.

## §2 Codebase reconnaissance
> Greenfield: no existing system to reconcile. Decisions below are unconstrained.

Sibling slices this slice fixes or feeds:
- `slices/build-pipeline.md` — `build.rs` invocation form and the workspace `Cargo.toml` profile knobs. This slice retroactively updates that slice's §3 Decision 2 (`--manifest-path` instead of `-p`) and §3 Decision 7 (profile declared in both workspace AND runner crate).
- `slices/runner.md` — runner binary size budget enforced via the release profile.

## §3 Decisions
1. **crates.io strategy.** Options: publish only `shrt` with bundled runner sources / publish both crates with `shrt` depending on `shrt-runner` via path-fallback / artifact-deps. **Chosen:** publish both crates separately AND bundle runner sources inside the `shrt` crate at publish time. Rationale: `shrt-runner` is already a workspace member with no deps, publishing it is free; bundling makes `shrt` self-contained (build.rs doesn't need `shrt-runner` to be a workspace member or a regular dep).
2. **Bundling location.** Options: `crates/shrt/runner-src/` written by CI / `runner/` directory committed / a binary blob committed. **Chosen:** `crates/shrt/runner-src/` populated by the publish script just before `cargo publish -p shrt`; not committed. `crates/shrt/Cargo.toml` declares `include = ["src/**", "build.rs", "runner-src/**", "Cargo.toml", "README.md"]`. Rationale: source repo stays clean; published crate is self-contained.
3. **`build.rs` two-context probe.** **Chosen:** probe `<CARGO_MANIFEST_DIR>/runner-src/Cargo.toml` first (published-crate context), else `<CARGO_MANIFEST_DIR>/../shrt-runner/Cargo.toml` (workspace context), else hard-error. Rationale: a single build.rs handles both contexts without conditional compilation.
4. **`build.rs` invocation form.** **Chosen:** `cargo build --manifest-path=<probed> --release --target=$TARGET --target-dir=$OUT_DIR/runner-target`. Rationale: `-p shrt-runner` requires the runner to be a workspace member, which it isn't in the published-crate context; `--manifest-path` works in both.
5. **Profile-knob redundancy.** **Chosen:** declare `[profile.release]` (with the size-stripping cocktail from `build-pipeline`) in BOTH the workspace `Cargo.toml` AND `crates/shrt-runner/Cargo.toml`. Rationale: the published-crate build runs without a workspace; the runner crate's own `[profile.release]` is the active one there.
6. **GitHub Releases asset matrix.** Options: x64 only / x64 + ARM64 / x64 + ARM64 + standalone-runner. **Chosen:** x64 + ARM64 `shrt.exe` plus a `sha256sums.txt`. Rationale: matches spec §8.1; standalone runner has no consumer for v0.1.
7. **Asset naming.** **Chosen:** `shrt-vX.Y.Z-x86_64-pc-windows-msvc.exe`, `shrt-vX.Y.Z-aarch64-pc-windows-msvc.exe`, `sha256sums.txt`. Rationale: triple-suffixed naming is conventional and survives Scoop's `architecture` mapping cleanly.
8. **Scoop bucket repo.** **Chosen:** separate repo `PatrickRuddiman/PersonalScoopBucket`, manifest at `bucket/shrt.json`. Rationale: spec §8.1 says "project bucket initially"; matches Scoop bucket convention.
9. **Scoop manifest auto-update.** Options: manual PR / GitHub Actions cross-repo push / Dependabot-style. **Chosen:** GH Action in `PatrickRuddiman/shrt` that, after the GitHub Release is published, generates the JSON and pushes a commit (or PR) to the bucket repo via a deploy key or fine-grained PAT. Rationale: zero manual steps after `git push --tags`.
10. **winget.** **Chosen:** out of scope per spec §8.1 (post-1.0). | Defer.
11. **Versioning policy.** **Chosen:** SemVer; tag format `vX.Y.Z`; one workspace-level `[workspace.package].version` shared by both crates; sidecar `version` field is independent (owned by `sidecar-format`). Rationale: matches spec §8.2.
12. **License.** Options: MIT / Apache-2.0 / `MIT OR Apache-2.0`. **Chosen:** `MIT OR Apache-2.0`. Rationale: Rust community default; broadest compatibility.
13. **Repo URL placeholder.** **Chosen:** `https://github.com/PatrickRuddiman/shrt`. Rationale: matches spec author/org.
14. **README.md location.** **Chosen:** repo root, single file. Required by spec §13. Rationale: standard.
15. **`cargo-binstall` metadata.** **Chosen:** `[package.metadata.binstall]` in `crates/shrt/Cargo.toml` pointing at the GitHub Release URL pattern. Rationale: free `cargo binstall shrt` support after release artifacts exist; no maintenance.
16. **Code-signing.** **Chosen:** out of scope. Rationale: not in spec.
17. **crates.io publish ordering.** **Chosen:** on `v*` tag — (i) `cargo publish -p shrt-runner`, (ii) run `scripts/bundle-runner-src.ps1` to populate `crates/shrt/runner-src/`, (iii) `cargo publish -p shrt`. Rationale: `shrt`'s build.rs needs the runner sources but does NOT need `shrt-runner` from the registry; publishing the runner first is a courtesy, not a dependency.
18. **Bundling script location.** Options: shell script / xtask binary / GH Action inline. **Chosen:** `scripts/bundle-runner-src.ps1`. Rationale: spec §9 is Windows-only for v0.1; PowerShell is native; ~10 lines — overkill to add an `xtask` crate.
19. **`shrt-runner` published as bin or lib.** **Chosen:** bin only. Rationale: no public API consumed externally.

## §4 Contracts & shapes

**Repo layout additions for distribution:**
```
shrt/
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── scripts/
│   └── bundle-runner-src.ps1
├── .github/workflows/
│   ├── ci.yml                 # owned by build-pipeline
│   └── release.yml            # owned by this slice
└── crates/
    ├── shrt/
    │   └── Cargo.toml         # includes [package.metadata.binstall]
    │                          # and at publish time, runner-src/ alongside
    └── shrt-runner/
        └── Cargo.toml         # has its own [profile.release]
```

**`scripts/bundle-runner-src.ps1` contract (~10 lines):**
- Inputs: none (hard-coded relative paths inside the repo).
- Effects:
  1. `Remove-Item -Recurse -Force crates/shrt/runner-src` if exists.
  2. `New-Item -ItemType Directory crates/shrt/runner-src/src`.
  3. `Copy-Item crates/shrt-runner/Cargo.toml crates/shrt/runner-src/`.
  4. `Copy-Item crates/shrt-runner/src/* crates/shrt/runner-src/src/ -Recurse`.
- Idempotent. Always run before `cargo publish -p shrt`. Run from repo root.

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

**`crates/shrt/Cargo.toml` cargo-binstall metadata:**
```
[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/v{ version }/shrt-v{ version }-{ target }.exe"
bin-dir = ""
pkg-fmt = "bin"
```

**`.github/workflows/release.yml` shape (separate from `ci.yml`):**

Triggers: `push` on tags matching `v*`.

Jobs (sequential):

| Job | Runs on | Steps |
|---|---|---|
| `build-x64` | `windows-latest` | checkout → setup-rust stable → `cargo build -p shrt --release --target x86_64-pc-windows-msvc` → upload artifact `shrt-x86_64.exe` |
| `build-arm64` | `windows-11-arm` | as above with `--target aarch64-pc-windows-msvc` |
| `release` | `ubuntu-latest` | gather artifacts → rename to release scheme → compute `sha256sums.txt` → `gh release create vX.Y.Z --title ... --notes ... <assets>` |
| `publish-crates` | `windows-latest` | checkout → setup-rust → `cargo publish -p shrt-runner` → `pwsh scripts/bundle-runner-src.ps1` → `cargo publish -p shrt` (uses `CARGO_REGISTRY_TOKEN` secret) |
| `update-scoop` | `ubuntu-latest` | clone bucket repo using deploy key → run a small node/python script to render `bucket/shrt.json` from the new release → commit + push |

Failure of any job aborts the rest. crates.io publish failure is recoverable; rerun with `gh workflow run release.yml -f tag=vX.Y.Z`.

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
4. `publish-crates` job runs `cargo publish -p shrt-runner`, then `scripts/bundle-runner-src.ps1`, then `cargo publish -p shrt`. The bundling step produces `crates/shrt/runner-src/` containing the runner source tree.
5. `update-scoop` job clones the bucket repo, generates a fresh `bucket/shrt.json` referencing the new release URLs and SHA256s, commits, pushes.

**End-user `cargo install shrt`:**
1. Cargo fetches the published `shrt` crate. It is self-contained (`runner-src/` is included).
2. Cargo invokes `crates/shrt/build.rs`. Probe finds `runner-src/Cargo.toml` (published-crate context).
3. build.rs spawns `cargo build --manifest-path=runner-src/Cargo.toml --release --target=$TARGET --target-dir=$OUT_DIR/runner-target`. The runner crate's own `[profile.release]` applies (no workspace).
4. build.rs copies the produced `shrt-runner.exe` to `$OUT_DIR/shrt-runner.exe`.
5. Cargo compiles `crates/shrt/src/`, embeds the runner via `include_bytes!`, links `shrt.exe`, installs to `~/.cargo/bin/shrt.exe`.

**End-user `scoop install shrt`** (after the bucket is added):
1. Scoop fetches the prebuilt `shrt-vX.Y.Z-<arch>.exe` from the GitHub Release for the user's arch, verifies SHA256, places at `~/scoop/apps/shrt/<version>/shrt.exe`, shims into `~/scoop/shims/shrt.exe`.
2. `post_install` prints the `shrt init` reminder.

**End-user upgrade flow** (matches spec §8.3):
1. `cargo install shrt --force` (or `scoop update shrt`).
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
