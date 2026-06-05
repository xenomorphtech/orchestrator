mod config;
mod data;

use std::fs;
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use chrono::Local;
use config::DashboardConfig;
use data::DataClient;
use regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

const AUTH_COOKIE_NAME: &str = "dash_auth";
const AUTH_COOKIE_MAX_AGE: i64 = 30 * 24 * 3600;
const PASSWORD_SALT: &str = "dashboard-salt-v1:";
const GENERAL_TALK_CHANNEL: &str = "general";
const TALK_WORKER_TIMEOUT_SECONDS: &str = "3600";

#[derive(Clone)]
struct AppState {
    cfg: DashboardConfig,
    data: DataClient,
    password_hash: String,
    secret_key: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = DashboardConfig::load()?;
    let state = Arc::new(AppState {
        password_hash: hash_password(&cfg.dashboard_password),
        secret_key: load_or_create_secret_key(&cfg.secret_key_path)?,
        data: DataClient::new(cfg.clone())?,
        cfg,
    });
    let app = router(state.clone());
    let addr: SocketAddr = state
        .cfg
        .bind_addr
        .parse()
        .with_context(|| format!("parse bind addr {}", state.cfg.bind_addr))?;
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;
    axum::serve(listener, app).await.context("serve dashboard")
}

fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/login", get(login_get).post(login_post))
        .route("/logout", get(logout))
        .route("/", get(dashboard))
        .route("/cycles", get(cycles))
        .route("/cycle/{cid}", get(cycle_detail))
        .route("/goals", get(goals))
        .route("/goaltree", get(goaltree))
        .route("/goal/{key}", get(goal_detail))
        .route("/paths", get(paths))
        .route("/agents", get(agents))
        .route("/facts", get(facts))
        .route("/services", get(services))
        .route("/memory", get(memory_index))
        .route("/memory/{name}", get(memory_detail))
        .route("/briefings", get(briefings_index))
        .route("/briefings/{name}", get(briefing_detail))
        .route("/artifacts", get(artifacts))
        .route("/artifacts/upload", post(artifact_upload))
        .route("/artifacts/context", post(artifact_context))
        .route("/artifacts/raw/{sha256}", get(artifact_raw))
        .route("/talk", get(talk_get).post(talk_post))
        .route("/talk/{channel}/since", get(talk_since))
        .route("/talk/new", post(talk_new))
        .route("/talk/clear", post(talk_clear))
        .route("/talk/delete", post(talk_delete))
        .route("/api/cycles", get(api_cycles))
        .route("/api/goals", get(api_goals))
        .route("/api/goal/{key}", get(api_goal_detail))
        .route("/api/paths", get(api_paths))
        .route("/api/agents", get(api_agents))
        .route("/api/facts", get(api_facts))
        .route("/api/services", get(api_services))
        .route("/api/memory", get(api_memory))
        .route("/api/briefings", get(api_briefings))
        .route("/api/adherence", get(api_adherence))
        .route("/api/progress", get(api_progress))
        .route("/api/goaltree/{goal_key}", get(api_goaltree))
        .route("/api/goaltree/{goal_key}/tick", post(api_goaltree_tick))
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
        .with_state(state)
}

#[derive(Deserialize)]
struct LoginForm {
    password: String,
}

async fn login_get(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if is_authed(&headers, &state) {
        return Redirect::to("/talk?c=general").into_response();
    }
    render_login(None).into_response()
}

async fn login_post(State(state): State<Arc<AppState>>, Form(form): Form<LoginForm>) -> Response {
    if hash_password(&form.password) == state.password_hash {
        let mut response = Redirect::to("/talk?c=general").into_response();
        let cookie = format!(
            "{}={}; Max-Age={}; HttpOnly; SameSite=Lax; Path=/",
            AUTH_COOKIE_NAME,
            signed_auth_value(&state),
            AUTH_COOKIE_MAX_AGE
        );
        response
            .headers_mut()
            .insert(header::SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());
        return response;
    }
    render_login(Some("Wrong password")).into_response()
}

async fn logout() -> Response {
    let mut response = Redirect::to("/login").into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("dash_auth=; Max-Age=0; HttpOnly; SameSite=Lax; Path=/"),
    );
    response
}

async fn dashboard(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let episodes = latest_episodes(&state, 8);
    let goals = goals_data(&state);
    let active: Vec<&Value> = goals
        .iter()
        .filter(|g| value_display(g.get("status")) == "active")
        .collect();
    let paths = paths_portfolio_data(&state);
    let services = rows_to_values(state.data.harness_table(&["services"]));

    let mut body = String::from("<h1 class=\"text-2xl mb-4 text-zinc-100\">Dashboard</h1>");

    // active campaigns
    body.push_str("<section class=\"mb-6\">");
    body.push_str(&format!(
        "<h2 class=\"text-lg text-zinc-100 mb-2\">Active campaigns ({} of {})</h2>",
        active.len(),
        goals.len()
    ));
    if active.is_empty() {
        body.push_str("<div class=\"text-zinc-500\">(no active goals)</div>");
    } else {
        body.push_str(
            "<table class=\"w-full\"><thead><tr class=\"text-zinc-500 text-left\">\
             <th class=\"py-1 pr-3\">goal</th><th>prio</th><th>title</th></tr></thead><tbody>",
        );
        for g in &active {
            let key = value_display(g.get("goal_key"));
            body.push_str(&format!(
                "<tr><td class=\"pr-3 align-top\"><a class=\"text-sky-400 hover:underline\" href=\"/goal/{}\">{}</a></td>\
                 <td class=\"pr-3 align-top\">{}</td>\
                 <td class=\"align-top\">{}</td></tr>",
                html_escape(&key),
                html_escape(&key),
                html_escape(&value_display(g.get("priority"))),
                html_escape(&value_display(g.get("title"))),
            ));
        }
        body.push_str("</tbody></table>");
    }
    body.push_str("</section>");

    // path portfolio summary
    if let Some(path_goals) = paths.get("goals").and_then(Value::as_object) {
        if !path_goals.is_empty() {
            body.push_str(
                "<section class=\"mb-6\"><h2 class=\"text-lg text-zinc-100 mb-2\">Path portfolio</h2>\
                 <table class=\"w-full\"><thead><tr class=\"text-zinc-500 text-left\">\
                 <th class=\"pr-3 py-1\">goal</th><th class=\"pr-3\">metric</th>\
                 <th class=\"pr-3\">progress</th><th>paths</th></tr></thead><tbody>",
            );
            for (gk, g) in path_goals {
                let cur = value_display_or(g.get("current"), "?");
                let tgt = value_display_or(g.get("target"), "?");
                let paths_arr = g.get("paths").and_then(Value::as_array);
                let n = paths_arr.map(|a| a.len()).unwrap_or(0);
                let statuses = paths_arr
                    .map(|a| {
                        a.iter()
                            .map(|p| value_display_or(p.get("status"), "?"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                body.push_str(&format!(
                    "<tr><td class=\"pr-3 align-top\">{}</td>\
                     <td class=\"pr-3 align-top\">{}</td>\
                     <td class=\"pr-3 align-top\">{}/{}</td>\
                     <td class=\"align-top\">{} ({})</td></tr>",
                    html_escape(gk),
                    html_escape(&value_display(g.get("metric_name"))),
                    html_escape(&cur),
                    html_escape(&tgt),
                    n,
                    html_escape(&statuses),
                ));
            }
            body.push_str("</tbody></table></section>");
        }
    }

    // recent cycles
    body.push_str(
        "<section class=\"mb-6\"><h2 class=\"text-lg text-zinc-100 mb-2\">Recent cycles \
         <a href=\"/cycles\" class=\"text-xs text-sky-400\">(all →)</a></h2>\
         <table class=\"w-full\"><thead><tr class=\"text-zinc-500 text-left\">\
         <th class=\"pr-3 py-1\">id</th><th class=\"pr-3\">when</th><th>summary</th></tr></thead><tbody>",
    );
    for e in &episodes {
        let id = value_display(e.get("id"));
        body.push_str(&format!(
            "<tr><td class=\"pr-3 align-top text-zinc-500\">{}</td>\
             <td class=\"pr-3 align-top text-xs text-zinc-400\">{}</td>\
             <td class=\"align-top\"><a class=\"hover:text-white\" href=\"/cycle/{}\">{}</a></td></tr>",
            html_escape(&id),
            html_escape(&truncate_chars(&value_display(e.get("created_at")), 19)),
            html_escape(&id),
            html_escape(&truncate_chars(&value_display(e.get("summary")), 180)),
        ));
    }
    body.push_str("</tbody></table></section>");

    // services
    if !services.is_empty() {
        body.push_str(
            "<section class=\"mb-6\"><h2 class=\"text-lg text-zinc-100 mb-2\">Services</h2>\
             <table class=\"w-full\"><thead><tr class=\"text-zinc-500 text-left\">\
             <th class=\"pr-3 py-1\">service</th><th class=\"pr-3\">type</th>\
             <th class=\"pr-3\">status</th><th>target</th></tr></thead><tbody>",
        );
        for s in &services {
            let status = value_display(s.get("last_status"));
            let cls = if status == "healthy" {
                "text-emerald-400"
            } else {
                "text-amber-400"
            };
            body.push_str(&format!(
                "<tr><td class=\"pr-3 align-top\">{}</td>\
                 <td class=\"pr-3 align-top\">{}</td>\
                 <td class=\"pr-3 align-top {}\">{}</td>\
                 <td class=\"align-top text-xs text-zinc-500\">{}</td></tr>",
                html_escape(&value_display(s.get("service_name"))),
                html_escape(&value_display(s.get("service_type"))),
                cls,
                html_escape(&status),
                html_escape(&value_display(s.get("check_target"))),
            ));
        }
        body.push_str("</tbody></table></section>");
    }

    render_page("dashboard", &body, 30).into_response()
}

async fn cycles(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let episodes = latest_episodes(&state, 200);
    let mut body = format!(
        "<h1 class=\"text-2xl mb-4\">Cycles <span class=\"text-sm text-zinc-500\">(latest {})</span></h1>",
        episodes.len()
    );
    if episodes.is_empty() {
        body.push_str("<p class=\"text-zinc-500\">No cycles recorded yet.</p>");
    } else {
        body.push_str(
            "<div class=\"overflow-x-auto rounded border border-zinc-800\"><table class=\"min-w-full text-left text-xs\">\
             <thead class=\"bg-zinc-900 text-zinc-400\"><tr>\
             <th class=\"px-3 py-2\">id</th><th class=\"px-3 py-2\">when</th><th class=\"px-3 py-2\">summary</th>\
             <th class=\"px-3 py-2\">frontier</th><th class=\"px-3 py-2\">stall</th></tr></thead><tbody>",
        );
        for episode in &episodes {
            let progress = episode.get("goal_progress_json");
            let frontier = progress.and_then(|v| v.get("frontier"));
            let id = value_display(episode.get("id"));
            let summary = value_display(episode.get("summary"));
            body.push_str(&format!(
                "<tr class=\"border-t border-zinc-800 align-top hover:bg-zinc-900/40\">\
                 <td class=\"px-3 py-2 text-zinc-500\"><a class=\"hover:text-sky-400\" href=\"/cycle/{}\">{}</a></td>\
                 <td class=\"px-3 py-2 text-zinc-400 whitespace-nowrap\">{}</td>\
                 <td class=\"px-3 py-2 text-zinc-100\"><a class=\"text-sky-300 hover:underline\" href=\"/cycle/{}\">{}</a></td>\
                 <td class=\"px-3 py-2 text-zinc-300\">{}</td>\
                 <td class=\"px-3 py-2 text-zinc-300\">{}</td></tr>",
                html_escape(&id),
                html_escape(&id),
                html_escape(&value_display(episode.get("created_at"))),
                html_escape(&id),
                html_escape(&summary),
                html_escape(&value_display(frontier.and_then(|v| v.get("id")))),
                html_escape(&value_display(frontier.and_then(|v| v.get("stall")))),
            ));
        }
        body.push_str("</tbody></table></div>");
    }
    render_page("cycles", &body, 30).into_response()
}

async fn cycle_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(cid): Path<i64>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let Some(e) = fetch_episode(&state, cid) else {
        return not_found_page();
    };
    let mut body = format!(
        "<h1 class=\"text-2xl mb-1\">Cycle {}</h1>",
        html_escape(&value_display(e.get("id")))
    );
    body.push_str(&format!(
        "<div class=\"text-zinc-500 text-xs mb-3\">{}</div>",
        html_escape(&value_display(e.get("created_at")))
    ));
    body.push_str(&format!(
        "<section class=\"mb-4\">\
         <h2 class=\"text-lg text-zinc-100 mb-2\">Full cycle description</h2>\
         <div class=\"bg-zinc-900 border border-zinc-800 rounded p-3\">\
         <pre class=\"whitespace-pre-wrap text-sm leading-6\">{}</pre></div></section>",
        html_escape(&value_display(e.get("summary")))
    ));
    body.push_str("<div class=\"grid grid-cols-1 lg:grid-cols-3 gap-3\">");
    for (k, label) in [
        ("agent_statuses_json", "Agents"),
        ("actions_taken_json", "Actions"),
        ("goal_progress_json", "Goal progress"),
    ] {
        let rendered = match e.get(k) {
            Some(v) if !json_is_empty(v) => {
                serde_json::to_string_pretty(v).unwrap_or_else(|_| "(none)".to_string())
            }
            _ => "(none)".to_string(),
        };
        body.push_str(&format!(
            "<div class=\"bg-zinc-900 border border-zinc-800 rounded p-3\">\
             <div class=\"text-xs text-zinc-500 mb-1\">{}</div>\
             <pre class=\"text-xs\">{}</pre></div>",
            label,
            html_escape(&rendered)
        ));
    }
    body.push_str("</div>");
    body.push_str(
        "<div class=\"mt-4\"><a class=\"text-sky-400 hover:underline\" href=\"/cycles\">← all cycles</a></div>",
    );
    render_page(&format!("cycle {cid}"), &body, 0).into_response()
}

fn fetch_episode(state: &AppState, cid: i64) -> Option<Value> {
    if let Ok(rows) = state.data.sql_query(&format!(
        "SELECT id, created_at, summary, agent_statuses_json, actions_taken_json, goal_progress_json FROM episodes WHERE id = {cid}"
    )) {
        if let Some(mut row) = rows.into_iter().next() {
            parse_json_field(&mut row, "agent_statuses_json");
            parse_json_field(&mut row, "actions_taken_json");
            parse_json_field(&mut row, "goal_progress_json");
            return Some(Value::Object(row.into_iter().collect()));
        }
    }
    latest_episodes(state, 500)
        .into_iter()
        .find(|e| value_i64(e.get("id")) == Some(cid))
}

fn json_is_empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Object(m) => m.is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::String(s) => s.is_empty(),
        _ => false,
    }
}

async fn goals(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let goals = goals_data(&state);
    let mut body = String::from("<h1 class=\"text-2xl mb-4\">Goals</h1>");
    body.push_str(
        "<table class=\"w-full\"><thead><tr class=\"text-zinc-500 text-left\">\
         <th class=\"pr-3 py-1\">status</th><th class=\"pr-3\">prio</th>\
         <th class=\"pr-3\">key</th><th>title</th></tr></thead><tbody>",
    );
    for goal in &goals {
        let status = value_display(goal.get("status"));
        let cls = match status.as_str() {
            "active" => "b-active",
            "pending" => "b-pending",
            "done" => "b-done",
            "cancelled" => "b-cancelled",
            _ => "b-pending",
        };
        let priority = value_display(goal.get("priority"));
        let goal_key = value_display(goal.get("goal_key"));
        let title = value_display(goal.get("title"));
        body.push_str(&format!(
            "<tr><td class=\"pr-3 align-top\"><span class=\"badge {}\">{}</span></td>\
             <td class=\"pr-3 align-top text-zinc-500\">{}</td>\
             <td class=\"pr-3 align-top\"><a class=\"text-sky-400 hover:underline\" href=\"/goal/{}\">{}</a></td>\
             <td class=\"align-top\">{}</td></tr>",
            cls,
            html_escape(&status),
            html_escape(&priority),
            html_escape(&goal_key),
            html_escape(&goal_key),
            html_escape(&title),
        ));
    }
    body.push_str("</tbody></table>");
    render_page("goals", &body, 0).into_response()
}

async fn goaltree(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let mut body = String::from("<h1 class=\"text-2xl mb-4\">Goal Tree</h1>");
    let tree = match goal_tree_data(&state, None) {
        Ok(tree) => tree,
        Err(err) => {
            body.push_str(&format!(
                "<div class=\"rounded border border-zinc-800 bg-zinc-900 p-3 text-zinc-400\">Goal tree is absent or unreadable from the DB.<br><span class=\"text-xs\">{}</span></div>",
                html_escape(&err.to_string())
            ));
            return render_page("goaltree", &body, 0).into_response();
        }
    };
    body.push_str(&render_goal_tree(&tree, "harness DB"));
    render_page("goaltree", &body, 0).into_response()
}

async fn goal_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let goals = goals_data(&state);
    let Some(g) = goals
        .iter()
        .find(|g| value_display(g.get("goal_key")) == key)
    else {
        return not_found_page();
    };
    let status = value_display(g.get("status"));
    let mut body = format!(
        "<h1 class=\"text-2xl mb-2\">{}</h1>",
        html_escape(&value_display(g.get("goal_key")))
    );
    body.push_str(&format!(
        "<div class=\"mb-2\"><span class=\"badge b-{}\">{}</span> \
         <span class=\"text-zinc-500 text-xs\">prio {}</span></div>",
        html_escape(&status),
        html_escape(&status),
        html_escape(&value_display(g.get("priority"))),
    ));
    body.push_str(&format!(
        "<div class=\"mb-3 text-zinc-300\">{}</div>",
        html_escape(&value_display(g.get("title")))
    ));
    let detail = value_display(g.get("detail"));
    if !detail.is_empty() && detail != "(none)" {
        body.push_str(&format!(
            "<div class=\"bg-zinc-900 border border-zinc-800 rounded p-3 mb-3\"><pre>{}</pre></div>",
            html_escape(&detail)
        ));
    }
    body.push_str("<div class=\"grid grid-cols-2 gap-3 text-xs\">");
    for k in [
        "success_fact_key",
        "completion_report",
        "created_at",
        "updated_at",
    ] {
        body.push_str(&format!(
            "<div class=\"bg-zinc-900 border border-zinc-800 rounded p-2\">\
             <div class=\"text-zinc-500\">{}</div>\
             <div>{}</div></div>",
            k,
            html_escape(&value_display(g.get(k))),
        ));
    }
    body.push_str("</div>");
    render_page(&format!("goal {key}"), &body, 0).into_response()
}

