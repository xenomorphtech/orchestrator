mod config;
mod data;

use std::fs;
use std::net::SocketAddr;
use std::path::Path as FsPath;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::{Path, Query, State};
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
        .route("/goal/{key}", get(goal_detail))
        .route("/paths", get(paths))
        .route("/agents", get(agents))
        .route("/facts", get(facts))
        .route("/services", get(services))
        .route("/memory", get(memory_index))
        .route("/memory/{name}", get(memory_detail))
        .route("/briefings", get(briefings_index))
        .route("/briefings/{name}", get(briefing_detail))
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
    let paths = read_json_file(&state.cfg.analysis_dir.join("paths.json"));
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
            body.push_str(&format!(
                "<tr class=\"border-t border-zinc-800 align-top\">\
                 <td class=\"px-3 py-2 text-zinc-500\">{}</td>\
                 <td class=\"px-3 py-2 text-zinc-400 whitespace-nowrap\">{}</td>\
                 <td class=\"px-3 py-2 text-zinc-100\">{}</td>\
                 <td class=\"px-3 py-2 text-zinc-300\">{}</td>\
                 <td class=\"px-3 py-2 text-zinc-300\">{}</td></tr>",
                html_escape(&value_display(episode.get("id"))),
                html_escape(&value_display(episode.get("created_at"))),
                html_escape(&value_display(episode.get("summary"))),
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
    authed_or_placeholder(&headers, &state, &format!("cycle {cid}"), "/cycle/<cid>")
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

async fn goal_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let goals = goals_data(&state);
    let Some(g) = goals.iter().find(|g| value_display(g.get("goal_key")) == key) else {
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
    let p = read_json_file(&state.cfg.analysis_dir.join("paths.json"));
    let mut body = String::from("<h1 class=\"text-2xl mb-4\">Path portfolio</h1>");
    let empty = p.is_null() || p.as_object().is_some_and(|o| o.is_empty());
    if empty {
        body.push_str(
            "<div class=\"text-zinc-500\">(empty / unreadable analysis/paths.json)</div>",
        );
    } else if let Some(goals) = p.get("goals").and_then(Value::as_object) {
        for (gk, g) in goals {
            body.push_str(
                "<section class=\"mb-5 bg-zinc-900 border border-zinc-800 rounded p-3\">",
            );
            body.push_str(&format!("<h2 class=\"text-lg mb-1\">{}</h2>", html_escape(gk)));
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
                        let cls = if status == "progressing" {
                            "b-progressing"
                        } else {
                            "b-stalled"
                        };
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
                            html_escape(&value_display_or(pt.get("stall_counter"), "0")),
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
        body.push_str(&format!(
            "<tr><td class=\"pr-3 align-top text-xs text-zinc-500\">{}</td>\
             <td class=\"pr-3 align-top\">{}</td>\
             <td class=\"align-top text-zinc-300\">{}</td></tr>",
            html_escape(&truncate_chars(&value_display(f.get("created_at")), 19)),
            html_escape(&value_display(f.get("fact_key"))),
            html_escape(&value_display(f.get("fact_value"))),
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
            html_escape(&truncate_chars(&value_display(sv.get("last_polled_at")), 19)),
            html_escape(&value_display(sv.get("check_target"))),
        ));
    }
    body.push_str("</tbody></table>");
    render_page("services", &body, 0).into_response()
}

async fn memory_index(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    authed_or_placeholder(&headers, &state, "memory", "/memory")
}

async fn memory_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    authed_or_placeholder(&headers, &state, &name, "/memory/<name>")
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
    let show = query.show.unwrap_or_else(|| "active".to_string());
    authed_or_placeholder(&headers, &state, &format!("briefings {show}"), "/briefings")
}

async fn briefing_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    authed_or_placeholder(&headers, &state, &name, "/briefings/<name>")
}

#[derive(Deserialize)]
struct TalkQuery {
    c: Option<String>,
}

async fn talk_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<TalkQuery>,
) -> Response {
    let channel =
        sanitize_channel_slug(query.c.as_deref()).unwrap_or_else(|| "general".to_string());
    authed_or_placeholder(&headers, &state, &format!("talk #{channel}"), "/talk")
}

