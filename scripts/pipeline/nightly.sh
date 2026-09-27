#!/usr/bin/env bash
# nightly pipeline run (deploy/systemd/pipeline.timer, 20:30 PT weekdays, after the research runner):
# advance every due transition (at most two backtest sweeps), regenerate docs/pipeline/status.md,
# commit ONLY the status page and the candidates' sweep args, push. mirrors scripts/research_runner.sh.
# sweeps and fetches inside pipeline.py take logs/.research.lock themselves.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"; cd "$ROOT"
mkdir -p logs
git pull -q --ff-only 2>/dev/null || true
python3 scripts/pipeline/pipeline.py advance --max-backtests "${PIPELINE_MAX_BACKTESTS:-2}"
rc=$?
python3 scripts/pipeline/pipeline.py report > /dev/null || true
shopt -s nullglob
git add docs/pipeline/status.md
args=(data/cand_*.args)
[[ ${#args[@]} -gt 0 ]] && git add -f "${args[@]}"   # data/*.args is gitignored; the plan wants the candidates' args tracked
if ! git diff --cached --quiet; then
    git commit -q -m "pipeline: nightly advance $(date +%F)" && git push -q origin HEAD || echo "[pipeline] commit/push failed" >&2
fi
exit $rc