async fn paths(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let p = paths_portfolio_data(&state);
    let mut body = String::from("<h1 class=\"text-2xl mb-4\">Path portfolio</h1>");
    let empty = p.is_null() || p.as_object().is_some_and(|o| o.is_empty());
    if empty {
        body.push_str(
            "<div class=\"text-zinc-500\">(empty / unreadable harness DB paths table)</div>",
        );
    } else if let Some(goals) = p.get("goals").and_then(Value::as_object) {
        for (gk, g) in goals {
            body.push_str(
                "<section class=\"mb-5 bg-zinc-900 border border-zinc-800 rounded p-3\">",
            );
            body.push_str(&format!(
                "<h2 class=\"text-lg mb-1\">{}</h2>",
                html_escape(gk)
            ));
            body.push_str(&format!(
                "<div class=\"text-xs text-zinc-500 mb-2\">{} — current {}/{} — last move {}</div>",
                html_escape(&value_display(g.get("metric_name"))),
                html_escape(&value_display_or(g.get("current"), "?")),
                html_escape(&value_display_or(g.get("target"), "?")),
                html_escape(&value_display_or(g.get("last_move_at"), "?")),
            ));
            match g.get("paths").and_then(Value::as_array) {
                Some(arr) if !arr.is_empty() => {
                    body.push_str(
                        "<table class=\"w-full text-xs\"><thead><tr class=\"text-zinc-500 text-left\">\
                         <th class=\"pr-2\">name</th><th class=\"pr-2\">status</th><th class=\"pr-2\">stall</th>\
                         <th class=\"pr-2\">worker</th><th>hypothesis / falsification</th></tr></thead><tbody>",
                    );
                    for pt in arr {
                        let status = value_display(pt.get("status"));
                        let cls = path_status_badge_class(&status);
                        body.push_str(&format!(
                            "<tr><td class=\"pr-2 align-top\">{}</td>\
                             <td class=\"pr-2 align-top\"><span class=\"badge {}\">{}</span></td>\
                             <td class=\"pr-2 align-top\">{}</td>\
                             <td class=\"pr-2 align-top text-zinc-400\">{}</td>\
                             <td class=\"align-top text-zinc-300\"><div>{}</div>\
                             <div class=\"text-zinc-500\">{}</div></td></tr>",
                            html_escape(&value_display(pt.get("name"))),
                            cls,
                            html_escape(&status),
                            html_escape(&stall_counter_display(pt.get("stall_counter"))),
                            html_escape(&value_display(pt.get("worker"))),
                            html_escape(&value_display(pt.get("hypothesis"))),
                            html_escape(&value_display(pt.get("falsification"))),
                        ));
                    }
                    body.push_str("</tbody></table>");
                }
                _ => body.push_str("<div class=\"text-zinc-500 text-xs\">(no paths)</div>"),
            }
            body.push_str("</section>");
        }
    }
    render_page("paths", &body, 0).into_response()
}

async fn agents(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let agents = rows_to_values(state.data.harness_table(&["agents"]));
    let mut body = String::from("<h1 class=\"text-2xl mb-4\">Agents</h1>");
    if agents.is_empty() {
        body.push_str("<div class=\"text-zinc-500\">(no agents registered)</div>");
    } else {
        body.push_str(
            "<table class=\"w-full\"><thead><tr class=\"text-zinc-500 text-left\">\
             <th class=\"pr-3 py-1\">name</th><th class=\"pr-3\">kind</th><th class=\"pr-3\">workdir</th>\
             <th>description</th></tr></thead><tbody>",
        );
        for ag in &agents {
            body.push_str(&format!(
                "<tr><td class=\"pr-3 align-top\">{}</td>\
                 <td class=\"pr-3 align-top text-xs text-zinc-500\">{}</td>\
                 <td class=\"pr-3 align-top text-xs\">{}</td>\
                 <td class=\"align-top\">{}</td></tr>",
                html_escape(&value_display(ag.get("agent_name"))),
                html_escape(&value_display(ag.get("kind"))),
                html_escape(&value_display(ag.get("workdir"))),
                html_escape(&value_display(ag.get("description"))),
            ));
        }
        body.push_str("</tbody></table>");
    }
    render_page("agents", &body, 0).into_response()
}

async fn facts(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let facts = facts_data(&state, 80);
    let mut body = String::from(
        "<h1 class=\"text-2xl mb-4\">Facts <span class=\"text-sm text-zinc-500\">(latest)</span></h1>",
    );
    body.push_str(
        "<table class=\"w-full\"><thead><tr class=\"text-zinc-500 text-left\">\
         <th class=\"pr-3 py-1\">when</th><th class=\"pr-3\">key</th><th>value</th></tr></thead><tbody>",
    );
    for f in &facts {
        // Older dashboard schemas exposed created_at/fact_value; the live
        // harness schema exposes updated_at/value_json. Keep both so the
        // when/value columns show real data either way.
        body.push_str(&format!(
            "<tr><td class=\"pr-3 align-top text-xs text-zinc-500\">{}</td>\
             <td class=\"pr-3 align-top\">{}</td>\
             <td class=\"align-top text-zinc-300\">{}</td></tr>",
            html_escape(&truncate_chars(
                &first_field(f, &["created_at", "updated_at"]),
                19
            )),
            html_escape(&value_display(f.get("fact_key"))),
            html_escape(&first_field(f, &["fact_value", "value_json"])),
        ));
    }
    body.push_str("</tbody></table>");
    render_page("facts", &body, 0).into_response()
}

async fn services(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let services = rows_to_values(state.data.harness_table(&["services"]));
    let mut body = String::from("<h1 class=\"text-2xl mb-4\">Services</h1>");
    body.push_str(
        "<table class=\"w-full\"><thead><tr class=\"text-zinc-500 text-left\">\
         <th class=\"pr-3 py-1\">name</th><th class=\"pr-3\">type</th>\
         <th class=\"pr-3\">status</th><th class=\"pr-3\">last poll</th><th>target</th></tr></thead><tbody>",
    );
    for sv in &services {
        let status = value_display(sv.get("last_status"));
        let cls = if status == "healthy" {
            "text-emerald-400"
        } else {
            "text-amber-400"
        };
        body.push_str(&format!(
            "<tr><td class=\"pr-3 align-top\">{}</td>\
             <td class=\"pr-3 align-top text-xs text-zinc-500\">{}</td>\
             <td class=\"pr-3 align-top {}\">{}</td>\
             <td class=\"pr-3 align-top text-xs text-zinc-500\">{}</td>\
             <td class=\"align-top text-xs\">{}</td></tr>",
            html_escape(&value_display(sv.get("service_name"))),
            html_escape(&value_display(sv.get("service_type"))),
            cls,
            html_escape(&status),
            html_escape(&truncate_chars(
                &value_display(sv.get("last_polled_at")),
                19
            )),
            html_escape(&value_display(sv.get("check_target"))),
        ));
    }
    body.push_str("</tbody></table>");
    render_page("services", &body, 0).into_response()
}

async fn artifacts(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let mut body = String::from("<h1 class=\"text-2xl mb-4\">Artifacts</h1>");
    body.push_str(&render_artifact_upload_form(None));
    body.push_str(&render_artifact_table(&artifacts_data(&state, 200), None));
    render_page("artifacts", &body, 0).into_response()
}

async fn artifact_upload(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<TalkQuery>,
    mut multipart: Multipart,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let channel = sanitize_channel_slug(query.c.as_deref());
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut context_parts: Vec<String> = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or_default().to_string();
        match field_name.as_str() {
            "context" => {
                if let Ok(text) = field.text().await {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        context_parts.push(trimmed.to_string());
                    }
                }
            }
            "file" => {
                let original_name = field.file_name().unwrap_or("upload.bin").to_string();
                let Ok(bytes) = field.bytes().await else {
                    continue;
                };
                if !bytes.is_empty() {
                    files.push((original_name, bytes.to_vec()));
                }
            }
            _ => {}
        }
    }
    let context_text = context_parts.join("\n\n");
    let context = (!context_text.trim().is_empty()).then_some(context_text.as_str());
    let mut saved = Vec::new();
    for (original_name, bytes) in files {
        if let Ok(record) =
            save_uploaded_artifact(&state, &original_name, &bytes, channel.as_deref(), context)
        {
            saved.push(record);
        }
    }
    if let Some(channel) = channel {
        if !saved.is_empty() {
            let hashes = saved
                .iter()
                .map(|artifact| truncate_chars(&artifact.sha256, 12))
                .collect::<Vec<_>>()
                .join(", ");
            append_talk_entry(
                &state,
                &channel,
                "system",
                &format!("uploaded {} artifact(s): {hashes}", saved.len()),
                None,
            );
            send_talk_worker_prompt(state.clone(), channel.clone(), None);
        }
        Redirect::to(&format!("/talk?c={channel}")).into_response()
    } else {
        Redirect::to("/artifacts").into_response()
    }
}

