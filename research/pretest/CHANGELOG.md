# pretest changelog

verdict changes only, newest first. appended by scripts/pretest_nightly.sh; the cell's own
reasoning is in `research/pretest/results/<cell>.json` (`verdict_text`).

- 2026-10-19 `c13_letf_close_momentum`: new -> dead — best tradable on SPY = large-move last-30-min momentum: -4.2 bps/trade, 0/5 years positive (needs 4); PF 0.73 (needs > 1.3); LOYO -44 (2/5)
- 2026-10-26 `c15_tom_day2_window`: new -> pass-pretest — long open->close on session k of the month on QQQ: PF 1.73, 4/5 years -> engine study worth it
- 2026-10-30 `c05_earnings_gap`: pass-pretest -> fragile — fade the gap from the open on AAPL+AMZN+MSFT+NVDA: grid mode=fade,exit=390: PF 1.36, 3/5 years; LOYO +392 (3/5) -> engine study worth it | downgraded (controls): LOYO +392 (3/5) fails the 4/5 bar
- 2026-10-30 `c11_tom_day_of_cycle`: pass-pretest -> fragile — long open->close on session k of the month on QQQ: PF 1.73, 4/5 years -> engine study worth it | downgraded (controls): LOYO +154 (3/5) fails the 4/5 bar
- 2026-10-30 `c15_tom_day2_window`: pass-pretest -> fragile — long open->close on session k of the month on QQQ: PF 1.73, 4/5 years -> engine study worth it | downgraded (controls): LOYO +154 (3/5) fails the 4/5 bar
