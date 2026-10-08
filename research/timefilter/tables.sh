#!/usr/bin/env bash
# every report table for the study's swept cells: grid, LOYO (unconstrained and volume-constrained), cards.
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"; cd "$ROOT" || exit 1
CELLS=(tf_x1300 tf_x1530 tf_r15_15 tf_c_r10 tf_sq04 tf_sq05 tf_sq06 tf_c_sq05 tf_sq05_1h05 tf_sq05_1h10
       tf_sq05_1h15 tf_sq05_x1300 tf_sq05_x1530 tf_sq04_1100 tf_sq05_1100 tf_sq06_1100 tf_c_sq05_1100)
A=research/timefilter/analyze.py
case "${1:-all}" in
  grid) python3 $A grid tf_am "${CELLS[@]}" ;;
  loyo) python3 $A loyo "${CELLS[@]}"; echo; python3 $A loyo_vol "${CELLS[@]}" ;;
  card) shift; python3 $A card "$@" ;;
  all) "$0" grid; echo; "$0" loyo ;;
esac