async fn artifact_raw(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(sha256): Path<String>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let Some(hash) = normalize_sha256(&sha256) else {
        return (StatusCode::BAD_REQUEST, "invalid sha256").into_response();
    };
    let Some(artifact) = artifact_by_sha256(&state, &hash) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    let path = value_display(artifact.get("path"));
    let requested = PathBuf::from(&path);
    if !artifact_path_allowed(&state, &requested) {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    let Ok(bytes) = fs::read(&requested) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    let filename = requested
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", header_escape(filename)),
        )
        .body(axum::body::Body::from(bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

async fn artifact_context(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(form): Form<ArtifactContextForm>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let channel = sanitize_channel_slug(form.c.as_deref());
    let Some(hash) = normalize_sha256(&form.sha256) else {
        return (StatusCode::BAD_REQUEST, "invalid sha256").into_response();
    };
    let Some(artifact) = artifact_by_sha256(&state, &hash) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    let context = form.context.unwrap_or_default();
    if update_artifact_context(&state, &artifact, &context).is_ok() {
        if let Some(channel) = channel.as_deref() {
            append_talk_entry(
                &state,
                channel,
                "system",
                &format!("updated artifact context: {}", truncate_chars(&hash, 12)),
                None,
            );
            send_talk_worker_prompt(state.clone(), channel.to_string(), None);
        }
    }
    if let Some(channel) = channel {
        Redirect::to(&format!("/talk?c={channel}")).into_response()
    } else {
        Redirect::to("/artifacts").into_response()
    }
}

async fn memory_index(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let files = list_md_json(&state.cfg.memory_dir);
    let mut body = format!(
        "<h1 class=\"text-2xl mb-4\">Memory <span class=\"text-sm text-zinc-500\">({})</span></h1>",
        files.len()
    );
    if let Some(idx) = read_md_file(&state.cfg.memory_dir, "MEMORY.md") {
        body.push_str(&format!(
            "<details open class=\"mb-4 bg-zinc-900 border border-zinc-800 rounded p-3\">\
             <summary class=\"cursor-pointer text-zinc-100\">MEMORY.md (index)</summary>\
             <pre class=\"text-xs mt-2\">{}</pre></details>",
            html_escape(&idx)
        ));
    }
    body.push_str("<ul class=\"space-y-1\">");
    for f in &files {
        let name = value_display(f.get("name"));
        if name == "MEMORY.md" {
            continue;
        }
        let mtime = f.get("mtime_unix").and_then(Value::as_u64).unwrap_or(0);
        body.push_str(&format!(
            "<li><a class=\"text-sky-400 hover:underline\" href=\"/memory/{}\">{}</a> \
             <span class=\"text-zinc-500 text-xs\">— {} ({}B, {})</span></li>",
            html_escape(&name),
            html_escape(&name),
            html_escape(&value_display(f.get("title"))),
            value_display(f.get("size")),
            html_escape(&format_mtime(mtime)),
        ));
    }
    body.push_str("</ul>");
    render_page("memory", &body, 0).into_response()
}

async fn memory_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    match read_md_file(&state.cfg.memory_dir, &name) {
        Some(content) => {
            let mut body = format!("<h1 class=\"text-xl mb-2\">{}</h1>", html_escape(&name));
            body.push_str(&format!(
                "<div class=\"bg-zinc-900 border border-zinc-800 rounded p-3\"><pre class=\"text-xs\">{}</pre></div>",
                html_escape(&content)
            ));
            body.push_str(
                "<div class=\"mt-3\"><a class=\"text-sky-400 hover:underline\" href=\"/memory\">← all memory</a></div>",
            );
            render_page(&name, &body, 0).into_response()
        }
        None => not_found_page(),
    }
}

#[derive(Deserialize)]
struct BriefingQuery {
    show: Option<String>,
}

async fn briefings_index(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<BriefingQuery>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let show = match query.show.as_deref() {
        Some("archived") => "archived",
        Some("all") => "all",
        _ => "active",
    };
    let rows = briefings_data(&state, show);
    let mut body = format!(
        "<h1 class=\"text-2xl mb-1\">Briefings <span class=\"text-sm text-zinc-500\">({} {})</span></h1>",
        rows.len(),
        html_escape(show)
    );
    body.push_str("<div class=\"flex gap-2 mb-4\">");
    for (label, val) in [
        ("active", "active"),
        ("archived", "archived"),
        ("all", "all"),
    ] {
        let cls = if show == val {
            "bg-sky-600/20 border-sky-500 text-sky-200"
        } else {
            "border-zinc-700 text-zinc-400 hover:text-white"
        };
        body.push_str(&format!(
            "<a href=\"/briefings?show={}\" class=\"rounded border px-2 py-1 text-xs {}\">{}</a>",
            val, cls, label
        ));
    }
    body.push_str("</div>");
    if rows.is_empty() {
        body.push_str("<div class=\"text-zinc-500\">(none)</div>");
        return render_page("briefings", &body, 0).into_response();
    }
    // Group by category (uncategorized last), names sorted within each group.
    let mut groups: Vec<(String, Vec<&Value>)> = Vec::new();
    for r in &rows {
        let cat = briefing_norm(&value_display(r.get("category")));
        let cat = if cat.is_empty() {
            "(uncategorized)".to_string()
        } else {
            cat
        };
        match groups.iter_mut().find(|(c, _)| *c == cat) {
            Some((_, v)) => v.push(r),
            None => groups.push((cat, vec![r])),
        }
    }
    groups.sort_by(|(a, _), (b, _)| {
        (a == "(uncategorized)")
            .cmp(&(b == "(uncategorized)"))
            .then_with(|| a.to_lowercase().cmp(&b.to_lowercase()))
    });
    for (cat, items) in &mut groups {
        items.sort_by(|a, b| value_display(a.get("name")).cmp(&value_display(b.get("name"))));
        body.push_str(&format!(
            "<details open class=\"mb-3 bg-zinc-900 border border-zinc-800 rounded p-3\">\
             <summary class=\"cursor-pointer text-zinc-100\">{} <span class=\"text-zinc-500 text-xs\">({})</span></summary>",
            html_escape(cat),
            items.len()
        ));
        body.push_str(
            "<table class=\"w-full mt-2\"><thead><tr class=\"text-zinc-500 text-left text-xs\">\
             <th class=\"pr-3 py-1\">name</th><th class=\"pr-3\">goal</th>\
             <th class=\"pr-3\">tags</th><th>updated</th></tr></thead><tbody>",
        );
        for r in items.iter() {
            let nm = value_display(r.get("name"));
            let arch = if value_display(r.get("archived")) == "true" {
                "<span class=\"ml-1 rounded bg-amber-900/40 text-amber-300 text-[10px] px-1\">archived</span>"
            } else {
                ""
            };
            let goal = briefing_norm(&value_display(r.get("goal_key")));
            body.push_str(&format!(
                "<tr><td class=\"pr-3 align-top\"><a class=\"text-sky-400 hover:underline\" href=\"/briefings/{}\">{}</a>{}</td>\
                 <td class=\"pr-3 align-top\">{}</td>\
                 <td class=\"pr-3 align-top\">{}</td>\
                 <td class=\"align-top text-xs text-zinc-500\">{}</td></tr>",
                html_escape(&nm),
                html_escape(&nm),
                arch,
                goal_chip(&goal),
                tag_chips(&value_display(r.get("tags"))),
                html_escape(&truncate_chars(&value_display(r.get("updated_at")), 19)),
            ));
        }
        body.push_str("</tbody></table></details>");
    }
    render_page("briefings", &body, 0).into_response()
}

async fn briefing_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let base = name.strip_suffix(".md").unwrap_or(&name).to_string();
    let meta_rows = briefings_data(&state, "all");
    let meta = meta_rows
        .iter()
        .find(|r| value_display(r.get("name")) == base);
    let mut body_md = state.data.harness_stdout(&["briefing-get", &base]);
    if body_md.trim().is_empty() || body_md.starts_with("(error") {
        match read_md_file(&state.cfg.briefings_dir, &format!("{base}.md")) {
            Some(content) => body_md = content,
            None => return not_found_page(),
        }
    }
    let mut body = format!("<h1 class=\"text-xl mb-2\">{}</h1>", html_escape(&base));
    if let Some(m) = meta {
        let mut chips = String::new();
        let goal = briefing_norm(&value_display(m.get("goal_key")));
        let cat = briefing_norm(&value_display(m.get("category")));
        if !goal.is_empty() {
            chips.push_str(&format!(
                "<span class=\"rounded bg-indigo-900/40 text-indigo-300 text-xs px-1.5 py-0.5\">goal: {}</span>",
                html_escape(&goal)
            ));
        }
        if !cat.is_empty() {
            chips.push_str(&format!(
                "<span class=\"rounded bg-emerald-900/40 text-emerald-300 text-xs px-1.5 py-0.5\">{}</span>",
                html_escape(&cat)
            ));
        }
        if value_display(m.get("archived")) == "true" {
            chips.push_str(
                "<span class=\"rounded bg-amber-900/40 text-amber-300 text-xs px-1.5 py-0.5\">archived</span>",
            );
        }
        body.push_str(&format!(
            "<div class=\"flex flex-wrap gap-1 items-center mb-3\">{}{}<span class=\"text-zinc-500 text-xs ml-2\">updated {}</span></div>",
            chips,
            tag_chips(&value_display(m.get("tags"))),
            html_escape(&truncate_chars(&value_display(m.get("updated_at")), 19)),
        ));
    }
    body.push_str(&format!(
        "<div class=\"bg-zinc-900 border border-zinc-800 rounded p-3\"><pre class=\"text-xs\">{}</pre></div>",
        html_escape(&body_md)
    ));
    body.push_str(
        "<div class=\"mt-3\"><a class=\"text-sky-400 hover:underline\" href=\"/briefings\">← all briefings</a></div>",
    );
    render_page(&base, &body, 0).into_response()
}

#[derive(Deserialize)]
struct TalkQuery {
    c: Option<String>,
}

#[derive(Deserialize)]
struct TalkPostForm {
    from: Option<String>,
    text: Option<String>,
}

#[derive(Deserialize)]
struct TalkNewForm {
    name: Option<String>,
    goal: Option<String>,
    context: Option<String>,
}

#[derive(Deserialize)]
struct TalkChannelForm {
    c: Option<String>,
}

#[derive(Deserialize)]
struct SinceQuery {
    n: Option<i64>,
}

#[derive(Deserialize)]
struct ArtifactContextForm {
    sha256: String,
    context: Option<String>,
    c: Option<String>,
}

async fn talk_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<TalkQuery>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let channel = sanitize_channel_slug(query.c.as_deref())
        .unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    render_talk_page(&state, &channel)
}

async fn talk_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<TalkQuery>,
    Form(form): Form<TalkPostForm>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let channel = sanitize_channel_slug(query.c.as_deref())
        .unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    let sender = form.from.unwrap_or_default();
    let sender = sender.trim();
    let sender = if sender.is_empty() { "user" } else { sender };
    let text = form.text.unwrap_or_default().trim().to_string();
    if !text.is_empty() {
        ensure_talk_conversation(&state, &channel, None, None);
        append_talk_entry(&state, &channel, sender, &text, None);
        if sender == "user" {
            send_talk_worker_prompt(state.clone(), channel.clone(), Some(text));
        }
        return Redirect::to(&format!("/talk?c={channel}")).into_response();
    }
    render_talk_page(&state, &channel)
}

async fn talk_since(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(channel): Path<String>,
    Query(query): Query<SinceQuery>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let slug =
        sanitize_channel_slug(Some(&channel)).unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    let since_n = query.n.unwrap_or(0).max(0) as usize;
    let entries = talk_entries(&state, &slug, 100_000);
    let total = entries.len();
    let new_entries = if since_n < total {
        entries[since_n..].to_vec()
    } else {
        Vec::new()
    };
    axum::Json(json!({"channel": slug, "count": total, "messages": new_entries})).into_response()
}

async fn talk_new(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<TalkQuery>,
    Form(form): Form<TalkNewForm>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let current = sanitize_channel_slug(query.c.as_deref())
        .unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    match sanitize_channel_slug(form.name.as_deref()) {
        None => Redirect::to(&format!("/talk?c={current}")).into_response(),
        Some(channel) => {
            let goal = form
                .goal
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let context = form
                .context
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            ensure_talk_conversation(&state, &channel, goal, context);
            send_talk_worker_prompt(state.clone(), channel.clone(), None);
            Redirect::to(&format!("/talk?c={channel}")).into_response()
        }
    }
}

async fn talk_clear(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<TalkQuery>,
    Form(form): Form<TalkChannelForm>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let channel = sanitize_channel_slug(form.c.as_deref().or(query.c.as_deref()))
        .unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    mark_talk_conversation_cleared(&state, &channel);
    Redirect::to(&format!("/talk?c={channel}")).into_response()
}

async fn talk_delete(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<TalkQuery>,
    Form(form): Form<TalkChannelForm>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let channel = sanitize_channel_slug(form.c.as_deref().or(query.c.as_deref()))
        .unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    if channel == GENERAL_TALK_CHANNEL {
        return (StatusCode::BAD_REQUEST, "cannot delete the default channel").into_response();
    }
    mark_talk_conversation_archived(&state, &channel);
    Redirect::to(&format!("/talk?c={GENERAL_TALK_CHANNEL}")).into_response()
}

#[derive(Deserialize)]
struct LimitQuery {
    limit: Option<usize>,
}

async fn api_cycles(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<LimitQuery>,
) -> Response {
    api_authed_json(
        &headers,
        &state,
        || json!({"cycles": latest_episodes(&state, query.limit.unwrap_or(200))}),
    )
}

async fn api_goals(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    api_authed_json(&headers, &state, || json!({"goals": goals_data(&state)}))
}

async fn api_goal_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Response {
    if !is_authed(&headers, &state) {
        return api_unauthorized();
    }
    let goals = goals_data(&state);
    match goals
        .into_iter()
        .find(|goal| goal.get("goal_key").and_then(Value::as_str) == Some(key.as_str()))
    {
        Some(goal) => axum::Json(json!({"goal": goal})).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            axum::Json(json!({"error": "goal not found"})),
        )
            .into_response(),
    }
}

async fn api_paths(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    api_authed_json(
        &headers,
        &state,
        || json!({"paths": paths_portfolio_data(&state)}),
    )
}

async fn api_agents(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    api_authed_json(
        &headers,
        &state,
        || json!({"agents": rows_to_values(state.data.harness_table(&["agents"]))}),
    )
}

async fn api_facts(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    api_authed_json(
        &headers,
        &state,
        || json!({"facts": facts_data(&state, 80)}),
    )
}

async fn api_services(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    api_authed_json(
        &headers,
        &state,
        || json!({"services": rows_to_values(state.data.harness_table(&["services"]))}),
    )
}

async fn api_memory(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    api_authed_json(&headers, &state, || {
        json!({
            "memory": {
                "files": list_md_json(&state.cfg.memory_dir),
                "index": read_md_file(&state.cfg.memory_dir, "MEMORY.md"),
            }
        })
    })
}

