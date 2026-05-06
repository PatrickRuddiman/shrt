# shrt

> Parameterized command shortcuts for Windows.

`shrt` (pronounced *short*) generates real `.exe` shims on PATH, each backed by a small TOML sidecar describing the command template. Because they're real executables, the shortcuts work in every shell and every tool that respects PATH — PowerShell 5/7, cmd.exe, Git Bash, VS Code task runners, Windows Terminal tabs.

## Why

PowerShell aliases can't accept parameters. PowerShell functions can, but they're shell-scoped, slow to load, and don't work outside PowerShell. Bash tools like `aliasme` have no clean Windows equivalent. `shrt` solves this with one `shrt add` call per shortcut.

## Install

### Scoop (recommended)

```
scoop bucket add patrickruddiman https://github.com/PatrickRuddiman/PersonalScoopBucket
scoop install shrt
shrt init
```

### Direct download

Grab `shrt-vX.Y.Z-x86_64-pc-windows-msvc.exe` (or the ARM64 build) from the [Releases page](https://github.com/PatrickRuddiman/shrt/releases), rename it to `shrt.exe`, drop it on your PATH, and run `shrt init`.

### From source

```
cargo install --git https://github.com/PatrickRuddiman/shrt --locked shrt
shrt init
```

`shrt init` creates `~/.shrt/bin` (the shim directory) and prints instructions for adding it to PATH. Restart your shell after updating PATH.

## Quickstart

Suppose you frequently invoke a tool with a long template prefix and want a one-word shortcut:

```
shrt add wt "copilot -p '/worktree create a worktree for {1}' --yolo"
```

This produces:

```
~/.shrt/bin/
├── wt.exe        # bytes-identical copy of the embedded runner
└── wt.shrt       # TOML sidecar describing the template
```

Now from anywhere on PATH:

```
wt "ado item 37839929"
```

resolves to:

```
copilot -p "/worktree create a worktree for ado item 37839929" --yolo
```

## Commands

| Command | What it does |
|---|---|
| `shrt init` | Create `~/.shrt/bin`; report whether it's on PATH. |
| `shrt add <name> <template>` | Create a new shim. First whitespace-delimited token of `<template>` is the target unless `--target` overrides. |
| `shrt remove <name>` | Delete `<name>.exe` and `<name>.shrt`. |
| `shrt list [--verbose]` | Print all shims, sorted alphabetically. |
| `shrt show <name>` | Print the raw contents of `<name>.shrt`. |
| `shrt sync` | Re-copy the embedded runner bytes into every shim's `.exe` (run after upgrading `shrt`). |
| `shrt path` | Print the shim directory path. |
| `shrt doctor` | Diagnose the shim setup. |

Global flags: `--shim-dir <path>` (also via `SHRT_DIR` env), `--quiet`, `--json`.

## Placeholder reference

Use these tokens inside the template body:

| Placeholder | Meaning | Missing behavior |
|---|---|---|
| `{1}`, `{2}`, …, `{9}` | Positional argument, 1-indexed | Exit 64 |
| `{1?}`, `{2?}`, … | Optional positional | Substitutes empty string |
| `{INPUT}` | All args joined with single spaces | Substitutes empty |
| `{@}` | All args, each Windows-CRT-quoted to preserve boundaries | Substitutes empty |
| `{ENV:NAME}` | Environment variable | Exit 78 |
| `{ENV:NAME:default}` | Env var with default | Substitutes default |
| `{{`, `}}` | Literal `{` and `}` | n/a |

When the resulting command line is split into argv (the default, `shell = false` mode), Windows CRT parsing rules apply. Set `--shell` on `shrt add` to instead pipe the substituted line through `cmd /c`, which enables shell metacharacters like `|` and `>` at the cost of automatic argument escaping — safe input is then your responsibility.

## Troubleshooting

### Shim dir not on PATH

After `shrt init`, you may see:

```
on PATH: false

Add the shim directory to PATH so its shims become invocable:
  PowerShell: $env:PATH = "$env:PATH;C:\Users\you\.shrt\bin"
  cmd.exe:    set PATH=%PATH%;C:\Users\you\.shrt\bin
  Git Bash:   export PATH="$PATH:C:\Users\you\.shrt\bin"
```

For a permanent change in PowerShell, add the line to your `$PROFILE`. In Windows GUI: System Properties → Environment Variables → edit `Path`.

### After upgrading `shrt`

```
scoop update shrt    # or re-download from Releases / re-run cargo install --git ... --force
shrt sync
```

`shrt sync` rewrites every existing shim's `.exe` with the freshly embedded runner bytes. Old shims keep working without sync; you only miss runner improvements.

### Diagnosing a broken shim

```
shrt doctor
```

Runs four per-shim checks: sidecar parses, `.exe` bytes match the embedded runner, target binary resolves on PATH. Use `shrt doctor --json` for machine-readable output.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Generic error |
| 64 | Usage error: invalid shim name, missing required `{N}` placeholder |
| 66 | Missing input: sidecar absent for `show` / `remove` / runner |
| 73 | Cannot create output: shim dir not writable, name collision without `--force` |
| 78 | Config error: sidecar parse failure, version mismatch, `{ENV:NAME}` not set |
| 127 | Target command not found on PATH |

## License

Dual-licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
