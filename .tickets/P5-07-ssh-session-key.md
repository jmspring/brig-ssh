# P5-07: Fix brig-ssh session key to enable per-user memory scoping

**Phase:** 5 — Gateway Hardening
**Severity:** HIGH
**Effort:** S (<15min)
**Component:** brig-ssh
**Personas:** 1/7 (product-owner)
**Depends on:** P2-01 (gateway token auth)
**Blocks:** none

## Problem

`brig-ssh/src/main.rs`: The session key format is `{prefix}-{user}` (e.g., `ssh-192.168.1.5`), which has only 2 hyphen-delimited segments. Per the session key contract in DAEMON_AND_GATEWAYS.md, keys with fewer than 3 segments fall back to `global` memory scope. All SSH users share a single memory pool. Discord and Telegram correctly use 3-segment keys.

## Files to change

- `brig-ssh/src/main.rs` — change session key format to 3 segments

## Fix

Change from `{prefix}-{user}` to `{prefix}-ssh-{user}`:
```rust
let session_key = format!("{}-ssh-{}", session_prefix, user_id);
```

Or use the IP-based approach: `{prefix}-{client_ip}-{user}`.

## Verification

- SSH session keys have 3+ hyphen-delimited segments
- Per-user memory scoping works (different SSH users get different memory)
- Session key is logged correctly in brig's session database
