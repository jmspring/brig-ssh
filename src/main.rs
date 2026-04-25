//! brig-ssh: SSH gateway for Brig
//!
//! Reads a task from stdin (or SSH_ORIGINAL_COMMAND), submits it to
//! Brig's unix domain socket, prints the response to stdout, and exits.
//! Intended as an SSH ForceCommand.

use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process;
use std::time::Duration;

#[derive(Serialize)]
struct BrigHello<'a> {
    #[serde(rename = "type")]
    msg_type: &'static str,
    name: &'a str,
    version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<&'a str>,
}

#[derive(Deserialize)]
struct BrigWelcome {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    capabilities: Vec<String>,
}

#[derive(Serialize)]
struct BrigTask {
    #[serde(rename = "type")]
    msg_type: &'static str,
    content: String,
    session: String,
}

#[derive(Deserialize)]
struct BrigMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    skill: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    code: String,
    #[serde(default)]
    message: String,
}

struct BrigConnection {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl BrigConnection {
    fn connect(socket_path: &str, gateway_name: &str, token: Option<&str>) -> Result<Self, String> {
        let stream = UnixStream::connect(socket_path)
            .map_err(|e| format!("failed to connect to brig socket at {}: {}", socket_path, e))?;
        stream.set_read_timeout(Some(Duration::from_secs(300)))
            .map_err(|e| format!("failed to set read timeout: {}", e))?;
        stream.set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| format!("failed to set write timeout: {}", e))?;
        let writer = stream.try_clone()
            .map_err(|e| format!("failed to clone socket: {}", e))?;
        let mut conn = BrigConnection { reader: BufReader::new(stream), writer };
        conn.handshake(gateway_name, token)?;
        Ok(conn)
    }

    fn handshake(&mut self, gateway_name: &str, token: Option<&str>) -> Result<(), String> {
        let hello = BrigHello {
            msg_type: "hello",
            name: gateway_name,
            version: env!("CARGO_PKG_VERSION"),
            token,
        };
        self.send(&hello)?;
        let welcome: BrigWelcome = self.recv()?;
        if welcome.msg_type != "welcome" {
            return Err(format!("expected welcome, got {}", welcome.msg_type));
        }
        if !welcome.capabilities.contains(&"submit_task".to_string()) {
            return Err("brig does not grant submit_task capability".to_string());
        }
        eprintln!("connected to brig, capabilities: {:?}", welcome.capabilities);
        Ok(())
    }

    fn send<T: Serialize>(&mut self, msg: &T) -> Result<(), String> {
        let json = serde_json::to_string(msg)
            .map_err(|e| format!("failed to serialize message: {}", e))?;
        writeln!(self.writer, "{}", json)
            .map_err(|e| format!("failed to write to socket: {}", e))?;
        self.writer.flush()
            .map_err(|e| format!("failed to flush socket: {}", e))?;
        Ok(())
    }

    fn recv<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T, String> {
        let line = read_line_bounded(&mut self.reader, MAX_MESSAGE_BYTES)?;
        serde_json::from_str(&line)
            .map_err(|e| format!("failed to parse message: {} (line: {})", e, line.trim()))
    }

    fn submit_task(&mut self, content: &str, session: &str) -> Result<String, String> {
        let task = BrigTask {
            msg_type: "task",
            content: content.to_string(),
            session: session.to_string(),
        };
        self.send(&task)?;
        loop {
            let msg: BrigMessage = self.recv()?;
            match msg.msg_type.as_str() {
                "response" => return Ok(msg.content),
                "status" => eprintln!("  [{}] {}", msg.skill, msg.state),
                "error" => return Err(format!("brig error {}: {}", msg.code, msg.message)),
                other => eprintln!("  unexpected message type: {}", other),
            }
        }
    }
}

/// Strip ANSI escape sequences and control characters from output.
/// Preserves newlines and tabs for readability.
fn sanitize_terminal_output(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_escape = false;
    let chars = s.chars();

    for c in chars {
        if c == '\x1b' {
            in_escape = true;
            continue;
        }
        if in_escape {
            // ESC [ ... letter ends a CSI sequence
            if c.is_ascii_alphabetic() || c == '~' {
                in_escape = false;
            }
            // Also handle OSC sequences (ESC ] ... BEL/ST)
            continue;
        }
        // Keep printable chars, newlines, and tabs
        if !c.is_control() || c == '\n' || c == '\t' {
            result.push(c);
        }
    }
    result
}

