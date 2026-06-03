#![allow(dead_code)]

use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde_json::Value;

use crate::config::DashboardConfig;

pub type Row = HashMap<String, String>;

#[derive(Clone)]
pub struct DataClient {
    cfg: DashboardConfig,
    http: Client,
}

impl DataClient {
    pub fn new(cfg: DashboardConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("build HTTP client")?;
        Ok(Self { cfg, http })
    }

    pub fn harness_table(&self, args: &[&str]) -> Vec<Row> {
        parse_table(&self.harness_stdout(args))
    }

    pub fn harness_stdout(&self, args: &[&str]) -> String {
        match Command::new(&self.cfg.harness_path).args(args).output() {
            Ok(output) => String::from_utf8_lossy(&output.stdout).into_owned(),
            Err(err) => format!("(error: {err})"),
        }
    }

    pub fn sql_query(&self, query: &str) -> Result<Vec<HashMap<String, Value>>> {
        let payload: Value = self
            .http
            .post(&self.cfg.sql_url)
            .header("content-type", "text/plain")
            .body(query.to_string())
            .send()
            .context("send SQL request")?
            .error_for_status()
            .context("SQL HTTP status")?
            .json()
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
        .map(|row| cols.iter().cloned().zip(row.iter().cloned()).collect())
        .collect()
}
