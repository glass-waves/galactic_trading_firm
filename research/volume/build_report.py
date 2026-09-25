#!/usr/bin/env python3
"""render the volume-research report (2026-09-24) as one static HTML file.

usage: build_report.py SPEC.json OUT.html
SPEC: {"title", "date", "baseline": "iex_v18", "intro_html", "studies": [
        {"id", "title", "question_html", "design_html", "cells": [{"tag", "label", "params": {..}, "note"}],
         "breakdown_html", "findings": ["..."], "verdict_html", "verdict": "pass|fail|partial"}],
       "conclusion_html", "commands": {"tag": "cmd"}}
stats come straight from data/<tag>_<year>_trades.csv (same maths as research/entries/summarize.py).
"""
import csv, glob, html, json, sys
from collections import defaultdict

YEARS = ["2022", "2023", "2024", "2025", "2026"]


def load(tag):
    rows = []
    for y in YEARS:
        for f in glob.glob(f"data/{tag}_{y}_trades.csv"):
            for r in csv.DictReader(open(f)):
                if r["row_type"] == "trade":
                    r["year"] = y
                    r["pnl"] = float(r["pnl"])
                    rows.append(r)
    return rows


def stats(rows):
    if not rows:
        return None
    pnl = sum(r["pnl"] for r in rows)
    wins = [r["pnl"] for r in rows if r["pnl"] > 0]
    losses = [-r["pnl"] for r in rows if r["pnl"] < 0]
    pf = (sum(wins) / sum(losses)) if losses else float("inf")
    daily = defaultdict(float)
    for r in rows:
        daily[r["date"]] += r["pnl"]
    eq = peak = dd = 0.0
    for d in sorted(daily):
        eq += daily[d]; peak = max(peak, eq); dd = min(dd, eq - peak)
    return {"pnl": pnl, "n": len(rows), "win": len(wins) / len(rows), "pf": pf, "dd": dd}


def cell_stats(tag, subset=None):
    rows = load(tag)
    if subset:
        rows = [r for r in rows if subset(r)]
    out = {"5y": stats(rows)}
    for y in YEARS:
        out[y] = stats([r for r in rows if r["year"] == y])
    return out


def fmt_pnl(v):
    return f"{v:+,.0f}"


def pf_class(pf):
    if pf is None: return ""
    if pf >= 1.3: return "good"
    if pf >= 1.0: return "meh"
    return "bad"


def year_cell(s):
    if not s or s["n"] == 0:
        return "<td class='num muted'>—</td>"
    cls = "pos" if s["pnl"] > 0 else "neg"
    return f"<td class='num {cls}'>{fmt_pnl(s['pnl'])}<span class='sub'>n {s['n']} · PF {s['pf']:.2f}</span></td>"


def grid_table(study, base):
    params = []
    for c in study["cells"]:
        for k in c.get("params", {}):
            if k not in params: params.append(k)
    head = "<tr><th>cell</th>" + "".join(f"<th>{html.escape(p)}</th>" for p in params) + \
           "<th>5y P&amp;L</th><th>trades</th><th>win</th><th>PF</th><th>max DD</th>" + \
           "".join(f"<th>{y}</th>" for y in YEARS) + "<th>+yrs</th><th>Δn</th><th>ΔP&amp;L</th></tr>"
    body = []
    b5 = base["5y"]
    for c in study["cells"]:
        st = c["_stats"]; s = st["5y"]
        if not s:
            body.append(f"<tr><td>{html.escape(c['label'])}</td><td colspan={len(params)+13} class='muted'>no trades</td></tr>"); continue
        pos = sum(1 for y in YEARS if st[y] and st[y]["pnl"] > 0)
        tds = [f"<td class='lbl'>{html.escape(c['label'])}{('<span class=note>'+html.escape(c['note'])+'</span>') if c.get('note') else ''}</td>"]
        tds += [f"<td>{html.escape(str(c.get('params',{}).get(p,'')))}</td>" for p in params]
        tds += [f"<td class='num {'pos' if s['pnl']>0 else 'neg'}'><b>{fmt_pnl(s['pnl'])}</b></td>", f"<td class='num'>{s['n']}</td>",
                f"<td class='num'>{s['win']*100:.0f}%</td>", f"<td class='num {pf_class(s['pf'])}'><b>{s['pf']:.2f}</b></td>",
                f"<td class='num'>{s['dd']:,.0f}</td>"]
        tds += [year_cell(st[y]) for y in YEARS]
        tds += [f"<td class='num'>{pos}/5</td>", f"<td class='num'>{s['n']-b5['n']:+d}</td>", f"<td class='num'>{fmt_pnl(s['pnl']-b5['pnl'])}</td>"]
        body.append("<tr class='" + ("best" if c.get("best") else "") + "'>" + "".join(tds) + "</tr>")
    return f"<div class='scroll'><table class='grid'>{head}{''.join(body)}</table></div>"