async fn api_briefings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<BriefingQuery>,
) -> Response {
    api_authed_json(
        &headers,
        &state,
        || json!({"briefings": briefings_data(&state, query.show.as_deref().unwrap_or("active"))}),
    )
}

async fn api_adherence(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    api_authed_json(&headers, &state, || adherence_data(&state))
}

async fn api_progress(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    api_authed_json(&headers, &state, || progress_data(&state))
}

async fn api_goaltree(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(goal_key): Path<String>,
) -> Response {
    if !is_authed(&headers, &state) {
        return api_unauthorized();
    }
    match goal_tree_data(&state, Some(&goal_key)) {
        Ok(tree) => axum::Json(tree).into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            axum::Json(json!({"error": err.to_string()})),
        )
            .into_response(),
    }
}

async fn api_goaltree_tick(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(goal_key): Path<String>,
) -> Response {
    if !is_authed(&headers, &state) {
        return api_unauthorized();
    }
    match increment_goal_tick(&state, &goal_key) {
        Ok(()) => match goal_tree_data(&state, Some(&goal_key)) {
            Ok(tree) => axum::Json(tree).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"error": err.to_string()})),
            )
                .into_response(),
        },
        Err(err) => (
            StatusCode::BAD_REQUEST,
            axum::Json(json!({"error": err.to_string()})),
        )
            .into_response(),
    }
}

fn api_authed_json<F>(headers: &HeaderMap, state: &AppState, build: F) -> Response
where
    F: FnOnce() -> Value,
{
    if !is_authed(headers, state) {
        return api_unauthorized();
    }
    axum::Json(build()).into_response()
}

fn api_unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        axum::Json(json!({"error": "unauthorized"})),
    )
        .into_response()
}

fn rows_to_values(rows: Vec<data::Row>) -> Vec<Value> {
    rows.into_iter()
        .map(|row| {
            Value::Object(
                row.into_iter()
                    .map(|(k, v)| (k, Value::String(v)))
                    .collect(),
            )
        })
        .collect()
}

fn latest_episodes(state: &AppState, limit: usize) -> Vec<Value> {
    let limit = limit.clamp(1, 1000);
    let rows = state
        .data
        .sql_query(
            "SELECT id, created_at, summary, agent_statuses_json, actions_taken_json, goal_progress_json FROM episodes",
        )
        .map(|mut rows| {
            rows.sort_by_key(|row| value_i64(row.get("id")).unwrap_or(0));
            rows.into_iter()
                .rev()
                .take(limit)
                .map(|mut row| {
                    parse_json_field(&mut row, "agent_statuses_json");
                    parse_json_field(&mut row, "actions_taken_json");
                    parse_json_field(&mut row, "goal_progress_json");
                    Value::Object(row.into_iter().collect())
                })
                .collect()
        });
    rows.unwrap_or_else(|_| {
        let mut rows = state
            .data
            .harness_table(&["episodes", "--limit", &limit.to_string()]);
        rows.reverse();
        rows_to_values(rows)
    })
}

fn goals_data(state: &AppState) -> Vec<Value> {
    let rows = state
        .data
        .sql_query(
            "SELECT goal_key, title, detail, status, priority, depends_on_goal_key, success_fact_key, metadata_json, completion_report, created_at, updated_at, tick, scope_note FROM goals",
        )
        .map(|rows| {
            rows.into_iter()
                .map(|mut row| {
                    parse_json_field(&mut row, "metadata_json");
                    Value::Object(row.into_iter().collect())
                })
                .collect::<Vec<_>>()
        });
    let mut goals = rows.unwrap_or_else(|_| rows_to_values(state.data.harness_table(&["goals"])));
    goals.sort_by(|a, b| {
        let a_active = value_display(a.get("status")) == "active";
        let b_active = value_display(b.get("status")) == "active";
        b_active
            .cmp(&a_active)
            .then_with(|| value_i64(b.get("priority")).cmp(&value_i64(a.get("priority"))))
    });
    goals
}

fn paths_portfolio_data(state: &AppState) -> Value {
    let goal_rows = state
        .data
        .sql_query(
            "SELECT goal_key, title, detail, status, priority, success_fact_key, metadata_json, updated_at FROM goals",
        )
        .unwrap_or_default();
    let anchor_rows = state
        .data
        .sql_query(
            "SELECT goal_key, metric_name, metric_current, metric_target, updated_at FROM goal_anchor",
        )
        .unwrap_or_default();
    let path_rows = state
        .data
        .sql_query(
            "SELECT path_name, goal_key, sub_goal_key, worker, worktree, hypothesis, falsification, status, stall_counter, last_metric_move_at, predicted_delta, substrate, notes, metadata_json, created_at, updated_at FROM paths",
        )
        .unwrap_or_default();

    let anchors: std::collections::BTreeMap<String, Value> = anchor_rows
        .into_iter()
        .filter_map(|row| {
            let key = value_display(row.get("goal_key"));
            if key.is_empty() {
                None
            } else {
                Some((key, Value::Object(row.into_iter().collect())))
            }
        })
        .collect();

    let mut goals = serde_json::Map::new();
    for mut row in goal_rows {
        parse_json_field(&mut row, "metadata_json");
        let goal_key = value_display(row.get("goal_key"));
        if goal_key.is_empty() {
            continue;
        }
        let anchor = anchors.get(&goal_key);
        let metadata = row
            .get("metadata_json")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let metric_name = first_nonempty(&[
            value_display(anchor.and_then(|a| a.get("metric_name"))),
            value_display(metadata.get("metric")),
        ]);
        let last_move_at = first_nonempty(&[
            value_display(metadata.get("last_move_at")),
            value_display(anchor.and_then(|a| a.get("updated_at"))),
            value_display(row.get("updated_at")),
        ]);

        let mut goal = serde_json::Map::new();
        insert_string_field(
            &mut goal,
            "title",
            value_display_or(row.get("title"), &goal_key),
        );
        insert_value_field(&mut goal, "metric_name", Value::String(metric_name));
        insert_optional_value(
            &mut goal,
            "current",
            anchor.and_then(|a| a.get("metric_current")),
        );
        insert_optional_value(
            &mut goal,
            "target",
            anchor.and_then(|a| a.get("metric_target")),
        );
        insert_optional_value(&mut goal, "status", row.get("status"));
        insert_optional_value(&mut goal, "priority", row.get("priority"));
        insert_optional_value(&mut goal, "success_fact_key", row.get("success_fact_key"));
        insert_optional_value(&mut goal, "completion", row.get("detail"));
        insert_value_field(&mut goal, "last_move_at", Value::String(last_move_at));
        goal.insert("paths".to_string(), Value::Array(Vec::new()));
        goals.insert(goal_key, Value::Object(goal));
    }

    for mut row in path_rows {
        parse_json_field(&mut row, "metadata_json");
        let goal_key = value_display(row.get("goal_key"));
        if goal_key.is_empty() {
            continue;
        }
        let goal_entry = goals.entry(goal_key.clone()).or_insert_with(|| {
            json!({
                "title": goal_key,
                "paths": []
            })
        });
        let metadata = row
            .get("metadata_json")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        let mut path = serde_json::Map::new();
        insert_string_field(&mut path, "name", value_display(row.get("path_name")));
        for (out_key, row_key) in [
            ("worker", "worker"),
            ("worktree", "worktree"),
            ("substrate", "substrate"),
            ("hypothesis", "hypothesis"),
            ("falsification", "falsification"),
            ("status", "status"),
            ("stall_counter", "stall_counter"),
            ("last_metric_move_at", "last_metric_move_at"),
            ("predicted_delta", "predicted_delta"),
            ("notes", "notes"),
            ("sub_goal_key", "sub_goal_key"),
        ] {
            insert_optional_value(&mut path, out_key, row.get(row_key));
        }
        for (key, value) in metadata {
            if key != "source" && !path.contains_key(&key) {
                insert_value_field(&mut path, &key, value);
            }
        }

        if let Some(paths) = goal_entry.get_mut("paths").and_then(Value::as_array_mut) {
            paths.push(Value::Object(path));
        }
        let moved = value_display(row.get("last_metric_move_at"));
        if !moved.is_empty() {
            if let Some(goal) = goal_entry.as_object_mut() {
                let current = value_display(goal.get("last_move_at"));
                if moved > current {
                    goal.insert("last_move_at".to_string(), Value::String(moved));
                }
            }
        }
    }

    json!({
        "schema_version": 1,
        "source": "harness-db",
        "goals": goals,
    })
}

fn insert_optional_value(
    map: &mut serde_json::Map<String, Value>,
    key: &str,
    value: Option<&Value>,
) {
    if let Some(value) = value {
        insert_value_field(map, key, value.clone());
    }
}

fn insert_string_field(map: &mut serde_json::Map<String, Value>, key: &str, value: String) {
    insert_value_field(map, key, Value::String(value));
}

fn insert_value_field(map: &mut serde_json::Map<String, Value>, key: &str, value: Value) {
    if json_is_empty(&value) || value_display(Some(&value)) == "(none)" {
        return;
    }
    map.insert(key.to_string(), value);
}

fn facts_data(state: &AppState, limit: usize) -> Vec<Value> {
    let mut rows = state.data.harness_table(&["facts"]);
    rows.reverse();
    rows_to_values(rows.into_iter().take(limit).collect())
}

fn briefings_data(state: &AppState, show: &str) -> Vec<Value> {
    let args: &[&str] = match show {
        "archived" => &["briefing-list", "--only-archived"],
        "all" => &["briefing-list", "--archived"],
        _ => &["briefing-list"],
    };
    rows_to_values(state.data.harness_table(args))
}

fn list_md_json(dir: &FsPath) -> Vec<Value> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let text = fs::read_to_string(&path).unwrap_or_default();
        let title = text
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(|line| {
                line.trim_start_matches('#')
                    .trim()
                    .chars()
                    .take(120)
                    .collect::<String>()
            })
            .unwrap_or_else(|| name.clone());
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        out.push(json!({"name": name, "title": title, "size": meta.len(), "mtime_unix": mtime}));
    }
    out.sort_by(|a, b| {
        b.get("mtime_unix")
            .and_then(Value::as_u64)
            .cmp(&a.get("mtime_unix").and_then(Value::as_u64))
    });
    out
}

fn read_md_file(dir: &FsPath, name: &str) -> Option<String> {
    if name.contains('/') || name.contains("..") {
        return None;
    }
    fs::read_to_string(dir.join(name)).ok()
}

fn goal_tree_data(state: &AppState, goal_key: Option<&str>) -> Result<Value> {
    let mut cmd = std::process::Command::new(&state.cfg.harness_path);
    cmd
        .arg("--server")
        .arg(&state.cfg.harness_server)
        .arg("--database")
        .arg(&state.cfg.harness_database)
        .arg("goal-tree")
        .arg("--json");
    if let Some(goal_key) = goal_key {
        cmd.arg(goal_key);
    }
    let output = cmd
        .output()
        .with_context(|| "run harness goal-tree --json".to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow::anyhow!(
            "harness goal-tree failed: {}{}",
            stdout,
            stderr
        ));
    }

    serde_json::from_slice(&output.stdout)
        .with_context(|| "parse harness goal-tree JSON".to_string())
}

fn render_goal_tree(tree: &Value, source_path: &str) -> String {
    let roots = tree
        .get("roots")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let total_nodes = roots.iter().map(goal_tree_node_count).sum::<usize>();
    let done_nodes = roots.iter().map(goal_tree_done_count).sum::<usize>();
    let mut body = String::new();
    body.push_str(&format!(
        "<section class=\"mb-5 rounded border border-zinc-800 bg-zinc-900 p-4\">\
         <div class=\"mb-2 flex flex-wrap items-baseline gap-3\">\
         <h2 class=\"text-xl text-zinc-100\">Goal Forest</h2>\
         <span class=\"text-sm text-zinc-500\">{}</span>\
         <span class=\"rounded bg-sky-950 px-2 py-0.5 text-xs text-sky-200\">{}/{}</span>\
         </div>",
        html_escape(source_path),
        done_nodes,
        total_nodes
    ));
    body.push_str("<div class=\"grid gap-2 text-xs text-zinc-400 md:grid-cols-3\">");
    body.push_str(&format!(
        "<div><span class=\"text-zinc-600\">schema</span><br>{}</div>",
        html_escape(&value_display_or(tree.get("schema"), "goal_tree.v2"))
    ));
    body.push_str(&format!(
        "<div><span class=\"text-zinc-600\">roots</span><br>{}</div>",
        roots.len()
    ));
    body.push_str(&format!(
        "<div><span class=\"text-zinc-600\">source</span><br>{}</div>",
        html_escape(source_path)
    ));
    body.push_str("</div></section>");

    if roots.is_empty() {
        body.push_str("<div class=\"text-zinc-500\">(no root goals in the harness DB)</div>");
        return body;
    }

    body.push_str("<section class=\"relative ml-2 border-l border-zinc-700 pl-6\">");
    for root in roots {
        body.push_str(&render_goal_tree_node(root, 0));
    }
    body.push_str("</section>");
    body
}

