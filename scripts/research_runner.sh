#!/usr/bin/env bash
# run approved entries from docs/research_queue.md with the local backtest engine and write the
# result back under the entry. one spoke of the desktop <-> routine wheel: the eod routine
# proposes, the human flips `status: proposed` to `status: approved`, this runs overnight.
#
# entry format (see docs/routines/eod-report.md §6). the `command:` line must be a single
# shell invocation runnable from the repo root; multi-line commands continue on indented lines.
# results: per-year P&L / trades / PF from research/entries/summarize.py on every tag the
# command produced (tags are inferred from `run_cached_sweep.sh <tag>`), plus stderr tail.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
Q="docs/research_queue.md"
[[ -f "$Q" ]] || exit 0
git pull -q --ff-only 2>/dev/null || true
set -a; source .env; set +a

python3 - "$Q" <<'EOF'
import re, subprocess, sys, datetime, shlex
q = sys.argv[1]
text = open(q).read()
blocks = re.split(r'(?m)^(?=### RQ-)', text)
changed = False
for i, b in enumerate(blocks):
    m = re.match(r'### (RQ-\d+)[^\n]*status:\s*approved', b)
    if not m:
        continue
    rq = m.group(1)
    cm = re.search(r'(?ms)^command:\s*(.+?)^(?=accept if:|result:|$)', b)
    if not cm or 'needs code' in cm.group(1):
        continue
    cmd = ' '.join(l.strip() for l in cm.group(1).strip().splitlines())
    print(f"[research_runner] {rq}: {cmd[:120]}")
    b = re.sub(r'status:\s*approved', 'status: running', b, count=1)
    open(q, 'w').write(''.join(blocks[:i]) + b + ''.join(blocks[i+1:]))
    t0 = datetime.datetime.now()
    r = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=6 * 3600)
    tags = re.findall(r'run_cached_sweep\.sh\s+(\S+)', cmd)
    summary = ''
    if tags:
        s = subprocess.run(['python3', 'research/entries/summarize.py', *tags], capture_output=True, text=True)
        summary = s.stdout.strip()
    err = "\n".join(l for l in r.stderr.strip().splitlines()[-5:] if not l.startswith("DATA_QUALITY"))
    status = 'done' if r.returncode == 0 else 'done (command exit %d)' % r.returncode
    result = (f"result: ran {t0:%Y-%m-%d %H:%M} PT, {int((datetime.datetime.now()-t0).total_seconds()/60)} min\n"
              f"```\n{summary or r.stdout.strip()[-3000:]}\n```\n" + (f"stderr: {err}\n" if err else ''))
    b = re.sub(r'status:\s*running', f'status: {status}', b, count=1)
    if re.search(r'(?m)^result:', b):
        b = re.sub(r'(?ms)^result:.*?(?=^### |\Z)', result, b, count=1)
    else:
        b = b.rstrip('\n') + '\n' + result
    blocks[i] = b
    open(q, 'w').write(''.join(blocks))
    changed = True
sys.exit(0 if changed else 3)
EOF
rc=$?
if [[ $rc -eq 0 ]]; then
    git add "$Q" data/*.args 2>/dev/null
    git commit -q -m "research: results for approved queue entries $(date +%F)" && git push -q origin HEAD || echo "[research_runner] commit/push failed" >&2
fi
exit 0
