---
id: ssh-12
status: done
wave: 12
version: v0.2.0
priority: 4
type: bug
tags: [manifest, correctness]
deps: []
---
# Fix manifest kind mismatch

## Problem

`manifest.toml:4` declares `kind = "declarative"` but also contains a `[persistent.socket]` section. These are contradictory: declarative skills are invoked per-call in ephemeral jails, while `[persistent.socket]` is for long-running services.

brig-ssh is invoked per-connection by sshd via `ForceCommand`, not as a persistent service managed by brig. The `[persistent.socket]` section is misleading and may cause brig to misinterpret the deployment model.

## Fix

Either:

1. **Remove `[persistent.socket]`** if brig-ssh is purely a per-connection binary invoked by sshd (most likely correct), or
2. **Change `kind = "persistent"`** if brig-ssh is intended to run as a brig-managed service with socket capabilities.

Option 1 is likely correct given the ForceCommand deployment model. Remove the `[persistent]` and `[persistent.socket]` sections entirely.

## Verification

- `manifest.toml` has no `[persistent]` section (if option 1)
- `kind` field matches the actual deployment model
- `cargo test` passes (manifest parsing tests)
