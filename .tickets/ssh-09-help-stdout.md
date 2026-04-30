---
id: ssh-09
status: done
wave: 12
version: v0.2.0
priority: 3
type: bug
tags: [cli, usability]
deps: []
---
# Write --help to stdout, not stderr

## Problem

`src/main.rs:246-258` — all help text uses `eprintln!`, which writes to stderr. This breaks standard CLI conventions where `--help` output goes to stdout. Piping fails: `brig-ssh --help | less` shows nothing because the help text goes to stderr while stdout is empty.

## Fix

Change `eprintln!` to `println!` for all help/usage output in the help printing block at `src/main.rs:246-258`. Keep error messages (invalid args, missing config) on stderr.

## Verification

- `brig-ssh --help | head -1` prints the first line of help text
- `brig-ssh --help 2>/dev/null` still shows help (stdout)
- `brig-ssh --help 1>/dev/null` shows nothing (no stderr leakage from help)
- `brig-ssh --badarg 2>&1 1>/dev/null` shows error on stderr
