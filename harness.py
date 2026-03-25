#!/usr/bin/env python3
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import re
import sqlite3
import subprocess
import sys
import time
import urllib.request
import urllib.error
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

BIOME_TERM_URL = os.environ.get("BIOME_TERM_URL", "http://localhost:3000")

PROJECT_DIR = "/home/sdancer/games/nmss"
TMUX_SESSION = "nmss"
POLL_SCROLLBACK_LINES = 20
REPEAT_STUCK_SECONDS = 600
MAX_HASH_BYTES = 2 * 1024 * 1024

SPINNER_CHARS = set("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏◐◓◑◒")
STUCK_PATTERNS = [
    r"traceback",
    r"exception",
    r"error:",
    r"permission denied",
    r"command not found",
    r"no such file",
    r"segmentation fault",
]

DEFAULT_AGENTS = [
    {
        "name": "oracle",
        "tmux_target": f"{TMUX_SESSION}:oracle",
        "workdir": PROJECT_DIR,
        "default_task": (
            "Continue the oracle task in /home/sdancer/games/nmss/. "
            "If done, test the oracle by running it. If already tested, save the results to VikingDB via "
            "`mcp openviking add_resource`."
        ),
    },
    {
        "name": "crypto",
        "tmux_target": f"{TMUX_SESSION}:crypto",
        "workdir": PROJECT_DIR,
        "default_task": (
            "Continue the crypto task in /home/sdancer/games/nmss/. "
            "If static analysis is done, try Unicorn emulation of the crypto function using nmss_emu.py. "
            "If stuck, hook sub_20bb48 and sub_2070a8 deeper with Capstone. "
            "Binary: output/decrypted/nmsscr.dec."
        ),
    },
    {
        "name": "hybrid",
        "tmux_target": f"{TMUX_SESSION}:hybrid",
        "workdir": PROJECT_DIR,
        "default_task": (
            "Continue the hybrid capture task in /home/sdancer/games/nmss/. "
            "If the capture script is done, test it. If crypto is not solved yet, stub the computation and "
            "validate that the capture path works."
        ),
    },
]

# Optional bootstrap content for the known NMSS workflow. Goals are NOT auto-created in the run loop.
BOOTSTRAP_GOALS = [
    {
        "goal_key": "orchestrator.deliver_tested_oracle",
        "title": "Deliver tested oracle and indexed evidence",
        "detail": "Drive the oracle from implementation through execution and VikingDB indexing.",
        "priority": 10,
        "success_fact_key": "oracle.indexed",
    },
    {
        "goal_key": "orchestrator.recover_crypto",
        "title": "Recover the crypto algorithm",
        "detail": "Drive crypto from static analysis through emulation to an identified algorithm.",
        "priority": 20,
        "success_fact_key": "crypto.algorithm_identified",
    },
    {
        "goal_key": "orchestrator.validate_capture_path",
        "title": "Validate the capture path",
        "detail": "Build and test the capture flow; use a stub until the crypto is fully solved.",
        "priority": 30,
        "success_fact_key": "hybrid.capture_tested",
    },
]

BOOTSTRAP_SUB_GOALS = [
    {
        "sub_goal_key": "oracle.test_oracle",
        "goal_key": "orchestrator.deliver_tested_oracle",
        "owner_agent": "oracle",
        "title": "Run the oracle",
        "detail": "Execute the oracle and capture concrete evidence.",
        "priority": 10,
        "success_fact_key": "oracle.tested",
        "instruction_text": "If you have finished the oracle implementation, run it now and capture the results.",
        "stuck_guidance_text": "If the build is complete, run the oracle now and record the output.",
    },
    {
        "sub_goal_key": "oracle.index_results",
        "goal_key": "orchestrator.deliver_tested_oracle",
        "owner_agent": "oracle",
        "title": "Index oracle results",
        "detail": "Save significant oracle artifacts to VikingDB.",
        "priority": 20,
        "depends_on_sub_goal_key": "oracle.test_oracle",
        "success_fact_key": "oracle.indexed",
        "instruction_text": "The oracle appears tested. Save significant results to VikingDB via `mcp openviking add_resource` and note what you indexed.",
    },
    {
        "sub_goal_key": "crypto.static_analysis",
        "goal_key": "orchestrator.recover_crypto",
        "owner_agent": "crypto",
        "title": "Finish static analysis",
        "detail": "Recover enough structure to drive emulation.",
        "priority": 10,
        "success_fact_key": "crypto.static_analysis_done",
        "instruction_text": "Keep advancing the crypto analysis. Hook sub_20bb48 and sub_2070a8 deeper with Capstone if needed. Binary: output/decrypted/nmsscr.dec.",
        "stuck_guidance_text": "Try hooking sub_20bb48 and sub_2070a8 deeper with Capstone. Binary: output/decrypted/nmsscr.dec.",
    },
    {
        "sub_goal_key": "crypto.unicorn_emulation",
        "goal_key": "orchestrator.recover_crypto",
        "owner_agent": "crypto",
        "title": "Run Unicorn emulation",
        "detail": "Use nmss_emu.py against output/decrypted/nmsscr.dec.",
        "priority": 20,
        "depends_on_sub_goal_key": "crypto.static_analysis",
        "success_fact_key": "crypto.unicorn_done",
        "instruction_text": "Static analysis appears complete. Try Unicorn emulation of the crypto function using nmss_emu.py. Binary: output/decrypted/nmsscr.dec. Capture any concrete inputs, outputs, or recovered constants.",
    },
    {
        "sub_goal_key": "crypto.identify_algorithm",
        "goal_key": "orchestrator.recover_crypto",
        "owner_agent": "crypto",
        "title": "Identify the algorithm",
        "detail": "Turn the recovered behavior into a concrete algorithm description and evidence.",
        "priority": 30,
        "depends_on_sub_goal_key": "crypto.unicorn_emulation",
        "success_fact_key": "crypto.algorithm_identified",
        "instruction_text": "Turn the recovered behavior into a concrete algorithm description with evidence, constants, and any validation runs.",
    },
    {
        "sub_goal_key": "hybrid.capture_script",
        "goal_key": "orchestrator.validate_capture_path",
        "owner_agent": "hybrid",
        "title": "Build the capture script",
        "detail": "Produce a script that captures session data.",
        "priority": 10,
        "success_fact_key": "hybrid.capture_script_done",
        "instruction_text": "Keep iterating on the capture script. If crypto remains unresolved, use a stub to validate the session capture path.",
        "stuck_guidance_text": "If crypto is not solved yet, stub the computation and validate the capture path anyway.",
    },
    {
        "sub_goal_key": "hybrid.capture_test",
        "goal_key": "orchestrator.validate_capture_path",
        "owner_agent": "hybrid",
        "title": "Validate the capture path",
        "detail": "Test the capture flow; stub crypto if needed.",
        "priority": 20,
        "depends_on_sub_goal_key": "hybrid.capture_script",
        "success_fact_key": "hybrid.capture_tested",
        "instruction_text": "The capture script appears ready. Test it now; if crypto is not solved yet, stub the computation and validate that capture works.",
    },
]

