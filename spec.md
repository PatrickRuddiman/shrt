\# shrt — Specification



\*\*Version:\*\* 0.1 (initial design)

\*\*Author:\*\* Patrick Ruddiman

\*\*Status:\*\* Draft for implementation



\---



\## 1. Overview



`shrt` is a Windows-first CLI tool that generates parameterized command shims as real `.exe` files on PATH. Each shim is a copy of a small runner binary plus a sidecar config file describing the command template. When invoked, the shim reads its sidecar, performs placeholder substitution against the user's arguments, and execs the target command.



Tagline: \*\*"shrt (pronounced \*short\*) — parameterized shortcuts for Windows."\*\*



\### 1.1 Motivation



PowerShell aliases cannot accept parameters. PowerShell functions can, but they're shell-scoped, slow to load, and don't work outside PowerShell (cmd.exe, Git Bash, VS Code task runners, etc.). Bash tools like `aliasme` have no clean Windows equivalent.



`shrt` solves this by generating real executables, so a single `shrt add` call produces a shortcut that works in every shell and every tool that respects PATH.



\### 1.2 Example



```

shrt add wt "copilot -p '/worktree create a worktree for {1}' --yolo"

```



Produces `\~/.shrt/bin/wt.exe` and `\~/.shrt/bin/wt.shrt`. Then:



```

wt "ado item 37839929"

```



Resolves to:



```

copilot -p "/worktree create a worktree for ado item 37839929" --yolo

```



\---



\## 2. Architecture



\### 2.1 Workspace layout



Single Cargo workspace with two binaries:



```

shrt/

├── Cargo.toml              # workspace

├── crates/

│   ├── shrt/               # CLI binary (the `shrt` command)

│   │   ├── Cargo.toml

│   │   └── src/

│   │       ├── main.rs

│   │       ├── cli.rs      # clap definitions

│   │       ├── commands/

│   │       │   ├── add.rs

│   │       │   ├── remove.rs

│   │       │   ├── list.rs

│   │       │   ├── show.rs

│   │       │   ├── edit.rs

│   │       │   ├── sync.rs

│   │       │   ├── path.rs

│   │       │   └── init.rs

│   │       ├── config.rs   # sidecar (.shrt) read/write

│   │       ├── shim.rs     # runner extraction + copy

│   │       └── paths.rs    # shim dir resolution

│   └── shrt-runner/        # the tiny shim binary

│       ├── Cargo.toml

│       └── src/main.rs

└── README.md

```



\### 2.2 Build flow



`shrt-runner` builds first and produces `shrt-runner.exe`. `shrt` embeds those bytes at compile time using `include\_bytes!`, so a single `shrt.exe` ships with the runner inside it. No separate runner binary on disk; `shrt add` writes the embedded bytes to the new shim location.



`build.rs` in the `shrt` crate orchestrates the dependency: it runs `cargo build -p shrt-runner --release` and copies the resulting `.exe` to a known location that `include\_bytes!` references.



\### 2.3 Runtime topology



```

\~/.shrt/

├── bin/                    # on PATH

│   ├── wt.exe              # bytes-identical copy of shrt-runner.exe

│   ├── wt.shrt             # sidecar config

│   ├── review.exe

│   ├── review.shrt

│   └── ...

└── shrt.toml               # global shrt config (optional, future use)

```



\### 2.4 Shim invocation flow



1\. User types `wt foo bar`

2\. Windows resolves `wt.exe` from PATH, executes it

3\. Runner determines its own path via `std::env::current\_exe()`

4\. Runner derives sidecar path: replace `.exe` with `.shrt`

5\. Runner reads and parses sidecar

6\. Runner substitutes placeholders in `template` using `args\[1..]`

7\. Runner spawns target command with substituted args, inheriting stdin/stdout/stderr

8\. Runner waits for child, exits with child's exit code



\---



\## 3. Sidecar file format



INI-like TOML. Chosen for human editability and standard library support via the `toml` crate.



\### 3.1 Schema



```toml

\# wt.shrt

target = "copilot"

template = "-p \\"/worktree create a worktree for {1}\\" --yolo"



\# Optional fields

shell = false                # if true, run via cmd /c (allows pipes, redirects)

cwd = ""                     # working directory; empty = inherit

description = ""             # shown in `shrt list`

created = "2026-05-06T10:00:00Z"  # ISO-8601, set by `shrt add`

version = 1                  # sidecar schema version

```



\### 3.2 Field semantics



| Field         | Type    | Required | Description                                                                   |

|---------------|---------|----------|-------------------------------------------------------------------------------|

| `target`      | string  | yes      | Command to execute. Resolved against PATH if not absolute.                    |

| `template`    | string  | yes      | Argument template with placeholders. See §4.                                  |

| `shell`       | bool    | no       | If true, run via `cmd /c "<target> <substituted\_template>"`. Default false.   |

