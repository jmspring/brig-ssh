---
id: ssh-11
status: done
wave: 12
version: v0.2.0
priority: 3
type: docs
tags: [documentation, env]
deps: []
---
# Document BRIG_TOKEN in README

## Problem

The `BRIG_TOKEN` environment variable is referenced in `--help` output but is not listed in the README.md environment variable table. Users reading the README have no way to discover that token-based authentication is available without running `--help`.

## Fix

Add `BRIG_TOKEN` to the environment variables table in `README.md`. Include:

- Variable name: `BRIG_TOKEN`
- Description: Pre-shared key for authenticating to the brig daemon socket
- Required: No (optional, but recommended for production)
- Note: If unset, a warning is emitted but the connection proceeds without authentication

## Verification

- `BRIG_TOKEN` appears in the README env table
- Description matches the behavior in `src/main.rs:213-215`
