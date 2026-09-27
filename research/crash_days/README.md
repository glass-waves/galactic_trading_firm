# crash-days hindsight study (2026-09-27)

report: `2026-09-27_crash_days_hindsight.md`, built by `analysis.py` (numpy + csv/json, no rust) from
`data/bars_iex/*.csv`, `research/swing/daily/*.csv`, `crates/indicators/src/custom/event_calendar.rs`
(FOMC dates), `research/entries/data/earnings_*.txt` and `data/iex_v18_<year>_trades.csv`; read-only,
data-only — no strategy or config changes. Verdict: no new standalone strategy study warranted; confirms
and extends `research/stress/2026-09-26_stress_mode.md`'s rejected intraday-threshold trigger from the
precursor/anatomy side rather than contradicting it.
