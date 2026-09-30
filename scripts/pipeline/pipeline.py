#!/usr/bin/env python3
"""research pipeline runner: candidate edges and tickers move proposed → backtest gate → shadow trial →
evaluation gate → promotion proposal → human promotes (docs/plans/2026-09-26_pipeline_and_books.md §4, §9).

  pipeline.py propose --name NAME --kind ticker --ticker T [--gate default-ticker] [--notes ...] [--source human]
  pipeline.py propose --name NAME --kind config --patch FILE.json [--patch FILE2.json] [--tickers A,B]
                      [--gate volume-config|quality-config|stress-mode] [--base ROW] [--notes ...]
  pipeline.py list [--stage S]
  pipeline.py show NAME
  pipeline.py advance [--max-backtests N] [--dry-run]      # the nightly entry point
  pipeline.py backtest NAME | evaluate NAME [--force] | withdraw NAME [--reason R] | retire-shadow NAME [--reason R]
  pipeline.py regate NAME --gate GATE                       # re-gate an existing backtest_passed/failed candidate,
                                                             # reusing its sweep + materialized row; no new sweep
  pipeline.py report [--markdown]                           # regenerates docs/pipeline/status.md

python3 stdlib only; the database is reached through scripts/psql.sh. never prints API keys.
switches (environment): PIPELINE_SHADOW_BOOKS=1 lets `advance` create shadow `books` rows (default: print only);
PIPELINE_ACTOR overrides the actor recorded in pipeline_events; PIPELINE_VERIFY_DATES = comma-separated dates
for the materialize-vs-patch check.
"""
from __future__ import annotations

import argparse
import copy
import csv
import datetime as dt
import fcntl
import hashlib
import io
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from zoneinfo import ZoneInfo

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import db  # noqa: E402
import gates  # noqa: E402
import materialize  # noqa: E402
import metrics  # noqa: E402
import report  # noqa: E402
import shadow  # noqa: E402

ROOT = HERE.parents[1]
DATA = ROOT / "data"
BARS_DIR = DATA / "bars_iex"
LOGS = ROOT / "logs"
BACKTEST_BIN = ROOT / "target" / "release" / "backtest"
SWEEP_SH = ROOT / "scripts" / "run_cached_sweep.sh"
NOTIFY_SH = ROOT / "scripts" / "notify.sh"
REPLAY_BOOK_SH = ROOT / "scripts" / "replay_book.sh"
RESEARCH_LOCK = LOGS / ".research.lock"
PIPELINE_LOCK = LOGS / ".pipeline.lock"
PARITY_DIR = DATA / "pipeline" / "parity"
ET = ZoneInfo("America/New_York")

SIZING_ARGS = ["--sizing-fraction", "0.36", "--max-position-pct", "0.36"]   # research sizing (gate sweeps only)
CROSS_ARGS = ["--cross-index", "SPY"]
COST_ARGS = ["--slippage-bps", "3.0", "--half-spread", "0.005"]
GATE_SWEEP_ARGS = SIZING_ARGS + CROSS_ARGS
FETCH_START = "2022-01-01"
COOLDOWN_DAYS = 180
MAX_NEW_TICKER_FETCHES = 2
COVERAGE_RETRY_DAYS = 7
TERMINAL = {"backtest_failed", "shadow_failed", "rejected", "withdrawn", "promoted"}
SHADOW_BOOKS_ENABLED = os.environ.get("PIPELINE_SHADOW_BOOKS", "0") == "1"
VERIFY_DATES = [d for d in os.environ.get("PIPELINE_VERIFY_DATES", "2025-04-04,2025-05-12,2026-08-12").split(",") if d]


class PipelineError(RuntimeError):
    pass


STEP_ERRORS = (PipelineError, db.DbError, materialize.PatchError, OSError)


def log(msg: str) -> None:
    print(f"[pipeline] {msg}", flush=True)


def notify(level: str, msg: str) -> None:
    try:
        subprocess.run([str(NOTIFY_SH), level, msg], capture_output=True, text=True, timeout=30)
    except Exception as e:  # never fail a transition on a notification
        log(f"notify failed: {e}")


def today_et() -> dt.date:
    return dt.datetime.now(ET).date()


def actor_for(default: str) -> str:
    return os.environ.get("PIPELINE_ACTOR") or default


