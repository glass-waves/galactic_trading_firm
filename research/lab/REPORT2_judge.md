# judge (round 2) — 2026-11-12 01:26 — 5 finalists (Šidák over 5) vs research/lab/specs/base/row25.json

### r2-entries/peersnotall_thr  [T]
| window | P&L | n | PF | dd | worst day | by year | base P&L / n / PF / dd | paired diff · P(≤0) | book alone | corr |
|---|---:|---:|---:|---:|---:|---|---|---|---|---:|
| train | +2839 | 282 | 1.98 | -218 | -119 | 22:+1460/114/2.51 23:+98/56/1.12 24:+743/74/1.86 25:+538/38/2.93 | +2610 / 320 / 1.75 / -238 | +229 · 0.169 | — | — |
| holdout | +678 | 105 | 1.77 | -133 | -45 | 25:+162/32/1.74 26:+516/73/1.79 | +899 / 111 / 2.00 / -133 | -221 · 0.973 | — | — |
| target | +1216 | 143 | 2.05 | -133 | -57 | 25:+700/70/2.41 26:+516/73/1.79 | +1279 / 152 / 1.96 / -217 | -63 · 0.656 | — | — |

- fail — T1 train: paired P<0.010, PF≥1.6, n≥250, yr≥−300, ex-best PF≥1.4
- fail — T2 holdout: P&L>0, PF≥1.5, paired P(≤0)<0.25
- fail — T3 target: PF≥1.4, n≥150
- PASS — T4 risk: target dd≥−600, worst day≥−300 (train, target)

**does not pass**

### r2-entries/pdl15  [T]
| window | P&L | n | PF | dd | worst day | by year | base P&L / n / PF / dd | paired diff · P(≤0) | book alone | corr |
|---|---:|---:|---:|---:|---:|---|---|---|---|---:|
| train | +2634 | 256 | 2.16 | -174 | -95 | 22:+1151/98/2.66 23:+330/60/1.48 24:+723/70/1.99 25:+430/28/3.83 | +2610 / 320 / 1.75 / -238 | +24 · 0.469 | — | — |
| holdout | +891 | 78 | 2.50 | -134 | -45 | 25:+230/20/2.50 26:+661/58/2.50 | +899 / 111 / 2.00 / -133 | -7 · 0.504 | — | — |
| target | +1322 | 106 | 2.77 | -134 | -45 | 25:+661/48/3.16 26:+661/58/2.50 | +1279 / 152 / 1.96 / -217 | +43 · 0.442 | — | — |

- fail — T1 train: paired P<0.010, PF≥1.6, n≥250, yr≥−300, ex-best PF≥1.4
- fail — T2 holdout: P&L>0, PF≥1.5, paired P(≤0)<0.25
- fail — T3 target: PF≥1.4, n≥150
- PASS — T4 risk: target dd≥−600, worst day≥−300 (train, target)

**does not pass**

### r2-sizing-universe/tier_x150  [T]
| window | P&L | n | PF | dd | worst day | by year | base P&L / n / PF / dd | paired diff · P(≤0) | book alone | corr |
|---|---:|---:|---:|---:|---:|---|---|---|---|---:|
| train | +2984 | 320 | 1.77 | -266 | -199 | 22:+1632/119/2.53 23:+194/71/1.19 24:+708/89/1.56 25:+450/41/1.92 | +2610 / 320 / 1.75 / -238 | +374 · 0.002 | — | — |
| holdout | +1047 | 111 | 2.05 | -141 | -48 | 25:+232/32/1.94 26:+815/79/2.08 | +899 / 111 / 2.00 / -133 | +148 · 0.004 | — | — |
| target | +1497 | 152 | 2.01 | -249 | -199 | 25:+683/73/1.93 26:+815/79/2.08 | +1279 / 152 / 1.96 / -217 | +218 · 0.003 | — | — |

- PASS — T1 train: paired P<0.010, PF≥1.6, n≥250, yr≥−300, ex-best PF≥1.4
- PASS — T2 holdout: P&L>0, PF≥1.5, paired P(≤0)<0.25
- PASS — T3 target: PF≥1.4, n≥150
- PASS — T4 risk: target dd≥−600, worst day≥−300 (train, target)

**PASSES ALL**

### base/row25+r2-books/m5_r3_ls5_dm175  [A]  (books pooled — dollars assume 2× capital)
| window | P&L | n | PF | dd | worst day | by year | base P&L / n / PF / dd | paired diff · P(≤0) | book alone | corr |
|---|---:|---:|---:|---:|---:|---|---|---|---|---:|
| train | +3681 | 417 | 1.73 | -333 | -176 | 22:+1698/163/2.03 23:+262/99/1.19 24:+583/108/1.38 25:+1138/47/3.36 | +2610 / 320 / 1.75 / -238 | +1071 · 0.096 | +1071 / 97 / 1.69 (worst yr -67) | +0.00 |
| holdout | +816 | 120 | 1.79 | -190 | -61 | 25:+219/33/2.03 26:+598/87/1.73 | +899 / 111 / 2.00 / -133 | -82 · 0.890 | -82 / 9 / 0.34 (worst yr -98) | +0.02 |
| target | +1954 | 167 | 2.29 | -217 | -168 | 25:+1357/80/2.95 26:+598/87/1.73 | +1279 / 152 / 1.96 / -217 | +675 · 0.301 | +675 / 15 / 4.93 (worst yr -98) | +0.00 |

- PASS — A1 train book: n≥60, PF≥1.3, worst yr≥−200
- PASS — A2 train pooled: PF≥1.5, dd≥1.5×base dd
- PASS — A3 train: daily corr<0.3
- fail — A4 holdout: book P&L>0
- PASS — A5 target pooled: PF≥1.4, n≥150

**does not pass**

### base/row25+r2-books/m1_r3_ls5  [A]  (books pooled — dollars assume 2× capital)
| window | P&L | n | PF | dd | worst day | by year | base P&L / n / PF / dd | paired diff · P(≤0) | book alone | corr |
|---|---:|---:|---:|---:|---:|---|---|---|---|---:|
| train | +3545 | 473 | 1.62 | -386 | -176 | 22:+1729/188/1.89 23:+188/116/1.12 24:+585/114/1.37 25:+1044/55/2.69 | +2610 / 320 / 1.75 / -238 | +935 · 0.150 | +935 / 153 / 1.42 (worst yr -65) | +0.01 |
| holdout | +808 | 132 | 1.69 | -168 | -74 | 25:+106/36/1.33 26:+702/96/1.83 | +899 / 111 / 2.00 / -133 | -91 · 0.771 | -91 / 21 / 0.66 (worst yr -97) | +0.01 |
| target | +1851 | 187 | 2.04 | -182 | -168 | 25:+1150/91/2.22 26:+702/96/1.83 | +1279 / 152 / 1.96 / -217 | +572 · 0.321 | +572 / 35 / 2.29 (worst yr +6) | +0.00 |

- PASS — A1 train book: n≥60, PF≥1.3, worst yr≥−200
- fail — A2 train pooled: PF≥1.5, dd≥1.5×base dd
- PASS — A3 train: daily corr<0.3
- fail — A4 holdout: book P&L>0
- PASS — A5 target pooled: PF≥1.4, n≥150

**does not pass**
