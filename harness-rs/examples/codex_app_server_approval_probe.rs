// Approval handshake probe for the `codex app-server` client.
//
// Goal: provoke a real server-initiated approval request, confirm we reply
// with the right shape/decision string, and confirm the turn terminates
// cleanly. The smoke driver only verified the happy path; this driver
// specifically targets the auto_reply code path that pane-mode users rely
// on to keep Codex unblocked when no human keystroke is available.
//
// Usage:
//   cargo run --features cli --example codex_app_server_approval_probe
//   cargo run --features cli --example codex_app_server_approval_probe -- --variant write-outside
//   cargo run --features cli --example codex_app_server_approval_probe -- --variant untrusted-shell
//
// Variants pick (approvalPolicy, sandboxPolicy, prompt) tuples expected to
// surface different approval methods. Each run writes a tagged JSONL of all
// inbound/outbound JSON-RPC traffic to
// analysis/codex-sessions/<run_id>/events.jsonl.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use orchestrator_harness::codex_app_server::{InboundEvent, Session};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy)]
enum Variant {
    /// approvalPolicy=on-request, workspace-write sandbox, prompt asks to write outside writable roots.
    WriteOutside,
    /// approvalPolicy=untrusted, workspace-write sandbox, prompt does any shell op (untrusted asks for almost everything).
    UntrustedShell,
    /// approvalPolicy=on-request, workspace-write sandbox, network-disabled, prompt requires network.
    NetworkBlocked,
}

impl Variant {
    fn parse(s: &str) -> Result<Variant> {
        match s {
            "write-outside" => Ok(Variant::WriteOutside),
            "untrusted-shell" => Ok(Variant::UntrustedShell),
            "network-blocked" => Ok(Variant::NetworkBlocked),
            _ => Err(anyhow!(
                "unknown variant {s:?}; valid: write-outside, untrusted-shell, network-blocked"
            )),
        }
    }
}

struct Plan {
    approval_policy: &'static str,
    /// turn/start.sandboxPolicy (camelCase map form). thread/start.sandbox (string) is set in spawn.
    sandbox_policy: Value,
    sandbox_kebab: &'static str,
    prompt: &'static str,
    label: &'static str,
}

fn plan_for(variant: Variant, workspace: &PathBuf) -> Plan {
    match variant {
        Variant::WriteOutside => Plan {
            approval_policy: "on-request",
            sandbox_policy: json!({
                "type": "workspaceWrite",
                "writableRoots": [workspace.to_string_lossy()],
                "networkAccess": false,
                "excludeTmpdirEnvVar": false,
                "excludeSlashTmp": false,
            }),
            sandbox_kebab: "workspace-write",
            prompt: "Write the file /etc/codex_approval_probe_marker.txt with content 'hello'. \
                     Use the shell tool. If the system blocks you or asks for approval, request \
                     additional permissions and report what happened.",
            label: "write-outside",
        },
        Variant::UntrustedShell => Plan {
            approval_policy: "untrusted",
            sandbox_policy: json!({
                "type": "workspaceWrite",
                "writableRoots": [workspace.to_string_lossy()],
                "networkAccess": false,
                "excludeTmpdirEnvVar": false,
                "excludeSlashTmp": false,
            }),
            sandbox_kebab: "workspace-write",
            prompt: "Run the shell command `echo hello-from-probe` and report the output.",
            label: "untrusted-shell",
        },
        Variant::NetworkBlocked => Plan {
            approval_policy: "on-request",
            sandbox_policy: json!({
                "type": "workspaceWrite",
                "writableRoots": [workspace.to_string_lossy()],
                "networkAccess": false,
                "excludeTmpdirEnvVar": false,
                "excludeSlashTmp": false,
            }),
            sandbox_kebab: "workspace-write",
            prompt: "Use the shell tool to run `curl -sS https://example.com/` and tell me the HTTP status. \
                     Network access is blocked by sandbox; if it asks, request permission to access the network.",
            label: "network-blocked",
        },
    }
}

