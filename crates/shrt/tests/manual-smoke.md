# Manual cross-shell smoke checklist

This checklist verifies that `shrt`-generated shims work in every Windows shell `shrt` is intended to support. Run before each release; not automated. The integration test suite (`cargo test --workspace`) covers the spawn-and-exec mechanics; this checklist verifies the cross-shell PATH integration that automated tests can't easily exercise.

## Setup

1. Install `shrt`: `cargo install --path crates/shrt --force` (or fetch a release binary).
2. Run `shrt init`. Add the printed shim directory to PATH if it isn't already.
3. Restart each shell after updating PATH.
4. Create a test shim:

   ```
   shrt add wt-smoke "cmd /c echo Hello {1}" --shell
   ```

## Per-shell checklist

For each shell, open a fresh session and run the same command. Expected output: `Hello world`.

### PowerShell 7 (`pwsh`)

- [ ] `wt-smoke world` prints `Hello world`.
- [ ] `wt-smoke world; $LASTEXITCODE` reports the child's exit code (not PowerShell's own).
- [ ] Tab completion of `wt-smoke` (if completion files were generated) works.

### Windows PowerShell 5.1 (`powershell.exe`)

- [ ] `wt-smoke world` prints `Hello world`.
- [ ] `$LASTEXITCODE` after `wt-smoke world` is `0`.

### cmd.exe

- [ ] `wt-smoke world` prints `Hello world`.
- [ ] `echo %ERRORLEVEL%` after the call shows `0`.

### Git Bash (MSYS2 environment)

- [ ] `wt-smoke world` prints `Hello world`.
- [ ] `echo $?` shows `0`.
- [ ] Path-style arguments (e.g. `/c/Users/you/file.txt`) round-trip correctly when the target binary expects POSIX-ish paths.

### VS Code integrated terminal

- [ ] With the default profile (whatever it is locally): `wt-smoke world` prints `Hello world`.
- [ ] Switching to each of the configured profiles (PowerShell, cmd, Git Bash) gives the same result.

### Windows Terminal

- [ ] Each profile (PowerShell, cmd, Git Bash, WSL) sees the shim. WSL is expected to NOT find the shim because PATH translation across the Windows/WSL boundary is out of scope for v0.1.

## Use case: shim as a Git external diff driver

`shrt` shims are real executables, so they can be used wherever Git expects an external command:

```
shrt add wtdiff "code --diff {1} {2}"
git config --global diff.tool wtdiff
git config --global difftool.wtdiff.cmd "wtdiff $LOCAL $REMOTE"
```

- [ ] `git difftool` invokes the shim with the two file paths.
- [ ] Exit code of the diff tool propagates back to git correctly.

## Use case: shim invoked from a `.bat` or other shim

- [ ] Wrap a shim in another shim: `shrt add wt-wrap "wt-smoke {1}"` then `wt-wrap world` prints `Hello world`. Tests the runner spawning a shim via PATH+PATHEXT (`wt-smoke.exe` resolves through the shim directory).

## Cleanup

```
shrt remove wt-smoke
shrt remove wtdiff
shrt remove wt-wrap
```