def bars_svg(study, base):
    """5y P&L (bar) and PF (dot) per cell, tiny inline chart."""
    cells = [c for c in study["cells"] if c["_stats"]["5y"]]
    if not cells: return ""
    w, h, lp = 720, 40 + 22 * (len(cells) + 1), 150
    vals = [c["_stats"]["5y"]["pnl"] for c in cells] + [base["5y"]["pnl"]]
    lo, hi = min(0, min(vals)), max(0, max(vals))
    span = (hi - lo) or 1
    x0 = lp + (0 - lo) / span * (w - lp - 20)
    out = [f"<svg viewBox='0 0 {w} {h}' class='bars' role='img' aria-label='5-year P&L per cell'>"]
    rows = [("baseline " + base["tag"], base["5y"])] + [(c["label"], c["_stats"]["5y"]) for c in cells]
    for i, (lbl, s) in enumerate(rows):
        y = 20 + i * 22
        x1 = lp + (s["pnl"] - lo) / span * (w - lp - 20)
        cls = "bpos" if s["pnl"] >= 0 else "bneg"
        if i == 0: cls += " bbase"
        out.append(f"<text x='{lp-6}' y='{y+13}' text-anchor='end' class='lbl'>{html.escape(lbl[:26])}</text>")
        out.append(f"<rect x='{min(x0,x1):.1f}' y='{y}' width='{abs(x1-x0):.1f}' height='16' class='{cls}'/>")
        out.append(f"<text x='{max(x0,x1)+4:.1f}' y='{y+13}' class='val'>{fmt_pnl(s['pnl'])} · PF {s['pf']:.2f} · n {s['n']}</text>")
    out.append(f"<line x1='{x0:.1f}' y1='10' x2='{x0:.1f}' y2='{h-10}' class='zero'/></svg>")
    return "".join(out)


def split_table(study, base):
    """long-subset and short-subset stats per cell (5y + per-year P&L/PF)."""
    head = "<tr><th>cell</th><th>side</th><th>5y P&amp;L</th><th>trades</th><th>PF</th><th>max DD</th>" + "".join(f"<th>{y}</th>" for y in YEARS) + "<th>+yrs</th></tr>"
    body = []
    for c in study["cells"]:
        for side in ("long", "short"):
            st = cell_stats(c["tag"], lambda r, side=side: r["direction"].lower() == side); s = st["5y"]
            if not s: continue
            pos = sum(1 for y in YEARS if st[y] and st[y]["pnl"] > 0)
            body.append(f"<tr><td class='lbl'>{html.escape(c['label'])}</td><td>{side}</td><td class='num {'pos' if s['pnl']>0 else 'neg'}'><b>{fmt_pnl(s['pnl'])}</b></td><td class='num'>{s['n']}</td>"
                        f"<td class='num {pf_class(s['pf'])}'><b>{s['pf']:.2f}</b></td><td class='num'>{s['dd']:,.0f}</td>" + "".join(year_cell(st[y]) for y in YEARS) + f"<td class='num'>{pos}/5</td></tr>")
    return "<h3>Long side vs short side</h3><div class='scroll'><table class='grid'>" + head + "".join(body) + "</table></div>"


