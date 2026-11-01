#!/usr/bin/env python3
"""run a lab spec on only the session days its event_calendar flags (all other days provably 0 trades:
the only enabled entry window requires the calendar flag). identical cache keys, summary and ledger
format to `lab.py run`; the summary uses the full window's session list."""
import sys, json, csv, datetime as dt, os, argparse
sys.path.insert(0, "/home/dylmet/Projects/galactic_trading_firm/scripts/lab")
import lab

ap = argparse.ArgumentParser(); ap.add_argument("spec"); ap.add_argument("--window", default="train")
ap.add_argument("--quiet", action="store_true")
a = ap.parse_args()
spec = lab.load_spec(a.spec)
start, end = lab.window_bounds(a.window)
lab.check_reveal(a.window, start, end)
days = lab.sessions(lab.ROOT / spec["bars_dir"], start, end)
flag = set()
for p in spec["patches"]:
    pj = json.loads((lab.ROOT / p).read_text())
    for ind in pj.get("indicators", []):
        if ind["indicator_type"] == "event_calendar":
            assert not ind["params"].get("before_days") and not ind["params"].get("after_days")
            flag |= set(ind["params"]["dates"])
assert flag, "no event calendar in spec"
ev = [d for d in days if d.isoformat() in flag]
rows, info = lab.run_spec(spec, ev, quiet=True)
m = lab.summarize(rows, days, start, end)
m["event_days_run"] = len(ev)
lab.print_summary(spec["name"], m, info["errors"])
safe = spec["name"].replace("/", "__")
lab.RUNS.mkdir(parents=True, exist_ok=True)
(lab.RUNS / f"{safe}.{a.window}.json").write_text(json.dumps(
    {"spec": {k: v for k, v in spec.items() if not k.startswith("_")}, "spec_path": spec["_path"],
     "fingerprint": info["fingerprint"], "window": a.window, "metrics": m, "errors": info["errors"],
     "runner": "event-days-only (tom-event desk)"}, indent=1))
with open(lab.RUNS / f"{safe}.{a.window}.trades.csv", "w", newline="") as fh:
    w = csv.writer(fh); w.writerow(lab.HEADER.split(","))
    for r in rows:
        w.writerow([r.get(c, "") for c in lab.HEADER.split(",")])
lab.ledger_append({"ts": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"),
                   "agent": os.environ.get("LAB_AGENT", "coordinator"), "name": spec["name"],
                   "spec_path": spec["_path"], "fingerprint": info["fingerprint"], "window": a.window,
                   "note": spec.get("note", "") + " [event-days-only runner]", **lab.compact(m)})
