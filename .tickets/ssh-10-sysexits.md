---
id: ssh-10
status: done
wave: 12
version: v0.2.0
priority: 3
type: quality
tags: [cli, conventions]
deps: []
---
# Use sysexits.h exit codes

## Problem

`src/main.rs:267` — the binary exits with code 1 for every error class. This loses information for callers (sshd, scripts, monitoring) that could distinguish between configuration errors, connection failures, and runtime problems.

## Fix

Map error types to standard sysexits.h codes:

- `EX_USAGE` (64) — bad command-line arguments, missing required args
- `EX_DATAERR` (65) — malformed config file
- `EX_UNAVAILABLE` (69) — brig daemon socket not reachable
- `EX_TEMPFAIL` (75) — transient connection or timeout errors
- `EX_CONFIG` (78) — missing config file, missing required config keys

Define constants or an enum mapping `BrigSshError` variants to the appropriate exit code. Apply at the top-level error handler in `main()`.

## Verification

- `brig-ssh --badarg; echo $?` prints 64
- With missing config file: exit code is 78
- With unreachable socket: exit code is 69
- Normal successful session still exits 0
