"""thin wrapper around scripts/psql.sh for the research pipeline (stdlib only).

- queries return python lists of dicts: the SQL is wrapped in `json_agg` so no delimiter
  parsing is needed (values may contain '|' or newlines).
- writes are fed on stdin (one transaction per call when the caller wraps BEGIN/COMMIT);
  values are rendered with `lit()` (single quotes doubled), never interpolated raw.
- `.env` is parsed here for the API-key environment of subprocesses; nothing is printed.
"""
from __future__ import annotations

import datetime as dt
import json
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PSQL = ROOT / "scripts" / "psql.sh"


class DbError(RuntimeError):
    pass


class Raw(str):
    """a SQL expression that lit() inserts verbatim (e.g. Raw("now()"))."""


def read_env() -> dict:
    """KEY=VALUE pairs from .env (quotes stripped). never log the result."""
    env = {}
    p = ROOT / ".env"
    if not p.exists():
        return env
    for line in p.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        if line.startswith("export "):
            line = line[len("export "):]
        k, v = line.split("=", 1)
        v = v.strip()
        if len(v) >= 2 and v[0] == v[-1] and v[0] in "\"'":
            v = v[1:-1]
        env[k.strip()] = v
    return env


def subprocess_env() -> dict:
    env = dict(os.environ)
    env.update(read_env())
    return env


def lit(v) -> str:
    """render a python value as a SQL literal."""
    if v is None:
        return "NULL"
    if isinstance(v, Raw):
        return str(v)
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        if isinstance(v, float) and (v != v or v in (float("inf"), float("-inf"))):
            return "NULL"
        return repr(v)
    if isinstance(v, (dt.datetime, dt.date)):
        return "'" + v.isoformat() + "'"
    if isinstance(v, (list, tuple)) and all(isinstance(x, str) for x in v):
        return "ARRAY[" + ",".join(lit(x) for x in v) + "]::text[]"
    if isinstance(v, (dict, list, tuple)):
        return jsonlit(v)
    return "'" + str(v).replace("'", "''") + "'"


def jsonlit(v) -> str:
    return "'" + json.dumps(v, separators=(",", ":"), default=str).replace("'", "''") + "'::jsonb"


def _run(sql: str, args: list[str]) -> str:
    r = subprocess.run(
        [str(PSQL), *args],
        input=sql,
        capture_output=True,
        text=True,
        cwd=str(ROOT),
    )
    if r.returncode != 0:
        err = (r.stderr or r.stdout or "").strip()
        raise DbError(f"psql failed (exit {r.returncode}): {err[-2000:]}\n--- sql ---\n{sql[:2000]}")
    return r.stdout


def execute(sql: str) -> str:
    """run one or more statements; returns raw psql stdout (unaligned, tuples only)."""
    return _run(sql, ["-At", "-q"])


def query(sql: str) -> list[dict]:
    """run a SELECT and return rows as dicts (json roundtrip; timestamps come back as strings)."""
    wrapped = f"SELECT coalesce(jsonb_agg(t), '[]'::jsonb)::text FROM ({sql.rstrip().rstrip(';')}) t;"
    out = _run(wrapped, ["-At", "-q"]).strip()
    return json.loads(out or "[]")


def query_one(sql: str) -> dict | None:
    rows = query(sql)
    return rows[0] if rows else None


def scalar(sql: str):
    row = query_one(sql)
    if not row:
        return None
    return next(iter(row.values()))


def parse_ts(s: str | None) -> dt.datetime | None:
    if not s:
        return None
    s = s.replace("Z", "+00:00")
    try:
        return dt.datetime.fromisoformat(s)
    except ValueError:
        # postgres json emits e.g. 2026-09-27T05:46:39.498561+00:00 already; fall back to date
        return dt.datetime.fromisoformat(s[:19]).replace(tzinfo=dt.timezone.utc)
