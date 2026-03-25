#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sqlite3
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

PROJECT_DIR = "/home/sdancer/games/nmss"
MAX_HASH_BYTES = 2 * 1024 * 1024

SCHEMA_SQL = """
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS artifacts (
    path TEXT PRIMARY KEY,
    sha256 TEXT,
    size_bytes INTEGER NOT NULL,
    mtime_ns INTEGER NOT NULL,
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    significance TEXT NOT NULL DEFAULT 'normal',
    indexed_state TEXT NOT NULL DEFAULT 'pending',
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS facts (
    fact_key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 1.0,
    source_type TEXT NOT NULL DEFAULT 'artifact_scan',
    source_ref TEXT,
    updated_at TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    message TEXT NOT NULL,
    payload_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_artifacts_last_seen ON artifacts(last_seen_at DESC, path);
CREATE INDEX IF NOT EXISTS idx_facts_updated ON facts(updated_at DESC, fact_key);
CREATE INDEX IF NOT EXISTS idx_events_created ON events(created_at DESC, id DESC);
"""


def utcnow() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat()


def to_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True)


def from_json(value: str | None, default: Any = None) -> Any:
    if value is None:
        return default
    try:
        return json.loads(value)
    except json.JSONDecodeError:
        return default


def connect(db_path: str) -> sqlite3.Connection:
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    return conn


def init_db(conn: sqlite3.Connection) -> None:
    conn.executescript(SCHEMA_SQL)
    conn.commit()


def log_event(conn: sqlite3.Connection, event_type: str, message: str, payload: Any | None = None) -> None:
    conn.execute(
        "INSERT INTO events(event_type, message, payload_json, created_at) VALUES (?, ?, ?, ?)",
        (event_type, message, to_json(payload or {}), utcnow()),
    )


def set_fact(
    conn: sqlite3.Connection,
    fact_key: str,
    value: Any,
    confidence: float = 1.0,
    source_type: str = "artifact_scan",
    source_ref: str | None = None,
) -> None:
    conn.execute(
        """
        INSERT INTO facts(fact_key, value_json, confidence, source_type, source_ref, updated_at, metadata_json)
        VALUES (?, ?, ?, ?, ?, ?, '{}')
        ON CONFLICT(fact_key) DO UPDATE SET
            value_json=excluded.value_json,
            confidence=excluded.confidence,
            source_type=excluded.source_type,
            source_ref=excluded.source_ref,
            updated_at=excluded.updated_at
        """,
        (fact_key, to_json(value), confidence, source_type, source_ref, utcnow()),
    )


def sha256_for_file(path: Path) -> str:
    digest = hashlib.sha256()
    remaining = MAX_HASH_BYTES
    with path.open("rb") as fh:
        while remaining > 0:
            chunk = fh.read(min(1024 * 1024, remaining))
            if not chunk:
                break
            digest.update(chunk)
            remaining -= len(chunk)
    return digest.hexdigest()


def significance_for_path(path: Path) -> str:
    important_suffixes = {".py", ".rs", ".js", ".ts", ".md", ".txt", ".json", ".yaml", ".yml", ".sh", ".log"}
    if path.name == "NMSS_CERT_VALUE.md":
        return "high"
    if path.suffix.lower() in important_suffixes:
        return "high"
    return "normal"


def derive_artifact_facts(conn: sqlite3.Connection) -> None:
    interesting = {
        "crypto.has_nmss_emu": "%/nmss_emu.py",
        "global.has_analysis_doc": "%/NMSS_CERT_VALUE.md",
    }
    for fact_key, pattern in interesting.items():
        exists = conn.execute("SELECT 1 FROM artifacts WHERE path LIKE ? LIMIT 1", (pattern,)).fetchone() is not None
        set_fact(conn, fact_key, exists, source_ref=pattern)


