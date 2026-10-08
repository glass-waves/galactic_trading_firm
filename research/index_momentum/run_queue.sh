#!/usr/bin/env bash
# run cells one after another (each takes the research lock itself). usage: run_queue.sh cell ...
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
for c in "$@"; do "$ROOT/research/index_momentum/run_cell.sh" "$c" 2>&1 | grep -E "start|done \(|fixup|timeout" ; done