SCHEMA_SQL = """
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS agents (
    name TEXT PRIMARY KEY,
    tmux_target TEXT NOT NULL,
    workdir TEXT,
    default_task TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'unknown',
    current_goal_key TEXT,
    current_sub_goal_key TEXT,
    last_seen_at TEXT,
    last_capture_hash TEXT,
    last_capture_preview TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS goals (
    goal_key TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    detail TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 50,
    depends_on_goal_key TEXT,
    success_fact_key TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(depends_on_goal_key) REFERENCES goals(goal_key)
);

CREATE TABLE IF NOT EXISTS sub_goals (
    sub_goal_key TEXT PRIMARY KEY,
    goal_key TEXT NOT NULL,
    owner_agent TEXT NOT NULL,
    title TEXT NOT NULL,
    detail TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 50,
    depends_on_sub_goal_key TEXT,
    success_fact_key TEXT,
    instruction_text TEXT,
    stuck_guidance_text TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(goal_key) REFERENCES goals(goal_key),
    FOREIGN KEY(owner_agent) REFERENCES agents(name),
    FOREIGN KEY(depends_on_sub_goal_key) REFERENCES sub_goals(sub_goal_key)
);

CREATE TABLE IF NOT EXISTS facts (
    fact_key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 1.0,
    source_type TEXT NOT NULL DEFAULT 'manual',
    source_ref TEXT,
    updated_at TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_name TEXT NOT NULL,
    kind TEXT NOT NULL,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(agent_name) REFERENCES agents(name)
);

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

CREATE TABLE IF NOT EXISTS actions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_name TEXT,
    action_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    reason TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    executed_at TEXT,
    result_text TEXT,
    FOREIGN KEY(agent_name) REFERENCES agents(name)
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_name TEXT,
    event_type TEXT NOT NULL,
    message TEXT NOT NULL,
    payload_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    FOREIGN KEY(agent_name) REFERENCES agents(name)
);

CREATE INDEX IF NOT EXISTS idx_observations_agent_created ON observations(agent_name, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_actions_status_created ON actions(status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_events_agent_created ON events(agent_name, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_goals_status_priority ON goals(status, priority, goal_key);
CREATE INDEX IF NOT EXISTS idx_sub_goals_owner_priority ON sub_goals(owner_agent, status, priority, sub_goal_key);
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


def parse_json_value(raw: str) -> Any:
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        low = raw.lower()
        if low == "true":
            return True
        if low == "false":
            return False
        if low == "null":
            return None
        return raw


def connect(db_path: str) -> sqlite3.Connection:
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    return conn


def column_exists(conn: sqlite3.Connection, table: str, column: str) -> bool:
    rows = conn.execute(f"PRAGMA table_info({table})").fetchall()
    return any(row[1] == column for row in rows)


def ensure_column(conn: sqlite3.Connection, table: str, column: str, definition: str) -> None:
    if not column_exists(conn, table, column):
        conn.execute(f"ALTER TABLE {table} ADD COLUMN {column} {definition}")


def init_db(conn: sqlite3.Connection) -> None:
    conn.executescript(SCHEMA_SQL)
    # Lightweight migrations for older DBs.
    ensure_column(conn, "agents", "current_goal_key", "TEXT")
    ensure_column(conn, "agents", "current_sub_goal_key", "TEXT")
    ensure_column(conn, "goals", "metadata_json", "TEXT NOT NULL DEFAULT '{}' ")
    ensure_column(conn, "sub_goals", "instruction_text", "TEXT")
    ensure_column(conn, "sub_goals", "stuck_guidance_text", "TEXT")
    ensure_column(conn, "sub_goals", "metadata_json", "TEXT NOT NULL DEFAULT '{}' ")
    ensure_column(conn, "agents", "biome_pane_id", "TEXT")
    conn.commit()


def log_event(conn: sqlite3.Connection, agent_name: str | None, event_type: str, message: str, payload: Any | None = None) -> None:
    conn.execute(
        "INSERT INTO events(agent_name, event_type, message, payload_json, created_at) VALUES (?, ?, ?, ?, ?)",
        (agent_name, event_type, message, to_json(payload or {}), utcnow()),
    )


def seed_agents(conn: sqlite3.Connection) -> None:
    for agent in DEFAULT_AGENTS:
        conn.execute(
            """
            INSERT INTO agents(name, tmux_target, workdir, default_task, status, metadata_json)
            VALUES (?, ?, ?, ?, 'unknown', '{}')
            ON CONFLICT(name) DO UPDATE SET
                tmux_target=excluded.tmux_target,
                workdir=excluded.workdir,
                default_task=excluded.default_task
            """,
            (agent["name"], agent["tmux_target"], agent["workdir"], agent["default_task"]),
        )
    conn.commit()


def upsert_agent(
    conn: sqlite3.Connection,
    name: str,
    biome_pane_id: str,
    workdir: str = "",
    default_task: str = "Continue the current task.",
    tmux_target: str = "",
) -> None:
    conn.execute(
        """
        INSERT INTO agents(name, tmux_target, workdir, default_task, status, biome_pane_id, metadata_json)
        VALUES (?, ?, ?, ?, 'unknown', ?, '{}')
        ON CONFLICT(name) DO UPDATE SET
            tmux_target=excluded.tmux_target,
            workdir=excluded.workdir,
            default_task=excluded.default_task,
            biome_pane_id=excluded.biome_pane_id
        """,
        (name, tmux_target, workdir, default_task, biome_pane_id),
    )
    log_event(conn, name, "agent.registered", f"biome_pane_id={biome_pane_id}")
    conn.commit()


def remove_agent(conn: sqlite3.Connection, name: str, *, delete: bool = False) -> None:
    row = conn.execute("SELECT name FROM agents WHERE name=?", (name,)).fetchone()
    if row is None:
        raise ValueError(f"unknown agent: {name}")
    if delete:
        now = utcnow()
        conn.execute("UPDATE sub_goals SET status='cancelled', updated_at=? WHERE owner_agent=? AND status NOT IN ('done','cancelled')", (now, name))
        conn.execute("DELETE FROM sub_goals WHERE owner_agent=?", (name,))
        conn.execute("DELETE FROM observations WHERE agent_name=?", (name,))
        conn.execute("DELETE FROM actions WHERE agent_name=?", (name,))
        conn.execute("DELETE FROM events WHERE agent_name=?", (name,))
        conn.execute("DELETE FROM agents WHERE name=?", (name,))
        log_event(conn, None, "agent.deleted", name)
    else:
        conn.execute("UPDATE agents SET status='unknown', biome_pane_id=NULL WHERE name=?", (name,))
        log_event(conn, name, "agent.deregistered", name)
    conn.commit()


def upsert_goal(
    conn: sqlite3.Connection,
    goal_key: str,
    title: str,
    detail: str = "",
    status: str = "pending",
    priority: int = 50,
    depends_on_goal_key: str | None = None,
    success_fact_key: str | None = None,
    metadata: dict[str, Any] | None = None,
) -> None:
    now = utcnow()
    conn.execute(
        """
        INSERT INTO goals(
            goal_key, title, detail, status, priority, depends_on_goal_key,
            success_fact_key, metadata_json, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(goal_key) DO UPDATE SET
            title=excluded.title,
            detail=excluded.detail,
            status=excluded.status,
            priority=excluded.priority,
            depends_on_goal_key=excluded.depends_on_goal_key,
            success_fact_key=excluded.success_fact_key,
            metadata_json=excluded.metadata_json,
            updated_at=excluded.updated_at
        """,
        (
            goal_key,
            title,
            detail,
            status,
            priority,
            depends_on_goal_key,
            success_fact_key,
            to_json(metadata or {}),
            now,
            now,
        ),
    )
    log_event(conn, None, "goal.upserted", goal_key, {"status": status})
    conn.commit()


def update_goal(
    conn: sqlite3.Connection,
    goal_key: str,
    *,
    title: str | None = None,
    detail: str | None = None,
    status: str | None = None,
    priority: int | None = None,
    depends_on_goal_key: str | None = None,
    success_fact_key: str | None = None,
    metadata: dict[str, Any] | None = None,
    clear_depends: bool = False,
    clear_success_fact: bool = False,
) -> None:
    row = conn.execute("SELECT * FROM goals WHERE goal_key=?", (goal_key,)).fetchone()
    if row is None:
        raise ValueError(f"unknown goal: {goal_key}")
    new_metadata = from_json(row["metadata_json"], {})
    if metadata:
        new_metadata.update(metadata)
    conn.execute(
        """
        UPDATE goals SET
            title=?, detail=?, status=?, priority=?, depends_on_goal_key=?, success_fact_key=?,
            metadata_json=?, updated_at=?
        WHERE goal_key=?
        """,
        (
            title if title is not None else row["title"],
            detail if detail is not None else row["detail"],
            status if status is not None else row["status"],
            priority if priority is not None else row["priority"],
            None if clear_depends else (depends_on_goal_key if depends_on_goal_key is not None else row["depends_on_goal_key"]),
            None if clear_success_fact else (success_fact_key if success_fact_key is not None else row["success_fact_key"]),
            to_json(new_metadata),
            utcnow(),
            goal_key,
        ),
    )
    log_event(conn, None, "goal.updated", goal_key, {"status": status})
    conn.commit()


def remove_goal(conn: sqlite3.Connection, goal_key: str, *, delete: bool = False, cascade: bool = False) -> None:
    row = conn.execute("SELECT goal_key FROM goals WHERE goal_key=?", (goal_key,)).fetchone()
    if row is None:
        raise ValueError(f"unknown goal: {goal_key}")
    if delete:
        sub_goal_count = conn.execute("SELECT COUNT(*) FROM sub_goals WHERE goal_key=?", (goal_key,)).fetchone()[0]
        if sub_goal_count and not cascade:
            raise ValueError(f"goal {goal_key} still has {sub_goal_count} sub-goal(s); use --cascade or cancel instead")
        if cascade:
            conn.execute("DELETE FROM sub_goals WHERE goal_key=?", (goal_key,))
        conn.execute("UPDATE agents SET current_goal_key=NULL, current_sub_goal_key=NULL WHERE current_goal_key=?", (goal_key,))
        conn.execute("DELETE FROM goals WHERE goal_key=?", (goal_key,))
        log_event(conn, None, "goal.deleted", goal_key, {"cascade": cascade})
    else:
        conn.execute("UPDATE goals SET status='cancelled', updated_at=? WHERE goal_key=?", (utcnow(), goal_key))
        conn.execute("UPDATE sub_goals SET status='cancelled', updated_at=? WHERE goal_key=? AND status!='done'", (utcnow(), goal_key))
        conn.execute("UPDATE agents SET current_goal_key=NULL, current_sub_goal_key=NULL WHERE current_goal_key=?", (goal_key,))
        log_event(conn, None, "goal.cancelled", goal_key, {})
    conn.commit()


def upsert_sub_goal(
    conn: sqlite3.Connection,
    sub_goal_key: str,
    goal_key: str,
    owner_agent: str,
    title: str,
    detail: str = "",
    status: str = "pending",
    priority: int = 50,
    depends_on_sub_goal_key: str | None = None,
    success_fact_key: str | None = None,
    instruction_text: str | None = None,
    stuck_guidance_text: str | None = None,
    metadata: dict[str, Any] | None = None,
) -> None:
    now = utcnow()
    conn.execute(
        """
        INSERT INTO sub_goals(
            sub_goal_key, goal_key, owner_agent, title, detail, status, priority,
            depends_on_sub_goal_key, success_fact_key, instruction_text, stuck_guidance_text,
            metadata_json, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(sub_goal_key) DO UPDATE SET
            goal_key=excluded.goal_key,
            owner_agent=excluded.owner_agent,
            title=excluded.title,
            detail=excluded.detail,
            status=excluded.status,
            priority=excluded.priority,
            depends_on_sub_goal_key=excluded.depends_on_sub_goal_key,
            success_fact_key=excluded.success_fact_key,
            instruction_text=excluded.instruction_text,
            stuck_guidance_text=excluded.stuck_guidance_text,
            metadata_json=excluded.metadata_json,
            updated_at=excluded.updated_at
        """,
        (
            sub_goal_key,
            goal_key,
            owner_agent,
            title,
            detail,
            status,
            priority,
            depends_on_sub_goal_key,
            success_fact_key,
            instruction_text,
            stuck_guidance_text,
            to_json(metadata or {}),
            now,
            now,
        ),
    )
    log_event(conn, owner_agent, "sub_goal.upserted", sub_goal_key, {"goal_key": goal_key, "status": status})
    conn.commit()


def update_sub_goal(
    conn: sqlite3.Connection,
    sub_goal_key: str,
    *,
    goal_key: str | None = None,
    owner_agent: str | None = None,
    title: str | None = None,
    detail: str | None = None,
    status: str | None = None,
    priority: int | None = None,
    depends_on_sub_goal_key: str | None = None,
    success_fact_key: str | None = None,
    instruction_text: str | None = None,
    stuck_guidance_text: str | None = None,
    metadata: dict[str, Any] | None = None,
    clear_depends: bool = False,
    clear_success_fact: bool = False,
    clear_instruction: bool = False,
    clear_stuck_guidance: bool = False,
) -> None:
    row = conn.execute("SELECT * FROM sub_goals WHERE sub_goal_key=?", (sub_goal_key,)).fetchone()
    if row is None:
        raise ValueError(f"unknown sub-goal: {sub_goal_key}")
    new_metadata = from_json(row["metadata_json"], {})
    if metadata:
        new_metadata.update(metadata)
    conn.execute(
        """
        UPDATE sub_goals SET
            goal_key=?, owner_agent=?, title=?, detail=?, status=?, priority=?,
            depends_on_sub_goal_key=?, success_fact_key=?, instruction_text=?, stuck_guidance_text=?,
            metadata_json=?, updated_at=?
        WHERE sub_goal_key=?
        """,
        (
            goal_key if goal_key is not None else row["goal_key"],
            owner_agent if owner_agent is not None else row["owner_agent"],
            title if title is not None else row["title"],
            detail if detail is not None else row["detail"],
            status if status is not None else row["status"],
            priority if priority is not None else row["priority"],
            None if clear_depends else (depends_on_sub_goal_key if depends_on_sub_goal_key is not None else row["depends_on_sub_goal_key"]),
            None if clear_success_fact else (success_fact_key if success_fact_key is not None else row["success_fact_key"]),
            None if clear_instruction else (instruction_text if instruction_text is not None else row["instruction_text"]),
            None if clear_stuck_guidance else (stuck_guidance_text if stuck_guidance_text is not None else row["stuck_guidance_text"]),
            to_json(new_metadata),
            utcnow(),
            sub_goal_key,
        ),
    )
    log_event(conn, owner_agent or row["owner_agent"], "sub_goal.updated", sub_goal_key, {"status": status})
    conn.commit()


def remove_sub_goal(conn: sqlite3.Connection, sub_goal_key: str, *, delete: bool = False) -> None:
    row = conn.execute("SELECT owner_agent FROM sub_goals WHERE sub_goal_key=?", (sub_goal_key,)).fetchone()
    if row is None:
        raise ValueError(f"unknown sub-goal: {sub_goal_key}")
    if delete:
        conn.execute("UPDATE agents SET current_sub_goal_key=NULL WHERE current_sub_goal_key=?", (sub_goal_key,))
        conn.execute("DELETE FROM sub_goals WHERE sub_goal_key=?", (sub_goal_key,))
        log_event(conn, row["owner_agent"], "sub_goal.deleted", sub_goal_key, {})
    else:
        conn.execute("UPDATE sub_goals SET status='cancelled', updated_at=? WHERE sub_goal_key=?", (utcnow(), sub_goal_key))
        conn.execute("UPDATE agents SET current_sub_goal_key=NULL WHERE current_sub_goal_key=?", (sub_goal_key,))
        log_event(conn, row["owner_agent"], "sub_goal.cancelled", sub_goal_key, {})
    conn.commit()


def bootstrap_known_goals(conn: sqlite3.Connection) -> None:
    seed_agents(conn)
    for goal in BOOTSTRAP_GOALS:
        upsert_goal(
            conn,
            goal_key=goal["goal_key"],
            title=goal["title"],
            detail=goal.get("detail", ""),
            status=goal.get("status", "pending"),
            priority=goal.get("priority", 50),
            depends_on_goal_key=goal.get("depends_on_goal_key"),
            success_fact_key=goal.get("success_fact_key"),
        )
    for sub_goal in BOOTSTRAP_SUB_GOALS:
        upsert_sub_goal(
            conn,
            sub_goal_key=sub_goal["sub_goal_key"],
            goal_key=sub_goal["goal_key"],
            owner_agent=sub_goal["owner_agent"],
            title=sub_goal["title"],
            detail=sub_goal.get("detail", ""),
            status=sub_goal.get("status", "pending"),
            priority=sub_goal.get("priority", 50),
            depends_on_sub_goal_key=sub_goal.get("depends_on_sub_goal_key"),
            success_fact_key=sub_goal.get("success_fact_key"),
            instruction_text=sub_goal.get("instruction_text"),
            stuck_guidance_text=sub_goal.get("stuck_guidance_text"),
        )


def set_fact(
    conn: sqlite3.Connection,
    fact_key: str,
    value: Any,
    confidence: float = 1.0,
    source_type: str = "manual",
    source_ref: str | None = None,
    metadata: dict[str, Any] | None = None,
) -> None:
    conn.execute(
        """
        INSERT INTO facts(fact_key, value_json, confidence, source_type, source_ref, updated_at, metadata_json)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(fact_key) DO UPDATE SET
            value_json=excluded.value_json,
            confidence=excluded.confidence,
            source_type=excluded.source_type,
            source_ref=excluded.source_ref,
            updated_at=excluded.updated_at,
            metadata_json=excluded.metadata_json
        """,
        (fact_key, to_json(value), confidence, source_type, source_ref, utcnow(), to_json(metadata or {})),
    )
    log_event(conn, None, "fact.updated", f"{fact_key}={value!r}", {"fact_key": fact_key, "value": value})
    conn.commit()


def get_fact(conn: sqlite3.Connection, fact_key: str, default: Any = None) -> Any:
    row = conn.execute("SELECT value_json FROM facts WHERE fact_key=?", (fact_key,)).fetchone()
    if row is None:
        return default
    return from_json(row["value_json"], default)


def fact_is_true(conn: sqlite3.Connection, fact_key: str | None) -> bool:
    if not fact_key:
        return False
    return get_fact(conn, fact_key) is True


def run_cmd(cmd: list[str], cwd: str | None = None, timeout: int = 30) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=cwd, timeout=timeout, text=True, capture_output=True)


def capture_tmux(target: str, lines: int = POLL_SCROLLBACK_LINES) -> str | None:
    try:
        cp = run_cmd(["tmux", "capture-pane", "-t", target, "-p", "-S", str(-lines)], timeout=10)
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return None
    if cp.returncode != 0:
        return None
    return cp.stdout


def _biome_request(method: str, path: str, body: dict | None = None, timeout: int = 10) -> dict | list | None:
    url = f"{BIOME_TERM_URL}{path}"
    data = json.dumps(body).encode() if body else None
    req = urllib.request.Request(url, data=data, method=method)
    if data:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            if resp.status == 204:
                return {}
            return json.loads(resp.read())
    except (urllib.error.URLError, urllib.error.HTTPError, OSError, json.JSONDecodeError):
        return None


def capture_biome(pane_id: str, lines: int = POLL_SCROLLBACK_LINES) -> str | None:
    screen = _biome_request("GET", f"/panes/{pane_id}/screen")
    if screen is None:
        return None
    rows = screen.get("rows", [])
    tail = rows[-lines:] if len(rows) > lines else rows
    return "\n".join(tail)


def capture_agent(agent: sqlite3.Row, lines: int = POLL_SCROLLBACK_LINES) -> str | None:
    pane_id = agent["biome_pane_id"] if "biome_pane_id" in agent.keys() else None
    if pane_id:
        return capture_biome(pane_id, lines)
    return capture_tmux(agent["tmux_target"], lines)


def send_tmux_text(target: str, text: str) -> tuple[bool, str]:
    try:
        cp = run_cmd(["tmux", "send-keys", "-t", target, text, "Enter"], timeout=10)
    except (subprocess.TimeoutExpired, FileNotFoundError) as exc:
        return False, str(exc)
    if cp.returncode != 0:
        return False, (cp.stderr or cp.stdout or "tmux send-keys failed").strip()
    return True, "sent"


def send_biome_text(pane_id: str, text: str) -> tuple[bool, str]:
    encoded = base64.b64encode((text + "\r").encode()).decode()
    result = _biome_request("POST", f"/panes/{pane_id}/input", {"data": encoded})
    if result is None:
        return False, f"biome_term input failed for pane {pane_id}"
    return True, "sent"


def send_agent_text(agent: sqlite3.Row, text: str) -> tuple[bool, str]:
    pane_id = agent["biome_pane_id"] if "biome_pane_id" in agent.keys() else None
    if pane_id:
        return send_biome_text(pane_id, text)
    return send_tmux_text(agent["tmux_target"], text)


def hash_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8", errors="replace")).hexdigest()


def nonempty_lines(text: str) -> list[str]:
    return [line.rstrip("\n") for line in text.splitlines() if line.strip()]


def looks_idle(text: str) -> bool:
    lines = nonempty_lines(text)
    if not lines:
        return False
    tail = lines[-1].strip()
    return tail.startswith("❯") or tail == ">" or tail.endswith(" ❯")


def looks_working(text: str) -> bool:
    lower = text.lower()
    if any(token in lower for token in ("thinking", "analyzing", "processing", "working", "running")):
        return True
    return any(ch in SPINNER_CHARS for ch in text)


def looks_stuck(text: str) -> bool:
    lower = text.lower()
    return any(re.search(pattern, lower) for pattern in STUCK_PATTERNS)


def classify_capture(text: str | None, previous_hash: str | None, previous_seen_at: str | None) -> str:
    if text is None:
        return "dead"
    if looks_stuck(text):
        return "stuck"
    if looks_idle(text):
        return "idle"
    if looks_working(text):
        return "working"
    current_hash = hash_text(text)
    if previous_hash and current_hash == previous_hash and previous_seen_at:
        try:
            then = datetime.fromisoformat(previous_seen_at)
            age = (datetime.now(timezone.utc) - then).total_seconds()
            if age >= REPEAT_STUCK_SECONDS:
                return "stuck"
        except ValueError:
            pass
    return "working"


def poll_agents(conn: sqlite3.Connection) -> None:
    rows = conn.execute("SELECT * FROM agents ORDER BY name").fetchall()
    now = utcnow()
    for row in rows:
        if row["status"] == "paused":
            continue
        capture = capture_agent(row)
        status = classify_capture(capture, row["last_capture_hash"], row["last_seen_at"])
        content = capture or ""
        content_hash = hash_text(content) if capture is not None else ""
        preview = "\n".join(nonempty_lines(content)[-3:])[:500]
        conn.execute(
            """
            UPDATE agents
            SET status=?, last_seen_at=?, last_capture_hash=?, last_capture_preview=?
            WHERE name=?
            """,
            (status, now, content_hash, preview, row["name"]),
        )
        conn.execute(
            "INSERT INTO observations(agent_name, kind, content, content_hash, created_at) VALUES (?, 'tmux_capture', ?, ?, ?)",
            (row["name"], content, content_hash, now),
        )
        log_event(conn, row["name"], "agent.polled", f"status={status}", {"preview": preview})
    conn.commit()


def sha256_for_file(path: Path) -> str:
    h = hashlib.sha256()
    remaining = MAX_HASH_BYTES
    with path.open("rb") as fh:
        while remaining > 0:
            chunk = fh.read(min(1024 * 1024, remaining))
            if not chunk:
                break
            h.update(chunk)
            remaining -= len(chunk)
    return h.hexdigest()


def significance_for_path(path: Path) -> str:
    important_suffixes = {".py", ".js", ".ts", ".md", ".txt", ".json", ".yaml", ".yml", ".sh", ".log"}
    if path.name == "NMSS_CERT_VALUE.md":
        return "high"
    if path.suffix.lower() in important_suffixes:
        return "high"
    if "/output/" in str(path).replace("\\", "/"):
        return "normal"
    return "normal"


def scan_artifacts(conn: sqlite3.Connection, project_dir: str = PROJECT_DIR) -> None:
    root = Path(project_dir)
    if not root.exists():
        log_event(conn, None, "artifact.scan", f"project dir missing: {project_dir}")
        conn.commit()
        return

    ignore_dirs = {".git", ".venv", "node_modules", "__pycache__", ".mypy_cache", ".pytest_cache"}
    now = utcnow()
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in ignore_dirs]
        for filename in filenames:
            path = Path(dirpath) / filename
            try:
                stat = path.stat()
            except OSError:
                continue
            path_str = str(path)
            row = conn.execute("SELECT mtime_ns, sha256 FROM artifacts WHERE path=?", (path_str,)).fetchone()
            sha = None
            if row is None or row["mtime_ns"] != stat.st_mtime_ns:
                sha = sha256_for_file(path)
            if row is None:
                conn.execute(
                    """
                    INSERT INTO artifacts(path, sha256, size_bytes, mtime_ns, first_seen_at, last_seen_at, significance, metadata_json)
                    VALUES (?, ?, ?, ?, ?, ?, ?, '{}')
                    """,
                    (path_str, sha, stat.st_size, stat.st_mtime_ns, now, now, significance_for_path(path)),
                )
                log_event(conn, None, "artifact.new", path_str, {"size": stat.st_size})
            elif row["mtime_ns"] != stat.st_mtime_ns:
                conn.execute(
                    "UPDATE artifacts SET sha256=?, size_bytes=?, mtime_ns=?, last_seen_at=?, significance=? WHERE path=?",
                    (sha, stat.st_size, stat.st_mtime_ns, now, significance_for_path(path), path_str),
                )
                log_event(conn, None, "artifact.modified", path_str, {"size": stat.st_size})
            else:
                conn.execute("UPDATE artifacts SET last_seen_at=? WHERE path=?", (now, path_str))
    conn.commit()
    derive_artifact_facts(conn)


def derive_artifact_facts(conn: sqlite3.Connection) -> None:
    interesting = {
        "crypto.has_nmss_emu": "%/nmss_emu.py",
        "global.has_analysis_doc": "%/NMSS_CERT_VALUE.md",
    }
    for fact_key, pattern in interesting.items():
        exists = conn.execute("SELECT 1 FROM artifacts WHERE path LIKE ? LIMIT 1", (pattern,)).fetchone() is not None
        set_fact(conn, fact_key, exists, source_type="artifact_scan", source_ref=pattern)


def resolve_goal_states(conn: sqlite3.Connection) -> None:
    goals = conn.execute("SELECT * FROM goals ORDER BY priority, goal_key").fetchall()
    for goal in goals:
        if goal["status"] in ("cancelled", "paused"):
            continue
        if fact_is_true(conn, goal["success_fact_key"]):
            new_status = "done"
        elif goal["depends_on_goal_key"]:
            dep = conn.execute("SELECT status FROM goals WHERE goal_key=?", (goal["depends_on_goal_key"],)).fetchone()
            new_status = "blocked" if dep and dep["status"] != "done" else "active"
        else:
            new_status = "active"
        if goal["status"] != new_status:
            conn.execute("UPDATE goals SET status=?, updated_at=? WHERE goal_key=?", (new_status, utcnow(), goal["goal_key"]))
            log_event(conn, None, "goal.resolved", goal["goal_key"], {"status": new_status})
    conn.commit()


def resolve_sub_goal_states(conn: sqlite3.Connection) -> None:
    sub_goals = conn.execute(
        "SELECT sg.*, g.status AS parent_goal_status, g.priority AS parent_goal_priority FROM sub_goals sg JOIN goals g ON g.goal_key = sg.goal_key ORDER BY g.priority, sg.priority, sg.sub_goal_key"
    ).fetchall()
    for sub_goal in sub_goals:
        if sub_goal["status"] in ("cancelled", "paused"):
            continue
        if fact_is_true(conn, sub_goal["success_fact_key"]):
            new_status = "done"
        elif sub_goal["parent_goal_status"] not in ("active", "pending"):
            new_status = "blocked"
        elif sub_goal["depends_on_sub_goal_key"]:
            dep = conn.execute("SELECT status FROM sub_goals WHERE sub_goal_key=?", (sub_goal["depends_on_sub_goal_key"],)).fetchone()
            new_status = "blocked" if dep and dep["status"] != "done" else "pending"
        else:
            new_status = "pending"
        if sub_goal["status"] != new_status:
            conn.execute("UPDATE sub_goals SET status=?, updated_at=? WHERE sub_goal_key=?", (new_status, utcnow(), sub_goal["sub_goal_key"]))
            log_event(conn, sub_goal["owner_agent"], "sub_goal.resolved", sub_goal["sub_goal_key"], {"status": new_status})
    conn.commit()


def resolve_active_sub_goals(conn: sqlite3.Connection) -> None:
    resolve_goal_states(conn)
    resolve_sub_goal_states(conn)

    agents = conn.execute("SELECT name FROM agents ORDER BY name").fetchall()
    for agent in agents:
        candidates = conn.execute(
            """
            SELECT sg.sub_goal_key, sg.goal_key
            FROM sub_goals sg
            JOIN goals g ON g.goal_key = sg.goal_key
            WHERE sg.owner_agent=?
              AND sg.status='pending'
              AND g.status='active'
            ORDER BY g.priority ASC, sg.priority ASC, sg.sub_goal_key ASC
            """,
            (agent["name"],),
        ).fetchall()
        if candidates:
            current_sub_goal_key = candidates[0]["sub_goal_key"]
            current_goal_key = candidates[0]["goal_key"]
            conn.execute("UPDATE sub_goals SET status='active', updated_at=? WHERE sub_goal_key=?", (utcnow(), current_sub_goal_key))
            conn.execute(
                "UPDATE sub_goals SET status='pending', updated_at=? WHERE owner_agent=? AND sub_goal_key<>? AND status='active'",
                (utcnow(), agent["name"], current_sub_goal_key),
            )
            conn.execute(
                "UPDATE agents SET current_goal_key=?, current_sub_goal_key=? WHERE name=?",
                (current_goal_key, current_sub_goal_key, agent["name"]),
            )
            log_event(conn, agent["name"], "agent.sub_goal_assigned", current_sub_goal_key, {"goal_key": current_goal_key})
        else:
            conn.execute(
                "UPDATE agents SET current_goal_key=NULL, current_sub_goal_key=NULL WHERE name=?",
                (agent["name"],),
            )
    conn.commit()


def current_sub_goal(conn: sqlite3.Connection, agent_name: str) -> sqlite3.Row | None:
    return conn.execute(
        """
        SELECT sg.*, g.title AS goal_title, g.detail AS goal_detail
        FROM agents a
        LEFT JOIN sub_goals sg ON sg.sub_goal_key = a.current_sub_goal_key
        LEFT JOIN goals g ON g.goal_key = a.current_goal_key
        WHERE a.name=?
        """,
        (agent_name,),
    ).fetchone()


def queue_action(conn: sqlite3.Connection, action_type: str, payload: dict[str, Any], reason: str, agent_name: str | None = None) -> None:
    existing = conn.execute(
        """
        SELECT 1 FROM actions
        WHERE status='pending' AND action_type=? AND agent_name IS ? AND payload_json=?
        LIMIT 1
        """,
        (action_type, agent_name, to_json(payload)),
    ).fetchone()
    if existing:
        return
    conn.execute(
        "INSERT INTO actions(agent_name, action_type, payload_json, reason, status, created_at) VALUES (?, ?, ?, ?, 'pending', ?)",
        (agent_name, action_type, to_json(payload), reason, utcnow()),
    )
    log_event(conn, agent_name, "action.queued", f"{action_type}: {reason}", payload)
    conn.commit()


def fallback_instruction_for_sub_goal(conn: sqlite3.Connection, sub_goal: sqlite3.Row) -> str:
    key = sub_goal["sub_goal_key"]
    if key == "hybrid.capture_test" and get_fact(conn, "crypto.algorithm_identified") is True:
        return "The capture script appears ready. Test it end-to-end now and record the captured session data."
    return (
        f"Continue sub-goal `{sub_goal['sub_goal_key']}` under goal `{sub_goal['goal_key']}`. "
        f"Title: {sub_goal['title']}. Detail: {sub_goal['detail'] or 'n/a'}"
    )


def current_task_prompt(conn: sqlite3.Connection, agent_name: str) -> str:
    sub_goal = current_sub_goal(conn, agent_name)
    if sub_goal and sub_goal["sub_goal_key"]:
        return (sub_goal["instruction_text"] or fallback_instruction_for_sub_goal(conn, sub_goal)).strip()
    agent = conn.execute("SELECT default_task FROM agents WHERE name=?", (agent_name,)).fetchone()
    return agent["default_task"] if agent else "Continue the current task."


def corrective_prompt(conn: sqlite3.Connection, agent_name: str, preview: str) -> str:
    sub_goal = current_sub_goal(conn, agent_name)
    base = f"You look stuck. Recover from the last issue and keep moving. Current tail:\n\n{preview}\n"
    if sub_goal and sub_goal["stuck_guidance_text"]:
        return base + sub_goal["stuck_guidance_text"]
    if agent_name == "crypto":
        return base + (
            "Try hooking sub_20bb48 and sub_2070a8 deeper with Capstone. "
            "If static analysis is already sufficient, move to Unicorn emulation using nmss_emu.py against "
            "output/decrypted/nmsscr.dec."
        )
    if agent_name == "oracle":
        return base + "If the build is complete, run the oracle now and record the output."
    if agent_name == "hybrid":
        return base + "If crypto is not solved yet, stub the computation and validate the capture path anyway."
    return base


def follow_up_prompt(conn: sqlite3.Connection, agent_name: str) -> str | None:
    sub_goal = current_sub_goal(conn, agent_name)
    if sub_goal and sub_goal["sub_goal_key"]:
        return current_task_prompt(conn, agent_name)
    return None


def cross_pollinate(conn: sqlite3.Connection) -> None:
    algo_details = get_fact(conn, "crypto.algorithm_details")
    if algo_details and get_fact(conn, "hybrid.algorithm_shared") is not True:
        queue_action(
            conn,
            "send_prompt",
            {"text": f"Crypto recovered the algorithm details below. Use them to replace the stub and validate capture end-to-end:\n\n{algo_details}"},
            "Share crypto findings with hybrid",
            "hybrid",
        )
        set_fact(conn, "hybrid.algorithm_shared", True, source_type="policy", source_ref="cross_pollinate")

    session_data = get_fact(conn, "hybrid.session_data")
    if session_data and get_fact(conn, "crypto.session_shared") is not True:
        queue_action(
            conn,
            "send_prompt",
            {"text": f"Hybrid captured new session data. Use it to drive crypto analysis and emulation:\n\n{session_data}"},
            "Share hybrid capture with crypto",
            "crypto",
        )
        set_fact(conn, "crypto.session_shared", True, source_type="policy", source_ref="cross_pollinate")


def queue_index_actions(conn: sqlite3.Connection) -> None:
    rows = conn.execute(
        "SELECT path FROM artifacts WHERE significance='high' AND indexed_state='pending' ORDER BY last_seen_at DESC LIMIT 25"
    ).fetchall()
    for row in rows:
        queue_action(conn, "index_artifact", {"path": row["path"]}, "Index significant artifact")


def decide_actions(conn: sqlite3.Connection) -> None:
    resolve_active_sub_goals(conn)
    agents = conn.execute("SELECT * FROM agents ORDER BY name").fetchall()
    for agent in agents:
        name = agent["name"]
        status = agent["status"]
        if status == "paused":
            continue
        preview = agent["last_capture_preview"] or ""
        if status == "dead":
            queue_action(conn, "restart_agent", {"task": current_task_prompt(conn, name)}, "Agent appears dead", name)
            continue
        if status == "stuck":
            queue_action(conn, "send_prompt", {"text": corrective_prompt(conn, name, preview)}, "Agent appears stuck", name)
            continue
        if status == "idle":
            follow_up = follow_up_prompt(conn, name)
            if follow_up:
                queue_action(conn, "send_prompt", {"text": follow_up}, "Idle follow-up", name)

    cross_pollinate(conn)
    queue_index_actions(conn)


def execute_action(conn: sqlite3.Connection, row: sqlite3.Row) -> None:
    payload = from_json(row["payload_json"], {})
    action_type = row["action_type"]
    agent_name = row["agent_name"]
    ok = False
    result = ""

    if action_type == "send_prompt":
        agent = conn.execute("SELECT * FROM agents WHERE name=?", (agent_name,)).fetchone()
        if not agent:
            ok, result = False, f"unknown agent: {agent_name}"
        else:
            ok, result = send_agent_text(agent, payload["text"])

    elif action_type == "restart_agent":
        agent = conn.execute("SELECT * FROM agents WHERE name=?", (agent_name,)).fetchone()
        if not agent:
            ok, result = False, f"unknown agent: {agent_name}"
        else:
            pane_id = agent["biome_pane_id"] if "biome_pane_id" in agent.keys() else None
            if pane_id:
                # Restart via biome_term: kill pane, create new one, launch agent
                _biome_request("DELETE", f"/panes/{pane_id}")
                new_pane = _biome_request("POST", "/panes", {"name": agent["name"], "cols": 220, "rows": 50})
                if new_pane and "id" in new_pane:
                    new_id = new_pane["id"]
                    conn.execute("UPDATE agents SET biome_pane_id=? WHERE name=?", (new_id, agent_name))
                    conn.commit()
                    workdir = agent["workdir"] or "~"
                    ok1, res1 = send_biome_text(new_id, f"cd {workdir} && claude --dangerously-skip-permissions")
                    time.sleep(5)
                    ok2, res2 = send_biome_text(new_id, payload.get("task") or current_task_prompt(conn, agent_name))
                    ok = ok1 and ok2
                    result = f"biome restart={res1}; task={res2}; new_pane={new_id}"
                else:
                    ok, result = False, "failed to create biome_term pane"
            else:
                ok1, res1 = send_tmux_text(agent["tmux_target"], "claude --dangerously-skip-permissions")
                time.sleep(5)
                ok2, res2 = send_tmux_text(agent["tmux_target"], payload.get("task") or current_task_prompt(conn, agent_name))
                ok = ok1 and ok2
                result = f"restart={res1}; task={res2}"

    elif action_type == "index_artifact":
        path = payload["path"]
        try:
            cp = run_cmd(["mcp", "openviking", "add_resource", path], timeout=60)
            ok = cp.returncode == 0
            result = (cp.stdout or cp.stderr or "").strip()
            if ok:
                conn.execute("UPDATE artifacts SET indexed_state='indexed' WHERE path=?", (path,))
        except (subprocess.TimeoutExpired, FileNotFoundError) as exc:
            ok = False
            result = str(exc)

    else:
        ok, result = False, f"unknown action type: {action_type}"

    conn.execute(
        "UPDATE actions SET status=?, executed_at=?, result_text=? WHERE id=?",
        ("done" if ok else "failed", utcnow(), result, row["id"]),
    )
    log_event(conn, agent_name, "action.executed", f"{action_type} -> {'ok' if ok else 'failed'}", {"result": result})
    conn.commit()


def execute_pending_actions(conn: sqlite3.Connection, limit: int = 10) -> None:
    rows = conn.execute("SELECT * FROM actions WHERE status='pending' ORDER BY id ASC LIMIT ?", (limit,)).fetchall()
    for row in rows:
        execute_action(conn, row)


def list_agents(conn: sqlite3.Connection) -> None:
    for row in conn.execute("SELECT * FROM agents ORDER BY name"):
        print(dict(row))


def list_goals(conn: sqlite3.Connection) -> None:
    for row in conn.execute("SELECT * FROM goals ORDER BY priority, goal_key"):
        print(dict(row))


def list_sub_goals(conn: sqlite3.Connection) -> None:
    for row in conn.execute("SELECT * FROM sub_goals ORDER BY goal_key, owner_agent, priority, sub_goal_key"):
        print(dict(row))


def list_facts(conn: sqlite3.Connection) -> None:
    for row in conn.execute("SELECT fact_key, value_json, confidence, source_type, source_ref, updated_at FROM facts ORDER BY fact_key"):
        print(dict(row))


def print_summary(conn: sqlite3.Connection) -> None:
    print("Agents")
    for row in conn.execute(
        "SELECT name, status, current_goal_key, current_sub_goal_key, last_seen_at, last_capture_preview FROM agents ORDER BY name"
    ):
        print(
            f"- {row['name']}: {row['status']} | goal={row['current_goal_key'] or '-'} | sub_goal={row['current_sub_goal_key'] or '-'} | seen={row['last_seen_at'] or '-'}"
        )
        if row["last_capture_preview"]:
            print(f"  tail: {row['last_capture_preview'].replace(chr(10), ' | ')[:240]}")

    print("\nGoals")
    for row in conn.execute("SELECT goal_key, status, priority, success_fact_key FROM goals ORDER BY priority, goal_key"):
        print(f"- {row['goal_key']}: {row['status']} prio={row['priority']} success={row['success_fact_key'] or '-'}")

    print("\nSub-goals")
    for row in conn.execute(
        "SELECT sub_goal_key, goal_key, owner_agent, status, priority FROM sub_goals ORDER BY goal_key, priority, sub_goal_key"
    ):
        print(
            f"- {row['sub_goal_key']}: {row['status']} | goal={row['goal_key']} | owner={row['owner_agent']} | prio={row['priority']}"
        )

    print("\nRecent actions")
    for row in conn.execute("SELECT id, agent_name, action_type, status, reason FROM actions ORDER BY id DESC LIMIT 10"):
        print(f"- #{row['id']} {row['action_type']} for {row['agent_name'] or 'system'}: {row['status']} | {row['reason'] or ''}")

    print("\nRecent facts")
    for row in conn.execute("SELECT fact_key, value_json, updated_at FROM facts ORDER BY updated_at DESC LIMIT 10"):
        print(f"- {row['fact_key']} = {row['value_json']} @ {row['updated_at']}")


def run_once(conn: sqlite3.Connection, project_dir: str, execute: bool) -> None:
    poll_agents(conn)
    scan_artifacts(conn, project_dir)
    decide_actions(conn)
    if execute:
        execute_pending_actions(conn)
    print_summary(conn)


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description="Simple SQLite-backed harness for the NMSS tmux agents")
    p.add_argument("--db", default="nmss_harness.db", help="SQLite database path")
    sub = p.add_subparsers(dest="cmd", required=True)

    sub.add_parser("init-db")
    sub.add_parser("seed-agents")
    sub.add_parser("bootstrap-known-goals")
    sub.add_parser("agents")
    sub.add_parser("goals")
    sub.add_parser("sub-goals")
    sub.add_parser("facts")

    poll = sub.add_parser("poll")
    poll.add_argument("--project-dir", default=PROJECT_DIR)

    decide = sub.add_parser("decide")
    decide.add_argument("--project-dir", default=PROJECT_DIR)

    execute = sub.add_parser("execute")
    execute.add_argument("--limit", type=int, default=10)

    once = sub.add_parser("run-once")
    once.add_argument("--project-dir", default=PROJECT_DIR)
    once.add_argument("--execute", action="store_true", help="Execute queued actions after deciding")

    fact = sub.add_parser("fact-set")
    fact.add_argument("key")
    fact.add_argument("value")
    fact.add_argument("--confidence", type=float, default=1.0)
    fact.add_argument("--source-type", default="manual")
    fact.add_argument("--source-ref")

    goal_add = sub.add_parser("goal-add")
    goal_add.add_argument("goal_key")
    goal_add.add_argument("title")
    goal_add.add_argument("--detail", default="")
    goal_add.add_argument("--status", default="pending", choices=["pending", "active", "blocked", "done", "cancelled", "paused"])
    goal_add.add_argument("--priority", type=int, default=50)
    goal_add.add_argument("--depends-on-goal-key")
    goal_add.add_argument("--success-fact-key")

    goal_update = sub.add_parser("goal-update")
    goal_update.add_argument("goal_key")
    goal_update.add_argument("--title")
    goal_update.add_argument("--detail")
    goal_update.add_argument("--status", choices=["pending", "active", "blocked", "done", "cancelled", "paused"])
    goal_update.add_argument("--priority", type=int)
    goal_update.add_argument("--depends-on-goal-key")
    goal_update.add_argument("--success-fact-key")
    goal_update.add_argument("--clear-depends", action="store_true")
    goal_update.add_argument("--clear-success-fact", action="store_true")

    goal_remove = sub.add_parser("goal-remove")
    goal_remove.add_argument("goal_key")
    goal_remove.add_argument("--delete", action="store_true", help="Hard delete instead of cancelling")
    goal_remove.add_argument("--cascade", action="store_true", help="Also delete child sub-goals when hard deleting")

    goal_set = sub.add_parser("goal-set")
    goal_set.add_argument("goal_key")
    goal_set.add_argument("status", choices=["pending", "active", "blocked", "done", "cancelled", "paused"])

    sub_goal_add = sub.add_parser("sub-goal-add")
    sub_goal_add.add_argument("sub_goal_key")
    sub_goal_add.add_argument("goal_key")
    sub_goal_add.add_argument("owner_agent")
    sub_goal_add.add_argument("title")
    sub_goal_add.add_argument("--detail", default="")
    sub_goal_add.add_argument("--status", default="pending", choices=["pending", "active", "blocked", "done", "cancelled", "paused"])
    sub_goal_add.add_argument("--priority", type=int, default=50)
    sub_goal_add.add_argument("--depends-on-sub-goal-key")
    sub_goal_add.add_argument("--success-fact-key")
    sub_goal_add.add_argument("--instruction-text")
    sub_goal_add.add_argument("--stuck-guidance-text")

    sub_goal_update = sub.add_parser("sub-goal-update")
    sub_goal_update.add_argument("sub_goal_key")
    sub_goal_update.add_argument("--goal-key")
    sub_goal_update.add_argument("--owner-agent")
    sub_goal_update.add_argument("--title")
    sub_goal_update.add_argument("--detail")
    sub_goal_update.add_argument("--status", choices=["pending", "active", "blocked", "done", "cancelled", "paused"])
    sub_goal_update.add_argument("--priority", type=int)
    sub_goal_update.add_argument("--depends-on-sub-goal-key")
    sub_goal_update.add_argument("--success-fact-key")
    sub_goal_update.add_argument("--instruction-text")
    sub_goal_update.add_argument("--stuck-guidance-text")
    sub_goal_update.add_argument("--clear-depends", action="store_true")
    sub_goal_update.add_argument("--clear-success-fact", action="store_true")
    sub_goal_update.add_argument("--clear-instruction", action="store_true")
    sub_goal_update.add_argument("--clear-stuck-guidance", action="store_true")

    sub_goal_remove = sub.add_parser("sub-goal-remove")
    sub_goal_remove.add_argument("sub_goal_key")
    sub_goal_remove.add_argument("--delete", action="store_true", help="Hard delete instead of cancelling")

    sub_goal_set = sub.add_parser("sub-goal-set")
    sub_goal_set.add_argument("sub_goal_key")
    sub_goal_set.add_argument("status", choices=["pending", "active", "blocked", "done", "cancelled", "paused"])

    agent_add = sub.add_parser("agent-add")
    agent_add.add_argument("name")
    agent_add.add_argument("--biome-pane-id", required=True, help="biome_term pane UUID")
    agent_add.add_argument("--workdir", default="")
    agent_add.add_argument("--default-task", default="Continue the current task.")
    agent_add.add_argument("--tmux-target", default="", help="Optional tmux target (legacy)")

    agent_remove = sub.add_parser("agent-remove")
    agent_remove.add_argument("name")
    agent_remove.add_argument("--delete", action="store_true", help="Hard delete instead of clearing fields")

    q = sub.add_parser("queue-prompt")
    q.add_argument("agent_name")
    q.add_argument("text")

    summary = sub.add_parser("summary")
    summary.add_argument("--verbose", action="store_true")

    return p


def main(argv: list[str]) -> int:
    args = build_parser().parse_args(argv)
    conn = connect(args.db)
    init_db(conn)

    try:
        if args.cmd == "init-db":
            print(f"initialized {args.db}")
            return 0
        if args.cmd == "seed-agents":
            seed_agents(conn)
            print("seeded agents")
            return 0
        if args.cmd == "bootstrap-known-goals":
            bootstrap_known_goals(conn)
            print("bootstrapped known orchestrator goals and sub-goals")
            return 0
        if args.cmd == "agents":
            list_agents(conn)
            return 0
        if args.cmd == "goals":
            list_goals(conn)
            return 0
        if args.cmd == "sub-goals":
            list_sub_goals(conn)
            return 0
        if args.cmd == "facts":
            list_facts(conn)
            return 0
        if args.cmd == "poll":
            seed_agents(conn)
            poll_agents(conn)
            scan_artifacts(conn, args.project_dir)
            resolve_active_sub_goals(conn)
            print_summary(conn)
            return 0
        if args.cmd == "decide":
            seed_agents(conn)
            poll_agents(conn)
            scan_artifacts(conn, args.project_dir)
            decide_actions(conn)
            print_summary(conn)
            return 0
        if args.cmd == "execute":
            execute_pending_actions(conn, args.limit)
            print_summary(conn)
            return 0
        if args.cmd == "run-once":
            run_once(conn, args.project_dir, args.execute)
            return 0
        if args.cmd == "fact-set":
            set_fact(conn, args.key, parse_json_value(args.value), args.confidence, args.source_type, args.source_ref)
            print(f"set {args.key}")
            return 0
        if args.cmd == "goal-add":
            upsert_goal(
                conn,
                goal_key=args.goal_key,
                title=args.title,
                detail=args.detail,
                status=args.status,
                priority=args.priority,
                depends_on_goal_key=args.depends_on_goal_key,
                success_fact_key=args.success_fact_key,
            )
            print(f"added {args.goal_key}")
            return 0
        if args.cmd == "goal-update":
            update_goal(
                conn,
                args.goal_key,
                title=args.title,
                detail=args.detail,
                status=args.status,
                priority=args.priority,
                depends_on_goal_key=args.depends_on_goal_key,
                success_fact_key=args.success_fact_key,
                clear_depends=args.clear_depends,
                clear_success_fact=args.clear_success_fact,
            )
            print(f"updated {args.goal_key}")
            return 0
        if args.cmd == "goal-remove":
            remove_goal(conn, args.goal_key, delete=args.delete, cascade=args.cascade)
            print(f"removed {args.goal_key}" if args.delete else f"cancelled {args.goal_key}")
            return 0
        if args.cmd == "goal-set":
            update_goal(conn, args.goal_key, status=args.status)
            print(f"updated {args.goal_key} -> {args.status}")
            return 0
        if args.cmd == "sub-goal-add":
            upsert_sub_goal(
                conn,
                sub_goal_key=args.sub_goal_key,
                goal_key=args.goal_key,
                owner_agent=args.owner_agent,
                title=args.title,
                detail=args.detail,
                status=args.status,
                priority=args.priority,
                depends_on_sub_goal_key=args.depends_on_sub_goal_key,
                success_fact_key=args.success_fact_key,
                instruction_text=args.instruction_text,
                stuck_guidance_text=args.stuck_guidance_text,
            )
            print(f"added {args.sub_goal_key}")
            return 0
        if args.cmd == "sub-goal-update":
            update_sub_goal(
                conn,
                args.sub_goal_key,
                goal_key=args.goal_key,
                owner_agent=args.owner_agent,
                title=args.title,
                detail=args.detail,
                status=args.status,
                priority=args.priority,
                depends_on_sub_goal_key=args.depends_on_sub_goal_key,
                success_fact_key=args.success_fact_key,
                instruction_text=args.instruction_text,
                stuck_guidance_text=args.stuck_guidance_text,
                clear_depends=args.clear_depends,
                clear_success_fact=args.clear_success_fact,
                clear_instruction=args.clear_instruction,
                clear_stuck_guidance=args.clear_stuck_guidance,
            )
            print(f"updated {args.sub_goal_key}")
            return 0
        if args.cmd == "sub-goal-remove":
            remove_sub_goal(conn, args.sub_goal_key, delete=args.delete)
            print(f"removed {args.sub_goal_key}" if args.delete else f"cancelled {args.sub_goal_key}")
            return 0
        if args.cmd == "sub-goal-set":
            update_sub_goal(conn, args.sub_goal_key, status=args.status)
            print(f"updated {args.sub_goal_key} -> {args.status}")
            return 0
        if args.cmd == "agent-add":
            upsert_agent(
                conn,
                name=args.name,
                biome_pane_id=args.biome_pane_id,
                workdir=args.workdir,
                default_task=args.default_task,
                tmux_target=args.tmux_target,
            )
            print(f"registered agent {args.name} (biome_pane_id={args.biome_pane_id})")
            return 0
        if args.cmd == "agent-remove":
            remove_agent(conn, args.name, delete=args.delete)
            print(f"{'deleted' if args.delete else 'deregistered'} agent {args.name}")
            return 0
        if args.cmd == "queue-prompt":
            queue_action(conn, "send_prompt", {"text": args.text}, "manual queue", args.agent_name)
            print("queued")
            return 0
        if args.cmd == "summary":
            print_summary(conn)
            if args.verbose:
                print("\nLast 20 events")
                for row in conn.execute("SELECT created_at, agent_name, event_type, message FROM events ORDER BY id DESC LIMIT 20"):
                    print(f"- {row['created_at']} {row['agent_name'] or 'system'} {row['event_type']}: {row['message']}")
            return 0
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 2

    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

