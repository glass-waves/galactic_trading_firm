#!/usr/bin/env bash
# repair a sweep leg after rate-limit failures: find weekdays that are missing OR
# incomplete (fewer ticker summary rows than the leg's ticker count), strip those
# days from data/<tag>_<year>_trades.csv, and rerun them sequentially.
# usage: ./scripts/fill_sweep_gaps.sh <tag> <start> <end> [extra backtest args...]
#        (with no extra args, data/<tag>.args is used if present)
set -uo pipefail
TAG="$1"; START="$2"; END="$3"; shift 3
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [[ $# -eq 0 && -f "$ROOT/data/$TAG.args" ]]; then
    # shellcheck disable=SC2207
    EXTRA=($(cat "$ROOT/data/$TAG.args"))
else
    EXTRA=("$@")
fi
bad=$(python3 - "$TAG" "$START" "$END" "$ROOT" <<'PY'
import sys, csv, glob, datetime, collections
tag, start, end, root = sys.argv[1:5]
import re
files=sorted(f for f in glob.glob(f"{root}/data/{tag}_*_trades.csv")
             if re.fullmatch(rf"{re.escape(root)}/data/{re.escape(tag)}_\d{{4}}_trades\.csv", f))
rows_by_file={}; per_day=collections.Counter()
for f in files:
    rows=list(csv.DictReader(open(f)))
    rows_by_file[f]=rows
    for r in rows:
        if r.get("row_type")=="summary": per_day[r["date"]]+=1
expected=max(per_day.values()) if per_day else 0
d=datetime.date.fromisoformat(start); e=datetime.date.fromisoformat(end); bad=[]
while d<=e:
    s=d.isoformat()
    if d.weekday()<5 and 0<per_day.get(s,0)<expected: bad.append(s)      # incomplete
    if d.weekday()<5 and per_day.get(s,0)==0: bad.append(s)             # missing
    d+=datetime.timedelta(days=1)
badset=set(bad)
# strip incomplete days so the rerun does not double count
for f,rows in rows_by_file.items():
    keep=[r for r in rows if r.get("date") not in badset]
    if len(keep)!=len(rows):
        with open(f,"w",newline="") as fh:
            w=csv.DictWriter(fh, fieldnames=list(rows[0].keys())); w.writeheader(); w.writerows(keep)
print("\n".join(bad))
PY
)
if [[ -z "$bad" ]]; then echo "[$TAG] no gaps" >&2; exit 0; fi
echo "[$TAG] repairing $(echo "$bad" | wc -l | tr -d ' ') days (holidays will skip again)" >&2
for d in $bad; do
    DAY_SLEEP="${DAY_SLEEP:-2}" "$ROOT/scripts/backtest_range.sh" "$TAG" "$d" "$d" "${EXTRA[@]}"
done