| `cwd`         | string  | no       | Working directory. Supports `\~` and env var expansion. Empty = inherit.       |

| `description` | string  | no       | Human-readable description for `shrt list`.                                   |

| `created`     | string  | no       | ISO-8601 timestamp. Informational.                                            |

| `version`     | int     | no       | Schema version. Default 1. Runner refuses unknown versions.                   |



\### 3.3 Resolution rules



\- `target` is resolved using the `which` crate. If absolute, used as-is. If relative, searched on PATH. If not found, runner exits with code 127 and a clear error.

\- `cwd` expansion supports `\~` (home dir) and `${VAR}` env vars.

\- Sidecar parse errors exit with code 78 (`EX\_CONFIG`) and a clear error message naming the offending file and field.



\---



\## 4. Placeholder substitution



\### 4.1 Syntax



| Placeholder        | Meaning                                                        | Missing behavior          |

|--------------------|----------------------------------------------------------------|---------------------------|

| `{1}`, `{2}`, ...  | Positional argument, 1-indexed                                 | Error, exit code 64       |

| `{1?}`, `{2?}`     | Optional positional, empty string if absent                    | Substitutes empty         |

| `{INPUT}`          | All args joined with single spaces                             | Substitutes empty         |

| `{@}`              | All args, each shell-quoted preserving boundaries              | Substitutes empty         |

| `{ENV:NAME}`       | Environment variable                                           | Error, exit code 78       |

| `{ENV:NAME:val}`   | Environment variable with default                              | Substitutes default       |

| `{{`, `}}`         | Literal `{` and `}`                                            | n/a                       |



\### 4.2 Quoting and `{@}` semantics



