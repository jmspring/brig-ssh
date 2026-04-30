# P5-02: Strip terminal escape sequences from SSH gateway output

**Phase:** 5 — Gateway Hardening
**Severity:** MEDIUM
**Effort:** S (~30min)
**Component:** brig-ssh
**Personas:** 1/7 (adversarial-llm)
**Depends on:** P2-01 (gateway token auth)
**Blocks:** none

## Problem

`brig-ssh/src/main.rs:167`: `println!("{}", response)` outputs raw LLM text with no control character or ANSI escape stripping. A compromised LLM can produce escape sequences that change terminal title, modify terminal state, or in rare emulators execute commands via OSC sequences.

## Files to change

- `brig-ssh/src/main.rs:167` — strip ANSI escapes and non-printable control characters

## Fix

Add a sanitization function:
```rust
fn strip_control_chars(s: &str) -> String {
    s.chars().filter(|c| {
        !c.is_control() || *c == '\n' || *c == '\t'
    }).collect()
}
```

Apply before printing: `println!("{}", strip_control_chars(&response));`

For more thorough protection, also strip ANSI escape sequences with a regex: `\x1b\[[0-9;]*[a-zA-Z]`

## Verification

- Response containing `\x1b[31mred\x1b[0m` prints without color codes
- Response containing `\x1b]0;evil title\x07` does NOT change terminal title
- Normal text with newlines and tabs preserved
