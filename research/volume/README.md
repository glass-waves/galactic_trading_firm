# research/volume — the volume report round (closed 2026-09-25)

what this directory holds now:
- write-ups: `pctile.md` (VPIN percentile study A), `tiered.md` (tiered sizing, study B), `rs_long.md` (relative-strength long), `regime_long.md` (long regime cells), `walk_forward.md` (+ `walk_forward.py`, `walk_forward_{pf,pnl,sharpe}12.txt`).
- `report_spec.json` + `build_report.py` → `docs/reports/research_2026-09-24_volume.html` (the sweep csvs it reads live under `data/`, not here).
- patches a later step consumes: `rl_*.json` (pipeline seeds for the bear-bounce / regime long candidates), `smoke_*.json`, and the best cells `vp_w1950_p0.8.json`, `tier_l0.12_m0.67.json`, `rs_win_x0.3.json`, `rs_solo_x0.5.json`.
- analysis scripts: `displacement.py`, `frontrun.py`, `tier2_breakdown.py`, `tiered_tables.py`.

pruned in the commit following 1f196a4d019c954ca68820d869205eeb150fe53a (`docs/plans/2026-09-26_pipeline_and_books.md` §10): every other grid cell — `vp_w{390,1170,1950}_p*.json`, `tier_l*_m*.json`, `tier3_*.json`, `rs_{novpin,pair,solo35}_x*.json`, the remaining `rs_solo_x*` / `rs_win_x*` cells. the md write-ups and `report_spec.json` still name them as the commands that produced the tables; `git show 1f196a4:research/volume/<file>` recovers any of them.
