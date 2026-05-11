// Minimal end-to-end smoke for the codex app-server client.
//
// Spawns `codex app-server` rooted at a temp workspace, runs initialize ->
// thread/start -> turn/start with a tiny prompt, streams events to a JSONL
// file under analysis/codex-sessions/<run_id>/events.jsonl, exits when the
// terminal `turn/completed` arrives. Auto-replies to any server-initiated
// requests via Session::auto_reply.
//
// Run with:
//   cargo run --features cli --example codex_app_server_smoke

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use orchestrator_harness::codex_app_server::{append_event_jsonl, Session};

fn main() -> Result<()> {
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
    let workspace = PathBuf::from(format!("/tmp/codex_test/{run_id}"));
    std::fs::create_dir_all(&workspace).context("create temp workspace")?;
    std::fs::create_dir_all(&session_dir).context("create session dir")?;
    let events_path = session_dir.join("events.jsonl");

    eprintln!("[smoke] run_id={run_id}");
    eprintln!("[smoke] workspace={}", workspace.display());
    eprintln!("[smoke] session_dir={}", session_dir.display());
    eprintln!("[smoke] events_jsonl={}", events_path.display());

    let session = Session::spawn(&workspace, &session_dir).context("spawn session")?;

    let init = session
        .initialize("orchestrator-harness", "Orchestrator Harness", "0.1.0")
        .context("initialize")?;
    eprintln!(
        "[smoke] initialized userAgent={:?} platformOs={:?} codexHome={:?}",
        init.user_agent, init.platform_os, init.codex_home
    );

    let thread = session.thread_start(&workspace).context("thread/start")?;
    eprintln!("[smoke] thread.id={}", thread.thread_id);

    let prompt = "Reply with just the word OK and nothing else.";
    let turn = session
        .turn_start(&thread.thread_id, prompt, &workspace, "smoke turn")
        .context("turn/start")?;
    eprintln!("[smoke] turn.id={}", turn.turn_id);

    let deadline = Instant::now() + Duration::from_secs(120);
    let mut completed = false;
    let mut completed_status: Option<String> = None;
    let mut turn_event_count: u64 = 0;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match session.events.recv_timeout(remaining.min(Duration::from_secs(5))) {
            Ok(ev) => {
                append_event_jsonl(&events_path, &ev).ok();
                turn_event_count += 1;
                // Auto-reply to server-initiated requests under approvalPolicy=never.
                let replied = session.auto_reply(&ev).unwrap_or(false);
                if !replied {
                    eprintln!("[smoke] event method={} id={:?}", ev.method, ev.id);
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
                return Err(anyhow!("event channel closed before turn/completed"));
            }
        }
    }

    if !completed {
        eprintln!("[smoke] timed out waiting for turn/completed (events seen: {turn_event_count})");
        let _ = session.shutdown();
        return Err(anyhow!("timed out waiting for turn/completed"));
    }

    eprintln!(
        "[smoke] turn/completed status={:?} events_seen={turn_event_count}",
        completed_status
    );
    session.shutdown().ok();
    eprintln!("[smoke] done. events.jsonl -> {}", events_path.display());
    Ok(())
}
