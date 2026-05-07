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

`shrt init` creates `~/.shrt/bin` (the shim directory) and adds it to your user PATH (`HKCU\Environment\Path`) automatically. **Open a new shell** for the PATH change to take effect.

## Quickstart

Suppose you frequently invoke a tool with a long template prefix and want a one-word shortcut:

```
shrt add worktree 'copilot -p "/worktree create a worktree for {1}" --yolo'
```

This produces:

```
~/.shrt/bin/
├── worktree.exe        # bytes-identical copy of the embedded runner
└── worktree.shrt       # TOML sidecar describing the template
```

Now from anywhere on PATH:

```
worktree "ado item 37839929"
```

resolves to:

```
copilot -p "/worktree create a worktree for ado item 37839929" --yolo
```

> **Quoting note**: use *double* quotes inside the template body to keep multi-word arguments grouped. The runner's argv tokenizer follows Microsoft's CRT rules, where only `"` is special — single quotes are literal characters and won't preserve argument boundaries. From PowerShell, wrap the whole `shrt add` template in single quotes (as shown above) so the inner double quotes pass through.

## Commands

| Command | What it does |
|---|---|
| `shrt init` | Create `~/.shrt/bin`; auto-add it to user PATH; report status. |
| `shrt add <name> <template>` | Create a new shim. First whitespace-delimited token of `<template>` is the target unless `--target` overrides. |
| `shrt remove <name>` | Delete `<name>.exe` and `<name>.shrt`. |
| `shrt list [--verbose]` | Print all shims, sorted alphabetically. |
| `shrt show <name>` | Print the raw contents of `<name>.shrt`. |
| `shrt sync` | Re-copy the embedded runner bytes into every shim's `.exe` (run after upgrading `shrt`). |
| `shrt path` | Print the shim directory path. |
| `shrt doctor` | Diagnose the shim setup. |

Global flags: `--shim-dir <path>` (also via `SHRT_DIR` env), `--quiet`, `--json`.

`shrt add` also runs two safety checks:

- **Target on PATH.** If the named target binary doesn't resolve via PATH+PATHEXT (and you didn't pass `--shell`), it warns to stderr suggesting `--shell` for shell builtins (`echo`, `dir`, `cd`, `set`) or `--target` for a full path. The shim is still created.
- **Shim name not shadowed.** If the chosen shim name already resolves to a different binary on PATH (e.g. `wt` → Windows Terminal), the add fails with exit 64 — your shim would never be invoked. Pick a different name or remove the shadowing binary.

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

### After `shrt init`, my shim still isn't found

`shrt init` writes `~/.shrt/bin` to your **user** PATH in the registry, but **already-running shells don't pick up environment changes** — that's a Windows fundamental, not a `shrt` quirk. Open a new shell tab/window and the dir will be on PATH. `shrt init` is idempotent so it's safe to re-run.

If you need to add it manually (rare — auto-add fails only on locked-down machines / group policy):

```
PowerShell: $env:PATH = "$env:PATH;C:\Users\you\.shrt\bin"
cmd.exe:    set PATH=%PATH%;C:\Users\you\.shrt\bin
Git Bash:   export PATH="$PATH:C:\Users\you\.shrt\bin"
```

For a permanent fallback: add to `$PROFILE`, or in the Windows GUI go to System Properties → Environment Variables → edit `Path`.

### `shrt add` says "shim name … is shadowed by an existing binary"

The shell would resolve `<name>` to that existing binary first, never reaching your shim. Common shadowers on Win11:

- **Windows App Execution Aliases**: `wt`, `python`, `winget`, etc., living in `%LocalAppData%\Microsoft\WindowsApps`. Disable in Settings → Apps → Advanced app settings → App execution aliases, then re-run `shrt add`.
- **Pre-existing tools** earlier in PATH. Either uninstall them or pick a different shim name.

`Get-Command <name> -All` shows every PATH entry that matches the name; the first one wins.

### `shrt add` warns "target … not found on PATH"

Either the target binary isn't installed yet (the shim is still created — install it later and the shim works), or the target is a shell builtin like `echo`/`dir`/`cd`/`set`. For a builtin, re-run with `--shell` so the runner pipes through `cmd /c`:

```
shrt add say 'echo {1}' --shell --force
```

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

Runs four per-shim checks: sidecar parses, `.exe` bytes match the embedded runner, target binary resolves on PATH, plus a PATH-membership check. Use `shrt doctor --json` for machine-readable output.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Generic error |
| 64 | Usage error: invalid shim name, shim name shadowed by existing PATH binary, missing required `{N}` placeholder at runtime, control char in template/cwd/description |
| 66 | Missing input: sidecar absent for `show` / `remove` / runner |
| 73 | Cannot create output: shim dir not writable, name collision without `--force` |
| 78 | Config error: sidecar parse failure, version mismatch, `{ENV:NAME}` not set |
| 127 | Target command not found on PATH at runtime |

## License

Dual-licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
