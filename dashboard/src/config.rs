#![allow(dead_code)]

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

const DEFAULT_SERVER: &str = "http://127.0.0.1:3000";
const DEFAULT_DATABASE: &str = "orchestrator-harness";
const DEFAULT_BIOME_URL: &str = "http://127.0.0.1:3021";
const DEFAULT_PASSWORD: &str = "welcome to my w0rld";

#[derive(Debug, Clone)]
pub struct DashboardConfig {
    pub bind_addr: String,
    pub orchestrator_dir: PathBuf,
    pub harness_path: PathBuf,
    pub harness_server: String,
    pub harness_database: String,
    pub sql_url: String,
    pub biome_url: String,
    pub biome_api_key: String,
    pub dashboard_password: String,
    pub secret_key_path: PathBuf,
    pub analysis_dir: PathBuf,
    pub briefings_dir: PathBuf,
    pub memory_dir: PathBuf,
    pub talk_log: PathBuf,
    pub talk_channels_dir: PathBuf,
    pub artifact_upload_dir: PathBuf,
}

#[derive(Debug, Default, Deserialize)]
struct HarnessToml {
    server: Option<String>,
    database: Option<String>,
    biome_url: Option<String>,
    biome_api_key: Option<String>,
}

impl DashboardConfig {
    pub fn load() -> Result<Self> {
        let home_dir = dirs_home();
        let orchestrator_dir = home_dir.join("orchestrator");
        let _ = dotenvy::from_path(orchestrator_dir.join(".env"));
        let harness_toml = read_harness_toml(&orchestrator_dir);
        let env_map: HashMap<String, String> = env::vars().collect();

        let harness_server = env_map
            .get("HARNESS_SERVER")
            .cloned()
            .or_else(|| harness_toml.server.clone())
            .unwrap_or_else(|| DEFAULT_SERVER.to_string());
        let harness_database = env_map
            .get("HARNESS_DATABASE")
            .cloned()
            .or_else(|| harness_toml.database.clone())
            .unwrap_or_else(|| DEFAULT_DATABASE.to_string());
        let biome_url = env_map
            .get("HARNESS_BIOME_URL")
            .cloned()
            .or_else(|| harness_toml.biome_url.clone())
            .unwrap_or_else(|| DEFAULT_BIOME_URL.to_string());
        let biome_api_key = env_map
            .get("HARNESS_BIOME_API_KEY")
            .cloned()
            .or_else(|| harness_toml.biome_api_key.clone())
            .unwrap_or_default();
        let sql_url = format!("{harness_server}/v1/database/{harness_database}/sql");
        let analysis_dir = orchestrator_dir.join("analysis");

        Ok(Self {
            bind_addr: env_map
                .get("DASHBOARD_BIND_ADDR")
                .cloned()
                .unwrap_or_else(|| "0.0.0.0:3030".to_string()),
            orchestrator_dir: orchestrator_dir.clone(),
            harness_path: env_map
                .get("HARNESS")
                .map(PathBuf::from)
                .unwrap_or_else(|| orchestrator_dir.join("harness")),
            harness_server,
            harness_database,
            sql_url,
            biome_url,
            biome_api_key,
            dashboard_password: env_map
                .get("DASHBOARD_PASSWORD")
                .cloned()
                .unwrap_or_else(|| DEFAULT_PASSWORD.to_string()),
            secret_key_path: orchestrator_dir.join(".dash_secret_key"),
            briefings_dir: orchestrator_dir.join("briefings"),
            memory_dir: PathBuf::from(
                "/home/sdancer/.claude/projects/-home-sdancer-orchestrator/memory",
            ),
            talk_log: analysis_dir.join("talk.jsonl"),
            talk_channels_dir: analysis_dir.join("talk_channels"),
            artifact_upload_dir: analysis_dir.join("dashboard_uploads"),
            analysis_dir,
        })
    }
}

fn read_harness_toml(orchestrator_dir: &Path) -> HarnessToml {
    let candidates = [
        orchestrator_dir.join("harness.toml"),
        dirs_home().join(".config/harness/harness.toml"),
        PathBuf::from("harness.toml"),
    ];
    for path in candidates {
        match read_toml_file(&path) {
            Ok(Some(cfg)) => return cfg,
            Ok(None) => {}
            Err(_) => {}
        }
    }
    HarnessToml::default()
}

fn read_toml_file(path: &Path) -> Result<Option<HarnessToml>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let parsed = toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(Some(parsed))
}

fn dirs_home() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/sdancer"))
}
