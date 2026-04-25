# brig-ssh

SSH gateway for [Brig](https://github.com/jmspring/brig).

A minimal bridge that reads a task from stdin (or `SSH_ORIGINAL_COMMAND`), submits it to Brig's unix domain socket, prints the response to stdout, and exits. No async, no SSH library — sshd handles the SSH protocol, brig-ssh is just a command.

## Prerequisites

- Brig running in daemon mode (`brig -d`)
- SSH access to the brig host

## Build

```sh
cargo build --release
```

## Install

```sh
make                     # build release binary
sudo make install        # install binary + skill manifest
```

This installs:
- `/usr/local/bin/brig-ssh`
- `/usr/local/share/brig/skills/ssh-gateway/manifest.toml`

Then register with brig:

```sh
brig skill add /usr/local/share/brig/skills/ssh-gateway
```

## SSH Setup

### ForceCommand in authorized_keys

Restrict an SSH key to only run brig-ssh:

```
# ~/.ssh/authorized_keys (on the brig host)
command="/usr/local/bin/brig-ssh" ssh-ed25519 AAAA... user@laptop
```

Then from the client:

```sh
ssh brig-host "check disk usage"
```

### Dedicated brig user with sshd_config

```
# /etc/ssh/sshd_config (or /usr/local/etc/ssh/sshd_config on FreeBSD)
Match User brig
    ForceCommand /usr/local/bin/brig-ssh
    AllowTcpForwarding no
    X11Forwarding no
```

Then:

```sh
ssh brig@brig-host "list all jails"
```

### Direct invocation

For testing without SSH:

```sh
echo "check disk usage" | brig-ssh
echo "what skills are available?" | BRIG_SOCKET=/tmp/brig.sock brig-ssh
```

## How it works

1. Reads the task from `SSH_ORIGINAL_COMMAND` (ForceCommand mode) or one line from stdin
2. Builds a session key: `{prefix}-{user}` (e.g., `ssh-192.168.1.5`)
3. Connects to Brig's unix socket at `/var/brig/sock/brig.sock` (or `BRIG_SOCKET`)
4. Sends a hello handshake, receives capabilities
5. Submits the task, prints status lines to stderr
6. Prints the response to stdout and exits

## Socket protocol

The gateway uses Brig's newline-delimited JSON protocol:

```
→ {"type":"hello","name":"ssh-gateway","version":"0.1.3"}
← {"type":"welcome","capabilities":["submit_task","read_status"]}
→ {"type":"task","content":"check disk usage","session":"ssh-192.168.1.5"}
← {"type":"status","skill":"shell","jail":"w-xxx","state":"running"}
← {"type":"response","content":"All pools healthy.","session":"ssh-192.168.1.5"}
```

## Environment variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `BRIG_SOCKET` | No | `/var/brig/sock/brig.sock` | Path to Brig's unix socket |
| `BRIG_GATEWAY_NAME` | No | `ssh-gateway` | Gateway identity for brig (audit/logging) |
| `BRIG_SESSION_PREFIX` | No | `ssh` | Session key prefix (e.g., `ssh-{user}`) |
| `BRIG_SSH_USER` | No | (from `SSH_CLIENT`) | Override user in session key |

## License

BSD-2-Clause
