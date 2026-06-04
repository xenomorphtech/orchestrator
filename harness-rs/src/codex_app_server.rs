// JSON-RPC stdio client for `codex app-server` (Codex v2 protocol).
//
// Synchronous: spawns child, two reader threads (stdout JSON parser, stderr tee
// to log file), and a small registry of pending requests behind a Mutex. Caller
// drives the protocol by issuing `initialize` -> `initialized` (notification)
// -> `thread/start` -> `turn/start` and pulling events off a channel until a
// terminal `turn/completed` arrives.

#![cfg(feature = "cli")]

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::{Receiver, Sender, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};

/// Anything inbound from codex that isn't a response to one of our requests.
/// `id` is `Some` for server-initiated requests (we must reply), `None` for
/// notifications.
#[derive(Debug, Clone)]
pub struct InboundEvent {
    pub method: String,
    pub id: Option<Value>,
    pub params: Value,
    pub raw: Value,
}

/// What the writer thread does next.
enum WriteCmd {
    Send(String),
    Shutdown,
}

type PendingMap = Arc<Mutex<HashMap<i64, SyncSender<Result<Value, String>>>>>;

pub struct Session {
    next_id: AtomicI64,
    pending: PendingMap,
    writer_tx: Sender<WriteCmd>,
    pub events: Receiver<InboundEvent>,
    pub stderr_log_path: PathBuf,
    pub session_dir: PathBuf,
    pub model: String,
    child: Mutex<Option<Child>>,
    workers: Mutex<Vec<JoinHandle<()>>>,
}

#[derive(Debug, Clone)]
pub struct InitializeResult {
    pub user_agent: Option<String>,
    pub codex_home: Option<String>,
    pub platform_family: Option<String>,
    pub platform_os: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct ThreadStartResult {
    pub thread_id: String,
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct TurnStartResult {
    pub turn_id: String,
    pub raw: Value,
}

impl Session {
    /// Spawn `codex app-server` rooted at `cwd`. `session_dir` is where stderr
    /// log gets written.
    pub fn spawn(cwd: &Path, session_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(session_dir)
            .with_context(|| format!("create session dir {}", session_dir.display()))?;
        let stderr_log_path = session_dir.join("codex.stderr.log");
        let stderr_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&stderr_log_path)
            .with_context(|| format!("open stderr log {}", stderr_log_path.display()))?;

        let model = codex_model();
        let mut child = Command::new("codex")
            .arg("app-server")
            .arg("-c")
            .arg(format!("model={}", toml_string_literal(&model)))
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("spawn `codex app-server` with model {model}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("child stdin missing"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("child stdout missing"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("child stderr missing"))?;

        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let (writer_tx, writer_rx) = std::sync::mpsc::channel::<WriteCmd>();
        let (events_tx, events_rx) = std::sync::mpsc::channel::<InboundEvent>();

        let mut workers: Vec<JoinHandle<()>> = Vec::new();

        // stderr -> log file (best effort).
        workers.push(thread::spawn(move || {
            let mut log = stderr_log;
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let _ = log.write_all(line.as_bytes());
                        let _ = log.flush();
                    }
                    Err(_) => break,
                }
            }
        }));

        // stdin writer.
        workers.push(thread::spawn(move || {
            let mut stdin: ChildStdin = stdin;
            for cmd in writer_rx {
                match cmd {
                    WriteCmd::Send(line) => {
                        if stdin.write_all(line.as_bytes()).is_err() {
                            break;
                        }
                        if stdin.write_all(b"\n").is_err() {
                            break;
                        }
                        if stdin.flush().is_err() {
                            break;
                        }
                    }
                    WriteCmd::Shutdown => break,
                }
            }
        }));

