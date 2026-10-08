#!/usr/bin/env bash
# block until research/timefilter cell <cell>'s sweep has finished (its main log has the fixup line).
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
until grep -q "fixup:" "$ROOT/logs/sweeps/tf_$1_main.log" 2>/dev/null; do sleep 15; done
