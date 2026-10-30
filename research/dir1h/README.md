# dir1h — a directional 1h sum (RQ-13)

`dir1h.patch.json` replaces row 12's 1h `supertrend` and `adx` instances in place (same ids, weights,
timescales) with `standard: true` / `signed: true` — the corrected modes added 2026-10-29 after the
engine-numerics audit found the legacy supertrend never turns bearish and the legacy ADX is scored
direction-free. defaults are unchanged, so the record is untouched until a candidate opts in.
run: see docs/research_queue.md RQ-13. verify on one day with `--dump-ticks` that `is_bullish` takes both values.
