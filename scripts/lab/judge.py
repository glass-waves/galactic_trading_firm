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


R2 = {  # research/lab/ROUND2.md "success tests" (+ the pre-reveal amendment to A's pooled clause)
    "t_train_p": 0.05, "t_train_pf": 1.6, "t_train_n": 250, "t_year_floor": -300.0, "t_exb_pf": 1.4,
    "t_hold_pf": 1.5, "t_hold_p": 0.25, "t_target_pf": 1.4, "t_target_n": 150, "t_dd": -600.0, "t_worst_day": -300.0,
    "a_added_n": 60, "a_added_pf": 1.3, "a_added_worst_year": -200.0, "a_pooled_pf": 1.5, "a_pooled_dd_mult": 1.5, "a_corr": 0.3,
}


def judge_r2(ref: str, base_ref: str, n_finalists: int) -> dict:
    """round 2: a tweak (T) must beat the base on train and hold on the holdout; an additive union "base+book" (A) is
    judged on the book alone, the pooled record and the correlation. every window is paired against the base."""
    kind = "A" if "+" in ref else "T"
    out = {"ref": ref, "kind": kind, "windows": {}, "tests": {}}
    for w in ("train", "holdout", "target"):
        start, end = lab.window_bounds(w)
        lab.check_reveal(w, start, end)
        days = lab.sessions(lab.ROOT / "data" / "bars_iex", start, end)
        dl = [d.isoformat() for d in days]
        name, rows, books = rows_for(ref, start, end)
        out["name"], out["books"] = name, books
        _, brows, _ = rows_for(base_ref, start, end)
        e = {"m": lab.summarize(rows, days, start, end), "base": lab.summarize(brows, days, start, end),
             "paired": pm.paired_day_bootstrap(rows, brows, session_list=dl, n=5000, seed=0)}
        if kind == "A":
            _, krows = lab._rows_for(ref.split("+")[-1], start, end)
            e["book"] = lab.summarize(krows, days, start, end)
            e["corr"] = pm.pearson_corr(pm.daily_series(krows, dl), pm.daily_series(brows, dl))
        out["windows"][w] = e
    t, W = out["tests"], out["windows"]
    tr, ho, ta = W["train"], W["holdout"], W["target"]
    alpha = sidak_alpha(R2["t_train_p"], n_finalists)
    if kind == "T":
        m = tr["m"]
        t[f"T1 train: paired P<{alpha:.3f}, PF≥1.6, n≥250, yr≥−300, ex-best PF≥1.4"] = (
            tr["paired"]["p_le_0"] < alpha and m["total"]["pf"] >= R2["t_train_pf"] and m["total"]["n"] >= R2["t_train_n"]
            and all(b["pnl"] >= R2["t_year_floor"] for b in m["by_year"].values())
            and m.get("ex_best_year", {}).get("pf", 0) >= R2["t_exb_pf"])
        h = ho["m"]
        t["T2 holdout: P&L>0, PF≥1.5, paired P(≤0)<0.25"] = (h["total"]["pnl"] > 0 and h["total"]["pf"] >= R2["t_hold_pf"]
                                                            and ho["paired"]["p_le_0"] < R2["t_hold_p"])
        g = ta["m"]
        t["T3 target: PF≥1.4, n≥150"] = g["total"]["pf"] >= R2["t_target_pf"] and g["total"]["n"] >= R2["t_target_n"]
        t["T4 risk: target dd≥−600, worst day≥−300 (train, target)"] = (g["total"]["dd"] >= R2["t_dd"]
                                                                         and min(m.get("worst_day", 0), g.get("worst_day", 0)) >= R2["t_worst_day"])
    else:
        k = tr["book"]
        t["A1 train book: n≥60, PF≥1.3, worst yr≥−200"] = (k["total"]["n"] >= R2["a_added_n"] and k["total"]["pf"] >= R2["a_added_pf"]
                                                           and all(b["pnl"] >= R2["a_added_worst_year"] for b in k["by_year"].values()))
        t["A2 train pooled: PF≥1.5, dd≥1.5×base dd"] = (tr["m"]["total"]["pf"] >= R2["a_pooled_pf"]
                                                        and tr["m"]["total"]["dd"] >= R2["a_pooled_dd_mult"] * tr["base"]["total"]["dd"])
        t["A3 train: daily corr<0.3"] = abs(tr["corr"]) < R2["a_corr"]
        t["A4 holdout: book P&L>0"] = ho["book"]["total"]["pnl"] > 0
        t["A5 target pooled: PF≥1.4, n≥150"] = ta["m"]["total"]["pf"] >= R2["t_target_pf"] and ta["m"]["total"]["n"] >= R2["t_target_n"]
    out["pass_all"] = all(t.values())
    return out


def fmt_r2(j: dict) -> str:
    lines = [f"### {j['name']}  [{j['kind']}]" + ("  (books pooled — dollars assume 2× capital)" if j["books"] > 1 else "")]
    lines.append("| window | P&L | n | PF | dd | worst day | by year | base P&L / n / PF / dd | paired diff · P(≤0) | book alone | corr |")
    lines.append("|---|---:|---:|---:|---:|---:|---|---|---|---|---:|")
    for w, e in j["windows"].items():
        m, b, p = e["m"]["total"], e["base"]["total"], e["paired"]
        yrs = " ".join(f"{y[2:]}:{v['pnl']:+.0f}/{v['n']}/{v['pf']:.2f}" for y, v in e["m"]["by_year"].items())
        book = (f"{e['book']['total']['pnl']:+.0f} / {e['book']['total']['n']} / {e['book']['total']['pf']:.2f} (worst yr "
                f"{min(v['pnl'] for v in e['book']['by_year'].values()):+.0f})") if "book" in e else "—"
        corr = f"{e['corr']:+.2f}" if "corr" in e else "—"
        lines.append(f"| {w} | {m['pnl']:+.0f} | {m['n']} | {m['pf']:.2f} | {m['dd']:.0f} | {e['m'].get('worst_day', 0):.0f} | {yrs} | "
                     f"{b['pnl']:+.0f} / {b['n']} / {b['pf']:.2f} / {b['dd']:.0f} | {p['pnl_diff']:+.0f} · {p['p_le_0']:.3f} | {book} | {corr} |")
    lines.append("")
    for k, v in j["tests"].items():
        lines.append(f"- {'PASS' if v else 'fail'} — {k}")
    lines.append(f"\n**{'PASSES ALL' if j['pass_all'] else 'does not pass'}**\n")
    return "\n".join(lines)


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
    ap.add_argument("--round2", action="store_true", help="apply research/lab/ROUND2.md tests T/A instead of PLAN.md 1–6")
    a = ap.parse_args()
    windows = a.windows.split(",")
    if a.round2:
        results = [judge_r2(s, a.baseline, len(a.specs)) for s in a.specs]
        text = f"# judge (round 2) — {dt.datetime.now():%Y-%m-%d %H:%M} — {len(a.specs)} finalists (Šidák over {len(a.specs)}) vs {a.baseline}\n\n"
        text += "\n".join(fmt_r2(j) for j in results)
    else:
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