`{INPUT}` joins with single spaces and is suitable when args don't contain spaces or special characters. `{@}` preserves argument boundaries by quoting each arg individually using Windows command-line quoting rules (double-quote wrapping, internal `"` escaped per \[Microsoft's argument parsing rules](https://learn.microsoft.com/en-us/cpp/cpp/main-function-command-line-args)).



When `shell = true`, the entire substituted command is passed to `cmd /c "..."`, and shell metacharacters in args are NOT auto-escaped. The user is responsible for safe input. Document this clearly.



When `shell = false` (default), `target` is invoked directly via `CreateProcess` and arg arrays are passed natively; no quoting is needed at the OS level. The substitution still produces a single template string which is then \*\*tokenized\*\* using a Windows command-line parser before being passed as separate args. The `winsplit` or equivalent parsing logic must match Windows argv parsing exactly.



\### 4.3 Substitution algorithm



1\. Tokenize the template into literal segments and placeholder segments.

2\. For each placeholder, resolve its value using user args / env.

3\. If any required placeholder is unresolved, exit with code 64.

4\. Concatenate resolved segments into the substituted command line.

5\. If `shell = false`, parse the substituted command line into argv tokens.

6\. If `shell = true`, pass the substituted line as a single arg to `cmd /c`.



\### 4.4 Example substitutions



Given `template = "-p \\"/worktree create a worktree for {1}\\" --yolo"` and user runs `wt "ado item 37839929"`:



\- `{1}` resolves to `ado item 37839929`

\- Substituted line: `-p "/worktree create a worktree for ado item 37839929" --yolo`

\- Tokenized argv: `\["-p", "/worktree create a worktree for ado item 37839929", "--yolo"]`

\- Spawned: `copilot.exe` with that argv



\---



\## 5. CLI surface



\### 5.1 Commands



```

shrt init

&#x20;   Create \~/.shrt/bin if missing. Print instructions for adding it to PATH.

&#x20;   Idempotent. If already initialized, print path and PATH status.



shrt add <name> <template> \[--target <cmd>] \[--shell] \[--cwd <path>] \[--desc <text>]

&#x20;   Create a new shim. By default, the first whitespace-delimited token of

&#x20;   <template> is taken as `target` and the rest as the template body. Override

&#x20;   with --target.



&#x20;   Examples:

&#x20;     shrt add wt "copilot -p '/worktree create a worktree for {1}' --yolo"

&#x20;     shrt add wt --target copilot "-p '/worktree create a worktree for {1}' --yolo"



&#x20;   Refuses to overwrite existing shims unless --force is passed.

&#x20;   Refuses names containing path separators or shell metacharacters.



shrt remove <name>

&#x20;   Delete <name>.exe and <name>.shrt from the shim directory.

&#x20;   Errors cleanly if shim doesn't exist.



shrt list \[--verbose]

&#x20;   Print all shims. Default: name + target. Verbose: full template + description.



shrt show <name>

&#x20;   Print the contents of <name>.shrt to stdout.



shrt edit <name>

&#x20;   Open <name>.shrt in $EDITOR (or notepad on Windows if unset).



shrt sync

&#x20;   Re-copy the embedded runner bytes to every shim's .exe file.

&#x20;   Run this after upgrading shrt itself to propagate runner improvements.



shrt path

&#x20;   Print the shim directory (for use in shell init: `$env:PATH += ';' + (shrt path)`).



shrt doctor

&#x20;   Diagnostic: check that shim dir is on PATH, runner version matches across

&#x20;   shims, sidecars all parse, target binaries all resolve. Print findings.

```



\### 5.2 Global flags



```

\--shim-dir <path>    Override default shim directory (also via SHRT\_DIR env var)

\--quiet              Suppress non-error output

\--json               Machine-readable output where applicable

```



\### 5.3 Exit codes



Follow BSD sysexits where reasonable:



| Code | Meaning                                       |

|------|-----------------------------------------------|

| 0    | Success                                       |

| 1    | Generic error                                 |

| 64   | Usage error (bad args, missing required arg)  |

| 66   | Cannot open input (sidecar missing)           |

| 73   | Cannot create output (shim dir not writable)  |

| 78   | Config error (malformed sidecar, bad version) |

| 127  | Target command not found on PATH              |



\---



\## 6. Runner specification



The runner (`shrt-runner.exe`) is intentionally minimal. Hard requirements:



\### 6.1 Constraints



\- \*\*No external crate dependencies\*\* beyond `std`. Goal: smallest possible binary, fastest possible startup. Hand-roll TOML parsing for the limited subset needed.

\- \*\*Binary size target:\*\* under 300 KB stripped.

\- \*\*Cold-start target:\*\* under 10 ms on modern hardware.

\- \*\*Must not panic\*\* on malformed input. All errors produce a clean stderr message and a documented exit code.



\### 6.2 Logic



```

fn main() -> ExitCode {

&#x20;   let exe = current\_exe()?;                          // e.g. C:\\Users\\x\\.shrt\\bin\\wt.exe

&#x20;   let sidecar = exe.with\_extension("shrt");          // wt.shrt

&#x20;   let config = parse\_sidecar(\&sidecar)?;             // §3

&#x20;   let user\_args: Vec<OsString> = args\_os().skip(1).collect();

&#x20;   let resolved = substitute(\&config.template, \&user\_args, \&env)?;  // §4

&#x20;   let argv = if config.shell {

&#x20;       vec!\["cmd", "/c", \&format!("{} {}", config.target, resolved)]

&#x20;   } else {

&#x20;       let mut v = vec!\[config.target.clone()];

&#x20;       v.extend(parse\_argv(\&resolved));

&#x20;       v

&#x20;   };

&#x20;   let target\_path = which(\&argv\[0])?;                // resolve PATH

&#x20;   let status = Command::new(target\_path)

&#x20;       .args(\&argv\[1..])

&#x20;       .current\_dir(resolve\_cwd(\&config.cwd)?)

&#x20;       .stdin(Stdio::inherit())

&#x20;       .stdout(Stdio::inherit())

&#x20;       .stderr(Stdio::inherit())

&#x20;       .status()?;

&#x20;   ExitCode::from(status.code().unwrap\_or(1) as u8)

}

```



\### 6.3 Sidecar parsing in the runner



To keep the runner dependency-free, implement a minimal TOML reader that handles only the documented schema (§3.1). Rules:



\- Parse line by line. Strip comments (`#` to end of line).

\- Recognize `key = value` pairs at top level only. No tables, no arrays.

\- String values: strip surrounding `"..."`, decode `\\"`, `\\\\`, `\\n`, `\\t`.

\- Bool values: `true` / `false`.

\- Int values: decimal only.

\- Unknown keys: ignored with a warning to stderr.

\- Missing required keys (`target`, `template`): error 78.



The full `shrt` CLI uses the real `toml` crate for writing and validation, so any sidecar `shrt` produces is guaranteed parseable by the minimal runner reader.



\---



\## 7. Path and dependency choices



\### 7.1 `shrt` CLI dependencies



| Crate         | Purpose                                                   |

|---------------|-----------------------------------------------------------|

| `clap`        | CLI parsing, with `derive` feature                        |

| `serde`       | Sidecar deserialization                                   |

| `toml`        | Sidecar (de)serialization                                 |

| `directories` | Resolving home / config dir cross-platform                |

| `which`       | Resolving target commands on PATH                         |

| `anyhow`      | Error propagation in the CLI (not the runner)             |

| `chrono`      | ISO-8601 timestamps for `created` field                   |



\### 7.2 `shrt-runner` dependencies



\*\*None.\*\* Only `std`. This is a hard constraint to keep the runner small and fast.



\---



\## 8. Distribution



\### 8.1 Channels



\- \*\*Scoop:\*\* maintain a project bucket initially, request inclusion in `extras` once stable

\- \*\*GitHub Releases:\*\* prebuilt `shrt.exe` for `x86\_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`

\- \*\*Source:\*\* `cargo install --git https://github.com/PatrickRuddiman/shrt --locked shrt` for users who want to build locally

\- \*\*winget:\*\* add manifest once 1.0 ships



\### 8.2 Versioning



Semantic versioning. Sidecar `version` field is independent of crate version and only bumps when the schema changes incompatibly.



\### 8.3 Upgrade flow



When a user upgrades `shrt` itself:



1\. New `shrt.exe` replaces old via cargo / scoop / etc.

2\. User runs `shrt sync` to re-copy embedded runner bytes to every existing shim.

3\. `shrt doctor` confirms all shims now report the new runner version.



The runner version can be embedded as a string constant in the runner binary and reported via a magic flag (e.g. `<shim>.exe --shrt-runner-version` — but this requires the runner to special-case this flag before reading the sidecar). Alternative: store runner version in the sidecar at `shrt add` time.



\---



\## 9. Cross-platform notes



Initial release is \*\*Windows-only\*\*. The architecture works on macOS and Linux with two changes:



1\. Generated shims have no `.exe` extension and need `chmod +x`.

2\. `cmd /c` becomes `sh -c` when `shell = true`.



These changes are mechanical. Defer until there's user demand.



\---



\## 10. Security considerations



\### 10.1 Threat model



`shrt` is a developer tool. Threat model assumes a trusted user creating shims for their own use. Not designed for:



\- Multi-user systems where shim configs are shared

\- Untrusted input flowing into `template` strings

\- Sandboxed execution of target commands



\### 10.2 Hardening rules



\- Reject shim names containing path separators (`/`, `\\`), `..`, or shell metacharacters.

\- When `shell = false`, never invoke a shell — use `CreateProcess` directly via `std::process::Command`.

\- When `shell = true`, document loudly that arg content is not escaped.

\- Sidecar files should have user-only ACLs on Windows (set during `shrt add`). Not enforced on read.

\- The runner does not log args (to avoid leaking secrets passed via `{1}`).



\---



\## 11. Testing strategy



\### 11.1 Unit tests



\- Substitution engine: every placeholder type, missing args, escape sequences, quoting edge cases.

\- Sidecar parser (both the full `toml`-based one and the minimal runner one): malformed input, missing fields, unknown fields, version mismatch.

\- Path resolution: `\~`, env vars, missing dirs.



\### 11.2 Integration tests



\- End-to-end: `shrt add` → invoke shim → assert correct command spawned with correct args.

\- Use a stub target binary (e.g. a tiny Rust program that prints its argv as JSON) to verify exact arg passing.

\- Test `shell = true` path with pipes and redirects.

\- Test exit code propagation.

\- Test stdin/stdout/stderr passthrough.



\### 11.3 Manual smoke tests



\- Verify shims work in: PowerShell 7, Windows PowerShell 5.1, cmd.exe, Git Bash, VS Code integrated terminal, Windows Terminal tabs.

\- Verify shims work as targets of other tools (e.g. as a Git external diff/merge driver).



\---



\## 12. Open questions



1\. \*\*Multiple shim directories?\*\* Should `shrt` support a stack of directories (project-local, user, system)? Defer to v0.2.

2\. \*\*Shim groups / namespaces?\*\* e.g. `shrt add ado/wt "..."` creating `ado-wt.exe`. Defer.

3\. \*\*Interactive prompts in templates?\*\* `{?ticket}` to prompt if not provided. Nice but adds complexity. Defer to v0.2.

4\. \*\*Tab completion for shim names?\*\* Generate clap completion files for PowerShell, bash, zsh as part of `shrt init`. Worth doing in v0.1.

5\. \*\*Runner self-update via shrt itself?\*\* A magic `<shim>.exe --shrt-update-runner` invocation that copies fresh bytes from the parent `shrt.exe`. Probably not — `shrt sync` is enough.



\---



\## 13. v0.1 acceptance criteria



The first usable release must:



\- \[x] Implement `init`, `add`, `remove`, `list`, `show`, `path`, `sync`, `doctor`

\- \[x] Support `{1}`–`{9}`, `{1?}`, `{INPUT}`, `{@}`, `{ENV:NAME}`, `{ENV:NAME:default}`, `{{`, `}}`

\- \[x] Generate working shims that pass through stdio and exit codes correctly

\- \[x] Run on Windows 10+ and Windows 11, both x64 and ARM64

\- \[x] Ship via Scoop and GitHub Releases with no extra setup

\- \[x] Runner binary under 300 KB stripped

\- \[x] Have integration tests covering the spawn-and-exec path

\- \[x] Have a README with: install, quickstart, placeholder reference, troubleshooting



`edit` and the Scoop / winget channels can land in v0.2.