fn parse_args() -> Result<Variant> {
    let mut args = std::env::args().skip(1);
    let mut variant = Variant::UntrustedShell; // default: most likely to fire approvals
    while let Some(a) = args.next() {
        match a.as_str() {
            "--variant" => {
                let v = args
                    .next()
                    .ok_or_else(|| anyhow!("--variant requires a value"))?;
                variant = Variant::parse(&v)?;
            }
            "-h" | "--help" => {
                eprintln!(
                    "usage: codex_app_server_approval_probe [--variant write-outside|untrusted-shell|network-blocked]"
                );
                std::process::exit(0);
            }
            other => return Err(anyhow!("unknown arg {other:?}")),
        }
    }
    Ok(variant)
}

/// Append a row to events.jsonl with a `tag` field so the round-trip is easy
/// to find later. Mirrors `append_event_jsonl` but writes our tagged shape.
fn append_tagged(path: &std::path::Path, row: Value) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open events jsonl {}", path.display()))?;
    let line = serde_json::to_string(&row)?;
    f.write_all(line.as_bytes())?;
    f.write_all(b"\n")?;
    f.flush()?;
    Ok(())
}

fn classify_inbound(ev: &InboundEvent) -> &'static str {
    if ev.id.is_some() {
        "INBOUND_REQUEST"
    } else {
        "INBOUND_NOTIFICATION"
    }
}

/// Return the auto-reply payload we would send for the given method, if any.
/// Mirrors `Session::auto_reply` so we can also LOG the outgoing reply
/// before sending. Keep in sync with the real impl.
fn planned_reply(method: &str) -> Option<Value> {
    match method {
        "item/commandExecution/requestApproval"
        | "item/fileChange/requestApproval"
        | "item/permissions/requestApproval" => Some(json!({ "decision": "acceptForSession" })),
        "applyPatchApproval" | "execCommandApproval" => {
            Some(json!({ "decision": "approved_for_session" }))
        }
        "item/tool/call" => Some(json!({
            "success": false,
            "output": "unknown tool",
            "contentItems": [{ "type": "inputText", "text": "unknown tool" }],
        })),
        "item/tool/requestUserInput" => {
            Some(json!({ "contentItems": [{ "type": "inputText", "text": "" }] }))
        }
        "mcpServer/elicitation/request" => Some(json!({ "action": "decline" })),
        "account/chatgptAuthTokens/refresh" => Some(json!({ "tokens": Value::Null })),
        _ => None,
    }
}

