$ErrorActionPreference = 'Stop'

# Extract version from the workspace [workspace.package] section.
$workspaceToml = Get-Content 'Cargo.toml' -Raw
if ($workspaceToml -notmatch '(?s)\[workspace\.package\].*?version\s*=\s*"([^"]+)"') {
    throw "Could not find [workspace.package].version in workspace Cargo.toml"
}
$version = $matches[1]

$dest = 'crates/shrt/runner-src'
if (Test-Path $dest) {
    Remove-Item -Recurse -Force $dest
}
New-Item -ItemType Directory -Path "$dest/src" -Force | Out-Null

# Write a self-contained Cargo.toml — outside the workspace context the
# `*.workspace = true` inheritance from crates/shrt-runner/Cargo.toml would
# fail, so we materialize concrete values here. Profile knobs duplicated
# per slices/build-pipeline.md §3 Decision 7 + slices/distribution.md §3
# Decision 5.
$cargoToml = @"
[package]
name = "shrt-runner"
version = "$version"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "Embedded runner binary for the shrt CLI."

[[bin]]
name = "shrt-runner"
path = "src/main.rs"

# Empty [workspace] table so cargo treats this manifest as its own
# workspace root when invoked inside the parent shrt crate (which is
# itself in a workspace). Without this, cargo would complain that
# the bundled crate "believes it's in a workspace when it's not."
[workspace]

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = "symbols"
"@

Set-Content -Path "$dest/Cargo.toml" -Value $cargoToml -Encoding UTF8
Copy-Item 'crates/shrt-runner/src/*' "$dest/src/" -Recurse

Write-Host "bundled shrt-runner sources to $dest (version $version)"
