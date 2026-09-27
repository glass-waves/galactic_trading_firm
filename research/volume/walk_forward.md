# walk-forward refit of the whole book (2026-09-26)

question: is the 2023/2024 weakness (and the long book's failure) something a rolling refit
"to current conditions" can adapt to? if yes, fitting on recent data is legitimate; if no,
fitting on 2026 alone is curve-fitting.

design (`research/volume/walk_forward.py`, no new sweeps): candidate book = one of 12 short
cells (SPY band ±0.2/±0.3 % × VPIN floor none/0.12/0.15/0.18/0.217/0.26, all on the IEX cache)
plus one of 5 long options (off / v17-filtered longs all days / longs only when SPY < SMA50 /
SPY 20-day return < 0 / SPY > SMA20). every quarter from 2023Q1 the combo with the best
trailing-window objective (min trade count) is chosen and traded out of sample for the quarter.
60 combos, so the selector has plenty of freedom.

| selector | OOS 2023–2026Q3 | trades | PF | max DD | Sharpe |
|---|---|---|---|---|---|
| **static v18 (no refit)** | **+589** | 316 | **1.16** | **−526** | **0.86** |
| static v18 + bear-bounce longs | +864 | 524 | 1.14 | −1,041 | 0.80 |
| refit quarterly, 12 m, best Sharpe | +602 | 337 | 1.15 | −549 | 0.85 |
| refit quarterly, 12 m, best PF | +581 | 644 | 1.08 | −1,380 | 0.43 |
| refit quarterly, 12 m, best P&L | +61 | 1,624 | 1.00 | −2,135 | 0.02 |
| refit quarterly, 24 m, best PF | +509 | 325 | 1.13 | −737 | 0.74 |
| refit quarterly, 24 m, best Sharpe | +346 | 318 | 1.09 | −752 | 0.51 |
| refit quarterly, 6 m, best PF | −246 | 811 | 0.97 | −1,893 | −0.17 |

reading:
- no refit rule beats the static book out of sample; the best one (Sharpe, 12 m) ties it by
  choosing almost the same thing (VPIN 0.26 shorts-only for eleven straight quarters, then v18).
- shorter memory is strictly worse (6 m → negative). P&L-maximising selection is the worst:
  it keeps turning the long book on right before it loses (2023Q3 −975, 2025Q2 −1,376).
- 2023 is negative under every selector (−289 WF vs −181 static). the weak years are not a
  regime the book can chase; the edge is what it is and the honest fix is a better trigger,
  not a moving target.
- the "fit on 2026 only" exhibit: with a PF objective the 2026 winners (VPIN 0.26, ± bear-bounce
  longs) happen to hold up in 2022–25 (PF 1.34–1.53); with a P&L objective 2026 picks
  "longs on when SPY > SMA20" (2026 PF 1.21) whose 2022–25 record is PF 1.02 on 4,000 trades.
  a 2026-only fit is only as safe as the objective you happen to choose, which is the argument
  against it.

verdict: rolling refit rejected as a method; keep fitting on all five years with per-year
positivity as the gate. side result: VPIN 0.26 shorts-only was the trailing-12-month choice
for most of 2023–2025 (5y +1,952 / 306 / PF 1.58, 4 positive years) — the quality/volume
trade-off in one number, not a volume lever.

raw runs: walk_forward_{pf,pnl,sharpe}12.txt in this directory.