def scan_artifacts(conn: sqlite3.Connection, project_dir: str) -> None:
    root = Path(project_dir)
    if not root.exists():
        log_event(conn, "artifact.scan", f"project dir missing: {project_dir}")
        conn.commit()
        return

    ignore_dirs = {".git", ".venv", "node_modules", "__pycache__", ".mypy_cache", ".pytest_cache", "target"}
    now = utcnow()

    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [name for name in dirnames if name not in ignore_dirs]
        for filename in filenames:
            path = Path(dirpath) / filename
            try:
                stat = path.stat()
            except OSError:
                continue

            path_str = str(path)
            row = conn.execute("SELECT mtime_ns FROM artifacts WHERE path=?", (path_str,)).fetchone()
            sha256 = None
            if row is None or row["mtime_ns"] != stat.st_mtime_ns:
                sha256 = sha256_for_file(path)

            if row is None:
                conn.execute(
                    """
                    INSERT INTO artifacts(path, sha256, size_bytes, mtime_ns, first_seen_at, last_seen_at, significance, metadata_json)
                    VALUES (?, ?, ?, ?, ?, ?, ?, '{}')
                    """,
                    (path_str, sha256, stat.st_size, stat.st_mtime_ns, now, now, significance_for_path(path)),
                )
                log_event(conn, "artifact.new", path_str, {"size": stat.st_size})
            elif row["mtime_ns"] != stat.st_mtime_ns:
                conn.execute(
                    """
                    UPDATE artifacts
                    SET sha256=?, size_bytes=?, mtime_ns=?, last_seen_at=?, significance=?
                    WHERE path=?
                    """,
                    (sha256, stat.st_size, stat.st_mtime_ns, now, significance_for_path(path), path_str),
                )
                log_event(conn, "artifact.modified", path_str, {"size": stat.st_size})
            else:
                conn.execute("UPDATE artifacts SET last_seen_at=? WHERE path=?", (now, path_str))

    derive_artifact_facts(conn)
    conn.commit()


def print_artifacts(conn: sqlite3.Connection, limit: int) -> None:
    rows = conn.execute(
        """
        SELECT path, sha256, size_bytes, significance, indexed_state, first_seen_at, last_seen_at
        FROM artifacts
        ORDER BY last_seen_at DESC, path ASC
        LIMIT ?
        """,
        (limit,),
    ).fetchall()
    for row in rows:
        print(dict(row))


def print_facts(conn: sqlite3.Connection) -> None:
    rows = conn.execute(
        "SELECT fact_key, value_json, confidence, source_type, source_ref, updated_at FROM facts ORDER BY fact_key"
    ).fetchall()
    for row in rows:
        item = dict(row)
        item["value_json"] = from_json(item["value_json"], item["value_json"])
        print(item)


def print_events(conn: sqlite3.Connection, limit: int) -> None:
    rows = conn.execute(
        "SELECT created_at, event_type, message, payload_json FROM events ORDER BY id DESC LIMIT ?",
        (limit,),
    ).fetchall()
    for row in rows:
        item = dict(row)
        item["payload_json"] = from_json(item["payload_json"], item["payload_json"])
        print(item)


def print_summary(conn: sqlite3.Connection) -> None:
    artifact_count = conn.execute("SELECT COUNT(*) FROM artifacts").fetchone()[0]
    fact_count = conn.execute("SELECT COUNT(*) FROM facts").fetchone()[0]
    recent = conn.execute(
        "SELECT path, significance, last_seen_at FROM artifacts ORDER BY last_seen_at DESC, path ASC LIMIT 10"
    ).fetchall()
    print(f"artifacts={artifact_count} facts={fact_count}")
    for row in recent:
        print(f"- {row['path']} | {row['significance']} | seen={row['last_seen_at']}")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Filesystem artifact scanner and fact derivation tool")
    parser.add_argument("--db", default="fs-check.db", help="SQLite database path")
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("init-db")

    scan = sub.add_parser("scan")
    scan.add_argument("--project-dir", default=PROJECT_DIR)

    artifacts = sub.add_parser("artifacts")
    artifacts.add_argument("--limit", type=int, default=100)

    sub.add_parser("facts")

    events = sub.add_parser("events")
    events.add_argument("--limit", type=int, default=50)

    summary = sub.add_parser("summary")
    summary.add_argument("--project-dir", default=PROJECT_DIR)
    summary.add_argument("--scan", action="store_true", help="Run a scan before printing the summary")

    return parser


def main(argv: list[str]) -> int:
    args = build_parser().parse_args(argv)
    conn = connect(args.db)
    init_db(conn)

    if args.cmd == "init-db":
        print(f"initialized {args.db}")
        return 0
    if args.cmd == "scan":
        scan_artifacts(conn, args.project_dir)
        print_summary(conn)
        return 0
    if args.cmd == "artifacts":
        print_artifacts(conn, args.limit)
        return 0
    if args.cmd == "facts":
        print_facts(conn)
        return 0
    if args.cmd == "events":
        print_events(conn, args.limit)
        return 0
    if args.cmd == "summary":
        if args.scan:
            scan_artifacts(conn, args.project_dir)
        print_summary(conn)
        return 0

    print(f"unknown command: {args.cmd}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
