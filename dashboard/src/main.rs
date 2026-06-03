mod config;
mod data;

use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use chrono::Local;
use config::DashboardConfig;
use data::DataClient;
use regex::Regex;
use serde::Deserialize;
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
    authed_or_placeholder(&headers, &state, "dashboard", "/")
}

async fn cycles(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    authed_or_placeholder(&headers, &state, "cycles", "/cycles")
}

async fn cycle_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(cid): Path<i64>,
) -> Response {
    authed_or_placeholder(&headers, &state, &format!("cycle {cid}"), "/cycle/<cid>")
}

async fn goals(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    authed_or_placeholder(&headers, &state, "goals", "/goals")
}

async fn goal_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Response {
    authed_or_placeholder(&headers, &state, &format!("goal {key}"), "/goal/<key>")
}

async fn paths(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    authed_or_placeholder(&headers, &state, "paths", "/paths")
}

async fn agents(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !is_authed(&headers, &state) {
        return Redirect::to("/login").into_response();
    }
    let rows = state.data.harness_table(&["agents"]);
    render_page("agents", &format!("<pre>{rows:#?}</pre>"), 0).into_response()
}

async fn facts(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    authed_or_placeholder(&headers, &state, "facts", "/facts")
}

async fn services(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    authed_or_placeholder(&headers, &state, "services", "/services")
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
         <style>.mono{{font-family:ui-monospace,Menlo,Consolas,monospace}} pre{{white-space:pre-wrap;word-break:break-word}}</style>\
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
