Parent spec: [shrt — Specification](../spec.md)

# shrt — build-pipeline

## §1 Summary
Defines the Cargo workspace topology, the `crates/shrt/build.rs` recipe that produces the embedded runner bytes, the cross-architecture build flow (x64 + ARM64 Windows), and the release profile that gets `shrt-runner.exe` under the spec §6.1 size budget. Owns CI build steps but not artifact upload — that's `distribution`. Owns the recipe but not the publish-to-crates.io wrinkle for `cargo install` from a single-crate fetch — also `distribution`.

## §2 Codebase reconnaissance
> Greenfield: no existing system to reconcile. Decisions below are unconstrained.

Sibling slices already locked:
- `slices/runner.md` — runner is std-only, source layout under `crates/shrt-runner/src/`, must hit < 300 KB stripped + < 10 ms cold start.
- `slices/sidecar-format.md`, `slices/substitution-engine.md` — runner source modules (no extra deps).

## §3 Decisions
1. **Workspace topology.** Options: single crate with two `[[bin]]` / two-crate workspace / three-crate workspace with shared lib. **Chosen:** two-crate workspace per spec §2.1 — `crates/shrt` and `crates/shrt-runner`. Rationale: matches spec; isolates runner's std-only constraint at the crate level (independent dep set).
2. **How the runner binary gets embedded.** Options: `cargo build` subprocess from `build.rs` / artifact-dependencies (`-Z bindeps`, nightly) / pre-built bytes committed to repo. **Chosen:** `build.rs` invokes `cargo build --manifest-path=<runner-cargo-toml> --release --target=$TARGET --target-dir=$OUT_DIR/runner-target`, then copies the produced `.exe` to `$OUT_DIR/shrt-runner.exe`. The exact manifest path is probed by build.rs (workspace vs. published-crate context, owned by `distribution` slice §3 Decision 3). Rationale: only stable-Rust path; matches spec §2.2; `--manifest-path` works in both workspace and single-crate-fetch contexts (`-p` does not).
3. **Cross-architecture correctness.** Options: hard-code x64 / read `TARGET` env / accept a build-time flag. **Chosen:** read `TARGET` from cargo's env in `build.rs` and pass it to the runner subprocess. Rationale: ensures `cargo build --target=aarch64-pc-windows-msvc` for the shrt crate produces an ARM64 runner; otherwise the embedded bytes don't run on the user's machine.
4. **Embedding declaration site.** Options: build.rs writes a generated source file / `include_bytes!` referencing OUT_DIR / consume via `env!`. **Chosen:** `crates/shrt/src/shim.rs` declares `pub const RUNNER_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shrt-runner.exe"));`. Rationale: idiomatic; no codegen step.
5. **Subprocess lock contention.** Options: share `target-dir` / nest under `OUT_DIR`. **Chosen:** subprocess uses `--target-dir=$OUT_DIR/runner-target`. Rationale: parent cargo holds a target-dir lock; sharing it deadlocks recursive cargo invocations.
6. **Release profile knobs.** **Chosen:** `[profile.release]` at workspace level with `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = "symbols"`. Rationale: standard size-stripping cocktail; together they take a hello-world Windows Rust binary from ~3 MB to ~150 KB; runner has only file IO + spawn + a tiny TOML reader + a tiny scanner so it fits the < 300 KB budget without nightly tricks.
7. **Where the profile lives.** Options: workspace-level / per-crate / both. **Chosen:** **both** — workspace `Cargo.toml` (covers dev workspace builds) AND `crates/shrt-runner/Cargo.toml` declares the same `[profile.release]` (covers the published-crate path where the workspace is absent and the runner's own profile is the active one). Rationale: redundant knobs are tolerable; the alternative is a different `shrt-runner.exe` size between workspace and published builds.
8. **Toolchain pinning.** Options: `rust-toolchain.toml` channel pin / no pin / minor pin. **Chosen:** `rust-toolchain.toml` at workspace root with `channel = "stable"`. Rationale: keeps everyone on stable but always-current; minor pinning isn't worth the maintenance for v0.1.
9. **Re-run-if-changed triggers in `build.rs`.** **Chosen:** emit three triggers — `../shrt-runner/src`, `../shrt-runner/Cargo.toml`, and the workspace `Cargo.lock`. Rationale: any of these changing implies the embedded runner needs a rebuild; nothing else in the workspace affects the runner output.
10. **CI workflow location and form.** Options: separate test/build/release workflows / single workflow with conditionals. **Chosen:** single `.github/workflows/ci.yml`. Rationale: small project; conditionals on event type are clearer than three files referencing the same matrix.
11. **CI build matrix.** Options: x64 + ARM64 via `cross` from x64 / native ARM64 GitHub-hosted runner / x64 only with deferred ARM64. **Chosen:** native runners — `windows-latest` for x64 and `windows-11-arm` for ARM64. Rationale: native-runner build is faster and avoids cross-toolchain breakage; both runner SKUs are available on GitHub-hosted from late 2024.
12. **Edition + MSRV.** Options: edition pin only / edition + MSRV / no pins. **Chosen:** edition `2021`, no `rust-version` field in Cargo.toml. Rationale: avoids MSRV maintenance; current stable is the de-facto minimum.
13. **`shrt-runner` declaration in `shrt`'s manifest.** Options: regular `[dependencies]` / `[build-dependencies]` / no declaration. **Chosen:** no declaration. Rationale: build-deps build for HOST, wrong target; regular deps would link the runner's lib (it has no lib). Workspace member-ship is enough for `build.rs` to invoke `cargo build -p shrt-runner`.
14. **Lock file.** **Chosen:** `Cargo.lock` committed at workspace root. Rationale: app, not library; reproducible builds matter.
15. **Publishing-to-crates.io single-crate fetch wrinkle.** **Chosen:** out of scope here. Rationale: this slice's recipe is correct for dev (full workspace), CI (full workspace with explicit `--target`), and prebuilt-binary distribution. Making `cargo install shrt` work from a single-crate fetch is `distribution`'s problem (likely: bundle runner sources into the published `shrt` crate via `include = [...]`).

## §4 Contracts & shapes

**Repo layout** (matches spec §2.1 plus build-pipeline additions):
```
shrt/
├── Cargo.toml                 # workspace manifest + [profile.release]
├── Cargo.lock
├── rust-toolchain.toml        # channel = "stable"
├── .github/workflows/ci.yml   # CI: build + test on push/PR; release on tag
├── crates/
│   ├── shrt/
│   │   ├── Cargo.toml         # CLI deps (clap, serde, toml, ...)
│   │   ├── build.rs           # embeds runner bytes
│   │   └── src/
│   │       ├── main.rs
│   │       ├── shim.rs        # contains const RUNNER_BYTES
│   │       └── ...
│   └── shrt-runner/
│       ├── Cargo.toml         # std-only; no deps
│       └── src/
│           ├── main.rs
│           ├── sidecar.rs
│           ├── substitute.rs
│           ├── argv.rs
│           └── path.rs
└── slices/                    # design docs (this directory)
```

**Workspace `Cargo.toml` shape:**
- `[workspace]` `members = ["crates/*"]`, `resolver = "2"`.
- `[profile.release]`: `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = "symbols"`.
- `[workspace.package]` for shared metadata (`version`, `edition = "2021"`, `license`, etc.).

**`crates/shrt/build.rs` contract:**
- Inputs (from cargo env): `OUT_DIR`, `TARGET`, `CARGO`, `PROFILE`.
- Probes for the runner manifest in this order: `${CARGO_MANIFEST_DIR}/runner-src/Cargo.toml` (published-crate context, populated by the publish script); else `${CARGO_MANIFEST_DIR}/../shrt-runner/Cargo.toml` (workspace context); else hard-error.
- Effects: spawns `${CARGO} build --manifest-path=<probed> --release --target=${TARGET} --target-dir=${OUT_DIR}/runner-target`. On nonzero exit, propagate stderr and `panic!` to fail the parent build.
- Locates the produced binary at `${OUT_DIR}/runner-target/${TARGET}/release/shrt-runner.exe` and copies (not moves) to `${OUT_DIR}/shrt-runner.exe`.
- Emits `cargo:rerun-if-changed=../shrt-runner/src`, `cargo:rerun-if-changed=../shrt-runner/Cargo.toml`, `cargo:rerun-if-changed=../../Cargo.lock`.
- Always uses `--release`. Even if the parent build is `--debug`, the *embedded* runner is release. (Otherwise the size budget can never be checked from a dev build, and the user's installed shims would be debug-bloated.)

**`crates/shrt/src/shim.rs` embed shape:**
- `pub const RUNNER_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shrt-runner.exe"));`
- Used by the `shrt add` command (owned by `shim-management` slice) to write a new shim's `.exe` file.

**`crates/shrt-runner/Cargo.toml` shape:**
- `[package]` standard fields, edition 2021.
- No `[dependencies]`, no `[build-dependencies]`, no `[dev-dependencies]`. (Dev-deps for tests would be tolerated if integration tests live here, but per `runner` slice §3 Decision 13 the runner has no internal tests; integration coverage lives in `testing-harness`.)
- `[[bin]] name = "shrt-runner"` (single binary).

**CI pipeline (`.github/workflows/ci.yml`) shape:**
- Triggers: `push` to `main`, `pull_request`, and `push` to tags matching `v*`.
- Job `test` (matrix: `windows-latest`, `windows-11-arm`): checkout → setup-rust (stable) → `cargo test --workspace --release`.
- Job `build-release` (matrix as above; runs on tag only): checkout → setup-rust (stable) → `cargo build -p shrt --release` → upload the produced `shrt.exe` as a workflow artifact.
- Tag-driven artifact upload to GitHub Releases is owned by `distribution`.

## §5 Sequence

**`cargo build -p shrt` (dev or CI), per workspace `Cargo.toml`:**
1. Cargo resolves the workspace dep graph; `shrt-runner` is a workspace sibling (not a dep edge).
2. Cargo builds `shrt`, which triggers `crates/shrt/build.rs`.
3. `build.rs` reads `TARGET`, `OUT_DIR`, `CARGO`, `CARGO_MANIFEST_DIR` from env.
4. `build.rs` probes for the runner manifest (see §4 above), then spawns `cargo build --manifest-path=<probed> --release --target=$TARGET --target-dir=$OUT_DIR/runner-target` and waits.
5. Subprocess builds `shrt-runner` for the requested target; in workspace mode the workspace-level `[profile.release]` knobs apply (subprocess inherits the workspace via `--manifest-path` resolution to the workspace root); produces `$OUT_DIR/runner-target/$TARGET/release/shrt-runner.exe`.
6. `build.rs` copies that file to `$OUT_DIR/shrt-runner.exe`. Failure to find the source path → panic.
7. `build.rs` emits the three `cargo:rerun-if-changed=` lines.
8. Parent cargo proceeds to compile `crates/shrt/src/`. The `include_bytes!` in `shim.rs` resolves to `$OUT_DIR/shrt-runner.exe`. Linker produces `target/$TARGET/release/shrt.exe` with the runner bytes embedded.

**`cargo build --release --target=aarch64-pc-windows-msvc -p shrt`:**
- Identical to the above with `TARGET=aarch64-pc-windows-msvc`. The runner subprocess inherits the cross target. The produced `shrt.exe` is ARM64 with an ARM64 runner inside it.

**CI on push to `main`:**
1. GitHub Actions matrix fans out to `windows-latest` and `windows-11-arm`.
2. Each runner: checkout → install stable Rust → `cargo test --workspace --release`.

**CI on tag push (`v*`):**
1. Same matrix. After tests, `cargo build -p shrt --release` produces `shrt.exe`.
2. The `shrt.exe` is uploaded as a workflow artifact. Release-page assembly handed off to `distribution`.

## §6 Out of scope
- Source-bundling logistics so `cargo install shrt` works from a single-crate fetch on crates.io. Owned by `distribution`.
- Scoop bucket manifest, winget manifest, GitHub Release page assembly. All `distribution`.
- The `shim-management` write path (taking `RUNNER_BYTES` and writing it to a new shim file). That slice consumes `RUNNER_BYTES`; this slice produces it.
- Code-signing (Authenticode signatures on `shrt.exe` or shim `.exe` files). Not in spec; defer.
- Test infrastructure (stub target binary, integration harness). Owned by `testing-harness`.

> If the parent spec is ambiguous on anything this slice depends on, stop and update the spec. Do not invent behavior here.