CSS = """
:root{--bg:#f7f7f5;--fg:#1c1c1a;--muted:#6b6b66;--line:#dedcd6;--card:#fff;--pos:#1f7a4d;--neg:#b3392f;--good:#e4f3ea;--meh:#fbf2d9;--bad:#f9e3e0;--acc:#2f5f8f}
@media(prefers-color-scheme:dark){:root{--bg:#141413;--fg:#ececea;--muted:#a3a39d;--line:#33332f;--card:#1d1d1b;--pos:#63c78f;--neg:#ef7f74;--good:#173324;--meh:#3a2f12;--bad:#3d1c18;--acc:#8ab4dd}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--fg);font:15px/1.5 system-ui,-apple-system,Segoe UI,Roboto,sans-serif}
main{max-width:1180px;margin:0 auto;padding:24px 16px 64px}h1{font-size:26px;margin:0 0 4px}h2{font-size:20px;margin:36px 0 8px;padding-top:12px;border-top:1px solid var(--line)}h3{font-size:16px;margin:20px 0 6px}
.meta{color:var(--muted);margin-bottom:18px}.card{background:var(--card);border:1px solid var(--line);border-radius:10px;padding:14px 16px;margin:12px 0}
.kpis{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:10px;margin:12px 0}.kpi{background:var(--card);border:1px solid var(--line);border-radius:10px;padding:10px 12px}.kpi .v{font-size:22px;font-weight:600}.kpi .k{color:var(--muted);font-size:12px}
.scroll{overflow-x:auto;margin:10px 0}table.grid{border-collapse:collapse;font-size:13px;min-width:900px}table.grid th{position:sticky;top:0;background:var(--card);text-align:left;padding:6px 8px;border-bottom:2px solid var(--line);white-space:nowrap}
table.grid td{padding:6px 8px;border-bottom:1px solid var(--line);vertical-align:top;white-space:nowrap}td.num{text-align:right;font-variant-numeric:tabular-nums}td.lbl{font-weight:600}.sub{display:block;color:var(--muted);font-size:11px;font-weight:400}
.pos{color:var(--pos)}.neg{color:var(--neg)}td.good{background:var(--good)}td.meh{background:var(--meh)}td.bad{background:var(--bad)}tr.best td{outline:2px solid var(--acc);outline-offset:-2px}.note{display:block;color:var(--muted);font-size:11px;font-weight:400}.muted{color:var(--muted)}
.verdict{border-left:5px solid var(--acc);padding:10px 14px;border-radius:6px;background:var(--card)}.verdict.pass{border-color:var(--pos)}.verdict.fail{border-color:var(--neg)}.verdict.partial{border-color:#c98a1a}
svg.bars{width:100%;height:auto;margin:8px 0}svg .lbl{font-size:11px;fill:var(--fg)}svg .val{font-size:11px;fill:var(--muted)}svg .bpos{fill:var(--pos);opacity:.8}svg .bneg{fill:var(--neg);opacity:.8}svg .bbase{opacity:.45}svg .zero{stroke:var(--muted);stroke-width:1}
pre{background:var(--card);border:1px solid var(--line);border-radius:8px;padding:10px 12px;overflow-x:auto;font-size:12px}ul{padding-left:20px}li{margin:4px 0}code{font-size:.92em}
details summary{cursor:pointer;color:var(--acc)}
"""


def main():
    spec = json.load(open(sys.argv[1])); out = sys.argv[2]
    base_tag = spec.get("baseline", "iex_v18")
    base = cell_stats(base_tag); base["tag"] = base_tag
    for st in spec["studies"]:
        for c in st["cells"]:
            c["_stats"] = cell_stats(c["tag"])
    parts = [f"<!doctype html><html lang='en'><head><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'>"
             f"<title>{html.escape(spec['title'])}</title><style>{CSS}</style></head><body><main>"]
    parts.append(f"<h1>{html.escape(spec['title'])}</h1><div class='meta'>{html.escape(spec['date'])} · replay on the IEX 1-minute cache · honest costs (3 bps + $0.005 per leg) · research sizing 36 % · three concurrent positions · baseline <code>{base_tag}</code></div>")
    b = base["5y"]
    parts.append("<div class='kpis'>" + "".join(
        f"<div class='kpi'><div class='v'>{v}</div><div class='k'>{k}</div></div>" for k, v in [
            ("baseline 5y P&L", fmt_pnl(b["pnl"])), ("baseline trades", b["n"]), ("baseline PF", f"{b['pf']:.2f}"),
            ("baseline positive years", f"{sum(1 for y in YEARS if base[y] and base[y]['pnl']>0)}/5"),
            ("bar to pass", "PF ≥ 1.3 · ≥ 4/5 years · more trades")]) + "</div>")
    parts.append(spec.get("intro_html", ""))
    for st in spec["studies"]:
        parts.append(f"<h2 id='{st['id']}'>{html.escape(st['title'])}</h2>")
        parts.append(f"<div class='card'><b>Question.</b> {st.get('question_html','')}<br><b>Design.</b> {st.get('design_html','')}</div>")
        parts.append(bars_svg(st, base))
        parts.append(grid_table(st, base))
        if st.get("split_direction"): parts.append(split_table(st, base))
        if st.get("breakdown_html"): parts.append(st["breakdown_html"])
        if st.get("findings"): parts.append("<h3>What the grid says</h3><ul>" + "".join(f"<li>{f}</li>" for f in st["findings"]) + "</ul>")
        parts.append(f"<div class='verdict {st.get('verdict','')}'><b>Verdict.</b> {st.get('verdict_html','')}</div>")
    parts.append("<h2 id='conclusion'>Conclusion and recommendation</h2>" + spec.get("conclusion_html", ""))
    if spec.get("commands"):
        parts.append("<h2 id='repro'>Reproduce</h2><details><summary>exact commands per cell</summary><pre>" +
                     "\n".join(f"# {html.escape(t)}\n{html.escape(c)}" for t, c in spec["commands"].items()) + "</pre></details>")
    parts.append("</main></body></html>")
    open(out, "w").write("".join(parts))
    print(f"wrote {out} ({len(''.join(parts))//1024} KB)")


if __name__ == "__main__":
    main()
