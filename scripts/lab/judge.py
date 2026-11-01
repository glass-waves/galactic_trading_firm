#!/usr/bin/env python3
"""judge — apply research/lab/PLAN.md tests 1–5 to finalist specs (coordinator only; needs LAB_REVEAL=1).

    LAB_REVEAL=1 python3 scripts/lab/judge.py SPEC.json [SPEC.json ...] [--baseline research/lab/specs/base/v18.json]
                                              [--windows train,holdout,target] [--md research/lab/REPORT_table.md]

a finalist may be a union of independent books written as "a.json+b.json" (their trades are pooled; each book keeps its
own capital, so the pooled dollars assume twice the capital — the table says so).
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "pipeline"))
import lab  # noqa: E402
import metrics as pm  # noqa: E402
from gates import sidak_alpha  # noqa: E402

TESTS = {
    "t1_target_pf": 1.4, "t1_target_n": 150, "t2_holdout_pf": 1.3, "t3_train_pf": 1.3, "t3_signflip": 0.05,
    "t3_year_floor": -300.0, "t3_exb_pf": 1.2, "t4_alpha": 0.10, "t5_dd": -1000.0, "t5_worst_day": -300.0,
}


def rows_for(ref: str, start: dt.date, end: dt.date) -> tuple[str, list[dict], int]:
    names, rows, books = [], [], 0
    for part in ref.split("+"):
        n, r = lab._rows_for(part, start, end)
        names.append(n)
        rows.extend(r)
        books += 1
    rows.sort(key=lambda r: r["entry_time"])
    return "+".join(names), rows, books


def judge(ref: str, base_ref: str, windows: list[str], n_finalists: int) -> dict:
    out = {"ref": ref, "windows": {}, "tests": {}}
    for w in windows:
        start, end = lab.window_bounds(w)
        lab.check_reveal(w, start, end)
        days = lab.sessions(lab.ROOT / "data" / "bars_iex", start, end)
        name, rows, books = rows_for(ref, start, end)
        out["name"], out["books"] = name, books
        m = lab.summarize(rows, days, start, end)
        out["windows"][w] = m
        if w == "target":
            _, brows, _ = rows_for(base_ref, start, end)
            p = pm.paired_day_bootstrap(rows, brows, session_list=[d.isoformat() for d in days], n=5000, seed=0)
            out["paired_target"] = p
    t = out["tests"]
    if "target" in out["windows"]:
        m = out["windows"]["target"]
        by = m["by_year"]
        t["1 target PF≥1.4, n≥150, 25&26>0"] = (m["total"]["pf"] >= TESTS["t1_target_pf"] and m["total"]["n"] >= TESTS["t1_target_n"]
                                                and by.get("2025", {}).get("pnl", 0) > 0 and by.get("2026", {}).get("pnl", 0) > 0)
        t["5 dd≥−1000, worst day≥−300"] = (m["total"]["dd"] >= TESTS["t5_dd"] and m.get("worst_day", 0) >= TESTS["t5_worst_day"])
        alpha = sidak_alpha(TESTS["t4_alpha"], n_finalists)
        p = out["paired_target"]
        t[f"4 paired vs base P(≤0)<{alpha:.3f}"] = p["p_le_0"] < alpha
    if "holdout" in out["windows"]:
        m = out["windows"]["holdout"]
        t["2 holdout PF≥1.3, P&L>0"] = m["total"]["pf"] >= TESTS["t2_holdout_pf"] and m["total"]["pnl"] > 0
    if "train" in out["windows"]:
        m = out["windows"]["train"]
        exb = m.get("ex_best_year", {}).get("pf", 0)
        t["3 train PF≥1.3, p<0.05, yr≥−300, exb≥1.2"] = (
            m["total"]["pf"] >= TESTS["t3_train_pf"] and (m.get("sign_flip_p") or 1) < TESTS["t3_signflip"]
            and all(b["pnl"] >= TESTS["t3_year_floor"] for b in m["by_year"].values()) and exb >= TESTS["t3_exb_pf"])
    out["pass_all"] = all(t.values()) if t else False
    return out


def fmt(j: dict) -> str:
    lines = [f"### {j['name']}" + (f"  ({j['books']} books pooled — dollars assume {j['books']}× capital)" if j["books"] > 1 else "")]
    lines.append("| window | P&L | trades | PF | dd | worst day | p | ex-best-yr PF | by year |")
    lines.append("|---|---:|---:|---:|---:|---:|---:|---:|---|")
    for w, m in j["windows"].items():
        tt = m["total"]
        yrs = " ".join(f"{y[2:]}:{b['pnl']:+.0f}/{b['n']}/{b['pf']:.2f}" for y, b in m["by_year"].items())
        lines.append(f"| {w} | {tt['pnl']:+.0f} | {tt['n']} | {tt['pf']:.2f} | {tt['dd']:.0f} | {m.get('worst_day', 0):.0f} | "
                     f"{m.get('sign_flip_p', '-')} | {m.get('ex_best_year', {}).get('pf', '-')} | {yrs} |")
    if "paired_target" in j:
        p = j["paired_target"]
        q = ", ".join(f"q{int(a * 100):02d} {v:+.0f}" for a, v in p["pnl_quantiles"])
        lines.append(f"\npaired vs baseline on target: P&L diff {p['pnl_diff']:+.0f}, P(diff ≤ 0) {p['p_le_0']:.3f}; {q}")
    lines.append("")
    for k, v in j["tests"].items():
        lines.append(f"- {'PASS' if v else 'fail'} — {k}")
    lines.append(f"\n**{'STRONG CONTENDER' if j['pass_all'] else 'not a contender'}**\n")
    return "\n".join(lines)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("specs", nargs="+")
    ap.add_argument("--baseline", default="research/lab/specs/base/v18.json")
    ap.add_argument("--windows", default="train,holdout,target")
    ap.add_argument("--md")
    ap.add_argument("--json")
    a = ap.parse_args()
    windows = a.windows.split(",")
    results = [judge(s, a.baseline, windows, len(a.specs)) for s in a.specs]
    text = f"# judge — {dt.datetime.now():%Y-%m-%d %H:%M} — {len(a.specs)} finalists (Šidák over {len(a.specs)})\n\n"
    text += "\n".join(fmt(j) for j in results)
    print(text)
    if a.md:
        Path(a.md).write_text(text)
    if a.json:
        Path(a.json).write_text(json.dumps(results, indent=1, default=str))


if __name__ == "__main__":
    main()