fn main() -> Result<()> {
    let variant = parse_args()?;
    let now = chrono::Utc::now();
    let run_id = now.format("%Y%m%dT%H%M%SZ").to_string();
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow!("manifest dir has no parent"))?
        .to_path_buf();

    let session_dir = repo_root
        .join("analysis")
        .join("codex-sessions")
        .join(&run_id);
    let workspace = PathBuf::from(format!("/tmp/codex_approval_probe_{run_id}"));
    std::fs::create_dir_all(&workspace).context("create temp workspace")?;
    std::fs::create_dir_all(&session_dir).context("create session dir")?;
    let events_path = session_dir.join("events.jsonl");

    let plan = plan_for(variant, &workspace);

    eprintln!("[probe] run_id={run_id}");
    eprintln!("[probe] variant={}", plan.label);
    eprintln!("[probe] approval_policy={}", plan.approval_policy);
    eprintln!("[probe] sandbox_kebab={}", plan.sandbox_kebab);
    eprintln!("[probe] sandbox_policy={}", plan.sandbox_policy);
    eprintln!("[probe] workspace={}", workspace.display());
    eprintln!("[probe] session_dir={}", session_dir.display());
    eprintln!("[probe] events_jsonl={}", events_path.display());

    // Header row in JSONL so the file is self-describing.
    append_tagged(
        &events_path,
        json!({
            "tag": "RUN_HEADER",
            "run_id": run_id,
            "variant": plan.label,
            "approval_policy": plan.approval_policy,
            "sandbox_policy": plan.sandbox_policy,
            "prompt": plan.prompt,
            "workspace": workspace.to_string_lossy(),
        }),
    )?;

    let session = Session::spawn(&workspace, &session_dir).context("spawn session")?;

    // initialize + initialized
    let init = session
        .initialize("orchestrator-harness", "Orchestrator Harness", "0.1.0")
        .context("initialize")?;
    eprintln!(
        "[probe] initialized userAgent={:?} platformOs={:?}",
        init.user_agent, init.platform_os
    );
    append_tagged(
        &events_path,
        json!({
            "tag": "OUTBOUND_REQUEST_RESULT",
            "method": "initialize",
            "result": init.raw,
        }),
    )?;

    // thread/start with the chosen approval policy + sandbox kebab string.
    // Use a low-level request rather than session.thread_start so we can pass
    // arbitrary policy values.
    let thread_start_params = json!({
        "approvalPolicy": plan.approval_policy,
        "sandbox": plan.sandbox_kebab,
        "cwd": workspace.to_string_lossy(),
        "dynamicTools": [],
    });
    append_tagged(
        &events_path,
        json!({"tag": "OUTBOUND_REQUEST", "method": "thread/start", "params": thread_start_params}),
    )?;
    let thread_result = session
        .request("thread/start", thread_start_params, Duration::from_secs(15))
        .context("thread/start")?;
    append_tagged(
        &events_path,
        json!({"tag": "OUTBOUND_REQUEST_RESULT", "method": "thread/start", "result": thread_result}),
    )?;
    let thread_id = thread_result
        .get("thread")
        .and_then(|t| t.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("thread/start missing thread.id: {thread_result}"))?
        .to_string();
    eprintln!("[probe] thread.id={thread_id}");

    // turn/start with sandboxPolicy as map.
    let turn_start_params = json!({
        "threadId": thread_id,
        "input": [{ "type": "text", "text": plan.prompt }],
        "cwd": workspace.to_string_lossy(),
        "title": format!("approval probe ({})", plan.label),
        "approvalPolicy": plan.approval_policy,
        "sandboxPolicy": plan.sandbox_policy,
    });
    append_tagged(
        &events_path,
        json!({"tag": "OUTBOUND_REQUEST", "method": "turn/start", "params": turn_start_params}),
    )?;
    let turn_result = session
        .request("turn/start", turn_start_params, Duration::from_secs(30))
        .context("turn/start")?;
    append_tagged(
        &events_path,
        json!({"tag": "OUTBOUND_REQUEST_RESULT", "method": "turn/start", "result": turn_result}),
    )?;
    let turn_id = turn_result
        .get("turn")
        .and_then(|t| t.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("turn/start missing turn.id: {turn_result}"))?
        .to_string();
    eprintln!("[probe] turn.id={turn_id}");

    // Drive the event loop.
    let deadline = Instant::now() + Duration::from_secs(90);
    let mut completed = false;
    let mut completed_status: Option<String> = None;
    let mut event_count: u64 = 0;
    let mut approval_request_methods: Vec<String> = Vec::new();
    let mut auto_reply_methods_triggered: Vec<String> = Vec::new();
    let mut unhandled_server_requests: Vec<String> = Vec::new();
    let mut timed_out = false;

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match session
            .events
            .recv_timeout(remaining.min(Duration::from_secs(5)))
        {
            Ok(ev) => {
                event_count += 1;
                let tag = classify_inbound(&ev);
                append_tagged(
                    &events_path,
                    json!({
                        "tag": tag,
                        "method": ev.method,
                        "id": ev.id,
                        "params": ev.params,
                    }),
                )?;

                if tag == "INBOUND_REQUEST" {
                    let is_approval = matches!(
                        ev.method.as_str(),
                        "item/commandExecution/requestApproval"
                            | "item/fileChange/requestApproval"
                            | "item/permissions/requestApproval"
                            | "applyPatchApproval"
                            | "execCommandApproval"
                    );
                    if is_approval {
                        approval_request_methods.push(ev.method.clone());
                    }

                    if let Some(reply_payload) = planned_reply(&ev.method) {
                        // Log the reply payload BEFORE sending it.
                        append_tagged(
                            &events_path,
                            json!({
                                "tag": "OUTBOUND_REPLY",
                                "in_reply_to_method": ev.method,
                                "in_reply_to_id": ev.id,
                                "result": reply_payload,
                            }),
                        )?;
                        // Send via the existing client path so we exercise the real code.
                        let replied = session.auto_reply(&ev).unwrap_or(false);
                        if replied {
                            auto_reply_methods_triggered.push(ev.method.clone());
                            eprintln!("[probe] auto_replied to {} id={:?}", ev.method, ev.id);
                        } else {
                            // Should not happen since planned_reply matched.
                            eprintln!(
                                "[probe] WARN: planned_reply matched {} but auto_reply returned false",
                                ev.method
                            );
                        }
                    } else {
                        unhandled_server_requests.push(ev.method.clone());
                        eprintln!(
                            "[probe] UNHANDLED server-initiated request method={} id={:?}",
                            ev.method, ev.id
                        );
                        // Still send a generic error reply so the server doesn't hang.
                        if let Some(id) = ev.id.clone() {
                            let err = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "error": { "code": -32601, "message": format!("method not handled: {}", ev.method) }
                            });
                            // We don't have a public send_raw, so use reply_result with a plain object;
                            // but the server expects either result or error — reply_result only sends result,
                            // so just log we couldn't.
                            append_tagged(
                                &events_path,
                                json!({"tag": "OUTBOUND_ERROR_DROPPED", "in_reply_to_method": ev.method, "would_have_sent": err}),
                            )?;
                        }
                    }
                }

                if ev.method == "turn/completed" {
                    completed = true;
                    completed_status = ev
                        .params
                        .get("turn")
                        .and_then(|t| t.get("status"))
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    break;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!("[probe] event channel closed before turn/completed");
                break;
            }
        }
    }

    if !completed && Instant::now() >= deadline {
        timed_out = true;
        eprintln!("[probe] HARD TIMEOUT (90s) reached");
        // Best effort: try to interrupt and shut down cleanly.
        let _ = session.turn_interrupt(&thread_id, &turn_id);
    }

    let terminal_status = if timed_out {
        "timeout".to_string()
    } else if completed {
        completed_status.clone().unwrap_or_else(|| "unknown".into())
    } else {
        "channel_closed".to_string()
    };

    let pass = !approval_request_methods.is_empty()
        && !auto_reply_methods_triggered.is_empty()
        && completed
        && completed_status.as_deref() == Some("completed");

    let summary = json!({
        "tag": "RUN_SUMMARY",
        "outcome": if pass { "PASS" } else { "FAIL" },
        "variant": plan.label,
        "approval_policy": plan.approval_policy,
        "events_seen": event_count,
        "approval_request_methods": approval_request_methods,
        "auto_reply_methods_triggered": auto_reply_methods_triggered,
        "unhandled_server_requests": unhandled_server_requests,
        "turn_terminal_status": terminal_status,
        "timed_out": timed_out,
    });
    append_tagged(&events_path, summary.clone())?;

    eprintln!("[probe] ---- summary ----");
    eprintln!("[probe] {}", serde_json::to_string_pretty(&summary)?);

    let _ = session.shutdown();

    if pass {
        eprintln!("[probe] PASS");
        Ok(())
    } else {
        eprintln!(
            "[probe] FAIL (see summary above and {})",
            events_path.display()
        );
        // exit code 1 for FAIL but don't unwind a panic
        std::process::exit(1);
    }
}