async fn talk_post(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    authed_or_placeholder(&headers, &state, "talk post", "POST /talk")
}

async fn talk_since(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(channel): Path<String>,
) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let slug = sanitize_channel_slug(Some(&channel)).unwrap_or_else(|| "general".to_string());
    axum::Json(serde_json::json!({"channel": slug, "count": 0, "messages": []})).into_response()
}

async fn talk_new(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    authed_or_placeholder(&headers, &state, "talk new", "POST /talk/new")
}

async fn talk_clear(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    authed_or_placeholder(&headers, &state, "talk clear", "POST /talk/clear")
}

async fn talk_delete(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    authed_or_placeholder(&headers, &state, "talk delete", "POST /talk/delete")
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
        || json!({"paths": read_json_file(&state.cfg.analysis_dir.join("paths.json"))}),
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
    let mut rows = state.data.harness_table(&["goals"]);
    rows.sort_by(|a, b| {
        let a_active = a.get("status").is_some_and(|s| s == "active");
        let b_active = b.get("status").is_some_and(|s| s == "active");
        b_active
            .cmp(&a_active)
            .then_with(|| parse_i64(b.get("priority")).cmp(&parse_i64(a.get("priority"))))
    });
    rows_to_values(rows)
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

fn read_json_file(path: &FsPath) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null)
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

fn adherence_data(state: &AppState) -> Value {
    let goals = goals_data(state);
    let subgoals = rows_to_values(state.data.harness_table(&["sub-goals"]));
    let briefings_db = briefings_data(state, "all");
    let briefing_files = list_md_json(&state.cfg.briefings_dir);
    let paths = read_json_file(&state.cfg.analysis_dir.join("paths.json"));
    let file_goal_count = paths
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
    let total_checks = goals.len().max(1) + file_goal_count.max(1) + briefing_files.len().max(1);
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
            "path_file_goals": file_goal_count,
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

fn parse_i64(value: Option<&String>) -> i64 {
    value.and_then(|v| v.parse().ok()).unwrap_or(0)
}

fn value_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_u64().and_then(|u| i64::try_from(u).ok()))
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

/// Truncate to `n` Unicode scalar values (matches Python `s[:n]` closely enough for display).
fn truncate_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
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

fn authed_or_placeholder(
    headers: &HeaderMap,
    state: &AppState,
    title: &str,
    route: &str,
) -> Response {
    if !is_authed(headers, state) {
        return Redirect::to("/login").into_response();
    }
    let body = format!(
        "<h1 class=\"text-2xl mb-4\">{}</h1><p class=\"text-zinc-500\">Rust dashboard scaffold route: <code>{}</code>.</p>",
        html_escape(title),
        html_escape(route)
    );
    render_page(title, &body, 0).into_response()
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
         <a href=\"/goals\" class=\"hover:text-white\">goals</a><a href=\"/paths\" class=\"hover:text-white\">paths</a>\
         <a href=\"/agents\" class=\"hover:text-white\">agents</a><a href=\"/facts\" class=\"hover:text-white\">facts</a>\
         <a href=\"/services\" class=\"hover:text-white\">services</a><a href=\"/memory\" class=\"hover:text-white\">memory</a>\
         <a href=\"/briefings\" class=\"hover:text-white\">briefings</a><a href=\"/talk\" class=\"hover:text-white\">talk</a>\
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
    fn parses_pipe_table_like_flask() {
        let rows = parse_table("a | b\n--+--\n 1 | two \n(1 row)\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("a").unwrap(), "1");
        assert_eq!(rows[0].get("b").unwrap(), "two");
    }

    #[test]
    fn sanitizes_channels_like_flask() {
        assert_eq!(sanitize_channel_slug(Some(" A b!!c ")).unwrap(), "a-b-c");
        assert!(sanitize_channel_slug(Some("!!!")).is_none());
    }
}
