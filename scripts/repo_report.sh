#!/usr/bin/env bash
# what the repository weighs, for the monthly hygiene pass (docs/plans/2026-09-26_pipeline_and_books.md §10):
# tracked bytes by top-level and second-level directory, the 15 largest tracked files, and the
# untracked / ignored cache directories on disk. read-only. the EOD routine runs it on the first
# trading day of each month and proposes deletions; it never deletes anything.
# usage: scripts/repo_report.sh
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
TMP="$(mktemp)"; trap 'rm -f "$TMP"' EXIT
# bytes<TAB>path for every tracked file present in the working tree
git ls-files -z | xargs -0 stat -c $'%s\t%n' 2>/dev/null > "$TMP"
TOTAL=$(awk -F'\t' '{s+=$1} END{printf "%.0f", s/1024}' "$TMP")
echo "== repository: $(wc -l < "$TMP") tracked files, ${TOTAL} KB in the working tree; .git $(du -sk .git | cut -f1) KB; HEAD $(git rev-parse --short HEAD) =="

echo; echo "== tracked KB by top-level directory =="
awk -F'\t' '{ n=split($2,a,"/"); d=(n>1)?a[1]"/":"(root files)"; s[d]+=$1; c[d]++ }
     END { for (d in s) printf "%9.0f KB %5d files  %s\n", s[d]/1024, c[d], d }' "$TMP" | sort -rn

echo; echo "== tracked KB by second-level directory (top 25) =="
awk -F'\t' '{ n=split($2,a,"/"); if (n>2) { d=a[1]"/"a[2]"/"; s[d]+=$1; c[d]++ } }
     END { for (d in s) printf "%9.0f KB %5d files  %s\n", s[d]/1024, c[d], d }' "$TMP" | sort -rn | head -25

echo; echo "== 15 largest tracked files =="
sort -t$'\t' -k1,1rn "$TMP" | head -15 | awk -F'\t' '{ printf "%9.0f KB  %s\n", $1/1024, $2 }'

echo; echo "== untracked / ignored caches on disk (KB) =="
for d in data/bars data/bars_iex data/labels data/live logs target target2 cockpit/node_modules cockpit/.next research/*/daily research/*/cache; do
    [[ -d "$d" ]] || continue
    if git ls-files --error-unmatch "$d" >/dev/null 2>&1; then tag="(partly tracked)"; else tag=""; fi
    printf "%9s KB  %s %s\n" "$(du -sk "$d" 2>/dev/null | cut -f1)" "$d" "$tag"
done
echo; echo "== untracked (not ignored) files under data/ research/ docs/ scripts/ =="
git ls-files -o --exclude-standard -z -- data research docs scripts 2>/dev/null | xargs -0 -r du -k 2>/dev/null | sort -rn | head -10 | awk '{ printf "%9s KB  %s\n", $1, $2 }'
echo; echo "(policy: caches are never tracked; a closed research round keeps its report, scripts and best/candidate patches; generated docs/reports/*.html and docs/pipeline/status.md stay. propose, never delete.)"
