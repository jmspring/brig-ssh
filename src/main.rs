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

const DEFAULT_SOCKET: &str = "/var/brig/sock/brig.sock";

#[derive(Serialize)]
struct BrigHello<'a> {
    #[serde(rename = "type")]
    msg_type: &'static str,
    name: &'a str,
    version: &'static str,
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
    fn connect(socket_path: &str, gateway_name: &str) -> Result<Self, String> {
        let stream = UnixStream::connect(socket_path)
            .map_err(|e| format!("failed to connect to brig socket at {}: {}", socket_path, e))?;
        let writer = stream.try_clone()
            .map_err(|e| format!("failed to clone socket: {}", e))?;
        let mut conn = BrigConnection { reader: BufReader::new(stream), writer };
        conn.handshake(gateway_name)?;
        Ok(conn)
    }

    fn handshake(&mut self, gateway_name: &str) -> Result<(), String> {
        let hello = BrigHello {
            msg_type: "hello",
            name: gateway_name,
            version: "0.1.0",
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
        let mut line = String::new();
        self.reader.read_line(&mut line)
            .map_err(|e| format!("failed to read from socket: {}", e))?;
        if line.is_empty() {
            return Err("socket closed".to_string());
        }
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

fn build_session_key() -> String {
    let prefix = env::var("BRIG_SESSION_PREFIX").unwrap_or_else(|_| "ssh".to_string());
    // BRIG_SSH_USER, then source IP from SSH_CLIENT ("ip port port"), then "unknown"
    let user = env::var("BRIG_SSH_USER").ok()
        .or_else(|| {
            env::var("SSH_CLIENT").ok()
                .and_then(|val| val.split_whitespace().next().map(String::from))
        })
        .unwrap_or_else(|| "unknown".to_string());
    format!("{}-{}", prefix, user)
}

fn run() -> Result<(), String> {
    let socket_path = env::var("BRIG_SOCKET").unwrap_or_else(|_| DEFAULT_SOCKET.to_string());
    let gateway_name = env::var("BRIG_GATEWAY_NAME").unwrap_or_else(|_| "ssh-gateway".to_string());

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

    let mut brig = BrigConnection::connect(&socket_path, &gateway_name)?;
    let response = brig.submit_task(&task, &session)?;
    println!("{}", response);
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("fatal: {}", e);
        process::exit(1);
    }
}
