# frontier desk — fitting the individually-passing levers together (in progress)

Generator: `research/lab/patches/frontier/make.py NAME key=value…` writes `patches/frontier/NAME.json` + `specs/frontier/NAME.json`
(keys: am_end, k, pm_end, t1h_am/t1h_pm, vpin, c5/c1, core_am/core_pm, band, tier/tier_min, stress, pdl, se, noadx).
Scaffold `frontier/scaffold` (v18 re-created as `_am` windows) = `base/v18` on 2025h1 trade for trade (45/45, P&L diff 0).
`frontier/sq05` is byte-equivalent to `research/timefilter/spy_sqrt_band.patch.json`; `frontier/sq05_1h15` reproduces cand_45 / row 21
on 2024h2_25h1 (+392 / 65 / PF 1.54).

## stage 1 — one factor at a time on 2024h2_25h1 (legacy, ADX on)
(table filled at the end)
