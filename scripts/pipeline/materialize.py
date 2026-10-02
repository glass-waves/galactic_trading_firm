#!/usr/bin/env python3
"""materialize a config candidate: base config blob + patch -> a new config_versions row.

the patch format is exactly the backtest CLI's `--patch-json` (crates/backtest/src/main.rs), applied
in the same order so that `backtest --config-id <materialized>` and `backtest --config-id <base>
--patch-json <patch>` produce identical trades:

  disable    [instance_id, ...]   -> enabled=false on matching *action* instance ids
  indicators [IndicatorConfig...] -> appended
  actions    [ActionConfig...]    -> appended
  session    {...}                -> shallow-merged into `session`; `force_exit_by` is ALSO written
                                     into every session_close action's params
  tickers    [A, B]               -> replaces the ticker list

usage (library): apply_patch(blob, patch), merge_patches(p1, p2, ...), materialize(candidate_row)
usage (cli):     materialize.py --base ROW --patch FILE [--patch FILE...] [--dry-run]
"""
from __future__ import annotations

import argparse
import copy
import datetime as dt
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import db  # noqa: E402

PATCH_KEYS = ("disable", "indicators", "actions", "session", "tickers")
# meta keys: carried on the candidate's patch dict but never applied to the config blob (the
# backtest CLI's --patch-json and this module's apply_patch() both only read PATCH_KEYS, so a meta
# key is silently inert there; pipeline.py reads it separately to extend the sweep command).
# _sweep_args: str - extra argv tokens appended AFTER the gate sweep's standard args (so a later
#   occurrence of a flag such as --max-position-pct wins; see backtest/src/main.rs get_arg(), which
#   resolves a repeated flag to its LAST occurrence) - lets a candidate's sweep run at a cap the
#   standard --max-position-pct 0.36 would otherwise clamp. documented in docs/pipeline.md.
META_KEYS = ("_sweep_args",)


class PatchError(ValueError):
    pass


def validate_patch(patch: dict) -> list[str]:
    """return a list of problems (empty = ok). unknown keys are errors: the CLI would ignore them silently."""
    problems = []
    if not isinstance(patch, dict):
        return ["patch must be a json object"]
    for k in patch:
        if k not in PATCH_KEYS and k not in META_KEYS:
            problems.append(f"unknown patch key '{k}' (allowed: {', '.join(PATCH_KEYS)}, {', '.join(META_KEYS)})")
    if "disable" in patch and not (isinstance(patch["disable"], list) and all(isinstance(x, str) for x in patch["disable"])):
        problems.append("'disable' must be a list of instance ids")
    for k in ("indicators", "actions"):
        if k in patch:
            if not isinstance(patch[k], list) or not all(isinstance(x, dict) for x in patch[k]):
                problems.append(f"'{k}' must be a list of objects")
            else:
                need = ("indicator_type", "instance_id", "timescale") if k == "indicators" else ("action_type", "instance_id", "phase")
                for i, x in enumerate(patch[k]):
                    for f in need:
                        if f not in x:
                            problems.append(f"'{k}[{i}]' lacks '{f}'")
    if "session" in patch and not isinstance(patch["session"], dict):
        problems.append("'session' must be an object")
    if "tickers" in patch and not (isinstance(patch["tickers"], list) and all(isinstance(x, str) for x in patch["tickers"])):
        problems.append("'tickers' must be a list of strings")
    if "_sweep_args" in patch and not isinstance(patch["_sweep_args"], str):
        problems.append("'_sweep_args' must be a string")
    return problems


def merge_patches(*patches: dict) -> dict:
    """sequential application of several patch files == one merged patch (lists concatenate,
    session keys shallow-merge in order, the last `tickers` / `_sweep_args` wins)."""
    out: dict = {}
    for p in patches:
        if p.get("disable"):
            out.setdefault("disable", []).extend(p["disable"])
        if p.get("indicators"):
            out.setdefault("indicators", []).extend(copy.deepcopy(p["indicators"]))
        if p.get("actions"):
            out.setdefault("actions", []).extend(copy.deepcopy(p["actions"]))
        if isinstance(p.get("session"), dict):
            out.setdefault("session", {}).update(copy.deepcopy(p["session"]))
        if isinstance(p.get("tickers"), list):
            out["tickers"] = list(p["tickers"])
        if isinstance(p.get("_sweep_args"), str):
            out["_sweep_args"] = p["_sweep_args"]
    return out


