#!/usr/bin/env bash
# pass 2 (2026-09-27, after the replay's session-VWAP parity fix): reproduction first, then the
# eight most informative cells of pass 1, then the rest of the grid.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
Q="$ROOT/research/stress/run_queue.sh"
"$Q" am_only
"$Q" s10 s15 s15_core s15_core_novpin s15_novpin c10n15 n15 s15_core_only
"$Q" s05 n25 s20 s15_late s15_fall s15_h3 s15_se10
MAX_POS=0.54 TAG_SUFFIX=_big EXTRA="--patch-json $ROOT/research/stress/size_s15_x1.5.json" "$Q" s15_core
MAX_POS=0.54 TAG_SUFFIX=_big EXTRA="--patch-json $ROOT/research/stress/size_s15_x1.5.json" "$Q" s15
