#!/usr/bin/env bash
# nightly stage-0 pretest run (deploy/systemd/pretest.timer, 21:15 PT weekdays, after the 20:30
# pipeline job): run every registered cell in research/pretest/cells.py, regenerate
# research/pretest/results/summary.md, append a dated line to research/pretest/CHANGELOG.md for
# every cell whose verdict changed since the last run, and commit+push only summary.md,
# CHANGELOG.md and research/edge_matrix.md. mirrors scripts/research_runner.sh's git pattern
# (shares its lock: no engine sweep here, but the research lock still serializes research jobs
# on this machine).
# env: PRETEST_NO_GIT=1 skips the commit/push (testing).
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
mkdir -p "$ROOT/logs"; exec 9>"$ROOT/logs/.research.lock"; flock -w 14400 9 || { echo "[pretest] could not get the research lock in 4 h" >&2; exit 0; }
git pull -q --ff-only 2>/dev/null || true

RESULTS="research/pretest/results"
CHANGELOG="research/pretest/CHANGELOG.md"
mkdir -p "$RESULTS"
[[ -f "$CHANGELOG" ]] || printf '%s\n' "# pretest changelog" "" \
    "verdict changes only, newest first. appended by scripts/pretest_nightly.sh; the cell's own" \
    "reasoning is in \`research/pretest/results/<cell>.json\` (\`verdict_text\`)." "" > "$CHANGELOG"

verdict_of() {
    python3 -c "
import json, sys
try:
    d = json.load(open(sys.argv[1]))
    print(d.get('verdict', '?') + '|' + d.get('verdict_text', '').replace('\n', ' '))
except Exception:
    print('?|')
" "$1" 2>/dev/null
}

declare -A BEFORE
shopt -s nullglob
for f in "$RESULTS"/*.json; do
    id="$(basename "$f" .json)"
    BEFORE["$id"]="$(verdict_of "$f")"
    BEFORE["$id"]="${BEFORE[$id]%%|*}"
done

echo "[pretest] running research/pretest/harness.py"
python3 research/pretest/harness.py
rc=$?

TODAY="$(date +%F)"
changed=0
for f in "$RESULTS"/*.json; do
    id="$(basename "$f" .json)"
    after="$(verdict_of "$f")"
    av="${after%%|*}"; text="${after#*|}"
    before="${BEFORE[$id]:-new}"
    if [[ "$av" != "$before" ]]; then
        echo "- $TODAY \`$id\`: $before -> $av — $text" >> "$CHANGELOG"
        changed=1
    fi
done
[[ $changed -eq 1 ]] && echo "[pretest] CHANGELOG: $changed cell(s) changed verdict" || echo "[pretest] no verdict changes"

if [[ "${PRETEST_NO_GIT:-0}" == "1" ]]; then
    echo "[pretest] PRETEST_NO_GIT=1: skipping commit/push"
    exit "$rc"
fi

git add "$RESULTS/summary.md" "$CHANGELOG" research/edge_matrix.md 2>/dev/null
if ! git diff --cached --quiet; then
    git commit -q -m "pretest: nightly run $TODAY" && git push -q origin HEAD || echo "[pretest] commit/push failed" >&2
else
    echo "[pretest] nothing to commit"
fi
exit "$rc"