def apply_patch(blob: dict, patch: dict) -> dict:
    """pure: returns a new config blob. mirrors backtest/src/main.rs `--patch-json` exactly."""
    problems = validate_patch(patch)
    if problems:
        raise PatchError("; ".join(problems))
    out = copy.deepcopy(blob)
    out.setdefault("indicators", [])
    out.setdefault("actions", [])
    disable = set(patch.get("disable") or [])
    if disable:
        for a in out["actions"]:
            if a.get("instance_id") in disable:
                a["enabled"] = False
    if patch.get("indicators"):
        out["indicators"].extend(copy.deepcopy(patch["indicators"]))
    if patch.get("actions"):
        out["actions"].extend(copy.deepcopy(patch["actions"]))
    sess = patch.get("session")
    if isinstance(sess, dict):
        out.setdefault("session", {})
        for k, v in sess.items():
            out["session"][k] = copy.deepcopy(v)
        fe = sess.get("force_exit_by")
        if isinstance(fe, str):
            for a in out["actions"]:
                if a.get("action_type") == "session_close":
                    a.setdefault("params", {})["force_exit_by"] = fe
    if isinstance(patch.get("tickers"), list):
        out["tickers"] = [t for t in patch["tickers"] if isinstance(t, str)]
    return out


def stamp_blob(blob: dict, base_blob: dict, candidate_name: str) -> dict:
    """provenance fields inside the blob (config_id is set to the new row id at insert time)."""
    out = dict(blob)
    out["parent_config_id"] = base_blob.get("config_id")
    out["created_by"] = "pipeline"
    out["created_at"] = dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")
    return out


def load_blob(config_version_id: int) -> dict:
    row = db.query_one(f"SELECT config_blob FROM config_versions WHERE id = {int(config_version_id)}")
    if not row:
        raise PatchError(f"config_versions row {config_version_id} not found")
    return row["config_blob"]


def materialize(candidate: dict, actor: str = "pipeline") -> int:
    """insert the materialized row for a config candidate and link it, atomically.
    idempotent: returns the existing materialized_config_id when set."""
    if candidate.get("materialized_config_id"):
        return int(candidate["materialized_config_id"])
    if candidate["kind"] != "config":
        raise PatchError("only config candidates are materialized")
    base_id = candidate.get("base_config_version_id")
    if not base_id:
        raise PatchError("candidate has no base_config_version_id")
    base_blob = load_blob(base_id)
    patch = candidate.get("patch") or {}
    if candidate.get("tickers") and "tickers" not in patch:
        patch = dict(patch, tickers=list(candidate["tickers"]))
    blob = stamp_blob(apply_patch(base_blob, patch), base_blob, candidate["name"])
    reason = f"pipeline candidate '{candidate['name']}' (#{candidate['id']}): base row {base_id} + patch"
    sql = f"""
WITH s AS (SELECT nextval('config_versions_id_seq') AS nid),
ins AS (
  INSERT INTO config_versions (id, status, created_by, parent_version_id, mutation_reason, config_blob)
  SELECT nid, 'backtesting', 'pipeline', {int(base_id)}, {db.lit(reason)},
         jsonb_set({db.jsonlit(blob)}, '{{config_id}}', to_jsonb(nid))
  FROM s RETURNING id
),
upd AS (
  UPDATE pipeline_candidates p SET materialized_config_id = ins.id, updated_at = now()
  FROM ins WHERE p.id = {int(candidate['id'])} AND p.materialized_config_id IS NULL
  RETURNING p.id AS cand_id, p.stage, ins.id AS cfg_id
)
INSERT INTO pipeline_events (candidate_id, from_stage, to_stage, actor, detail)
SELECT cand_id, stage, stage, {db.lit(actor)},
       jsonb_build_object('event', 'materialized', 'config_version_id', cfg_id, 'base_config_version_id', {int(base_id)})
FROM upd RETURNING (detail->>'config_version_id')::bigint;
"""
    out = db.execute(sql).strip().splitlines()
    ids = [l for l in out if l.strip().isdigit()]
    if not ids:
        # a concurrent run linked it first; read back
        row = db.query_one(f"SELECT materialized_config_id FROM pipeline_candidates WHERE id = {int(candidate['id'])}")
        if row and row.get("materialized_config_id"):
            return int(row["materialized_config_id"])
        raise db.DbError("materialize: insert returned no id")
    return int(ids[-1])


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", type=int, required=True, help="config_versions row id to build on")
    ap.add_argument("--patch", action="append", required=True, help="patch json file (repeatable, applied in order)")
    ap.add_argument("--dry-run", action="store_true", help="print the resulting blob instead of inserting")
    a = ap.parse_args(argv)
    patches = [json.load(open(p)) for p in a.patch]
    merged = merge_patches(*patches)
    base = load_blob(a.base)
    blob = stamp_blob(apply_patch(base, merged), base, "cli")
    if a.dry_run:
        print(json.dumps(blob, indent=1))
        return 0
    print("materialize.py inserts only through pipeline.py backtest (a candidate row is required); use --dry-run", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
