#![allow(dead_code)]

use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;
use ureq::Agent;

use crate::config::DashboardConfig;

pub type Row = HashMap<String, String>;

#[derive(Clone)]
pub struct DataClient {
    cfg: DashboardConfig,
    http: Agent,
}

impl DataClient {
    pub fn new(cfg: DashboardConfig) -> Result<Self> {
        // ureq = pure-sync HTTP (no internal tokio runtime) -> safe to build + call from inside
        // the axum async runtime (reqwest::blocking panics there: "Cannot drop a runtime...").
        let http = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(10))
            .build();
        Ok(Self { cfg, http })
    }

    pub fn harness_table(&self, args: &[&str]) -> Vec<Row> {
        parse_table(&self.harness_stdout(args))
    }

    pub fn harness_stdout(&self, args: &[&str]) -> String {
        match Command::new(&self.cfg.harness_path)
            .arg("--server")
            .arg(&self.cfg.harness_server)
            .arg("--database")
            .arg(&self.cfg.harness_database)
            .args(args)
            .output()
        {
            Ok(output) => String::from_utf8_lossy(&output.stdout).into_owned(),
            Err(err) => format!("(error: {err})"),
        }
    }

    pub fn call_reducer(&self, reducer: &str, args: Vec<Value>) -> Result<()> {
        let url = format!(
            "{}/v1/database/{}/call/{}",
            self.cfg.harness_server, self.cfg.harness_database, reducer
        );
        self.http
            .post(&url)
            .set("content-type", "application/json")
            .send_string(&Value::Array(args).to_string())
            .with_context(|| format!("call reducer {reducer}"))?;
        Ok(())
    }

    pub fn sql_query(&self, query: &str) -> Result<Vec<HashMap<String, Value>>> {
        // ureq returns Err on non-2xx already (no error_for_status needed).
        let payload: Value = self
            .http
            .post(&self.cfg.sql_url)
            .set("content-type", "text/plain")
            .send_string(query)
            .context("send SQL request")?
            .into_json()
            .context("decode SQL JSON")?;
        Ok(parse_sql_payload(payload))
    }
}

pub fn parse_table(text: &str) -> Vec<Row> {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let header_line = lines[0];
    let header_trimmed = header_line.trim();
    if header_line.contains('+') || header_trimmed.chars().all(|ch| ch == '-' || ch == '+') {
        return Vec::new();
    }
    let cols: Vec<String> = header_line
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect();
    let mut rows = Vec::new();
    for line in lines.into_iter().skip(1) {
        let trimmed = line.trim();
        if trimmed.chars().all(|ch| ch == '-' || ch == '+') {
            continue;
        }
        if is_rows_tail(trimmed) {
            continue;
        }
        let parts: Vec<String> = line
            .split('|')
            .map(|cell| cell.trim().to_string())
            .collect();
        if parts.len() != cols.len() {
            continue;
        }
        rows.push(cols.iter().cloned().zip(parts.into_iter()).collect());
    }
    rows
}

fn is_rows_tail(value: &str) -> bool {
    if !value.starts_with('(') || !value.ends_with(')') {
        return false;
    }
    let inner = &value[1..value.len() - 1];
    inner
        .strip_suffix(" rows")
        .or_else(|| inner.strip_suffix(" row"))
        .is_some_and(|n| n.chars().all(|ch| ch.is_ascii_digit()))
}

fn parse_sql_payload(payload: Value) -> Vec<HashMap<String, Value>> {
    let Some(first) = payload.as_array().and_then(|items| items.first()) else {
        return Vec::new();
    };
    let cols: Vec<String> = first
        .pointer("/schema/elements")
        .and_then(Value::as_array)
        .map(|elements| {
            elements
                .iter()
                .filter_map(|element| {
                    element
                        .get("name")
                        .and_then(|name| name.get("some").or(Some(name)))
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .collect()
        })
        .unwrap_or_default();
    let Some(rows) = first.get("rows").and_then(Value::as_array) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(Value::as_array)
        .map(|row| {
            cols.iter()
                .cloned()
                .zip(row.iter().cloned().map(unwrap_sql_cell))
                .collect()
        })
        .collect()
}

fn unwrap_sql_cell(value: Value) -> Value {
    // SpacetimeDB encodes Option<T> cells as [0, value] / [1, []].
    match value {
        Value::Array(items) if items.len() == 2 => {
            let mut items = items.into_iter();
            let tag = items.next().unwrap_or(Value::Null);
            let payload = items.next().unwrap_or(Value::Null);
            match tag.as_i64() {
                Some(0) => unwrap_sql_cell(payload),
                Some(1) => Value::Null,
                _ => Value::Array(vec![tag, payload]),
            }
        }
        other => other,
    }
}
