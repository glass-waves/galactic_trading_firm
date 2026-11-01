# judge — 2026-11-01 03:31 — 6 finalists (Šidák over 6)

### frontier/na_bundle
| window | P&L | trades | PF | dd | worst day | p | ex-best-yr PF | by year |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| train | +2610 | 320 | 1.75 | -238 | -168 | 0.0007 | 1.468 | 22:+1443/119/2.43 23:+136/71/1.15 24:+650/89/1.58 25:+380/41/1.87 |
| holdout | +899 | 111 | 2.00 | -133 | -45 | 0.0107 | 1.958 | 25:+203/32/1.96 26:+695/79/2.01 |
| target | +1279 | 152 | 1.96 | -217 | -168 | 0.008 | 1.9 | 25:+584/73/1.90 26:+695/79/2.01 |

paired vs baseline on target: P&L diff +578, P(diff ≤ 0) 0.025; q00 -342, q00 -256, q00 -152, q01 -93, q02 -19, q05 +97, q10 +204, q25 +375, q50 +567, q75 +776, q90 +965, q95 +1085

- PASS — 1 target PF≥1.4, n≥150, 25&26>0
- PASS — 5 dd≥−1000, worst day≥−300
- fail — 4 paired vs base P(≤0)<0.017
- PASS — 2 holdout PF≥1.3, P&L>0
- PASS — 3 train PF≥1.3, p<0.05, yr≥−300, exb≥1.2

**not a contender**

### frontier/nb_v15
| window | P&L | trades | PF | dd | worst day | p | ex-best-yr PF | by year |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| train | +2735 | 397 | 1.61 | -267 | -168 | 0.0017 | 1.354 | 22:+1673/154/2.13 23:+212/87/1.21 24:+528/103/1.41 25:+322/53/1.47 |
| holdout | +632 | 137 | 1.51 | -178 | -56 | 0.0667 | 1.615 | 25:+172/39/1.61 26:+460/98/1.49 |
| target | +955 | 190 | 1.50 | -203 | -168 | 0.033 | 1.485 | 25:+495/92/1.51 26:+460/98/1.49 |

paired vs baseline on target: P&L diff +254, P(diff ≤ 0) 0.226; q00 -802, q00 -681, q00 -570, q01 -506, q02 -418, q05 -283, q10 -171, q25 +27, q50 +251, q75 +478, q90 +683, q95 +807

- PASS — 1 target PF≥1.4, n≥150, 25&26>0
- PASS — 5 dd≥−1000, worst day≥−300
- fail — 4 paired vs base P(≤0)<0.017
- PASS — 2 holdout PF≥1.3, P&L>0
- PASS — 3 train PF≥1.3, p<0.05, yr≥−300, exb≥1.2

**not a contender**

### indicators/r21_v020_noadx_t10
| window | P&L | trades | PF | dd | worst day | p | ex-best-yr PF | by year |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| train | +1997 | 287 | 1.69 | -250 | -133 | 0.0017 | 1.437 | 22:+1056/94/2.40 23:+78/71/1.09 24:+592/88/1.60 25:+271/34/1.80 |
| holdout | +787 | 108 | 2.02 | -125 | -45 | 0.015 | 2.135 | 25:+191/30/2.13 26:+596/78/1.98 |
| target | +1058 | 142 | 1.95 | -177 | -133 | 0.0083 | 1.914 | 25:+462/64/1.91 26:+596/78/1.98 |

paired vs baseline on target: P&L diff +357, P(diff ≤ 0) 0.079; q00 -402, q00 -332, q00 -276, q01 -212, q02 -146, q05 -52, q10 +34, q25 +180, q50 +343, q75 +522, q90 +688, q95 +800

- fail — 1 target PF≥1.4, n≥150, 25&26>0
- PASS — 5 dd≥−1000, worst day≥−300
- fail — 4 paired vs base P(≤0)<0.017
- PASS — 2 holdout PF≥1.3, P&L>0
- PASS — 3 train PF≥1.3, p<0.05, yr≥−300, exb≥1.2

