# brig-ssh

SSH gateway for Brig. Reads a task from stdin or SSH_ORIGINAL_COMMAND,
submits it to Brig's unix domain socket, prints the response, and exits.

## What This Is

A standalone, minimal gateway that:
- Reads one task from stdin or ForceCommand
- Submits it to Brig via unix socket
- Prints the response to stdout and exits

Not a daemon. Not interactive. Just a one-shot bridge.

## Project Structure

```
brig-ssh/
├── Cargo.toml          # 2 deps: serde, serde_json
├── src/main.rs         # ~150 lines, the entire program
├── manifest.toml       # Brig skill manifest (declarative)
├── Makefile            # BSD make (build, install, clean)
└── README.md           # Usage instructions
```

## Dependencies

Two crates, no more:
- `serde` (with derive) — serialization
- `serde_json` — JSON parsing

No HTTP library (stdin/stdout only). No async runtime. No SSH library.

## Socket Protocol

Newline-delimited JSON over unix domain socket:

```
→ {"type":"hello","name":"ssh-gateway","version":"0.3.0"}
← {"type":"welcome","capabilities":["submit_task","read_status"]}
→ {"type":"task","content":"check disk usage","session":"ssh-192.168.1.5"}
← {"type":"status","skill":"shell","jail":"w-xxx","state":"running"}
← {"type":"response","content":"All pools healthy.","session":"ssh-192.168.1.5"}
```

Session keys: `{prefix}-{user}` where user is from SSH_CLIENT IP or BRIG_SSH_USER.

## Environment Variables

| Variable | Required | Default |
|----------|----------|---------|
| `BRIG_SOCKET` | No | `/var/brig/sock/brig.sock` |
| `BRIG_GATEWAY_NAME` | No | `ssh-gateway` |
| `BRIG_SESSION_PREFIX` | No | `ssh` |
| `BRIG_SSH_USER` | No | (from SSH_CLIENT, or "unknown") |

## Build & Run

```sh
cargo build --release

# Test without SSH (requires brig daemon running)
echo "hello" | ./target/release/brig-ssh

# Test with custom socket
echo "hello" | BRIG_SOCKET=/tmp/brig.sock ./target/release/brig-ssh
```

## What Works

- Task input from SSH_ORIGINAL_COMMAND (ForceCommand mode)
- Task input from stdin (pipe mode)
- Brig socket handshake (hello/welcome)
- Task submission and response handling
- Session key derivation from SSH_CLIENT or BRIG_SSH_USER
- Status line printing to stderr
- Graceful error handling with actionable messages

## Design Constraints

From the main Brig project:
- No async — synchronous control flow throughout
- Minimal dependencies — 2 crates only
- No SSH library — sshd handles the protocol
- Separate repo — gateways don't share code with brig
- One file — the entire program lives in src/main.rs