class Lock:
    """flock on a file; wait_s=0 means fail fast."""

    def __init__(self, path: Path, wait_s: float, what: str):
        self.path, self.wait_s, self.what, self.fh = path, wait_s, what, None

    def __enter__(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.fh = open(self.path, "a+")
        deadline = time.monotonic() + self.wait_s
        while True:
            try:
                fcntl.flock(self.fh, fcntl.LOCK_EX | fcntl.LOCK_NB)
                return self
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    self.fh.close()
                    raise PipelineError(f"could not take the {self.what} lock ({self.path}) within {self.wait_s:.0f} s")
                time.sleep(15)

    def __exit__(self, *exc):
        try:
            fcntl.flock(self.fh, fcntl.LOCK_UN)
        finally:
            self.fh.close()


# ------------------------------------------------------------------ database access

CAND_COLS = ("id, name, kind, base_config_version_id, patch, tickers, stage, gate, materialized_config_id, backtest_tag, "
             "backtest_result, backtest_at, shadow_book, shadow_started_at, shadow_min_sessions, shadow_min_trades, "
             "shadow_result, shadow_evaluated_at, proposed_config_version_id, cooldown_until, source, notes, created_at, updated_at")


def candidates(stage: str | None = None, active_only: bool = False) -> list[dict]:
    where = []
    if stage:
        where.append(f"stage = {db.lit(stage)}")
    if active_only:
        where.append("stage NOT IN (" + ",".join(db.lit(s) for s in sorted(TERMINAL)) + ")")
    sql = f"SELECT {CAND_COLS} FROM pipeline_candidates" + (" WHERE " + " AND ".join(where) if where else "") + " ORDER BY id"
    return db.query(sql)


def get_candidate(ref: str, active_only: bool = False) -> dict:
    if ref.isdigit():
        rows = db.query(f"SELECT {CAND_COLS} FROM pipeline_candidates WHERE id = {int(ref)}")
    else:
        rows = db.query(f"SELECT {CAND_COLS} FROM pipeline_candidates WHERE name = {db.lit(ref)} ORDER BY id DESC")
    active = [r for r in rows if r["stage"] not in TERMINAL]
    if active:
        return active[0]
    if active_only or not rows:
        raise PipelineError(f"no {'active ' if active_only else ''}candidate '{ref}'")
    return rows[0]


def events(candidate_id: int | None = None, limit: int = 20) -> list[dict]:
    where = f"WHERE e.candidate_id = {int(candidate_id)}" if candidate_id else ""
    return db.query(f"SELECT e.id, e.candidate_id, c.name, e.ts, e.from_stage, e.to_stage, e.actor, e.detail "
                    f"FROM pipeline_events e JOIN pipeline_candidates c ON c.id = e.candidate_id {where} "
                    f"ORDER BY e.ts DESC, e.id DESC LIMIT {int(limit)}")


def promoted_row() -> dict:
    row = db.query_one("SELECT id, config_blob, promoted_at FROM config_versions WHERE status = 'promoted' ORDER BY promoted_at DESC NULLS LAST, id DESC LIMIT 1")
    if not row:
        raise PipelineError("no promoted config_versions row")
    return row


def config_status(config_version_id: int) -> str | None:
    row = db.query_one(f"SELECT status FROM config_versions WHERE id = {int(config_version_id)}")
    return row["status"] if row else None


def _set_clause(sets: dict | None) -> str:
    if not sets:
        return ""
    return "".join(f", {k} = {db.lit(v)}" for k, v in sets.items())


def transition_sql(cand: dict, to_stage: str, actor: str, detail: dict, sets: dict | None = None) -> str:
    """UPDATE guarded by the current stage + event insert, as one statement (atomic, idempotent)."""
    return f"""WITH upd AS (
  UPDATE pipeline_candidates SET stage = {db.lit(to_stage)}, updated_at = now(){_set_clause(sets)}
  WHERE id = {int(cand['id'])} AND stage = {db.lit(cand['stage'])} RETURNING id
)
INSERT INTO pipeline_events (candidate_id, from_stage, to_stage, actor, detail)
SELECT id, {db.lit(cand['stage'])}, {db.lit(to_stage)}, {db.lit(actor)}, {db.jsonlit(detail)} FROM upd RETURNING candidate_id;"""


def transition(cand: dict, to_stage: str, actor: str, detail: dict, sets: dict | None = None, extra_sql: str = "") -> None:
    sql = "BEGIN;\n" + transition_sql(cand, to_stage, actor, detail, sets) + "\n" + extra_sql + "\nCOMMIT;"
    out = db.execute(sql)
    if str(cand["id"]) not in out.split():
        raise PipelineError(f"stale stage: candidate #{cand['id']} is no longer '{cand['stage']}' (transition to {to_stage} skipped)")
    why = detail.get("event") or detail.get("reason") or ""
    log(f"#{cand['id']} {cand['name']}: {cand['stage']} → {to_stage}" + (f" ({why})" if why else ""))
    cand["stage"] = to_stage
    if sets:
        for k, v in sets.items():
            if not isinstance(v, db.Raw):
                cand[k] = v


def event_only(cand: dict, actor: str, detail: dict, sets: dict | None = None) -> None:
    sql = f"""WITH upd AS (
  UPDATE pipeline_candidates SET updated_at = now(){_set_clause(sets)} WHERE id = {int(cand['id'])} RETURNING id, stage
)
INSERT INTO pipeline_events (candidate_id, from_stage, to_stage, actor, detail)
SELECT id, stage, stage, {db.lit(actor)}, {db.jsonlit(detail)} FROM upd;"""
    db.execute(sql)
    if sets:
        for k, v in sets.items():
            if not isinstance(v, db.Raw):
                cand[k] = v


def last_event(cand: dict, event_name: str) -> dict | None:
    rows = db.query(f"SELECT ts, detail FROM pipeline_events WHERE candidate_id = {int(cand['id'])} "
                    f"AND detail->>'event' = {db.lit(event_name)} ORDER BY ts DESC LIMIT 1")
    return rows[0] if rows else None


# ------------------------------------------------------------------ subprocess helpers

def run_backtest_day(date: str, extra: list[str], sizing: bool, capital: float = 10000.0) -> tuple[list[str], str, str]:
    """one `backtest --date` run on the IEX cache; returns (trade rows, stdout, stderr)."""
    cmd = [str(BACKTEST_BIN), "--date", date, "--lookback-days", "8", "--capital", f"{capital:g}", *COST_ARGS,
           "--output-trades-csv", "--bars-dir", str(BARS_DIR), *CROSS_ARGS, *(SIZING_ARGS if sizing else []), *extra]
    r = subprocess.run(cmd, env=db.subprocess_env(), capture_output=True, text=True, cwd=str(ROOT))
    if r.returncode != 0:
        raise PipelineError(f"backtest --date {date} failed (exit {r.returncode}): {r.stderr.strip()[-500:]}")
    trades = [l for l in r.stdout.splitlines() if l.startswith("trade,")]
    return trades, r.stdout, r.stderr


def parse_trade_rows(text: str) -> list[dict]:
    rows = []
    for r in csv.DictReader(io.StringIO(text)):
        if r.get("row_type") != "trade":
            continue
        try:
            rows.append({"date": r["date"], "ticker": r["ticker"], "pnl": float(r["pnl"]), "entry_price": float(r["entry_price"]),
                         "size": float(r["size"]), "entry_time": r["entry_time"], "direction": r["direction"]})
        except (KeyError, ValueError):
            continue
    return rows


def run_sweep(tag: str, extra: list[str]) -> list[str]:
    """five-year sweep under the research lock; verifies every year finished."""
    if not BACKTEST_BIN.exists():
        raise PipelineError(f"{BACKTEST_BIN} missing (cargo build --release -p backtest)")
    env = db.subprocess_env()
    env["BARS_DIR"] = str(BARS_DIR)
    cmd = [str(SWEEP_SH), tag, *GATE_SWEEP_ARGS, *extra]
    log(f"sweep {tag}: {' '.join(GATE_SWEEP_ARGS + extra)} (waiting for {RESEARCH_LOCK.name} if busy)")
    t0 = time.monotonic()
    with Lock(RESEARCH_LOCK, 14400, "research"):
        r = subprocess.run(cmd, env=env, capture_output=True, text=True, cwd=str(ROOT))
    minutes = (time.monotonic() - t0) / 60
    problems, tails = [], []
    for y in metrics.YEARS:
        logf = LOGS / "sweeps" / f"{tag}_{y}.log"
        tail = ""
        try:
            tail = logf.read_text().strip().splitlines()[-1] if logf.exists() else ""
        except OSError:
            pass
        tails.append(tail)
        m = re.search(r"done: (\d+) days, (\d+) skipped", tail)
        if not m:
            problems.append(f"{y}: sweep did not finish ({tail[:120] or 'no log'})")
            continue
        days, skipped = int(m.group(1)), int(m.group(2))
        if days == 0:
            problems.append(f"{y}: 0 days replayed")
        elif skipped > 30:
            first = ""
            try:
                first = next((l for l in logf.read_text().splitlines() if "skipped:" in l), "")
            except OSError:
                pass
            problems.append(f"{y}: {skipped} days skipped ({first[:160]})")
    if not metrics.sweep_files_present(tag):
        problems.append("trade csvs missing for some years")
    if problems:
        raise PipelineError(f"sweep {tag} incomplete after {minutes:.1f} min: " + "; ".join(problems))
    log(f"sweep {tag} done in {minutes:.1f} min: " + " | ".join(t.split("] ", 1)[-1] for t in tails))
    return tails


def fetch_bars(tickers: list[str], start: str, end: str) -> None:
    """sequential IEX fetches under the research lock; retries once on exit 3 (a chunk failed)."""
    env = db.subprocess_env()
    if not env.get("APCA_API_KEY_ID") or not env.get("APCA_API_SECRET_KEY"):
        raise PipelineError("alpaca keys missing from .env; cannot fetch bars")
    with Lock(RESEARCH_LOCK, 14400, "research"):
        for t in tickers:
            for attempt in (1, 2):
                log(f"fetch-bars {t} {start}..{end} (iex) attempt {attempt}")
                cmd = [str(BACKTEST_BIN), "--fetch-bars", str(BARS_DIR), "--start", start, "--end", end, "--tickers", t, "--feed", "iex"]
                r = subprocess.run(cmd, env=env, capture_output=True, text=True, cwd=str(ROOT))
                lines = [l for l in r.stderr.splitlines() if l.startswith("[fetch-bars]")]
                for l in lines[-3:]:
                    log("  " + l)
                if r.returncode == 0:
                    break
                if r.returncode == 3 and attempt == 1:
                    log(f"  {t}: some chunks failed; retrying once")
                    time.sleep(5)
                    continue
                raise PipelineError(f"fetch-bars {t} failed (exit {r.returncode}): {r.stderr.strip()[-300:]}")


def yesterday_et() -> str:
    return (today_et() - dt.timedelta(days=1)).isoformat()


# ------------------------------------------------------------------ propose

def cmd_propose(a) -> int:
    actor = actor_for("human")
    kind = a.kind
    gate = gates.resolve(a.gate or "default", kind)
    if a.gate == "default-config":
        log("gate 'default-config' is now 'volume-config' (plan §9)")
    if kind == "config" and gate == "default-ticker":
        raise PipelineError("default-ticker is a ticker gate; config candidates use volume-config | quality-config | stress-mode")
    prom = promoted_row()
    base_id = int(a.base) if a.base else int(prom["id"])
    base_blob = prom["config_blob"] if base_id == prom["id"] else materialize.load_blob(base_id)
    if kind == "ticker":
        if not a.ticker:
            raise PipelineError("--ticker T is required for --kind ticker")
        t = a.ticker.strip().upper()
        name = a.name or f"ticker:{t}"
        if t in base_blob.get("tickers", []):
            raise PipelineError(f"{t} is already in config row {base_id}'s tickers")
        patch, tickers = {"ticker": t}, [t]
    else:
        if not a.name:
            raise PipelineError("--name is required for --kind config")
        if not a.patch:
            raise PipelineError("--patch FILE.json is required for --kind config")
        name = a.name
        loaded = []
        for f in a.patch:
            with open(f) as fh:
                p = json.load(fh)
            problems = materialize.validate_patch(p)
            if problems:
                raise PipelineError(f"{f}: " + "; ".join(problems))
            loaded.append(p)
        patch = materialize.merge_patches(*loaded)
        tickers = [x.strip().upper() for x in a.tickers.split(",") if x.strip()] if a.tickers else None
        if tickers:
            patch["tickers"] = tickers
        known = {x.get("instance_id") for x in base_blob.get("actions", [])}
        for d in patch.get("disable", []):
            if d not in known:
                log(f"warning: disable '{d}' matches no action in row {base_id}")
        materialize.apply_patch(base_blob, patch)  # raises on a malformed patch
        for key in ("indicators", "actions"):
            ids = [x.get("instance_id") for x in patch.get(key, [])]
            dup = sorted({i for i in ids if ids.count(i) > 1 or i in {x.get("instance_id") for x in base_blob.get(key, [])}})
            if dup:
                log(f"warning: {key} instance ids already present (appended, not replaced): {', '.join(dup)}")
    active = db.query(f"SELECT id, stage FROM pipeline_candidates WHERE name = {db.lit(name)} AND stage NOT IN ({','.join(db.lit(s) for s in sorted(TERMINAL))})")
    if active:
        raise PipelineError(f"'{name}' is already active (#{active[0]['id']}, {active[0]['stage']})")
    last = db.query_one(f"SELECT id, stage, cooldown_until FROM pipeline_candidates WHERE name = {db.lit(name)} ORDER BY id DESC LIMIT 1")
    if last and last.get("cooldown_until") and dt.date.fromisoformat(last["cooldown_until"]) >= today_et() and not a.force:
        raise PipelineError(f"'{name}' is in cooldown until {last['cooldown_until']} (#{last['id']} {last['stage']}); --force to override")
    detail = {"event": "proposed", "kind": kind, "gate": gate, "base_config_version_id": base_id, "tickers": tickers,
              "patch_files": list(a.patch or []), "source": a.source}
    sql = f"""WITH ins AS (
  INSERT INTO pipeline_candidates (name, kind, base_config_version_id, patch, tickers, gate, source, notes)
  VALUES ({db.lit(name)}, {db.lit(kind)}, {base_id}, {db.jsonlit(patch)}, {db.lit(tickers)}, {db.lit(gate)}, {db.lit(a.source)}, {db.lit(a.notes)})
  RETURNING id, stage
)
INSERT INTO pipeline_events (candidate_id, from_stage, to_stage, actor, detail)
SELECT id, NULL, stage, {db.lit(actor)}, {db.jsonlit(detail)} FROM ins RETURNING candidate_id;"""
    out = db.execute(sql).strip().splitlines()
    cid = int([l for l in out if l.strip().isdigit()][-1])
    log(f"proposed #{cid} '{name}' kind={kind} gate={gate} base=row {base_id} tickers={tickers or 'base'}")
    print(cid)
    return 0


# ------------------------------------------------------------------ backtest transition

def candidate_tickers(cand: dict, prom: dict) -> list[str]:
    if cand["kind"] == "ticker":
        return list(cand["tickers"])
    if cand.get("tickers"):
        return list(cand["tickers"])
    patch = cand.get("patch") or {}
    if patch.get("tickers"):
        return list(patch["tickers"])
    base_id = cand.get("base_config_version_id") or prom["id"]
    blob = prom["config_blob"] if base_id == prom["id"] else materialize.load_blob(base_id)
    return list(blob["tickers"])


def baseline_for(base_id: int, tickers: list[str], prom: dict) -> tuple[str, list[str] | None]:
    """(tag, sweep args to produce it or None when the research baseline already covers it)."""
    if base_id == prom["id"] and sorted(tickers) == sorted(prom["config_blob"]["tickers"]) and metrics.sweep_files_present("iex_v18"):
        args_file = DATA / "iex_v18.args"
        if not args_file.exists() or set(args_file.read_text().split()) == set(GATE_SWEEP_ARGS):
            return "iex_v18", None
    key = hashlib.sha1(f"{base_id}|{','.join(sorted(tickers))}|{' '.join(GATE_SWEEP_ARGS)}".encode()).hexdigest()[:8]
    return f"base_{key}", ["--config-id", str(base_id), "--tickers", ",".join(tickers)]


def ensure_sweep(tag: str, extra: list[str]) -> None:
    if metrics.sweep_files_present(tag):
        args_file = DATA / f"{tag}.args"
        if args_file.exists() and set(args_file.read_text().split()) == set(GATE_SWEEP_ARGS + extra):
            log(f"sweep {tag} already on disk")
            return
    run_sweep(tag, extra)


def verify_dates(tag: str, k: int = 3) -> list[str]:
    """the k days on which the candidate's own sweep traded most (a non-vacuous check), else the defaults."""
    counts = {}
    for r in metrics.load_trades(tag):
        counts[r["date"]] = counts.get(r["date"], 0) + 1
    if not counts:
        return VERIFY_DATES
    best = sorted(counts, key=lambda d: (-counts[d], d))[:k]
    return sorted(best)


def verify_materialization(cand: dict, base_id: int, mid: int) -> dict:
    """one-day replays: --config-id <materialized> vs --config-id <base> --patch-json <merged patch>; trade rows must match."""
    patch = dict(cand.get("patch") or {})
    if cand.get("tickers") and "tickers" not in patch:
        patch["tickers"] = list(cand["tickers"])
    scratch = LOGS / "pipeline"
    scratch.mkdir(parents=True, exist_ok=True)
    pfile = scratch / f"cand_{cand['id']}_patch.json"
    pfile.write_text(json.dumps(patch))
    out = {"dates": {}, "ok": True}
    dates = verify_dates(f"cand_{cand['id']}")
    for d in dates:
        try:
            a, _, _ = run_backtest_day(d, ["--config-id", str(mid)], sizing=True)
            b, _, _ = run_backtest_day(d, ["--config-id", str(base_id), "--patch-json", str(pfile)], sizing=True)
        except PipelineError as e:
            out["dates"][d] = {"error": str(e)[:300]}
            out["ok"] = False
            continue
        same = sorted(a) == sorted(b)
        out["dates"][d] = {"trades_materialized": len(a), "trades_patch": len(b), "identical": same}
        out["ok"] = out["ok"] and same
    log(f"materialize check row {mid} vs row {base_id}+patch: " + ", ".join(f"{d}: {v.get('trades_materialized', '?')} trades {'=' if v.get('identical') else '≠'}" for d, v in out["dates"].items()))
    return out


def run_backtest(cand: dict, actor: str = "pipeline", budget: list[int] | None = None, dry_run: bool = False) -> str:
    """proposed|backtesting → backtest_passed|backtest_failed. resumable at every step. returns the outcome."""
    budget = budget if budget is not None else [MAX_NEW_TICKER_FETCHES]
    prom = promoted_row()
    cid = cand["id"]
    if cand["stage"] == "proposed":
        if dry_run:
            log(f"[dry-run] #{cid} {cand['name']}: would move proposed → backtesting")
        else:
            transition(cand, "backtesting", actor, {"event": "backtest_start"})
    elif cand["stage"] == "backtesting":
        log(f"#{cid} {cand['name']}: resuming the backtest step (materialized={cand.get('materialized_config_id')}, tag={cand.get('backtest_tag')})")
        if not dry_run:
            event_only(cand, actor, {"event": "backtest_resume"})
    else:
        raise PipelineError(f"#{cid} is in stage '{cand['stage']}', not proposed/backtesting")
    base_id = int(cand.get("base_config_version_id") or prom["id"])
    tickers = candidate_tickers(cand, prom)
    mid = None
    if cand["kind"] == "config":
        mid = cand.get("materialized_config_id")
        if not mid:
            if dry_run:
                log(f"[dry-run] #{cid}: would materialize base row {base_id} + patch into a new config_versions row (status backtesting)")
            else:
                mid = materialize.materialize(cand, actor)
                cand["materialized_config_id"] = mid
                log(f"#{cid} {cand['name']}: materialized config_versions row {mid} (base {base_id})")
        sweep_extra = ["--config-id", str(mid)] if mid else ["--config-id", "<materialized>"]
    else:
        sweep_extra = ["--tickers", ",".join(tickers)]

    # bar coverage for every ticker the candidate trades (SPY is the cross index and is always present)
    coverage = {}
    for t in tickers:
        cov = metrics.coverage(t)
        if not cov["ok"]:
            new_fetch = not cov["exists"]
            if new_fetch and budget[0] <= 0:
                msg = f"#{cid} {cand['name']}: {t} needs a full bar fetch but this run's budget of {MAX_NEW_TICKER_FETCHES} new tickers is spent; deferred"
                log(msg)
                if not dry_run:
                    event_only(cand, actor, {"event": "deferred", "reason": "fetch budget", "ticker": t})
                return "deferred"
            if dry_run:
                log(f"[dry-run] #{cid}: would fetch {t} bars {FETCH_START}..{yesterday_et()} ({'; '.join(cov['problems'])})")
                continue
            fetch_bars([t], FETCH_START, yesterday_et())
            if new_fetch:
                budget[0] -= 1
            cov = metrics.coverage(t)
            if not cov["ok"]:
                detail = "; ".join(cov["problems"])
                log(f"#{cid} {cand['name']}: {t} coverage still bad after fetch: {detail}; refusing to gate")
                result = {"blocked": "coverage", "blocked_detail": f"{t}: {detail}", "coverage": {t: cov}, "checked_at": dt.datetime.now(dt.timezone.utc).isoformat()}
                event_only(cand, actor, {"event": "coverage_hole", "ticker": t, "why": detail}, sets={"backtest_result": result})
                notify("warning", f"pipeline: {cand['name']} bar cache for {t} has holes ({detail}); gate refused, retry in {COVERAGE_RETRY_DAYS} d")
                return "blocked"
        coverage[t] = {k: cov[k] for k in ("sessions", "spy_sessions", "missing", "max_gap", "first", "last") if k in cov}
    if dry_run:
        log(f"[dry-run] #{cid}: would sweep tag cand_{cid} with {' '.join(GATE_SWEEP_ARGS + sweep_extra)} then gate '{cand['gate']}'")
        return "dry-run"

    tag = f"cand_{cid}"
    if not (cand.get("backtest_tag") == tag and metrics.sweep_files_present(tag)):
        run_sweep(tag, sweep_extra)
        event_only(cand, actor, {"event": "sweep_done", "tag": tag}, sets={"backtest_tag": tag})
    else:
        log(f"#{cid}: sweep {tag} already complete; reusing")

    mcheck = None
    if cand["kind"] == "config":
        mcheck = verify_materialization(cand, base_id, mid)

    gate_name = gates.resolve(cand["gate"], cand["kind"])
    baseline_tag, baseline = None, None
    if gate_name in gates.NEEDS_BASELINE:
        baseline_tag, bargs = baseline_for(base_id, tickers, prom)
        if bargs:
            ensure_sweep(baseline_tag, bargs)
        baseline = metrics.full_metrics(baseline_tag)
    m = metrics.full_metrics(tag, tickers, baseline_tag=baseline_tag if gate_name in gates.NEEDS_MARGINAL else None)
    g = gates.run_gate(gate_name, m, baseline)
    if mcheck and not mcheck["ok"]:
        g["checks"].append(gates.check("materialize_equals_patch", False, "identical trade rows on the verify days", False))
        g["pass"] = False
    passed = g["pass"]
    result = {
        "gate": gate_name, "pass": passed, "gate_result": g, "metrics": m,
        "baseline": {"tag": baseline_tag, "metrics": baseline} if baseline_tag else None,
        "sweep": {"tag": tag, "args": GATE_SWEEP_ARGS + sweep_extra, "bars_dir": str(BARS_DIR.relative_to(ROOT)), "sweep_end": metrics.sweep_end().isoformat()},
        "sizing_note": "gate sweep at --sizing-fraction 0.36 --max-position-pct 0.36; live/shadow size at the blob's fraction",
        "coverage": coverage, "materialize_check": mcheck,
        "computed_at": dt.datetime.now(dt.timezone.utc).isoformat(),
    }
    to = "backtest_passed" if passed else "backtest_failed"
    sets = {"backtest_result": result, "backtest_at": db.Raw("now()")}
    if not passed:
        sets["cooldown_until"] = db.Raw(f"current_date + {COOLDOWN_DAYS}")
    extra_sql = ""
    if mid:
        extra_sql = f"UPDATE config_versions SET status = {db.lit('validated' if passed else 'rejected')} WHERE id = {int(mid)} AND status = 'backtesting';"
    failed = gates.failed_names(g)
    transition(cand, to, actor, {"event": "gate", "gate": gate_name, "pass": passed, "tag": tag, "failed": failed,
                                 "pnl": m["pnl"], "n": m["n"], "pf": m["pf"]}, sets=sets, extra_sql=extra_sql)
    summary = f"{m['pnl']:+.0f} / {m['n']} trades / PF {m['pf']:.2f}, years " + " ".join(f"{y[2:]}:{m['years'][y]['pnl']:+.0f}" for y in metrics.YEARS)
    log(f"#{cid} {cand['name']}: gate {gate_name} {'PASS' if passed else 'FAIL ' + ','.join(failed)} — {summary}")
    notify("info", f"pipeline: {cand['name']} backtest gate {gate_name} {'passed' if passed else 'failed (' + ', '.join(failed) + ')'}: {summary}")
    return to


# ------------------------------------------------------------------ shadow transitions

def book_name(cand: dict) -> str:
    return cand.get("shadow_book") or f"shadow:{cand['name']}"


def create_shadow(cand: dict, actor: str = "pipeline", dry_run: bool = False) -> str:
    name = book_name(cand)
    if cand["kind"] == "ticker":
        cfg, tickers = None, list(cand["tickers"])
    else:
        cfg, tickers = int(cand["materialized_config_id"]), None
    desc = f"book '{name}' (config_version_id={cfg if cfg else 'NULL → follows promoted'}, tickers={tickers or 'config'})"
    if dry_run or not SHADOW_BOOKS_ENABLED:
        why = "dry-run" if dry_run else "shadow books disabled: set PIPELINE_SHADOW_BOOKS=1 to let advance create them"
        log(f"[{why}] #{cand['id']} {cand['name']}: would create {desc} and move backtest_passed → shadow")
        return "would-create"
    purpose = f"pipeline trial #{cand['id']} {cand['name']} ({cand['kind']}, gate {cand['gate']})"
    extra_sql = f"""INSERT INTO books (name, role, config_version_id, tickers, capital, enabled, purpose, candidate_id, created_by)
VALUES ({db.lit(name)}, 'shadow', {db.lit(cfg)}, {db.lit(tickers)}, NULL, true, {db.lit(purpose)}, {int(cand['id'])}, 'pipeline')
ON CONFLICT (name) DO UPDATE SET enabled = true, retired_at = NULL, retire_reason = NULL, config_version_id = EXCLUDED.config_version_id,
  tickers = EXCLUDED.tickers, purpose = EXCLUDED.purpose, candidate_id = EXCLUDED.candidate_id;"""
    # the FK shadow_book → books(name) needs the book first: run the insert before the transition inside one transaction
    sql = "BEGIN;\n" + extra_sql + "\n" + transition_sql(cand, "shadow", actor, {"event": "shadow_start", "book": name},
                                                         sets={"shadow_book": name, "shadow_started_at": db.Raw("now()"), "shadow_result": None}) + "\nCOMMIT;"
    out = db.execute(sql)
    if str(cand["id"]) not in out.split():
        raise PipelineError(f"stale stage on #{cand['id']} while creating the shadow book")
    cand.update(stage="shadow", shadow_book=name)
    log(f"#{cand['id']} {cand['name']}: created {desc}; the trial starts at the next trader start")
    notify("info", f"pipeline: shadow trial {name} created ({cand['kind']}); starts at the next trader start")
    return "shadow"


def retire_book_sql(name: str, reason: str) -> str:
    return f"UPDATE books SET enabled = false, retired_at = coalesce(retired_at, now()), retire_reason = {db.lit(reason)} WHERE name = {db.lit(name)} AND role = 'shadow';"


def shadow_status(cand: dict) -> dict:
    book = book_name(cand)
    sess = db.query(f"SELECT session_date, config_version_id FROM book_sessions WHERE book = {db.lit(book)} ORDER BY session_date")
    trades = db.query(f"SELECT ticker, pnl_dollars AS pnl, entry_price, position_size AS size, "
                      f"(exit_fill_at AT TIME ZONE 'America/New_York')::date AS date, entry_fill_at AS entry_time, direction "
                      f"FROM trades WHERE book = {db.lit(book)} AND source = 'shadow' ORDER BY exit_fill_at")
    cal = db.query("SELECT session_date FROM book_sessions WHERE book = 'primary' ORDER BY session_date")
    return {
        "book": book,
        "session_dates": [s["session_date"] for s in sess],
        "session_configs": {s["session_date"]: s.get("config_version_id") for s in sess},
        "calendar": [dt.date.fromisoformat(c["session_date"]) for c in cal],
        "trades": trades,
        "sessions": len(sess), "n_trades": len(trades),
        "pnl": round(sum(t["pnl"] or 0.0 for t in trades), 2),
    }


def book_capital() -> float:
    env = db.read_env()
    try:
        return float(env.get("INITIAL_CAPITAL") or 10000)
    except ValueError:
        return 10000.0


def replay_trades_for(cand: dict, st: dict, prom: dict) -> list[dict]:
    """the same sessions replayed from the IEX cache with the candidate's config, no sizing override.
    reads data/live/<date>/replay_<book>.csv when export_day.sh produced it, else runs the replay."""
    book = st["book"]
    tickers = candidate_tickers(cand, prom)
    dates = st["session_dates"]
    if not dates:
        return []
    # bars for tickers outside the primary's set: top up the window first
    for t in tickers:
        cov = metrics.coverage(t, end=dt.date.fromisoformat(dates[-1]))
        have = set()
        if cov["exists"]:
            have = {d.isoformat() for d in metrics.sessions(BARS_DIR / f"{t}.csv")}
        missing = [d for d in dates if d not in have]
        if missing:
            fetch_bars([t], missing[0], missing[-1])
    row = db.query_one(f"SELECT capital FROM books WHERE name = {db.lit(book)}")
    capital = float(row["capital"]) if row and row.get("capital") else book_capital()
    safe = re.sub(r"[^A-Za-z0-9_.-]+", "_", book)
    PARITY_DIR.mkdir(parents=True, exist_ok=True)
    out = []
    for d in dates:
        f = DATA / "live" / d / f"replay_{book}.csv"
        if not f.exists():
            f = DATA / "live" / d / f"replay_{safe}.csv"
        cache = PARITY_DIR / f"{safe}_{d}.csv"
        if not f.exists() and not cache.exists() and REPLAY_BOOK_SH.exists():
            subprocess.run([str(REPLAY_BOOK_SH), book, d], env=db.subprocess_env(), capture_output=True, text=True, cwd=str(ROOT))
            f = DATA / "live" / d / f"replay_{book}.csv"
        if not f.exists() and not cache.exists():
            cfg = st["session_configs"].get(d) or (cand.get("materialized_config_id") if cand["kind"] == "config" else prom["id"])
            extra = ["--config-id", str(cfg)] + (["--tickers", ",".join(tickers)] if cand["kind"] == "ticker" else [])
            _, stdout, _ = run_backtest_day(d, extra, sizing=False, capital=capital)
            cache.write_text(stdout)
        src = f if f.exists() else cache
        out.extend(parse_trade_rows(src.read_text()))
    return out


def primary_pnl_on(dates: list[str]) -> float | None:
    if not dates:
        return None
    arr = "ARRAY[" + ",".join(db.lit(d) for d in dates) + "]::date[]"
    v = db.scalar(f"SELECT coalesce(sum(pnl_dollars), 0) AS pnl FROM trades WHERE book = 'primary' AND source = 'paper' "
                  f"AND (exit_fill_at AT TIME ZONE 'America/New_York')::date = ANY({arr})")
    return float(v) if v is not None else None


def fail_shadow(cand: dict, actor: str, verdict: str, result: dict, retire_reason: str) -> None:
    result = dict(result, verdict=verdict)
    extra = retire_book_sql(book_name(cand), retire_reason)
    if cand.get("materialized_config_id"):
        extra += f"\nUPDATE config_versions SET status = 'rejected' WHERE id = {int(cand['materialized_config_id'])} AND status IN ('backtesting', 'validated');"
    transition(cand, "shadow_failed", actor, {"event": "shadow_verdict", "verdict": verdict, "book": book_name(cand)},
               sets={"shadow_result": result, "shadow_evaluated_at": db.Raw("now()"), "cooldown_until": db.Raw(f"current_date + {COOLDOWN_DAYS}")},
               extra_sql=extra)
    notify("warning", f"pipeline: shadow {book_name(cand)} ended: {verdict}; book retired, cooldown {COOLDOWN_DAYS} d")


def evaluate_shadow(cand: dict, actor: str = "pipeline", force: bool = False, dry_run: bool = False) -> str:
    """shadow → shadow_passed (→ promotion_proposed) | shadow_failed, or a plumbing flag."""
    if cand["stage"] != "shadow":
        raise PipelineError(f"#{cand['id']} is in stage '{cand['stage']}', not shadow")
    prom = promoted_row()
    st = shadow_status(cand)
    prev = cand.get("shadow_result") or {}
    started = db.parse_ts(cand.get("shadow_started_at"))
    started_d = started.astimezone(ET).date() if started else today_et()
    flagged, elapsed = shadow.not_hosted(started_d, st["session_dates"], today_et(), st["calendar"])
    status = {"sessions": st["sessions"], "trades": st["n_trades"], "pnl": st["pnl"], "elapsed_trading_days": elapsed,
              "min_sessions": cand["shadow_min_sessions"], "min_trades": cand["shadow_min_trades"]}
    if flagged:
        if prev.get("flag") != "not_hosted" and not dry_run:
            event_only(cand, actor, {"event": "not_hosted", "why": f"no book_sessions row {elapsed} trading days after start"},
                       sets={"shadow_result": dict(prev, flag="not_hosted", flag_detail=f"no hosted session {elapsed} trading days after {started_d} (build failed or restart pending)", status=status)})
            notify("warning", f"pipeline: shadow {st['book']} not hosted {elapsed} trading days after start (build failed or restart pending?)")
        log(f"#{cand['id']} {cand['name']}: not hosted ({elapsed} trading days, no book_sessions row) — reported, clock stopped")
        return "not_hosted"
    if prev.get("flag") == "not_hosted" and st["sessions"] and not dry_run:
        prev = {k: v for k, v in prev.items() if k not in ("flag", "flag_detail")}
        event_only(cand, actor, {"event": "hosted", "why": "first book_sessions row seen"}, sets={"shadow_result": dict(prev, status=status)})
    is_due, why = shadow.due(st["sessions"], st["n_trades"], cand["shadow_min_sessions"], cand["shadow_min_trades"])
    status["due_reason"] = why
    if prev.get("flag") == "parity" and not force:
        log(f"#{cand['id']} {cand['name']}: parity flag set ({prev.get('flag_detail', '')}); clock stopped until `evaluate NAME --force`")
        return "flagged"
    if not is_due and not force:
        log(f"#{cand['id']} {cand['name']}: shadow not due — {why}")
        if not dry_run:
            db.execute(f"UPDATE pipeline_candidates SET shadow_result = coalesce(shadow_result, '{{}}'::jsonb) || {db.jsonlit({'status': status})} WHERE id = {int(cand['id'])};")
        return "not_due"
    if dry_run:
        log(f"[dry-run] #{cand['id']} {cand['name']}: would evaluate the shadow trial now ({why}): parity replay of {st['sessions']} sessions, then the edge rule")
        return "dry-run"
    if st["sessions"] >= shadow.MAX_SESSIONS and st["n_trades"] < cand["shadow_min_trades"]:
        log(f"#{cand['id']} {cand['name']}: {st['sessions']} sessions but only {st['n_trades']} trades → insufficient_activity")
        fail_shadow(cand, actor, "insufficient_activity", {"status": status}, "insufficient_activity")
        return "shadow_failed"
    replay = replay_trades_for(cand, st, prom)
    par = shadow.parity(st["trades"], replay)
    if not par["ok"]:
        detail = f"{par['count_detail']}; {par['bps_detail']}"
        log(f"#{cand['id']} {cand['name']}: PARITY FLAG (plumbing, not edge): {detail}")
        event_only(cand, actor, {"event": "parity_flag", "why": detail},
                   sets={"shadow_result": dict(prev, flag="parity", flag_detail=detail, parity=par, status=status)})
        notify("warning", f"pipeline: shadow {st['book']} parity failed vs the IEX replay ({detail}); plumbing check needed, clock stopped")
        return "flagged"
    primary = primary_pnl_on(st["session_dates"]) if cand["kind"] == "config" else None
    ed = shadow.edge(cand["kind"], st["trades"], replay, primary)
    result = {"status": status, "parity": par, "edge": ed, "primary_pnl_same_sessions": primary,
              "evaluated_at": dt.datetime.now(dt.timezone.utc).isoformat()}
    if not ed["pass"]:
        failed = [c["name"] for c in ed["checks"] if not c["ok"]]
        fail_shadow(cand, actor, "edge_failed: " + ",".join(failed), result, "shadow edge failed: " + ",".join(failed))
        return "shadow_failed"
    transition(cand, "shadow_passed", actor, {"event": "shadow_verdict", "verdict": "pass", "book": st["book"]},
               sets={"shadow_result": dict(result, verdict="pass"), "shadow_evaluated_at": db.Raw("now()")})
    propose_promotion(cand, actor)
    return "promotion_proposed"


def propose_promotion(cand: dict, actor: str = "pipeline") -> None:
    """shadow_passed → promotion_proposed: the row the human would promote, a proposal block, ntfy."""
    if cand["stage"] != "shadow_passed":
        raise PipelineError(f"#{cand['id']} is in stage '{cand['stage']}', not shadow_passed")
    prom = promoted_row()
    res = dict(cand.get("shadow_result") or {})
    full = None
    if cand["kind"] == "ticker":
        t = cand["tickers"][0]
        proposed = cand.get("proposed_config_version_id")
        if not proposed:
            blob = copy.deepcopy(prom["config_blob"])
            if t not in blob["tickers"]:
                blob["tickers"] = list(blob["tickers"]) + [t]
            blob = materialize.stamp_blob(blob, prom["config_blob"], cand["name"])
            reason = f"pipeline promotion proposal for candidate '{cand['name']}' (#{cand['id']}): promoted row {prom['id']} + {t}"
            sql = f"""WITH s AS (SELECT nextval('config_versions_id_seq') AS nid),
ins AS (
  INSERT INTO config_versions (id, status, created_by, parent_version_id, mutation_reason, config_blob)
  SELECT nid, 'validated', 'pipeline', {int(prom['id'])}, {db.lit(reason)}, jsonb_set({db.jsonlit(blob)}, '{{config_id}}', to_jsonb(nid)) FROM s RETURNING id
)
UPDATE pipeline_candidates p SET proposed_config_version_id = ins.id, updated_at = now() FROM ins WHERE p.id = {int(cand['id'])} AND p.proposed_config_version_id IS NULL RETURNING ins.id;"""
            out = db.execute(sql).strip().split()
            proposed = int([x for x in out if x.isdigit()][-1])
            cand["proposed_config_version_id"] = proposed
            event_only(cand, actor, {"event": "proposal_row", "config_version_id": proposed})
        # the full-set sweep of the proposed row, compared with the promoted row on its own set
        tag = f"cand_{cand['id']}_full"
        ensure_sweep(tag, ["--config-id", str(proposed)])
        btag, bargs = baseline_for(int(prom["id"]), list(prom["config_blob"]["tickers"]), prom)
        if bargs:
            ensure_sweep(btag, bargs)
        fm, bm = metrics.full_metrics(tag), metrics.full_metrics(btag)
        floor = bm["pnl"] - 0.10 * abs(bm["pnl"])
        full = {"tag": tag, "metrics": fm, "baseline_tag": btag, "baseline": bm, "pnl_not_worse_than_baseline_minus_10pct": fm["pnl"] >= floor}
    else:
        proposed = int(cand["materialized_config_id"])
        if config_status(proposed) not in ("validated", "promoted"):
            db.execute(f"UPDATE config_versions SET status = 'validated' WHERE id = {proposed} AND status = 'backtesting';")
    bt = (cand.get("backtest_result") or {}).get("metrics") or {}
    stt, par, ed = res.get("status") or {}, res.get("parity") or {}, res.get("edge") or {}
    lines = [f"### promote `{cand['name']}` → config row {proposed}",
             "",
             f"- candidate #{cand['id']} ({cand['kind']}, gate `{cand['gate']}`), shadow book `{book_name(cand)}`, proposed {dt.datetime.now(dt.timezone.utc).strftime('%Y-%m-%d %H:%M UTC')}",
             f"- backtest (36 % sizing): {metrics.fmt_stats(bt)}" + (" · years " + " ".join(f"{y[2:]}:{bt['years'][y]['pnl']:+.0f}" for y in metrics.YEARS) if bt.get("years") else ""),
             f"- shadow trial: {stt.get('sessions', '?')} sessions, {stt.get('trades', '?')} trades, P&L {stt.get('pnl', 0):+.0f}; parity vs replay: {par.get('count_detail', '')}, {par.get('bps_detail', '')}",
             "- edge: " + ", ".join(f"{c['name']} {c['value']} ({c['threshold']}) {'ok' if c['ok'] else 'FAIL'}" for c in ed.get("checks", []))]
    if full:
        fm, bm = full["metrics"], full["baseline"]
        warn = "" if full["pnl_not_worse_than_baseline_minus_10pct"] else " — **below baseline − 10 %, review before promoting**"
        lines.append(f"- full-set sweep of row {proposed} (`{full['tag']}`): {metrics.fmt_stats(fm)} vs `{full['baseline_tag']}` {metrics.fmt_stats(bm)}{warn}")
    lines += ["- to promote (human): `UPDATE config_versions SET status='superseded' WHERE status='promoted'; UPDATE config_versions SET status='promoted', promoted_at=now() WHERE id=" + str(proposed) + ";` — the trader hot-reloads; the pipeline retires the shadow book when it sees the promotion.",
              f"- to decline: `pipeline.py withdraw {cand['name']} --reason ...`"]
    md = "\n".join(lines)
    res.update(proposal_md=md, full_set=full)
    transition(cand, "promotion_proposed", actor, {"event": "promotion_proposed", "config_version_id": proposed},
               sets={"shadow_result": res, "proposed_config_version_id": proposed})
    write_status()
    notify("warning", f"promotion proposed: {cand['name']} → config row {proposed} (see docs/pipeline/status.md)")


def detect_promotions(cands: list[dict], actor: str = "pipeline", dry_run: bool = False) -> None:
    for c in cands:
        if c["stage"] in TERMINAL:
            continue
        ids = [c.get("proposed_config_version_id"), c.get("materialized_config_id")]
        for cid in [i for i in ids if i]:
            if config_status(int(cid)) == "promoted":
                if dry_run:
                    log(f"[dry-run] #{c['id']} {c['name']}: row {cid} is promoted → would mark promoted and retire {book_name(c)}")
                    break
                transition(c, "promoted", actor, {"event": "promoted", "config_version_id": cid, "book": c.get("shadow_book")},
                           extra_sql=retire_book_sql(book_name(c), f"promoted as config row {cid}") if c.get("shadow_book") else "")
                notify("info", f"pipeline: {c['name']} promoted (row {cid}); shadow book retired")
                break


# ------------------------------------------------------------------ commands

def rerun_backtest(cand: dict, actor: str, reason: str) -> None:
    """backtest_passed|backtest_failed → backtesting again (e.g. the backtest binary's semantics changed).
    keeps the materialized row (status back to backtesting) and forces a fresh sweep."""
    if cand["stage"] not in ("backtest_passed", "backtest_failed"):
        raise PipelineError(f"#{cand['id']} is in stage '{cand['stage']}'; --rerun applies to backtest_passed/backtest_failed")
    extra = ""
    if cand.get("materialized_config_id"):
        extra = f"UPDATE config_versions SET status = 'backtesting' WHERE id = {int(cand['materialized_config_id'])} AND status IN ('validated', 'rejected');"
    transition(cand, "backtesting", actor, {"event": "backtest_rerun", "reason": reason, "previous": (cand.get("backtest_result") or {}).get("pass")},
               sets={"backtest_tag": None, "backtest_result": None, "backtest_at": None, "cooldown_until": None}, extra_sql=extra)


def run_regate(cand: dict, new_gate_raw: str, actor: str = "pipeline") -> str:
    """backtest_passed|backtest_failed -> backtest_passed|backtest_failed under a different gate,
    reusing the existing sweep tag and materialized row (no new sweep, no new materialization).
    records a 'regate' event with the old and new gate names."""
    cid = cand["id"]
    if cand["stage"] not in ("backtest_passed", "backtest_failed"):
        raise PipelineError(f"#{cid} is in stage '{cand['stage']}'; regate applies to backtest_passed/backtest_failed only")
    old_gate = cand["gate"]
    new_gate = gates.resolve(new_gate_raw, cand["kind"])
    tag = cand.get("backtest_tag")
    if not tag or not metrics.sweep_files_present(tag):
        raise PipelineError(f"#{cid} {cand['name']}: no sweep on disk for tag '{tag}'; regate reuses the existing backtest, run `backtest {cand['name']}` first")
    prom = promoted_row()
    base_id = int(cand.get("base_config_version_id") or prom["id"])
    tickers = candidate_tickers(cand, prom)
    mid = cand.get("materialized_config_id")

    baseline_tag, baseline = None, None
    if new_gate in gates.NEEDS_BASELINE:
        baseline_tag, bargs = baseline_for(base_id, tickers, prom)
        if bargs:
            ensure_sweep(baseline_tag, bargs)
        baseline = metrics.full_metrics(baseline_tag)
    m = metrics.full_metrics(tag, tickers, baseline_tag=baseline_tag if new_gate in gates.NEEDS_MARGINAL else None)
    g = gates.run_gate(new_gate, m, baseline)
    prev = cand.get("backtest_result") or {}
    mcheck = prev.get("materialize_check")
    if mcheck and not mcheck.get("ok"):
        g["checks"].append(gates.check("materialize_equals_patch", False, "identical trade rows on the verify days", False))
        g["pass"] = False
    passed = g["pass"]
    result = {
        "gate": new_gate, "pass": passed, "gate_result": g, "metrics": m,
        "baseline": {"tag": baseline_tag, "metrics": baseline} if baseline_tag else None,
        "sweep": prev.get("sweep") or {"tag": tag},
        "sizing_note": prev.get("sizing_note") or "gate sweep at --sizing-fraction 0.36 --max-position-pct 0.36; live/shadow size at the blob's fraction",
        "coverage": prev.get("coverage"), "materialize_check": mcheck,
        "regated_from": old_gate,
        "computed_at": dt.datetime.now(dt.timezone.utc).isoformat(),
    }
    to = "backtest_passed" if passed else "backtest_failed"
    sets = {"gate": new_gate, "backtest_result": result, "backtest_at": db.Raw("now()")}
    if passed:
        sets["cooldown_until"] = None
    extra_sql = ""
    if mid:
        extra_sql = f"UPDATE config_versions SET status = {db.lit('validated' if passed else 'rejected')} WHERE id = {int(mid)};"
    failed = gates.failed_names(g)
    transition(cand, to, actor, {"event": "regate", "from_gate": old_gate, "to_gate": new_gate, "pass": passed, "failed": failed,
                                 "pnl": m["pnl"], "n": m["n"], "pf": m["pf"]}, sets=sets, extra_sql=extra_sql)
    summary = f"{m['pnl']:+.0f} / {m['n']} trades / PF {m['pf']:.2f}"
    mg = m.get("marginal")
    if mg:
        summary += f", added {mg['added']['pnl']:+.0f} / {mg['added']['n']} / PF {mg['added']['pf']:.2f}"
    log(f"#{cid} {cand['name']}: regate {old_gate} → {new_gate}: {'PASS' if passed else 'FAIL ' + ','.join(failed)} — {summary}")
    notify("info", f"pipeline: {cand['name']} regated {old_gate} -> {new_gate}: {'passed' if passed else 'failed (' + ', '.join(failed) + ')'}")
    return to


def cmd_regate(a) -> int:
    with Lock(PIPELINE_LOCK, 0, "pipeline"):
        # active_only=False: backtest_failed is a terminal stage, and regate must reach it too
        cand = get_candidate(a.name, active_only=False)
        out = run_regate(cand, a.gate, actor_for("pipeline"))
    log(f"outcome: {out}")
    write_status()
    return 0


def cmd_backtest(a) -> int:
    with Lock(PIPELINE_LOCK, 0, "pipeline"):
        cand = get_candidate(a.name, active_only=True)
        if a.rerun:
            rerun_backtest(cand, actor_for("pipeline"), a.reason or "rerun requested")
        out = run_backtest(cand, actor_for("pipeline"), budget=[MAX_NEW_TICKER_FETCHES])
    log(f"outcome: {out}")
    write_status()
    return 0 if out in ("backtest_passed", "backtest_failed") else 1


def cmd_evaluate(a) -> int:
    with Lock(PIPELINE_LOCK, 0, "pipeline"):
        cand = get_candidate(a.name, active_only=True)
        actor = actor_for("pipeline")
        if cand["stage"] == "shadow_passed":
            propose_promotion(cand, actor)
            out = "promotion_proposed"
        else:
            out = evaluate_shadow(cand, actor, force=a.force)
    log(f"outcome: {out}")
    write_status()
    return 0


def cmd_advance(a) -> int:
    actor = actor_for("pipeline")
    dry = a.dry_run
    lock = Lock(PIPELINE_LOCK, 0, "pipeline") if not dry else None
    if lock:
        lock.__enter__()
    try:
        cands = candidates(active_only=True)
        if not cands:
            log("no active candidates")
        if not SHADOW_BOOKS_ENABLED:
            log("shadow-book creation is disabled (PIPELINE_SHADOW_BOOKS != 1): backtest_passed candidates are reported, not started")
        detect_promotions(cands, actor, dry)
        for c in [c for c in cands if c["stage"] == "shadow"]:
            try:
                evaluate_shadow(c, actor, dry_run=dry)
            except STEP_ERRORS as e:
                log(f"#{c['id']} {c['name']}: evaluate failed: {e}")
                if not dry:
                    event_only(c, actor, {"event": "error", "why": str(e)[:500]})
        for c in [c for c in cands if c["stage"] == "shadow_passed"]:
            if dry:
                log(f"[dry-run] #{c['id']} {c['name']}: would write the promotion proposal")
                continue
            try:
                propose_promotion(c, actor)
            except STEP_ERRORS as e:
                log(f"#{c['id']} {c['name']}: promotion proposal failed: {e}")
                event_only(c, actor, {"event": "error", "why": str(e)[:500]})
        for c in [c for c in cands if c["stage"] == "backtest_passed"]:
            try:
                create_shadow(c, actor, dry_run=dry)
            except STEP_ERRORS as e:
                log(f"#{c['id']} {c['name']}: shadow creation failed: {e}")
        budget, n = [MAX_NEW_TICKER_FETCHES], 0
        queue = [c for c in cands if c["stage"] == "backtesting"] + [c for c in cands if c["stage"] == "proposed"]
        for c in queue:
            if n >= a.max_backtests:
                log(f"--max-backtests {a.max_backtests} reached; {len(queue) - n} candidate(s) wait for the next run")
                break
            hole = last_event(c, "coverage_hole") if c["stage"] == "backtesting" else None
            if hole and (dt.datetime.now(dt.timezone.utc) - db.parse_ts(hole["ts"])).days < COVERAGE_RETRY_DAYS:
                log(f"#{c['id']} {c['name']}: coverage hole flagged {hole['ts'][:10]}; retry after {COVERAGE_RETRY_DAYS} d")
                continue
            n += 1
            try:
                run_backtest(c, actor, budget=budget, dry_run=dry)
            except STEP_ERRORS as e:
                log(f"#{c['id']} {c['name']}: backtest failed: {e}")
                if not dry:
                    event_only(c, actor, {"event": "error", "why": str(e)[:500]})
                    notify("warning", f"pipeline: {c['name']} backtest step errored: {str(e)[:200]}")
    finally:
        if lock:
            lock.__exit__(None, None, None)
    if not dry:
        write_status()
    return 0


def cmd_withdraw(a) -> int:
    cand = get_candidate(a.name, active_only=True)
    reason = a.reason or "withdrawn by human"
    extra = ""
    if cand.get("shadow_book"):
        extra += retire_book_sql(cand["shadow_book"], reason) + "\n"
    for cid in {cand.get("materialized_config_id"), cand.get("proposed_config_version_id")} - {None}:
        extra += f"UPDATE config_versions SET status = 'rejected' WHERE id = {int(cid)} AND status IN ('backtesting', 'validated');\n"
    transition(cand, "withdrawn", actor_for("human"), {"event": "withdrawn", "reason": reason}, extra_sql=extra)
    write_status()
    return 0


def cmd_retire_shadow(a) -> int:
    cand = get_candidate(a.name, active_only=True)
    if cand["stage"] not in ("shadow", "shadow_passed"):
        raise PipelineError(f"#{cand['id']} is in stage '{cand['stage']}', not shadow")
    reason = a.reason or "retired by human"
    st = shadow_status(cand)
    fail_shadow(cand, actor_for("human"), f"retired: {reason}", {"status": {"sessions": st["sessions"], "trades": st["n_trades"], "pnl": st["pnl"]}}, reason)
    write_status()
    return 0


def cmd_list(a) -> int:
    rows = candidates(stage=a.stage)
    now = dt.datetime.now(dt.timezone.utc)
    print(f"{'id':>4} {'name':28} {'kind':6} {'stage':19} {'gate':14} {'since':>6}  numbers")
    for c in rows:
        r = c.get("backtest_result") or {}
        m = r.get("metrics")
        num = ""
        if m:
            g = r.get("gate_result") or {}
            num = f"5y {m['pnl']:+.0f} / {m['n']} / PF {m['pf']:.2f} · " + ("pass" if g.get("pass") else "fail " + ",".join(gates.failed_names(g)))
        elif r.get("blocked"):
            num = f"blocked: {r['blocked']}"
        s = (c.get("shadow_result") or {}).get("status")
        if s:
            num += f" · shadow {s.get('sessions')} sess / {s.get('trades')} tr / {s.get('pnl', 0):+.0f}"
        if (c.get("shadow_result") or {}).get("flag"):
            num += f" · FLAG {c['shadow_result']['flag']}"
        print(f"{c['id']:>4} {c['name'][:28]:28} {c['kind']:6} {c['stage']:19} {c['gate']:14} {report._age(c.get('updated_at'), now):>6}  {num}")
    return 0


def cmd_show(a) -> int:
    cand = get_candidate(a.name)
    out = dict(cand)
    if out.get("patch") and isinstance(out["patch"], dict):
        p = out["patch"]
        out["patch_summary"] = {k: (len(v) if isinstance(v, list) else v) for k, v in p.items()}
        out["patch"] = json.loads(json.dumps(p))  # deep copy before abbreviating long date lists
        for ind in out["patch"].get("indicators", []):
            if isinstance(ind.get("params", {}).get("dates"), list) and len(ind["params"]["dates"]) > 10:
                ind["params"]["dates"] = f"<{len(ind['params']['dates'])} dates>"
    out["events"] = list(reversed(events(cand["id"], limit=200)))
    if cand["stage"] in ("shadow", "shadow_passed", "promotion_proposed"):
        st = shadow_status(cand)
        out["shadow_status_now"] = {k: st[k] for k in ("book", "sessions", "n_trades", "pnl")}
    mg = ((cand.get("backtest_result") or {}).get("metrics") or {}).get("marginal")
    if mg:
        base, added = mg["base"], mg["added"]
        out["marginal_summary"] = {
            "base_vs_baseline": f"base {metrics.fmt_stats(base)} vs baseline {mg['baseline_tag']} {metrics.fmt_stats((cand.get('backtest_result') or {}).get('baseline', {}).get('metrics') or {})}",
            "added": f"{metrics.fmt_stats(added)}, worst year {added['worst_year_pnl']:+.0f}, worst trade {added['worst_trade']:+.0f}, worst day {added['worst_day']:+.0f}",
            "added_by_year": {y: metrics.fmt_stats(added["years"][y]) for y in metrics.YEARS},
        }
    print(json.dumps(out, indent=1, default=str))
    return 0


def write_status() -> Path:
    cands = candidates()
    for c in cands:
        if c["stage"] in ("shadow", "shadow_passed", "promotion_proposed"):
            try:
                st = shadow_status(c)
                due_ok, why = shadow.due(st["sessions"], st["n_trades"], c["shadow_min_sessions"], c["shadow_min_trades"])
                c["shadow_status"] = {"sessions": st["sessions"], "trades": st["n_trades"], "pnl": st["pnl"], "due_reason": ("due: " if due_ok else "") + why}
            except db.DbError as e:
                c["shadow_status"] = {"due_reason": f"status unavailable: {e}"[:120]}
    ev = events(limit=20)
    baseline = metrics.summarize("iex_v18") if metrics.sweep_files_present("iex_v18") else None
    md = report.render(cands, ev, baseline, {"shadow_books_enabled": SHADOW_BOOKS_ENABLED})
    return report.write(md)


def cmd_report(a) -> int:
    path = write_status()
    text = path.read_text()
    if a.markdown:
        print(text)
    else:
        # compact: everything up to the events table
        print(text.split("## last 20 events")[0].rstrip())
        print(f"\n(written {path.relative_to(ROOT)})")
    return 0


def build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(prog="pipeline.py", description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sp = ap.add_subparsers(dest="cmd", required=True)
    p = sp.add_parser("propose", help="add a candidate")
    p.add_argument("--name")
    p.add_argument("--kind", choices=["ticker", "config"], required=True)
    p.add_argument("--ticker")
    p.add_argument("--patch", action="append", help="patch json (repeatable; applied in order)")
    p.add_argument("--tickers", help="config kind: trade this set instead of the base's")
    p.add_argument("--gate", default="default", help="default-ticker | volume-config | quality-config | stress-mode")
    p.add_argument("--base", type=int, help="config_versions row to build on (default: the promoted row)")
    p.add_argument("--notes")
    p.add_argument("--source", default="human", help="human | routine | scout | agent")
    p.add_argument("--force", action="store_true", help="ignore a cooldown")
    p.set_defaults(fn=cmd_propose)
    p = sp.add_parser("list", help="table of candidates")
    p.add_argument("--stage")
    p.set_defaults(fn=cmd_list)
    p = sp.add_parser("show", help="everything about one candidate")
    p.add_argument("name")
    p.set_defaults(fn=cmd_show)
    p = sp.add_parser("advance", help="run every due transition (nightly)")
    p.add_argument("--max-backtests", type=int, default=2)
    p.add_argument("--dry-run", action="store_true")
    p.set_defaults(fn=cmd_advance)
    p = sp.add_parser("backtest", help="run the backtest gate for one candidate now")
    p.add_argument("name")
    p.add_argument("--rerun", action="store_true", help="re-sweep and re-gate a backtest_passed/failed candidate (same materialized row)")
    p.add_argument("--reason", help="why (recorded in pipeline_events with --rerun)")
    p.set_defaults(fn=cmd_backtest)
    p = sp.add_parser("regate", help="re-evaluate an existing backtest under a different gate (no new sweep)")
    p.add_argument("name")
    p.add_argument("--gate", required=True, help="volume-config | quality-config | stress-mode | additive-config | default-ticker")
    p.set_defaults(fn=cmd_regate)
    p = sp.add_parser("evaluate", help="evaluate a shadow trial now")
    p.add_argument("name")
    p.add_argument("--force", action="store_true", help="evaluate even if not due; clears a parity flag on success")
    p.set_defaults(fn=cmd_evaluate)
    p = sp.add_parser("withdraw", help="any stage → withdrawn (human)")
    p.add_argument("name")
    p.add_argument("--reason")
    p.set_defaults(fn=cmd_withdraw)
    p = sp.add_parser("retire-shadow", help="end a shadow trial as failed (human)")
    p.add_argument("name")
    p.add_argument("--reason")
    p.set_defaults(fn=cmd_retire_shadow)
    p = sp.add_parser("report", help="regenerate docs/pipeline/status.md")
    p.add_argument("--markdown", action="store_true", help="print the full markdown")
    p.set_defaults(fn=cmd_report)
    return ap


def main(argv=None) -> int:
    a = build_parser().parse_args(argv)
    try:
        return a.fn(a)
    except (PipelineError, db.DbError, materialize.PatchError, KeyError) as e:
        print(f"[pipeline] error: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