/// Read a line from a buffered reader with an upper bound on total bytes.
/// Prevents a malicious or misbehaving server from exhausting memory.
fn read_line_bounded(reader: &mut BufReader<UnixStream>, max_bytes: usize) -> Result<String, String> {
    let mut line = String::new();
    loop {
        let available = reader.fill_buf().map_err(|e| format!("read error: {}", e))?;
        if available.is_empty() {
            if line.is_empty() {
                return Err("connection closed".into());
            }
            return Ok(line);
        }
        if let Some(pos) = available.iter().position(|&b| b == b'\n') {
            line.push_str(&String::from_utf8_lossy(&available[..=pos]));
            let consume = pos + 1;
            reader.consume(consume);
            return Ok(line);
        }
        if line.len() + available.len() > max_bytes {
            return Err(format!("message exceeds {} byte limit", max_bytes));
        }
        line.push_str(&String::from_utf8_lossy(available));
        let len = available.len();
        reader.consume(len);
    }
}

/// Maximum size of a single JSON message from the brig socket (1 MB).
const MAX_MESSAGE_BYTES: usize = 1_048_576;

fn build_session_key() -> String {
    let prefix = env::var("BRIG_SESSION_PREFIX").unwrap_or_else(|_| "ssh".to_string());
    // BRIG_SSH_USER, then source IP from SSH_CLIENT ("ip port port"), then "unknown"
    let user = env::var("BRIG_SSH_USER").ok()
        .or_else(|| {
            env::var("SSH_CLIENT").ok()
                .and_then(|val| val.split_whitespace().next().map(String::from))
        })
        .unwrap_or_else(|| "unknown".to_string());
    format!("{}-ssh-{}", prefix, user)
}

fn run() -> Result<(), String> {
    let socket_path = env::var("BRIG_SOCKET").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| "/root".into());
        let user_path = format!("{}/.brig/sock/brig.sock", home);
        if std::path::Path::new(&user_path).exists() {
            user_path
        } else {
            "/var/brig/sock/brig.sock".into()
        }
    });
    let gateway_name = env::var("BRIG_GATEWAY_NAME").unwrap_or_else(|_| "ssh-gateway".to_string());

    let token = env::var("BRIG_TOKEN").ok();
    if token.is_none() {
        eprintln!("warning: BRIG_TOKEN not set — generate one with: brig token create ssh-gateway");
    }

    // Get task: SSH_ORIGINAL_COMMAND (ForceCommand mode) or stdin
    let task = if let Ok(cmd) = env::var("SSH_ORIGINAL_COMMAND") {
        if cmd.is_empty() {
            return Err("SSH_ORIGINAL_COMMAND is empty. Usage: ssh brig-host \"your task\"".to_string());
        }
        cmd
    } else {
        let mut line = String::new();
        io::stdin().read_line(&mut line)
            .map_err(|e| format!("failed to read from stdin: {}", e))?;
        let line = line.trim().to_string();
        if line.is_empty() {
            return Err("no task provided. Usage: echo \"your task\" | brig-ssh".to_string());
        }
        line
    };

    let session = build_session_key();
    eprintln!("[{}] <- {}", session, task);

    let mut brig = BrigConnection::connect(&socket_path, &gateway_name, token.as_deref())?;
    let response = brig.submit_task(&task, &session)?;
    println!("{}", sanitize_terminal_output(&response));
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!("brig-ssh — SSH gateway for Brig");
        eprintln!();
        eprintln!("Usage: brig-ssh");
        eprintln!("  Reads a task from stdin or SSH_ORIGINAL_COMMAND,");
        eprintln!("  submits to Brig via unix socket, prints response.");
        eprintln!();
        eprintln!("Environment variables:");
        eprintln!("  BRIG_TOKEN            Brig IPC authentication token (required)");
        eprintln!("  BRIG_SOCKET           Socket path (default: ~/.brig/sock/brig.sock)");
        eprintln!("  BRIG_GATEWAY_NAME     Gateway name (default: ssh-gateway)");
        eprintln!("  BRIG_SESSION_PREFIX   Session prefix (default: ssh)");
        eprintln!("  BRIG_SSH_USER         User identifier (default: from SSH_CLIENT)");
        std::process::exit(0);
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("brig-ssh {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    if let Err(e) = run() {
        eprintln!("fatal: {}", e);
        process::exit(1);
    }
}