fn render_goal_tree_node(node: &Value, depth: usize) -> String {
    let node_type = value_display_or(node.get("type"), "node");
    let key = value_display_or(node.get("key"), &node_type);
    let title = value_display_or(node.get("title"), &key);
    let status = value_display_or(node.get("status"), "pending");
    let worker = first_nonempty(&[
        value_display(node.get("owner_agent")),
        value_display(node.get("worker")),
    ]);
    let detail = value_display(node.get("detail"));
    let instruction = value_display(node.get("instruction_text"));
    let guidance = value_display(node.get("stuck_guidance_text"));
    let metric = value_display(node.get("metric"));
    let done_when = value_display(node.get("done_when"));
    let blocker = value_display(node.get("blocker").or_else(|| node.get("blocked_by")));
    let next_substep = value_display(node.get("next_substep"));
    let children = node
        .get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let indent = depth.saturating_mul(14).min(84);
    let kind_label = if node_type == "sub_goal" {
        "sub-goal"
    } else {
        "goal"
    };

    let mut body = String::new();
    body.push_str(&format!(
        "<article class=\"relative mb-4 rounded border border-zinc-800 bg-zinc-900 p-4\" style=\"margin-left:{}px\">\
         <span class=\"absolute -left-[31px] top-5 h-3 w-3 rounded-full border border-zinc-950 {}\"></span>\
         <div class=\"mb-2 flex flex-wrap items-center gap-2\">\
         <span class=\"text-zinc-500\">{}</span>\
         <h3 class=\"text-base text-zinc-100\">{}</h3>\
         <span class=\"badge {}\">{}</span>",
        indent,
        goal_tree_dot_class(&status),
        html_escape(&key),
        html_escape(&title),
        goal_tree_badge_class(&status),
        html_escape(&status)
    ));
    if !worker.is_empty() {
        body.push_str(&format!(
            "<span class=\"text-xs text-zinc-500\">worker {}</span>",
            html_escape(&worker)
        ));
    }
    body.push_str(&format!(
        "<span class=\"text-xs text-zinc-600\">{}</span></div>",
        html_escape(kind_label)
    ));

    body.push_str("<dl class=\"grid gap-3 text-xs md:grid-cols-2\">");
    body.push_str(&goal_tree_field("metric", &metric));
    body.push_str(&goal_tree_field("blocked_by", &blocker));
    body.push_str(&goal_tree_field("next_substep", &next_substep));
    body.push_str(&goal_tree_field("done_when", &done_when));
    body.push_str(&goal_tree_field("instruction", &instruction));
    body.push_str(&goal_tree_field("stuck_guidance", &guidance));
    body.push_str("</dl>");
    if !detail.is_empty() {
        body.push_str(&format!(
            "<div class=\"mt-3 text-xs text-zinc-400\">{}</div>",
            html_escape(&truncate_chars(&detail, 700))
        ));
    }
    body.push_str("</article>");

    for child in children {
        body.push_str(&render_goal_tree_node(child, depth + 1));
    }
    body
}

fn goal_tree_node_count(node: &Value) -> usize {
    1 + node
        .get("children")
        .and_then(Value::as_array)
        .map(|children| children.iter().map(goal_tree_node_count).sum::<usize>())
        .unwrap_or(0)
}

fn goal_tree_done_count(node: &Value) -> usize {
    let self_done = matches!(
        canonical_status(&value_display(node.get("status"))).as_str(),
        "done" | "complete" | "verified"
    ) as usize;
    self_done
        + node
            .get("children")
            .and_then(Value::as_array)
            .map(|children| children.iter().map(goal_tree_done_count).sum::<usize>())
            .unwrap_or(0)
}

fn goal_tree_field(label: &str, value: &str) -> String {
    let rendered = if value.is_empty() { "—" } else { value };
    format!(
        "<div><dt class=\"mb-1 text-zinc-600\">{}</dt><dd class=\"text-zinc-300\">{}</dd></div>",
        html_escape(label),
        html_escape(rendered)
    )
}

fn goal_tree_badge_class(status: &str) -> &'static str {
    match canonical_status(status).as_str() {
        "active" | "progressing" => "b-active",
        "pending" => "b-pending",
        "done" | "complete" | "verified" => "b-done",
        "gated-hold" | "gated" | "hold" => "b-cancelled",
        "cancelled" => "b-cancelled",
        _ => "b-pending",
    }
}

fn goal_tree_dot_class(status: &str) -> &'static str {
    match canonical_status(status).as_str() {
        "active" | "progressing" => "bg-emerald-500",
        "pending" => "bg-yellow-500",
        "done" | "complete" | "verified" => "bg-zinc-500",
        "gated-hold" | "gated" | "hold" | "cancelled" => "bg-zinc-600",
        _ => "bg-sky-500",
    }
}

fn path_status_badge_class(status: &str) -> &'static str {
    match canonical_status(status).as_str() {
        "active" | "progressing" => "b-progressing",
        "done" | "complete" | "verified" => "b-done",
        "pending" => "b-pending",
        "gated-hold" | "gated" | "hold" | "cancelled" => "b-cancelled",
        "stalled" | "blocked" => "b-stalled",
        _ => "b-pending",
    }
}

fn canonical_status(status: &str) -> String {
    status.trim().to_ascii_lowercase()
}

fn stall_counter_display(value: Option<&Value>) -> String {
    let rendered = value_display_or(value, "0");
    if rendered.trim() == "0" {
        String::new()
    } else {
        rendered
    }
}

fn find_goal_node(tree: &Value, goal_key: &str) -> Option<Value> {
    tree.get("roots")
        .and_then(Value::as_array)
        .and_then(|roots| roots.iter().find_map(|root| find_goal_node_in(root, goal_key)))
}

fn find_goal_node_in(node: &Value, goal_key: &str) -> Option<Value> {
    if value_display(node.get("type")) == "goal" && value_display(node.get("goal_key")) == goal_key {
        return Some(node.clone());
    }
    node.get("children")
        .and_then(Value::as_array)
        .and_then(|children| {
            children
                .iter()
                .find_map(|child| find_goal_node_in(child, goal_key))
        })
}

fn increment_goal_tick(state: &AppState, goal_key: &str) -> Result<()> {
    let current = goal_tree_data(state, Some(goal_key))?;
    let goal = find_goal_node(&current, goal_key).unwrap_or(Value::Null);
    let next_tick = value_i64(goal.get("tick")).unwrap_or(0) + 1;
    let title = value_display_or(goal.get("title"), goal_key);
    let status = value_display_or(goal.get("status"), "active");
    let priority = value_i64(goal.get("priority")).unwrap_or(100).max(1) as u32;
    let scope_note = value_display(goal.get("scope_note"));
    let metric = value_display(goal.get("metric"));
    let metadata = serde_json::to_string(&json!({
        "source": "dashboard-http",
        "tick": next_tick,
        "metric": metric,
        "scope_note": scope_note,
    }))?;
    run_harness_mutation(
        state,
        &[
            "goal-add",
            goal_key,
            &title,
            "--detail",
            &scope_note,
            "--status",
            &status,
            "--priority",
            &priority.to_string(),
            "--metadata",
            &metadata,
        ],
    )
}

fn run_harness_mutation(state: &AppState, args: &[&str]) -> Result<()> {
    let output = std::process::Command::new(&state.cfg.harness_path)
        .arg("--server")
        .arg(&state.cfg.harness_server)
        .arg("--database")
        .arg(&state.cfg.harness_database)
        .args(args)
        .output()
        .with_context(|| format!("run harness {}", args.join(" ")))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(anyhow::anyhow!(
        "harness {} failed: {}{}",
        args.join(" "),
        stdout,
        stderr
    ))
}

fn first_nonempty(values: &[String]) -> String {
    values
        .iter()
        .find(|value| !value.is_empty() && value.as_str() != "(none)")
        .cloned()
        .unwrap_or_default()
}

