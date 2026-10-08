#!/usr/bin/env bash
# rebuild the shared backtest binary while holding the research lock (other agents use it).
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
exec 9>"$ROOT/logs/.research.lock"
flock -w 14400 9 || { echo "lock timeout" >&2; exit 1; }
cd "$ROOT" && cargo build --release -p backtest 2>&1 | tail -2
flock -u 9
