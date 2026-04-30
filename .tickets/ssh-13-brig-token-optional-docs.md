---
id: ssh-13
status: done
wave: 15
version: v0.3.0
priority: 4
type: docs
tags: [documentation, auth]
deps: [ssh-11]
---
# BRIG_TOKEN marked optional but should document clearly

## Problem

`src/main.rs:213-215` — when `BRIG_TOKEN` is not set, the code emits a warning but continues connecting without authentication. The help text and README should make the security implications clear: without a token, any process that can reach the socket can submit tasks.

## Fix

In both `--help` output and README.md:

- State that `BRIG_TOKEN` is optional but recommended for production
- Explain that without it, the connection is unauthenticated
- Note that the daemon may reject unauthenticated connections depending on its configuration

## Verification

- `brig-ssh --help` mentions BRIG_TOKEN with clear optional/recommended language
- README.md env table entry for BRIG_TOKEN includes security guidance
- No code changes needed beyond help text and docs