        // stdout reader + dispatcher.
        let pending_for_reader = Arc::clone(&pending);
        let events_tx_for_reader = events_tx.clone();
        workers.push(thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(_) => break,
                }
                let trimmed = line.trim_end_matches(['\r', '\n']);
                if trimmed.is_empty() {
                    continue;
                }
                let parsed: Value = match serde_json::from_str(trimmed) {
                    Ok(v) => v,
                    Err(_) => {
                        // Forward as a synthetic event so the caller can see it.
                        let _ = events_tx_for_reader.send(InboundEvent {
                            method: "$malformed".to_string(),
                            id: None,
                            params: json!({ "raw": trimmed }),
                            raw: Value::String(trimmed.to_string()),
                        });
                        continue;
                    }
                };
                // Response to one of our requests has `id` and either `result`
                // or `error`, with no `method`.
                let has_method = parsed.get("method").is_some();
                let has_result_or_error =
                    parsed.get("result").is_some() || parsed.get("error").is_some();
                let id_val = parsed.get("id").cloned();
                if !has_method && has_result_or_error {
                    if let Some(num) = id_val.as_ref().and_then(|v| v.as_i64()) {
                        let mut guard = pending_for_reader.lock().unwrap();
                        if let Some(tx) = guard.remove(&num) {
                            if let Some(err) = parsed.get("error") {
                                let _ = tx.send(Err(err.to_string()));
                            } else {
                                let _ = tx
                                    .send(Ok(parsed.get("result").cloned().unwrap_or(Value::Null)));
                            }
                            continue;
                        }
                    }
                    // Response we weren't tracking — surface as event.
                    let _ = events_tx_for_reader.send(InboundEvent {
                        method: "$untracked_response".to_string(),
                        id: id_val,
                        params: parsed.clone(),
                        raw: parsed,
                    });
                    continue;
                }
                // Notification or server-initiated request.
                let method = parsed
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("$unknown")
                    .to_string();
                let params = parsed.get("params").cloned().unwrap_or(Value::Null);
                let _ = events_tx_for_reader.send(InboundEvent {
                    method,
                    id: id_val,
                    params,
                    raw: parsed,
                });
            }
        }));

        Ok(Session {
            next_id: AtomicI64::new(1),
            pending,
            writer_tx,
            events: events_rx,
            stderr_log_path,
            session_dir: session_dir.to_path_buf(),
            model,
            child: Mutex::new(Some(child)),
            workers: Mutex::new(workers),
        })
    }

    fn alloc_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    fn send_raw(&self, payload: Value) -> Result<()> {
        let line = serde_json::to_string(&payload).context("serialize jsonrpc message")?;
        self.writer_tx
            .send(WriteCmd::Send(line))
            .map_err(|_| anyhow!("writer thread closed"))?;
        Ok(())
    }

    /// Send a request and block until matching response arrives (or timeout).
    pub fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value> {
        let id = self.alloc_id();
        let (tx, rx) = sync_channel::<Result<Value, String>>(1);
        {
            let mut guard = self.pending.lock().unwrap();
            guard.insert(id, tx);
        }
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.send_raw(payload)?;
        match rx.recv_timeout(timeout) {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(anyhow!("rpc error from {method}: {e}")),
            Err(_) => {
                // Reap pending entry on timeout.
                let mut guard = self.pending.lock().unwrap();
                guard.remove(&id);
                Err(anyhow!("rpc timeout waiting for {method}"))
            }
        }
    }

    pub fn notify(&self, method: &str, params: Value) -> Result<()> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.send_raw(payload)
    }

    /// Reply to a server-initiated request with `{id, result}`.
    pub fn reply_result(&self, id: Value, result: Value) -> Result<()> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        });
        self.send_raw(payload)
    }

    pub fn initialize(
        &self,
        client_name: &str,
        client_title: &str,
        client_version: &str,
    ) -> Result<InitializeResult> {
        let result = self.request(
            "initialize",
            json!({
                "capabilities": { "experimentalApi": true },
                "clientInfo": {
                    "name": client_name,
                    "title": client_title,
                    "version": client_version,
                }
            }),
            Duration::from_secs(15),
        )?;
        let user_agent = result
            .get("userAgent")
            .and_then(|v| v.as_str())
            .map(String::from);
        let codex_home = result
            .get("codexHome")
            .and_then(|v| v.as_str())
            .map(String::from);
        let platform_family = result
            .get("platformFamily")
            .and_then(|v| v.as_str())
            .map(String::from);
        let platform_os = result
            .get("platformOs")
            .and_then(|v| v.as_str())
            .map(String::from);
        // Per protocol: send `initialized` notification right after.
        self.notify("initialized", json!({}))?;
        Ok(InitializeResult {
            user_agent,
            codex_home,
            platform_family,
            platform_os,
            raw: result,
        })
    }

    pub fn thread_start(&self, cwd: &Path) -> Result<ThreadStartResult> {
        // NOTE: thread/start.sandbox is the kebab-case STRING form;
        //       turn/start.sandboxPolicy below is the camelCase MAP form.
        let result = self.request(
            "thread/start",
            json!({
                "approvalPolicy": "never",
                "sandbox": "danger-full-access",
                "cwd": cwd.to_string_lossy(),
                "dynamicTools": [],
            }),
            Duration::from_secs(15),
        )?;
        let thread_id = result
            .get("thread")
            .and_then(|t| t.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("thread/start missing thread.id: {result}"))?
            .to_string();
        Ok(ThreadStartResult {
            thread_id,
            raw: result,
        })
    }

    pub fn turn_start(
        &self,
        thread_id: &str,
        prompt: &str,
        cwd: &Path,
        title: &str,
    ) -> Result<TurnStartResult> {
        let result = self.request(
            "turn/start",
            json!({
                "threadId": thread_id,
                "input": [{ "type": "text", "text": prompt }],
                "cwd": cwd.to_string_lossy(),
                "title": title,
                "approvalPolicy": "never",
                "sandboxPolicy": { "type": "dangerFullAccess" },
            }),
            Duration::from_secs(30),
        )?;
        let turn_id = result
            .get("turn")
            .and_then(|t| t.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("turn/start missing turn.id: {result}"))?
            .to_string();
        Ok(TurnStartResult {
            turn_id,
            raw: result,
        })
    }

    pub fn turn_interrupt(&self, thread_id: &str, turn_id: &str) -> Result<Value> {
        self.request(
            "turn/interrupt",
            json!({ "threadId": thread_id, "turnId": turn_id }),
            Duration::from_secs(10),
        )
    }

    /// Auto-reply to common server-initiated requests under approvalPolicy=never.
    /// Returns true if it was an approval/tool-call request and we replied,
    /// false if the caller should handle this event itself.
    pub fn auto_reply(&self, ev: &InboundEvent) -> Result<bool> {
        let Some(id) = ev.id.clone() else {
            return Ok(false);
        };
        let result = match ev.method.as_str() {
            // Approval-family server requests (v2): accept for the session.
            "item/commandExecution/requestApproval"
            | "item/fileChange/requestApproval"
            | "item/permissions/requestApproval" => {
                json!({ "decision": "acceptForSession" })
            }
            // Legacy flat methods (kept for compatibility).
            "applyPatchApproval" | "execCommandApproval" => {
                json!({ "decision": "approved_for_session" })
            }
            // Tool calls — we don't ship dynamic tools yet.
            "item/tool/call" => {
                json!({
                    "success": false,
                    "output": "unknown tool",
                    "contentItems": [{ "type": "inputText", "text": "unknown tool" }],
                })
            }
            // Tool input solicitation — refuse politely.
            "item/tool/requestUserInput" => {
                json!({ "contentItems": [{ "type": "inputText", "text": "" }] })
            }
            // Elicitation — decline.
            "mcpServer/elicitation/request" => {
                json!({ "action": "decline" })
            }
            // Token refresh — say we have nothing.
            "account/chatgptAuthTokens/refresh" => {
                json!({ "tokens": Value::Null })
            }
            _ => return Ok(false),
        };
        self.reply_result(id, result)?;
        Ok(true)
    }

    pub fn shutdown(&self) -> Result<()> {
        let _ = self.writer_tx.send(WriteCmd::Shutdown);
        let mut guard = self.child.lock().unwrap();
        if let Some(mut child) = guard.take() {
            // Give it a brief moment to exit on its own after stdin closes.
            for _ in 0..20 {
                match child.try_wait() {
                    Ok(Some(_)) => return Ok(()),
                    Ok(None) => thread::sleep(Duration::from_millis(50)),
                    Err(_) => break,
                }
            }
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.writer_tx.send(WriteCmd::Shutdown);
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        if let Ok(mut workers) = self.workers.lock() {
            for h in workers.drain(..) {
                let _ = h.join();
            }
        }
    }
}

fn codex_model() -> String {
    std::env::var("HARNESS_CODEX_MODEL")
        .or_else(|_| std::env::var("CODEX_MODEL"))
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "gpt-5.5".to_string())
}

fn toml_string_literal(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// Helper used by the example/tests: write each inbound event as one JSONL row.
pub fn append_event_jsonl(path: &Path, ev: &InboundEvent) -> Result<()> {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open events jsonl {}", path.display()))?;
    let row = json!({
        "method": ev.method,
        "id": ev.id,
        "params": ev.params,
    });
    let line = serde_json::to_string(&row)?;
    f.write_all(line.as_bytes())?;
    f.write_all(b"\n")?;
    f.flush()?;
    Ok(())
}
