use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{AppHandle, Emitter};

#[cfg(unix)]
extern crate libc;

fn log(msg: &str) {
    eprintln!("[ACP] {}", msg);
}

type SharedStdin = Arc<StdMutex<std::process::ChildStdin>>;

pub struct AcpClient {
    child: Child,
    stdin: SharedStdin,
    stdout_lines: std::io::Lines<std::io::BufReader<ChildStdout>>,
    request_id: AtomicU64,
    pub session_id: Option<String>,
}

impl AcpClient {
    pub fn spawn(kiro_cli_path: &str) -> Result<Self, String> {
        log(&format!("Spawning: {}", kiro_cli_path));
        let mut cmd = Command::new(kiro_cli_path);
        cmd.arg("acp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn kiro-cli acp: {}", e))?;

        log(&format!("Spawned pid: {}", child.id()));
        let stdin = child.stdin.take().ok_or("Failed to get stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to get stdout")?;
        let reader = std::io::BufReader::new(stdout);

        Ok(Self {
            child,
            stdin: Arc::new(StdMutex::new(stdin)),
            stdout_lines: reader.lines(),
            request_id: AtomicU64::new(0),
            session_id: None,
        })
    }

    pub fn cancel_handle(&self) -> SharedStdin {
        self.stdin.clone()
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    fn write_request(&self, id: u64, method: &str, params: Value) -> Result<(), String> {
        let request = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let msg = serde_json::to_string(&request).unwrap();
        log(&format!(">>> [id={}] {}", id, method));
        let mut stdin = self.stdin.lock().map_err(|e| format!("Lock error: {}", e))?;
        stdin.write_all(format!("{}\n", msg).as_bytes()).map_err(|e| format!("Write error: {}", e))?;
        stdin.flush().map_err(|e| format!("Flush error: {}", e))
    }

    fn send_request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id();
        self.write_request(id, method, params)?;
        self.read_response(id)
    }

    fn read_response(&mut self, expected_id: u64) -> Result<Value, String> {
        loop {
            match self.stdout_lines.next() {
                Some(Ok(line)) => {
                    if line.trim().is_empty() { continue; }
                    let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
                    if msg.get("id").is_none() { continue; }
                    if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                        if id == expected_id {
                            if let Some(error) = msg.get("error") {
                                return Err(format!("ACP error: {}", error));
                            }
                            return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
                        }
                    }
                }
                Some(Err(e)) => return Err(format!("Read error: {}", e)),
                None => return Err("ACP process closed".to_string()),
            }
        }
    }

    pub fn initialize(&mut self) -> Result<Value, String> {
        log("Initializing...");
        self.send_request("initialize", json!({
            "protocolVersion": 1,
            "clientCapabilities": {},
            "clientInfo": { "name": "kiro-notebook", "version": "0.1.0" }
        }))
    }

    pub fn new_session(&mut self, cwd: &str) -> Result<String, String> {
        log(&format!("Creating session, cwd={}", cwd));
        let result = self.send_request("session/new", json!({ "cwd": cwd, "mcpServers": [] }))?;
        let session_id = result.get("sessionId").and_then(|v| v.as_str())
            .ok_or("No sessionId in response")?.to_string();
        log(&format!("Session created: {}", session_id));
        self.session_id = Some(session_id.clone());
        Ok(session_id)
    }

    pub fn load_session(&mut self, session_id: &str, cwd: &str) -> Result<(), String> {
        log(&format!("Loading session: {}", session_id));
        let _result = self.send_request("session/load", json!({
            "sessionId": session_id,
            "cwd": cwd,
            "mcpServers": []
        }))?;
        self.session_id = Some(session_id.to_string());
        log(&format!("Session loaded: {}", session_id));
        Ok(())
    }

    pub fn set_model(&mut self, model_id: &str) -> Result<(), String> {
        let session_id = self.session_id.as_ref().ok_or("No active session")?.clone();
        log(&format!("Setting model: {}", model_id));
        self.send_request("session/set_model", json!({
            "sessionId": session_id,
            "modelId": model_id
        }))?;
        Ok(())
    }

    pub fn prompt_streaming(&mut self, text: &str, session_id: &str, app: &AppHandle) -> Result<String, String> {
        let id = self.next_id();
        log(&format!(">>> [id={}] prompt: {}...", id, &text[..text.len().min(80)]));
        self.write_request(id, "session/prompt", json!({
            "sessionId": session_id,
            "prompt": [{ "type": "text", "text": text }]
        }))?;

        let mut full_response = String::new();

        loop {
            match self.stdout_lines.next() {
                Some(Ok(line)) => {
                    if line.trim().is_empty() { continue; }
                    let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };

                    if let Some(msg_id) = msg.get("id").and_then(|v| v.as_u64()) {
                        if msg_id == id {
                            let _ = app.emit("acp-done", json!({ "sessionId": session_id }));
                            return Ok(full_response);
                        }
                    }

                    if msg.get("method").and_then(|v| v.as_str()) == Some("session/update") {
                        if let Some(update) = msg.pointer("/params/update") {
                            let update_type = update.get("sessionUpdate").and_then(|v| v.as_str()).unwrap_or("");
                            match update_type {
                                "agent_message_chunk" => {
                                    if let Some(text) = update.pointer("/content/text").and_then(|v| v.as_str()) {
                                        full_response.push_str(text);
                                        let _ = app.emit("acp-chunk", json!({ "sessionId": session_id, "text": text }));
                                    }
                                }
                                "tool_call" => {
                                    let title = update.get("title").and_then(|v| v.as_str()).unwrap_or("Working");
                                    let status = update.get("status").and_then(|v| v.as_str()).unwrap_or("pending");
                                    let _ = app.emit("acp-status", json!({ "sessionId": session_id, "type": "tool_call", "title": title, "status": status }));
                                }
                                "tool_call_update" => {
                                    let status = update.get("status").and_then(|v| v.as_str()).unwrap_or("");
                                    let _ = app.emit("acp-status", json!({ "sessionId": session_id, "type": "tool_update", "status": status }));
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Some(Err(e)) => return Err(format!("Read error: {}", e)),
                None => {
                    if !full_response.is_empty() { return Ok(full_response); }
                    return Err("ACP process closed".to_string());
                }
            }
        }
    }

    pub fn kill(&mut self) {
        let pid = self.child.id();
        #[cfg(unix)]
        {
            unsafe { libc::kill(-(pid as i32), libc::SIGTERM); }
            std::thread::sleep(std::time::Duration::from_millis(100));
            unsafe { libc::kill(-(pid as i32), libc::SIGKILL); }
        }
        #[cfg(not(unix))]
        {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

impl Drop for AcpClient {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Send cancel to a session's stdin without needing exclusive access to AcpClient
pub fn send_cancel(stdin: &SharedStdin, session_id: &str, request_id: u64) -> Result<(), String> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "session/cancel",
        "params": { "sessionId": session_id }
    });
    let msg = serde_json::to_string(&request).unwrap();
    log(&format!(">>> [id={}] session/cancel", request_id));
    let mut stdin = stdin.lock().map_err(|e| format!("Lock error: {}", e))?;
    stdin.write_all(format!("{}\n", msg).as_bytes()).map_err(|e| format!("Write error: {}", e))?;
    stdin.flush().map_err(|e| format!("Flush error: {}", e))
}