fn adherence_data(state: &AppState) -> Value {
    let goals = goals_data(state);
    let subgoals = rows_to_values(state.data.harness_table(&["sub-goals"]));
    let briefings_db = briefings_data(state, "all");
    let briefing_files = list_md_json(&state.cfg.briefings_dir);
    let paths = paths_portfolio_data(state);
    let path_goal_count = paths
        .get("goals")
        .and_then(Value::as_object)
        .map(|goals| goals.len())
        .unwrap_or(0);
    let db_goal_keys: std::collections::BTreeSet<String> = goals
        .iter()
        .filter_map(|g| {
            g.get("goal_key")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .collect();
    let unlinked_path_goals: Vec<String> = paths
        .get("goals")
        .and_then(Value::as_object)
        .map(|path_goals| {
            path_goals
                .keys()
                .filter(|key| !db_goal_keys.contains(*key))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let fresh_goal_anchors = goals
        .iter()
        .filter(|goal| {
            goal.get("updated_at")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty() && s != "(none)")
        })
        .count();
    let total_checks = goals.len().max(1) + path_goal_count.max(1) + briefing_files.len().max(1);
    let missing = unlinked_path_goals.len()
        + goals.len().saturating_sub(fresh_goal_anchors)
        + briefing_files.len().saturating_sub(briefings_db.len());
    let score =
        ((total_checks.saturating_sub(missing)) as f64 / total_checks as f64).clamp(0.0, 1.0);
    json!({
        "score": score,
        "counts": {
            "db_goals": goals.len(),
            "db_subgoals": subgoals.len(),
            "db_path_goals": path_goal_count,
            "db_briefings": briefings_db.len(),
            "briefing_files": briefing_files.len(),
            "fresh_goal_anchors": fresh_goal_anchors,
        },
        "issues": {
            "unlinked_path_goals": unlinked_path_goals,
            "briefing_files_without_db_rows": briefing_files.len().saturating_sub(briefings_db.len()),
            "goals_without_updated_at": goals.len().saturating_sub(fresh_goal_anchors),
        }
    })
}

fn progress_data(state: &AppState) -> Value {
    let facts = facts_data(state, 500);
    let interesting = [
        "albion_bot_ladder_rung",
        "autonomous_fresh_depth",
        "protocol_nativeness",
        "reproduction_proven",
    ];
    let mut metrics = serde_json::Map::new();
    for key in interesting {
        if let Some(fact) = facts
            .iter()
            .find(|f| f.get("fact_key").and_then(Value::as_str) == Some(key))
        {
            metrics.insert(
                key.to_string(),
                fact.get("fact_value").cloned().unwrap_or(Value::Null),
            );
        } else {
            metrics.insert(key.to_string(), Value::Null);
        }
    }
    let latest_cycle = latest_episodes(state, 1).into_iter().next();
    json!({
        "campaign": "dgm_self_improving_orchestrator",
        "metrics": metrics,
        "time": {
            "reported_at": Local::now().to_rfc3339(),
            "latest_cycle_created_at": latest_cycle.as_ref().and_then(|e| e.get("created_at")).cloned().unwrap_or(Value::Null),
            "rung_budget": Value::Null,
        },
        "in_game_progress": {
            "generations": [],
            "current_generation": Value::Null,
            "best_rung": metrics.get("albion_bot_ladder_rung").cloned().unwrap_or(Value::Null),
        },
        "latest_cycle": latest_cycle,
    })
}

fn value_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_u64().and_then(|u| i64::try_from(u).ok()))
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

fn value_u128(value: Option<&Value>) -> Option<u128> {
    value.and_then(|v| {
        v.as_u64()
            .map(u128::from)
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

fn value_display(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

/// Like Python `dict.get(key, default)`: returns `default` only when the value is
/// absent or JSON null, otherwise the displayed value.
fn value_display_or(value: Option<&Value>, default: &str) -> String {
    match value {
        None | Some(Value::Null) => default.to_string(),
        Some(v) => value_display(Some(v)),
    }
}

fn optional_string_value(value: Option<&str>) -> Value {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| Value::String(s.to_string()))
        .unwrap_or(Value::Null)
}

fn optional_u64_value(value: Option<u64>) -> Value {
    value.map(Value::from).unwrap_or(Value::Null)
}

fn reducer_optional_string(value: Option<&str>) -> Value {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(reducer_some_string)
        .unwrap_or_else(reducer_none)
}

fn reducer_some_string(value: &str) -> Value {
    json!({"some": value})
}

fn reducer_some_f64(value: f64) -> Value {
    json!({"some": value})
}

fn reducer_none() -> Value {
    json!({"none": []})
}

fn sanitize_filename(value: &str) -> String {
    let out: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = out.trim_matches('_').trim_matches('.');
    if trimmed.is_empty() {
        "upload.bin".to_string()
    } else {
        trimmed.chars().take(96).collect()
    }
}

fn header_escape(value: &str) -> String {
    value.replace(['"', '\r', '\n'], "_")
}

fn url_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Truncate to `n` Unicode scalar values (matches Python `s[:n]` closely enough for display).
fn truncate_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// First non-empty displayed value among `keys` (tolerates schema field renames).
fn first_field(obj: &Value, keys: &[&str]) -> String {
    for k in keys {
        let v = value_display(obj.get(k));
        if !v.is_empty() {
            return v;
        }
    }
    String::new()
}

/// Format a unix mtime to local "%Y-%m-%d %H:%M".
fn format_mtime(secs: u64) -> String {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_opt(secs as i64, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

/// Normalize a CLI-table cell: treat (none)/empty as "".
fn briefing_norm(v: &str) -> String {
    if v.is_empty() || v == "(none)" {
        String::new()
    } else {
        v.to_string()
    }
}

/// Render comma-separated tags as chips.
fn tag_chips(tags_str: &str) -> String {
    tags_str
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| {
            format!(
                "<span class=\"inline-block rounded bg-zinc-800 text-zinc-300 text-[10px] px-1.5 py-0.5 mr-1 mb-1\">#{}</span>",
                html_escape(t)
            )
        })
        .collect()
}

/// Render a goal chip, or an em-dash when absent.
fn goal_chip(goal: &str) -> String {
    if goal.is_empty() {
        "<span class=\"text-zinc-600 text-xs\">—</span>".to_string()
    } else {
        format!(
            "<span class=\"inline-block rounded bg-indigo-900/40 text-indigo-300 text-[10px] px-1.5 py-0.5\">{}</span>",
            html_escape(goal)
        )
    }
}

fn parse_json_field(row: &mut std::collections::HashMap<String, Value>, key: &str) {
    let parsed = row
        .get(key)
        .and_then(Value::as_str)
        .and_then(|s| serde_json::from_str::<Value>(s).ok());
    if let Some(value) = parsed {
        row.insert(key.to_string(), value);
    }
}

const TALK_SCRIPT: &str = r#"<script>
        (function () {
          const channel = __CHANNEL_JSON__;
          let seen = __INITIAL_COUNT__;
          const ta = document.querySelector('textarea[name="text"]');
          const list = document.getElementById('talk-messages');
          const empty = document.getElementById('talk-empty');

          // ---- draft persistence + Ctrl+Enter send (unchanged behavior) ----
          if (ta) {
            const KEY = 'talk_draft_v2:' + channel;
            const saved = sessionStorage.getItem(KEY);
            if (saved && !ta.value) ta.value = saved;
            ta.addEventListener('input', function () {
              sessionStorage.setItem(KEY, ta.value);
            });
            ta.addEventListener('keydown', function (e) {
              if (e.ctrlKey && e.key === 'Enter') {
                e.preventDefault();
                if (ta.form && typeof ta.form.requestSubmit === 'function') {
                  ta.form.requestSubmit();
                  return;
                }
                if (ta.form) ta.form.submit();
              }
            });
            if (ta.form) {
              ta.form.addEventListener('submit', function () {
                sessionStorage.removeItem(KEY);
              });
            }
          }

          // Defuse any legacy meta-refresh that might still be present.
          document.querySelectorAll('meta[http-equiv="refresh"]').forEach(function (meta) {
            meta.remove();
          });

          // ---- incremental append (no full-page reload) ----
          function badgeClass(sender) {
            if (sender === 'user') return 'bg-sky-500 text-white';
            if (sender === 'orchestrator') return 'bg-emerald-500 text-white';
            return 'bg-amber-500 text-black';
          }
          function buildMessage(entry) {
            const sender = String(entry.from || 'worker');
            const ts = String(entry.ts || '');
            const text = String(entry.text || '');
            const replyTo = String(entry.reply_to || '');
            const article = document.createElement('article');
            article.className = 'border border-zinc-800 rounded bg-zinc-950/70 p-3' + (replyTo ? ' pl-6' : '');
            const head = document.createElement('div');
            head.className = 'flex items-center gap-2 mb-2';
            const badge = document.createElement('span');
            badge.className = 'inline-block rounded px-2 py-0.5 text-xs ' + badgeClass(sender);
            badge.textContent = sender;
            const tsEl = document.createElement('span');
            tsEl.className = 'text-zinc-500 text-xs';
            tsEl.textContent = ts;
            head.appendChild(badge);
            head.appendChild(tsEl);
            if (replyTo) {
              const r = document.createElement('span');
              r.className = 'ml-2 text-zinc-600';
              r.textContent = 'reply_to ' + replyTo;
              head.appendChild(r);
            }
            const pre = document.createElement('pre');
            pre.className = 'text-sm text-zinc-200 whitespace-pre-wrap';
            pre.textContent = text;
            article.appendChild(head);
            article.appendChild(pre);
            return article;
          }
          function nearBottom() {
            const doc = document.documentElement;
            return (window.innerHeight + window.scrollY) >= (doc.scrollHeight - 80);
          }
          let polling = false;
          async function poll() {
            if (polling || !list) return;
            polling = true;
            try {
              const res = await fetch('/talk/' + encodeURIComponent(channel) + '/since?n=' + seen, {
                headers: {'Accept': 'application/json'},
                credentials: 'same-origin',
              });
              if (!res.ok) return;
              const data = await res.json();
              if (!Array.isArray(data.messages) || data.messages.length === 0) {
                if (typeof data.count === 'number') seen = data.count;
                return;
              }
              const stick = nearBottom();
              for (const entry of data.messages) {
                list.appendChild(buildMessage(entry));
              }
              if (empty) empty.classList.add('hidden');
              seen = (typeof data.count === 'number') ? data.count : (seen + data.messages.length);
              if (stick) window.scrollTo(0, document.documentElement.scrollHeight);
            } catch (e) {
              /* transient network error: keep marker, retry next tick */
            } finally {
              polling = false;
            }
          }
          window.setInterval(poll, 3000);
        })();
        </script>"#;

fn now_iso() -> String {
    Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn fact_set(
    state: &AppState,
    key: &str,
    value: &Value,
    source_type: &str,
    source_ref: Option<&str>,
    metadata: Value,
) -> Result<()> {
    state.data.call_reducer(
        "fact_set",
        vec![json!({
            "fact_key": key,
            "value_json": value.to_string(),
            "confidence": reducer_some_f64(1.0),
            "source_type": reducer_some_string(source_type),
            "source_ref": reducer_optional_string(source_ref),
            "metadata_json": reducer_some_string(&metadata.to_string()),
        })],
    )
}

fn list_talk_channels(state: &AppState) -> Vec<String> {
    ensure_talk_conversation(state, GENERAL_TALK_CHANNEL, None, None);
    let mut rows = talk_conversations_data(state);
    rows.sort_by(|a, b| {
        value_display(b.get("updated_at"))
            .cmp(&value_display(a.get("updated_at")))
            .then_with(|| value_display(a.get("slug")).cmp(&value_display(b.get("slug"))))
    });
    let mut channels = Vec::new();
    if !rows
        .iter()
        .any(|row| value_display(row.get("slug")) == GENERAL_TALK_CHANNEL)
    {
        channels.push(GENERAL_TALK_CHANNEL.to_string());
    }
    for row in rows {
        let slug = value_display(row.get("slug"));
        if !slug.is_empty()
            && value_display(row.get("status")) != "archived"
            && !channels.contains(&slug)
        {
            channels.push(slug);
        }
    }
    channels
}

fn talk_entries(state: &AppState, channel: &str, limit: usize) -> Vec<Value> {
    let slug =
        sanitize_channel_slug(Some(channel)).unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    let clear_after = talk_conversation_data(state, &slug)
        .and_then(|v| v.get("clear_after").cloned())
        .and_then(|v| value_u128(Some(&v)))
        .unwrap_or(0);
    let Ok(rows) = state
        .data
        .sql_query("SELECT fact_key, value_json, source_ref, updated_at, metadata_json FROM facts WHERE source_type = 'dashboard-talk-message'")
    else {
        return Vec::new();
    };
    let mut rows: Vec<Value> = rows
        .into_iter()
        .filter_map(|row| {
            let payload = row
                .get("value_json")
                .and_then(Value::as_str)
                .and_then(|s| serde_json::from_str::<Value>(s).ok())?;
            if payload.get("conversation_slug").and_then(Value::as_str) != Some(slug.as_str()) {
                return None;
            }
            if value_u128(payload.get("id")) <= Some(clear_after) {
                return None;
            }
            let metadata = payload
                .get("metadata_json")
                .and_then(Value::as_str)
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .unwrap_or(Value::Null);
            Some(json!({
                "id": payload.get("id").cloned().unwrap_or(Value::Null),
                "conversation_slug": payload.get("conversation_slug").cloned().unwrap_or(Value::Null),
                "from": payload.get("sender").cloned().unwrap_or(Value::Null),
                "text": payload.get("body").cloned().unwrap_or(Value::Null),
                "reply_to": payload.get("reply_to").cloned().unwrap_or(Value::Null),
                "ts": payload.get("created_at").cloned().unwrap_or_else(|| row.get("updated_at").cloned().unwrap_or(Value::Null)),
                "metadata": metadata,
            }))
        })
        .collect();
    rows.sort_by(|a, b| value_display(a.get("id")).cmp(&value_display(b.get("id"))));
    if rows.len() > limit {
        rows = rows.split_off(rows.len() - limit);
    }
    rows
}

fn append_talk_entry(
    state: &AppState,
    channel: &str,
    sender: &str,
    text: &str,
    reply_to: Option<u64>,
) {
    let slug =
        sanitize_channel_slug(Some(channel)).unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    ensure_talk_conversation(state, &slug, None, None);
    let id = now_nanos();
    let payload = json!({
        "id": id.to_string(),
        "conversation_slug": slug,
        "sender": sender,
        "body": text,
        "reply_to": optional_u64_value(reply_to),
        "created_at": now_iso(),
        "metadata_json": "{}",
    });
    let _ = fact_set(
        state,
        &format!(
            "talk.message.{}.{}",
            sanitize_channel_slug(Some(channel))
                .unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string()),
            id
        ),
        &payload,
        "dashboard-talk-message",
        Some(channel),
        json!({"source": "dashboard"}),
    );
}

fn talk_conversations_data(state: &AppState) -> Vec<Value> {
    let Ok(rows) = state.data.sql_query(
        "SELECT fact_key, value_json, source_ref, updated_at, metadata_json FROM facts WHERE source_type = 'dashboard-talk-conversation'",
    ) else {
        return Vec::new();
    };
    rows.into_iter()
        .filter_map(|row| {
            let value = row
                .get("value_json")
                .and_then(Value::as_str)
                .and_then(|s| serde_json::from_str::<Value>(s).ok())?;
            let mut obj = value.as_object()?.clone();
            if !obj.contains_key("updated_at") {
                if let Some(updated_at) = row.get("updated_at") {
                    obj.insert("updated_at".to_string(), updated_at.clone());
                }
            }
            Some(Value::Object(obj))
        })
        .collect()
}

fn talk_conversation_data(state: &AppState, channel: &str) -> Option<Value> {
    let slug =
        sanitize_channel_slug(Some(channel)).unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    talk_conversations_data(state)
        .into_iter()
        .find(|row| value_display(row.get("slug")) == slug)
}

fn ensure_talk_conversation(
    state: &AppState,
    channel: &str,
    goal: Option<&str>,
    context: Option<&str>,
) {
    let slug =
        sanitize_channel_slug(Some(channel)).unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    let title = if slug == GENERAL_TALK_CHANNEL {
        "General".to_string()
    } else {
        slug.replace('-', " ")
    };
    let args = vec![json!({
        "slug": slug.clone(),
        "title": title.clone(),
        "agent_name": talk_agent_name(channel),
        "goal_key": optional_string_value(goal),
        "context_md": optional_string_value(context),
        "status": "active",
        "metadata_json": json!({"source": "dashboard"}).to_string(),
    })];
    let mut payload = args.into_iter().next().unwrap_or_else(|| json!({}));
    if let Some(existing) = talk_conversation_data(state, &slug) {
        if payload.get("goal_key").is_none_or(Value::is_null) {
            payload["goal_key"] = existing.get("goal_key").cloned().unwrap_or(Value::Null);
        }
        if payload.get("context_md").is_none_or(Value::is_null) {
            payload["context_md"] = existing.get("context_md").cloned().unwrap_or(Value::Null);
        }
        payload["created_at"] = existing
            .get("created_at")
            .cloned()
            .unwrap_or_else(|| Value::String(now_iso()));
        payload["clear_after"] = existing.get("clear_after").cloned().unwrap_or(Value::Null);
    } else {
        payload["created_at"] = Value::String(now_iso());
        payload["clear_after"] = Value::Null;
    }
    payload["updated_at"] = Value::String(now_iso());
    let _ = fact_set(
        state,
        &format!("talk.conversation.{slug}"),
        &payload,
        "dashboard-talk-conversation",
        Some(&slug),
        json!({"source": "dashboard"}),
    );
}

fn mark_talk_conversation_cleared(state: &AppState, channel: &str) {
    ensure_talk_conversation(state, channel, None, None);
    let slug =
        sanitize_channel_slug(Some(channel)).unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    if let Some(mut payload) = talk_conversation_data(state, &slug) {
        payload["clear_after"] = Value::String(now_nanos().to_string());
        payload["updated_at"] = Value::String(now_iso());
        let _ = fact_set(
            state,
            &format!("talk.conversation.{slug}"),
            &payload,
            "dashboard-talk-conversation",
            Some(&slug),
            json!({"source": "dashboard", "action": "clear"}),
        );
    }
}

fn mark_talk_conversation_archived(state: &AppState, channel: &str) {
    ensure_talk_conversation(state, channel, None, None);
    let slug =
        sanitize_channel_slug(Some(channel)).unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    if let Some(mut payload) = talk_conversation_data(state, &slug) {
        payload["status"] = Value::String("archived".to_string());
        payload["archived_at"] = Value::String(now_iso());
        payload["updated_at"] = Value::String(now_iso());
        let _ = fact_set(
            state,
            &format!("talk.conversation.{slug}"),
            &payload,
            "dashboard-talk-conversation",
            Some(&slug),
            json!({"source": "dashboard", "action": "archive"}),
        );
    }
}

fn talk_agent_name(channel: &str) -> String {
    let slug =
        sanitize_channel_slug(Some(channel)).unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    format!("talk-{slug}")
}

fn spawn_talk_worker(state: Arc<AppState>, channel: String) {
    ensure_talk_conversation(&state, &channel, None, None);
    let agent = talk_agent_name(&channel);
    let metadata = json!({
        "kind": "codex_app_server",
        "role": "talk_worker",
        "talk_slug": channel.clone(),
        "spawned_by": "dashboard",
    })
    .to_string();
    let default_task = format!(
        "You are the dashboard talk worker for conversation #{channel}. \
Respond directly to context packets from the dashboard. Your final assistant text is captured and appended to the conversation automatically."
    );
    let workdir = state.cfg.orchestrator_dir.display().to_string();
    let _ = std::process::Command::new(&state.cfg.harness_path)
        .arg("--server")
        .arg(&state.cfg.harness_server)
        .arg("--database")
        .arg(&state.cfg.harness_database)
        .args([
            "agent-add",
            &agent,
            "--kind",
            "codex_app_server",
            "--workdir",
            &workdir,
            "--default-task",
            &default_task,
            "--metadata",
            &metadata,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

fn send_talk_worker_prompt(state: Arc<AppState>, channel: String, user_text: Option<String>) {
    spawn_talk_worker(state.clone(), channel.clone());
    std::thread::spawn(move || {
        let agent = talk_agent_name(&channel);
        let prompt = build_talk_worker_prompt(&state, &channel, user_text.as_deref());
        let output = std::process::Command::new(&state.cfg.harness_path)
            .arg("--server")
            .arg(&state.cfg.harness_server)
            .arg("--database")
            .arg(&state.cfg.harness_database)
            .args([
                "send",
                &agent,
                &prompt,
                "--wait",
                "--timeout",
                TALK_WORKER_TIMEOUT_SECONDS,
            ])
            .output();
        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let reply = stdout
                    .lines()
                    .rev()
                    .find_map(|line| serde_json::from_str::<Value>(line).ok())
                    .and_then(|v| {
                        v.get("last_assistant_text")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .unwrap_or_default();
                if !reply.trim().is_empty() {
                    append_talk_entry(&state, &channel, &agent, reply.trim(), None);
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                append_talk_entry(
                    &state,
                    &channel,
                    "system",
                    &format!(
                        "worker send failed with status {}: {}",
                        output.status,
                        truncate_chars(stderr.trim(), 400)
                    ),
                    None,
                );
            }
            Err(err) => append_talk_entry(
                &state,
                &channel,
                "system",
                &format!("worker send failed: {err}"),
                None,
            ),
        }
    });
}

fn build_talk_worker_prompt(state: &AppState, channel: &str, user_text: Option<&str>) -> String {
    let conversation = talk_conversation_data(state, channel);
    let context = conversation
        .as_ref()
        .and_then(|v| v.get("context_md"))
        .map(|v| value_display(Some(v)))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "(none)".to_string());
    let goal_key = conversation
        .as_ref()
        .and_then(|v| v.get("goal_key"))
        .map(|v| value_display(Some(v)))
        .unwrap_or_default();
    let recent_messages = talk_entries(state, channel, 30)
        .into_iter()
        .map(|entry| {
            format!(
                "[{}] {}: {}",
                value_display(entry.get("ts")),
                value_display(entry.get("from")),
                value_display(entry.get("text"))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let goals = goals_data(state)
        .into_iter()
        .filter(|g| value_display(g.get("status")) == "active")
        .take(8)
        .map(|g| {
            format!(
                "- {}: {}",
                value_display(g.get("goal_key")),
                value_display(g.get("title"))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let artifacts = artifacts_data(state, 25)
        .into_iter()
        .map(|a| {
            let metadata = a.get("metadata_json").cloned().unwrap_or(Value::Null);
            let original = metadata
                .get("original_name")
                .and_then(Value::as_str)
                .unwrap_or("");
            let context = artifact_context_text(&metadata);
            format!(
                "- sha256:{} path:{} ({} bytes) {}{}",
                value_display(a.get("sha256")),
                value_display(a.get("path")),
                value_display(a.get("size_bytes")),
                original,
                if context.is_empty() {
                    String::new()
                } else {
                    format!(" context: {}", context.replace('\n', " "))
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let latest = user_text.unwrap_or("(conversation opened or artifacts changed)");
    format!(
        "Dashboard talk context packet for #{channel}\n\n\
Conversation context:\n{context}\n\n\
Linked goal: {}\n\n\
Active goals:\n{}\n\n\
Recent messages:\n{}\n\n\
Uploaded artifacts:\n{}\n\n\
Latest user/event:\n{}\n\n\
Reply directly to the conversation. Be concise, concrete, and use the supplied context. \
Do not ask how to access files; artifact paths above are local paths in the orchestrator workspace.",
        if goal_key.is_empty() {
            "(none)"
        } else {
            &goal_key
        },
        if goals.is_empty() { "(none)" } else { &goals },
        if recent_messages.is_empty() {
            "(none)"
        } else {
            &recent_messages
        },
        if artifacts.is_empty() {
            "(none)"
        } else {
            &artifacts
        },
        latest
    )
}

fn artifacts_data(state: &AppState, limit: usize) -> Vec<Value> {
    let Ok(rows) = state.data.sql_query(
        "SELECT path, sha256, size_bytes, mtime_ns, first_seen_at, last_seen_at, significance, indexed_state, metadata_json FROM artifacts",
    ) else {
        return Vec::new();
    };
    let mut rows: Vec<Value> = rows
        .into_iter()
        .map(|mut row| {
            parse_json_field(&mut row, "metadata_json");
            Value::Object(row.into_iter().collect())
        })
        .filter(|row| value_display(row.get("indexed_state")) != "deleted")
        .collect();
    rows.sort_by(|a, b| {
        value_display(b.get("last_seen_at"))
            .cmp(&value_display(a.get("last_seen_at")))
            .then_with(|| value_display(a.get("path")).cmp(&value_display(b.get("path"))))
    });
    if rows.len() > limit {
        rows.truncate(limit);
    }
    rows
}

#[derive(Debug)]
struct SavedArtifact {
    sha256: String,
}

fn save_uploaded_artifact(
    state: &AppState,
    original_name: &str,
    bytes: &[u8],
    talk_slug: Option<&str>,
    context: Option<&str>,
) -> Result<SavedArtifact> {
    fs::create_dir_all(&state.cfg.artifact_upload_dir)
        .with_context(|| format!("create {}", state.cfg.artifact_upload_dir.display()))?;
    let safe_name = sanitize_filename(original_name);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let path = state
        .cfg
        .artifact_upload_dir
        .join(format!("{millis}_{safe_name}"));
    fs::write(&path, bytes).with_context(|| format!("write {}", path.display()))?;
    let sha = {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    };
    let metadata = fs::metadata(&path)?;
    let mtime_ns = metadata
        .modified()
        .ok()
        .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
        .map(|dur| dur.as_nanos() as i128)
        .unwrap_or(0);
    state.data.call_reducer(
        "artifact_upsert",
        vec![json!({
            "path": path.display().to_string(),
            "sha_256": reducer_some_string(&sha),
            "size_bytes": metadata.len(),
            "mtime_ns": mtime_ns,
            "significance": reducer_some_string("uploaded"),
            "indexed_state": reducer_some_string("uploaded"),
            "metadata_json": reducer_some_string(&json!({
                "original_name": original_name,
                "context": optional_string_value(context),
                "uploaded_via": "dashboard",
                "talk_slug": talk_slug,
                "stored_at": now_iso(),
            }).to_string()),
        })],
    )?;
    Ok(SavedArtifact { sha256: sha })
}

fn artifact_by_sha256(state: &AppState, sha256: &str) -> Option<Value> {
    let hash = normalize_sha256(sha256)?;
    artifacts_data(state, usize::MAX)
        .into_iter()
        .find(|artifact| value_display(artifact.get("sha256")).eq_ignore_ascii_case(&hash))
}

fn update_artifact_context(state: &AppState, artifact: &Value, context: &str) -> Result<()> {
    let path = value_display(artifact.get("path"));
    let sha256 = value_display(artifact.get("sha256"));
    let size_bytes = value_display(artifact.get("size_bytes"))
        .parse::<u64>()
        .unwrap_or(0);
    let mtime_ns = value_display(artifact.get("mtime_ns"))
        .parse::<i128>()
        .unwrap_or(0);
    let significance = value_display(artifact.get("significance"));
    let indexed_state = value_display(artifact.get("indexed_state"));
    let mut metadata = artifact
        .get("metadata_json")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !metadata.is_object() {
        metadata = json!({});
    }
    metadata["context"] = optional_string_value(Some(context));
    metadata["context_updated_at"] = Value::String(now_iso());
    state.data.call_reducer(
        "artifact_upsert",
        vec![json!({
            "path": path,
            "sha_256": reducer_optional_string(Some(&sha256)),
            "size_bytes": size_bytes,
            "mtime_ns": mtime_ns,
            "significance": reducer_optional_string(Some(&significance)),
            "indexed_state": reducer_optional_string(Some(&indexed_state)),
            "metadata_json": reducer_some_string(&metadata.to_string()),
        })],
    )?;
    Ok(())
}

fn artifact_context_text(metadata: &Value) -> String {
    value_display(metadata.get("context"))
}

fn normalize_sha256(value: &str) -> Option<String> {
    let hash = value
        .trim()
        .strip_prefix("sha256:")
        .unwrap_or_else(|| value.trim())
        .to_ascii_lowercase();
    if hash.len() == 64 && hash.as_bytes().iter().all(|byte| byte.is_ascii_hexdigit()) {
        Some(hash)
    } else {
        None
    }
}

fn artifact_path_allowed(state: &AppState, requested: &FsPath) -> bool {
    let Ok(base) = state.cfg.artifact_upload_dir.canonicalize() else {
        return false;
    };
    let Ok(path) = requested.canonicalize() else {
        return false;
    };
    path.starts_with(base)
}

fn render_artifact_upload_form(channel: Option<&str>) -> String {
    let action = channel
        .map(|c| format!("/artifacts/upload?c={}", url_encode(c)))
        .unwrap_or_else(|| "/artifacts/upload".to_string());
    format!(
        "<section class=\"mb-4 rounded border border-zinc-800 bg-zinc-900 p-4\">\
         <form method=\"post\" action=\"{}\" enctype=\"multipart/form-data\" class=\"flex flex-col gap-3\">\
         <label class=\"block\"><span class=\"mb-2 block text-xs text-zinc-500\">Artifact context</span>\
         <textarea name=\"context\" rows=\"3\" class=\"w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-sky-500 focus:outline-none\"></textarea></label>\
         <div class=\"flex flex-col gap-3 sm:flex-row sm:items-end\">\
         <label class=\"block min-w-0 flex-1\"><span class=\"mb-2 block text-xs text-zinc-500\">Upload artifact</span>\
         <input type=\"file\" name=\"file\" multiple class=\"block w-full text-sm text-zinc-300 file:mr-3 file:rounded file:border-0 file:bg-zinc-800 file:px-3 file:py-2 file:text-zinc-100 hover:file:bg-zinc-700\"></label>\
         <button type=\"submit\" class=\"rounded bg-zinc-800 px-4 py-2 text-sm text-zinc-100 hover:bg-zinc-700\">Upload</button>\
         </div></form></section>",
        html_escape(&action)
    )
}

fn render_artifact_table(artifacts: &[Value], channel: Option<&str>) -> String {
    if artifacts.is_empty() {
        return "<div class=\"text-zinc-500\">(no artifacts indexed)</div>".to_string();
    }
    let mut body = String::from(
        "<div class=\"overflow-x-auto rounded border border-zinc-800\"><table class=\"min-w-full text-left text-xs\">\
         <thead class=\"bg-zinc-900 text-zinc-400\"><tr>\
         <th class=\"px-3 py-2\">file</th><th class=\"px-3 py-2\">sha256</th><th class=\"px-3 py-2\">context</th>\
         <th class=\"px-3 py-2\">size</th><th class=\"px-3 py-2\">state</th><th class=\"px-3 py-2\">seen</th></tr></thead><tbody>",
    );
    for artifact in artifacts {
        let path = value_display(artifact.get("path"));
        let sha = value_display(artifact.get("sha256"));
        let metadata = artifact
            .get("metadata_json")
            .cloned()
            .unwrap_or(Value::Null);
        let context = artifact_context_text(&metadata);
        let original = metadata
            .get("original_name")
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                FsPath::new(&path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("artifact")
            });
        let href = normalize_sha256(&sha).map(|hash| format!("/artifacts/raw/{hash}"));
        let linked_name = href
            .as_ref()
            .map(|href| {
                format!(
                    "<a class=\"text-sky-300 hover:underline\" href=\"{}\">{}</a>",
                    html_escape(href),
                    html_escape(original)
                )
            })
            .unwrap_or_else(|| html_escape(original));
        let hash_link = href
            .as_ref()
            .map(|href| {
                format!(
                    "<a class=\"font-mono text-sky-300 hover:underline\" href=\"{}\">{}</a>",
                    html_escape(href),
                    html_escape(&truncate_chars(&sha, 16))
                )
            })
            .unwrap_or_else(|| html_escape(&truncate_chars(&sha, 16)));
        let context_form = normalize_sha256(&sha)
            .map(|hash| {
                let channel_input = channel
                    .map(|c| {
                        format!(
                            "<input type=\"hidden\" name=\"c\" value=\"{}\">",
                            html_escape(c)
                        )
                    })
                    .unwrap_or_default();
                format!(
                    "<form method=\"post\" action=\"/artifacts/context\" class=\"mt-2 flex min-w-64 flex-col gap-2\">\
                     <input type=\"hidden\" name=\"sha256\" value=\"{}\">{}\
                     <textarea name=\"context\" rows=\"2\" class=\"w-full rounded border border-zinc-700 bg-zinc-950 px-2 py-1 text-xs text-zinc-100 focus:border-sky-500 focus:outline-none\">{}</textarea>\
                     <button type=\"submit\" class=\"self-start rounded bg-zinc-800 px-2 py-1 text-xs text-zinc-100 hover:bg-zinc-700\">Save context</button>\
                     </form>",
                    html_escape(&hash),
                    channel_input,
                    html_escape(&context)
                )
            })
            .unwrap_or_default();
        body.push_str(&format!(
            "<tr class=\"border-t border-zinc-800 align-top\">\
             <td class=\"px-3 py-2\">{}<div class=\"mt-1 max-w-sm truncate text-zinc-600\">{}</div></td>\
             <td class=\"px-3 py-2\">{}</td>\
             <td class=\"px-3 py-2\"><div class=\"max-w-xl whitespace-pre-wrap text-zinc-300\">{}</div>{}</td>\
             <td class=\"px-3 py-2 text-zinc-400\">{}</td>\
             <td class=\"px-3 py-2 text-zinc-400\">{}</td>\
             <td class=\"px-3 py-2 text-zinc-500\">{}</td></tr>",
            linked_name,
            html_escape(&path),
            hash_link,
            html_escape(&context),
            context_form,
            html_escape(&value_display(artifact.get("size_bytes"))),
            html_escape(&value_display(artifact.get("indexed_state"))),
            html_escape(&truncate_chars(&value_display(artifact.get("last_seen_at")), 19)),
        ));
    }
    body.push_str("</tbody></table></div>");
    body
}

fn talk_admin_controls(channel: &str) -> String {
    let slug =
        sanitize_channel_slug(Some(channel)).unwrap_or_else(|| GENERAL_TALK_CHANNEL.to_string());
    let clear_confirm =
        serde_json::to_string(&format!("Clear all messages in #{slug}?")).unwrap_or_default();
    let mut parts = format!(
        "<form method=\"post\" action=\"/talk/clear\" style=\"display:inline\" onsubmit='return confirm({});'>\
         <input type=\"hidden\" name=\"c\" value=\"{}\">\
         <button type=\"submit\" class=\"ml-1 text-xs text-zinc-500 hover:text-amber-400\" title=\"clear\">clr</button></form>",
        clear_confirm,
        html_escape(&slug)
    );
    if slug != GENERAL_TALK_CHANNEL {
        let delete_confirm =
            serde_json::to_string(&format!("Delete channel #{slug}?")).unwrap_or_default();
        parts.push_str(&format!(
            "<form method=\"post\" action=\"/talk/delete\" style=\"display:inline\" onsubmit='return confirm({});'>\
             <input type=\"hidden\" name=\"c\" value=\"{}\">\
             <button type=\"submit\" class=\"ml-1 text-xs text-zinc-500 hover:text-red-500\" title=\"delete\">&times;</button></form>",
            delete_confirm,
            html_escape(&slug)
        ));
    }
    parts
}

fn render_talk_page(state: &AppState, channel: &str) -> Response {
    ensure_talk_conversation(state, channel, None, None);
    let channels = list_talk_channels(state);
    let conversation = talk_conversation_data(state, channel);
    let agent_name = conversation
        .as_ref()
        .and_then(|v| v.get("agent_name"))
        .map(|v| value_display(Some(v)))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| talk_agent_name(channel));
    let goal_key = conversation
        .as_ref()
        .and_then(|v| v.get("goal_key"))
        .map(|v| value_display(Some(v)))
        .unwrap_or_default();
    let context_md = conversation
        .as_ref()
        .and_then(|v| v.get("context_md"))
        .map(|v| value_display(Some(v)))
        .unwrap_or_default();
    let mut body = String::from("<h1 class=\"text-2xl mb-4\">Talk</h1>");
    body.push_str("<div class=\"flex flex-col gap-4 lg:flex-row\">");
    // sidebar
    body.push_str("<aside class=\"w-full shrink-0 lg:w-48\">");
    body.push_str("<section class=\"bg-zinc-900 border border-zinc-800 rounded p-4\">");
    body.push_str(&format!(
        "<form action=\"/talk/new?c={}\" method=\"post\" class=\"mb-4 space-y-2\">\
         <input type=\"text\" name=\"name\" maxlength=\"64\" placeholder=\"new conversation\" \
         class=\"w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2 text-zinc-100 placeholder:text-zinc-600 focus:border-sky-500 focus:outline-none\">\
         <input type=\"text\" name=\"goal\" maxlength=\"96\" placeholder=\"goal key\" \
         class=\"w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-600 focus:border-sky-500 focus:outline-none\">\
         <textarea name=\"context\" rows=\"3\" placeholder=\"context for worker\" \
         class=\"w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2 text-xs text-zinc-100 placeholder:text-zinc-600 focus:border-sky-500 focus:outline-none\"></textarea>\
         <button type=\"submit\" class=\"w-full rounded bg-zinc-800 px-3 py-2 text-sm text-zinc-100 hover:bg-zinc-700\">New worker chat</button>\
         </form>",
        html_escape(channel)
    ));
    body.push_str("<nav class=\"space-y-1\">");
    for slug in &channels {
        let link_cls = if slug == channel {
            "bg-sky-600/20 border-sky-500 text-sky-200"
        } else {
            "border-transparent text-zinc-300 hover:border-zinc-700 hover:bg-zinc-950/70 hover:text-white"
        };
        body.push_str(&format!(
            "<div class=\"flex items-center\">\
             <a href=\"/talk?c={}\" class=\"block min-w-0 flex-1 rounded border px-3 py-2 text-sm {}\">#{}</a>{}</div>",
            html_escape(slug),
            link_cls,
            html_escape(slug),
            talk_admin_controls(slug)
        ));
    }
    body.push_str("</nav></section></aside>");
    // main column
    body.push_str(
        "<section class=\"min-w-0 flex-1 bg-zinc-900 border border-zinc-800 rounded p-4\">",
    );
    body.push_str(&format!(
        "<div class=\"mb-4 flex flex-wrap items-center justify-between gap-3\"><div>\
         <div class=\"flex items-center\"><h2 class=\"text-lg text-zinc-100\">#{}</h2>{}</div>\
         <div class=\"text-xs text-zinc-500\">DB conversation. Worker: {}{}.</div></div>\
         <div class=\"text-xs text-zinc-500\">Ctrl+Enter sends. Enter inserts a newline.</div></div>",
        html_escape(channel),
        talk_admin_controls(channel),
        html_escape(&agent_name),
        if goal_key.is_empty() {
            String::new()
        } else {
            format!("; goal {}", html_escape(&goal_key))
        }
    ));
    if !context_md.is_empty() {
        body.push_str(&format!(
            "<details class=\"mb-4 rounded border border-zinc-800 bg-zinc-950/70 p-3\">\
             <summary class=\"cursor-pointer text-xs text-zinc-400\">conversation context</summary>\
             <pre class=\"mt-2 text-xs text-zinc-300\">{}</pre></details>",
            html_escape(&context_md)
        ));
    }
    let entries = talk_entries(state, channel, 200);
    let initial_count = entries.len();
    let empty_cls = if entries.is_empty() { "" } else { " hidden" };
    body.push_str(&format!(
        "<div id=\"talk-empty\" class=\"text-zinc-500 mb-4{empty_cls}\">(no messages yet)</div>"
    ));
    body.push_str("<div id=\"talk-messages\" class=\"space-y-3 mb-5\">");
    for entry in &entries {
        let sender = value_display(entry.get("from"));
        let sender = if sender.is_empty() { "worker" } else { &sender };
        let ts = value_display(entry.get("ts"));
        let text = value_display(entry.get("text"));
        let reply_to = value_display(entry.get("reply_to"));
        let badge = if sender == "user" {
            "bg-sky-500 text-white"
        } else if sender == "orchestrator" {
            "bg-emerald-500 text-white"
        } else {
            "bg-amber-500 text-black"
        };
        let indent = if reply_to.is_empty() { "" } else { " pl-6" };
        let reply_meta = if reply_to.is_empty() {
            String::new()
        } else {
            format!(
                "<span class=\"ml-2 text-zinc-600\">reply_to {}</span>",
                html_escape(&reply_to)
            )
        };
        body.push_str(&format!(
            "<article class=\"border border-zinc-800 rounded bg-zinc-950/70 p-3{}\">\
             <div class=\"flex items-center gap-2 mb-2\">\
             <span class=\"inline-block rounded px-2 py-0.5 text-xs {}\">{}</span>\
             <span class=\"text-zinc-500 text-xs\">{}</span>{}</div>\
             <pre class=\"text-sm text-zinc-200 whitespace-pre-wrap\">{}</pre></article>",
            indent,
            badge,
            html_escape(sender),
            html_escape(&ts),
            reply_meta,
            html_escape(&text)
        ));
    }
    body.push_str("</div>");
    body.push_str(&format!(
        "<form method=\"post\" action=\"/talk?c={}\" class=\"border-t border-zinc-800 pt-4\">\
         <input type=\"hidden\" name=\"from\" value=\"user\">\
         <label for=\"talk-text\" class=\"block text-xs text-zinc-500 mb-2\">Post a message</label>\
         <textarea id=\"talk-text\" name=\"text\" rows=\"3\" class=\"w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2 text-zinc-100 placeholder:text-zinc-600 focus:border-sky-500 focus:outline-none\" placeholder=\"Ask the orchestrator something...\"></textarea>\
         <div class=\"mt-3 flex items-center justify-between gap-3\">\
         <div class=\"text-xs text-zinc-500\">Drafts are stored per channel in sessionStorage.</div>\
         <button type=\"submit\" class=\"rounded bg-sky-600 px-4 py-2 text-sm text-white hover:bg-sky-500\">Send</button>\
         </div></form>",
        html_escape(channel)
    ));
    body.push_str("<div class=\"mt-5 border-t border-zinc-800 pt-4\">");
    body.push_str(&render_artifact_upload_form(Some(channel)));
    body.push_str("<h3 class=\"mb-2 text-sm text-zinc-300\">Recent artifacts</h3>");
    body.push_str(&render_artifact_table(
        &artifacts_data(state, 25),
        Some(channel),
    ));
    body.push_str("</div>");
    body.push_str("</section></div>");
    let script = TALK_SCRIPT
        .replace(
            "__CHANNEL_JSON__",
            &serde_json::to_string(channel).unwrap_or_else(|_| "\"general\"".to_string()),
        )
        .replace("__INITIAL_COUNT__", &initial_count.to_string());
    body.push_str(&script);
    render_page("talk", &body, 0).into_response()
}

fn not_found_page() -> Response {
    (
        StatusCode::NOT_FOUND,
        render_page(
            "404",
            "<h1 class=\"text-2xl mb-2\">404</h1><p class=\"text-zinc-500\">Not found.</p>",
            0,
        ),
    )
        .into_response()
}

fn render_login(error: Option<&str>) -> Html<String> {
    let mut body = String::from("<section class=\"mx-auto max-w-md pt-16\">");
    body.push_str(
        "<div class=\"rounded border border-zinc-800 bg-zinc-900 p-6 shadow-2xl shadow-black/30\">",
    );
    body.push_str("<h1 class=\"mb-2 text-2xl text-zinc-100\">Dashboard Login</h1>");
    body.push_str("<p class=\"mb-5 text-sm text-zinc-500\">Enter the dashboard password to access the talk panel and operator views.</p>");
    if let Some(error) = error {
        body.push_str(&format!(
            "<div class=\"mb-4 rounded border border-red-900/60 bg-red-950/60 px-3 py-2 text-sm text-red-200\">{}</div>",
            html_escape(error)
        ));
    }
    body.push_str(
        "<form method=\"post\" action=\"/login\" class=\"space-y-4\">\
         <div><label for=\"password\" class=\"mb-2 block text-xs uppercase tracking-[0.2em] text-zinc-500\">Password</label>\
         <input id=\"password\" type=\"password\" name=\"password\" autocomplete=\"current-password\" class=\"w-full rounded border border-zinc-700 bg-zinc-950 px-3 py-2 text-zinc-100 placeholder:text-zinc-600 focus:border-sky-500 focus:outline-none\"></div>\
         <button type=\"submit\" class=\"w-full rounded bg-sky-600 px-4 py-2 text-sm text-white hover:bg-sky-500\">Enter dashboard</button>\
         </form></div></section>",
    );
    render_page("login", &body, 0)
}

fn render_page(title: &str, body: &str, refresh: u64) -> Html<String> {
    let refresh_tag = if refresh > 0 {
        format!("<meta http-equiv=\"refresh\" content=\"{refresh}\">")
    } else {
        String::new()
    };
    Html(format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{} - orchestrator</title><script src=\"https://cdn.tailwindcss.com\"></script>{}\
         <style>.mono{{font-family:ui-monospace,Menlo,Consolas,monospace}}\
         .ellip{{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:100%}}\
         pre{{white-space:pre-wrap;word-break:break-word}}\
         table tr:hover{{background:rgba(255,255,255,.04)}}\
         .badge{{display:inline-block;padding:.1rem .4rem;border-radius:.2rem;font-size:.7rem;line-height:1rem}}\
         .b-active{{background:#16a34a;color:#fff}}\
         .b-pending{{background:#eab308;color:#000}}\
         .b-cancelled,.b-done{{background:#525252;color:#fff}}\
         .b-progressing{{background:#0ea5e9;color:#fff}}\
         .b-stalled{{background:#dc2626;color:#fff}}</style>\
         </head><body class=\"bg-zinc-950 text-zinc-200 mono text-sm\">\
         <nav class=\"bg-zinc-900 border-b border-zinc-800 px-4 py-2 flex gap-4 items-center\">\
         <a href=\"/\" class=\"font-bold text-zinc-100\">orchestrator</a>\
         <a href=\"/\" class=\"hover:text-white\">dashboard</a><a href=\"/cycles\" class=\"hover:text-white\">cycles</a>\
         <a href=\"/goals\" class=\"hover:text-white\">goals</a><a href=\"/goaltree\" class=\"hover:text-white\">goal tree</a><a href=\"/paths\" class=\"hover:text-white\">paths</a>\
         <a href=\"/agents\" class=\"hover:text-white\">agents</a><a href=\"/facts\" class=\"hover:text-white\">facts</a>\
         <a href=\"/services\" class=\"hover:text-white\">services</a><a href=\"/memory\" class=\"hover:text-white\">memory</a>\
         <a href=\"/briefings\" class=\"hover:text-white\">briefings</a><a href=\"/artifacts\" class=\"hover:text-white\">artifacts</a><a href=\"/talk\" class=\"hover:text-white\">talk</a>\
         <a href=\"/logout\" class=\"hover:text-white\">logout</a>\
         <span class=\"ml-auto text-zinc-500 text-xs\">{}</span></nav>\
         <main class=\"p-4 max-w-7xl mx-auto\">{}</main></body></html>",
        html_escape(title),
        refresh_tag,
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        body
    ))
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{PASSWORD_SALT}{password}"));
    hex::encode(hasher.finalize())
}

fn load_or_create_secret_key(path: &std::path::Path) -> Result<String> {
    if path.exists() {
        let key = fs::read_to_string(path)
            .with_context(|| format!("read dashboard secret {}", path.display()))?
            .trim()
            .to_string();
        if key.is_empty() {
            anyhow::bail!("{} is empty", path.display());
        }
        return Ok(key);
    }
    let key = hex::encode(Sha256::digest(format!(
        "{:?}",
        std::time::SystemTime::now()
    )));
    fs::write(path, &key).with_context(|| format!("write dashboard secret {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("chmod dashboard secret {}", path.display()))?;
    }
    Ok(key)
}

fn signed_auth_value(state: &AppState) -> String {
    let mut hasher = Sha256::new();
    hasher.update(&state.secret_key);
    hasher.update(&state.password_hash);
    format!("{}.{}", state.password_hash, hex::encode(hasher.finalize()))
}

fn is_authed(headers: &HeaderMap, state: &AppState) -> bool {
    let Some(cookie_header) = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    cookie_header
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .any(|(name, value)| name == AUTH_COOKIE_NAME && value == signed_auth_value(state))
}

fn sanitize_channel_slug(name: Option<&str>) -> Option<String> {
    let raw = name?.trim().to_ascii_lowercase();
    if raw.is_empty() {
        return None;
    }
    let ascii: String = raw.chars().filter(char::is_ascii).collect();
    let re_non_alnum = Regex::new(r"[^a-z0-9]+").ok()?;
    let re_dash = Regex::new(r"-{2,}").ok()?;
    let slug = re_dash
        .replace_all(&re_non_alnum.replace_all(&ascii, "-"), "-")
        .trim_matches('-')
        .chars()
        .take(32)
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if slug.is_empty() || !Regex::new(r"^[a-z0-9-]{1,32}$").ok()?.is_match(&slug) {
        return None;
    }
    Some(slug)
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::parse_table;

    #[test]
    fn parses_pipe_table() {
        let rows = parse_table("a | b\n--+--\n 1 | two \n(1 row)\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("a").unwrap(), "1");
        assert_eq!(rows[0].get("b").unwrap(), "two");
    }

    #[test]
    fn sanitizes_channels() {
        assert_eq!(sanitize_channel_slug(Some(" A b!!c ")).unwrap(), "a-b-c");
        assert!(sanitize_channel_slug(Some("!!!")).is_none());
    }
}