**not a contender**

### universe/nt10_b3
| window | P&L | trades | PF | dd | worst day | p | ex-best-yr PF | by year |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| train | +2212 | 251 | 1.90 | -256 | -133 | 0.0003 | 1.62 | 22:+1081/83/2.71 23:+82/66/1.10 24:+705/75/1.92 25:+345/27/2.37 |
| holdout | +605 | 95 | 1.86 | -137 | -45 | 0.0307 | 2.459 | 25:+201/27/2.46 26:+404/68/1.71 |
| target | +950 | 122 | 2.00 | -173 | -133 | 0.0103 | 1.714 | 25:+547/54/2.40 26:+404/68/1.71 |

paired vs baseline on target: P&L diff +249, P(diff ≤ 0) 0.199; q00 -623, q00 -549, q00 -466, q01 -423, q02 -343, q05 -222, q10 -124, q25 +47, q50 +241, q75 +437, q90 +632, q95 +733

- fail — 1 target PF≥1.4, n≥150, 25&26>0
- PASS — 5 dd≥−1000, worst day≥−300
- fail — 4 paired vs base P(≤0)<0.017
- PASS — 2 holdout PF≥1.3, P&L>0
- PASS — 3 train PF≥1.3, p<0.05, yr≥−300, exb≥1.2

**not a contender**

### frontier/na_pdl10
| window | P&L | trades | PF | dd | worst day | p | ex-best-yr PF | by year |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| train | +1562 | 170 | 2.11 | -145 | -83 | 0.0007 | 1.894 | 22:+651/58/2.68 23:+237/46/1.49 24:+458/50/1.98 25:+216/16/4.22 |
| holdout | +536 | 59 | 2.26 | -96 | -45 | 0.018 | 3.146 | 25:+214/16/3.15 26:+322/43/1.99 |
| target | +751 | 75 | 2.52 | -96 | -45 | 0.0037 | 1.986 | 25:+429/32/3.58 26:+322/43/1.99 |

paired vs baseline on target: P&L diff +50, P(diff ≤ 0) 0.457; q00 -1044, q00 -930, q00 -824, q01 -772, q02 -673, q05 -523, q10 -404, q25 -195, q50 +38, q75 +282, q90 +502, q95 +626

- fail — 1 target PF≥1.4, n≥150, 25&26>0
- PASS — 5 dd≥−1000, worst day≥−300
- fail — 4 paired vs base P(≤0)<0.017
- PASS — 2 holdout PF≥1.3, P&L>0
- PASS — 3 train PF≥1.3, p<0.05, yr≥−300, exb≥1.2

**not a contender**

### frontier/na_bundle+meta/qqq_noise_pm_dm07  (2 books pooled — dollars assume 2× capital)
| window | P&L | trades | PF | dd | worst day | p | ex-best-yr PF | by year |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| train | +3592 | 549 | 1.66 | -277 | -168 | 0.0003 | 1.456 | 22:+2043/237/2.01 23:+305/128/1.24 24:+628/113/1.48 25:+616/71/1.74 |
| holdout | +988 | 161 | 1.82 | -143 | -58 | 0.006 | 1.669 | 25:+195/41/1.67 26:+793/120/1.87 |
| target | +1604 | 232 | 1.79 | -182 | -168 | 0.0067 | 1.869 | 25:+811/112/1.73 26:+793/120/1.87 |

paired vs baseline on target: P&L diff +903, P(diff ≤ 0) 0.029; q00 -609, q00 -436, q00 -321, q01 -159, q02 -71, q05 +98, q10 +268, q25 +556, q50 +882, q75 +1233, q90 +1592, q95 +1809

- PASS — 1 target PF≥1.4, n≥150, 25&26>0
- PASS — 5 dd≥−1000, worst day≥−300
- fail — 4 paired vs base P(≤0)<0.017
- PASS — 2 holdout PF≥1.3, P&L>0
- PASS — 3 train PF≥1.3, p<0.05, yr≥−300, exb≥1.2

**not a contender**
