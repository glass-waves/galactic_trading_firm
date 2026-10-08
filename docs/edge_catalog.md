# edge catalog: documented intraday (and ≤1-day-hold) equity anomalies

Companion to `docs/intraday-trading-research.md` (ML/microstructure bibliography) and
`docs/research_synthesis.md` (what got built). This file catalogs *trading rules documented in
the literature on a specific instrument for a specific mechanism*, so we stop testing them on
instruments the mechanism doesn't predict. Rule from the owner: **replicate on the paper's own
instrument first, then vary.** Every claim below is cited; "[from memory]" flags anything I could
not re-verify with a URL this session. "Our status" cites the repo file with the actual verdict —
see `research/edge_matrix.md` for the full instrument × anomaly grid and a ranked pretest list.

### 1. Opening-range breakout (ORB)
- **Papers:** Zarattini & Aziz 2023, SSRN 4416622, "Can Day Trading Really Be Profitable?" (https://ssrn.com/abstract=4416622). Independent replication exists (GitHub giovannibrusco/zarattini-2023-orb-qqq) finding break-even near ~2.2¢/share slippage. [well-replicated at a practitioner level; one primary academic-adjacent source]
- **Home/mechanism:** QQQ, 2016–2023, 5-min opening range; retail/day-trader stop-run and continuation flow concentrated in the most liquid, most day-traded index ETF.
- **Window:** first 5 min of RTH, breakout entry, held to a stop/target or close.
- **Edge:** paper reports 1,484% vs 169% buy-hold 2016–2023 *with 3x leverage*; unlevered edge is much smaller and sensitive to slippage (replication: breakeven ~2¢/share).
- **Transfers to:** other high-ADV index/sector ETFs plausible | **Not to:** single stocks — Quantitativo-style reviews and our own §15-era notes find ORB-type rules "underperform on most individual stocks" (idiosyncratic noise swamps the breakout).
- **Status:** untested as the literal 5-min-ORB rule on QQQ/SPY. A close analog (noise-area breakout) was tested correctly on SPY/QQQ in `research/index_momentum/2026-10-07_index_intraday_momentum.md` — mixed (see #2). Single-name opening-range conditions were tried and pruned without a clear pass in `research/entries/README.md`.

### 2. Intraday momentum / "noise area" (full-day trend-following)
- **Papers:** Zarattini, Barbon & Aziz 2024, "Beat the Market: An Effective Intraday Momentum Strategy for S&P500 ETF (SPY)," SFI N°24-97 (https://www.sfi.ch/en/publications/n-24-97-beat-the-market-an-effective-intraday-momentum-strategy-for-s-p500-etf-spy). [single research group, pre-print/working-paper tier; not yet independently replicated in the peer-reviewed literature]
- **Home/mechanism:** SPY, 2007–2024; abnormal demand/supply imbalance filtered by a "noise area" band, held with a trailing stop — index-level flow (not stock-specific news).
- **Window:** all session, not limited to the last 30 min (unlike Gao et al., #3).
- **Edge:** paper reports strong cumulative outperformance 2007–2024; no independent net-of-cost replication found.
- **Transfers to:** QQQ, other high-ADV index ETFs plausible | **Not to:** single mega-caps — mechanism is index-basket order flow, not idiosyncratic.
- **Status:** tested correctly on SPY and QQQ in `research/index_momentum/2026-10-07_index_intraday_momentum.md` — full-day strategy fails on both at our costs; one QQQ afternoon/high-vol-day sub-cell passes and is pipeline candidate `qqq-noise-pm-vol` (shadow). Earlier attempt proxied this edge through single mega-caps (`docs/analysis/2026-09-15_second_window_research.md` candidate #3, `docs/paper_trading_plan_2026-09.md` §13) — **wrong-instrument test, correctly superseded** by the SPY/QQQ study. This is the one documented success case the owner is asking us to generalize.

### 3. Market intraday momentum — first half-hour predicts last half-hour
- **Papers:** Gao, Han, Li & Zhou, "Market Intraday Momentum," SSRN 2440866, Journal of Financial Economics 2018 [venue from memory, SSRN posting verified] (https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2440866). [well-replicated — extended to China, 10 other US ETFs, 16 global markets by follow-on papers]
- **Home/mechanism:** SPY and other high-ADV ETFs, 1993–2013; stronger on volatile/high-volume/recession/macro-news days — late-day institutional rebalancing completing a signal set early.
- **Window:** first 30 min → last 30 min of RTH.
- **Edge:** predictive R² 1.6% in-sample, 1.4–2.0% out-of-sample — small but "monthly-frequency-grade" per the authors; no net-of-cost figure given (academic predictability study, not a cost-aware backtest).
- **Transfers to:** other liquid ETFs (documented) | **Not to:** single stocks — opposite/no sign reported for individual names (see #5).
- **Status:** tested correctly on SPY/QQQ in `research/index_momentum/2026-10-07_index_intraday_momentum.md` as part of the same study — fails at our cost model. Earlier proxy on the 4 mega-caps in `docs/paper_trading_plan_2026-09.md` §13 found correlation ±0.1, under 3bps/day — **wrong-instrument test** (the paper never claims single-name predictability).

### 4. Hedging-demand intraday momentum (dealer/ETF flow)
- **Papers:** Baltussen, Da, Lammers & Martens, "Hedging Demand and Market Intraday Momentum," Journal of Financial Economics 2021, SSRN 3760365 (https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3760365). [well-replicated — 60+ futures across equities, bonds, commodities, FX, 1974–2020]
- **Home/mechanism:** equity, bond, commodity and FX index futures; dealers' intraday delta-hedging demand drives last-30-min momentum — a market-wide, multi-asset mechanism, not equity-specific.
- **Window:** last 30 min predicted by intraday-so-far return.
- **Edge:** "economically and statistically highly significant" per the paper; magnitude not independently net-of-cost verified by us.
- **Transfers to:** SPY/QQQ futures-equivalents, sector ETFs, plausibly bond/gold ETFs (documented in their own multi-asset sample) | **Not to:** single stocks (opposite sign, see #5).
- **Status:** untested here distinctly from #2/#3 (same family, different driving variable — hedging flow, not raw momentum). Good pretest candidate — see matrix.

### 5. End-of-day reversal in single stocks ("late-day reversal")
- **Papers:** Baltussen, Da & Soebhag, "End-of-Day Reversal," working paper 2024/2025, EFMA 2024 (https://academicweb.nd.edu/~zda/EOD.pdf). Companion/predecessor work by the same group's Lammers/Martens co-authors (#4) established the opposite index-level pattern. [single research program; strong internal replication across their own samples, not yet broadly independently replicated]
- **Home/mechanism:** individual stocks, cross-section; retail attention-induced buying of the day's losers plus short-covering/risk management into the close — explicitly *not* liquidity or gamma-hedging per the authors.
- **Window:** last 30 min, cross-sectional (today's intraday losers bounce).
- **Edge:** "economically and statistically highly significant" per the paper; no net-of-cost figure independently verified.
- **Transfers to:** any sufficiently liquid single stock (home class includes our mega-caps) | **Not to:** index/ETF level (opposite sign, #3/#4).
- **Status:** tested correctly on our mega-caps (a subset of the paper's own universe) in `research/eod/README.md` / `docs/paper_trading_plan_2026-09.md` §13.4 — **rejected**: "about zero net after costs," PF ≈0.9–1.04 across threshold sweep, only 2022 pays. Real edge, below our cost line.

### 6. Intraday return periodicity (half-hour continuation)
- **Papers:** Heston, Korajczyk & Sadka, "Intraday Patterns in the Cross-Section of Stock Returns," Journal of Finance 2010 (https://www.bauer.uh.edu/departments/finance/documents/Heston-Korajczyk-Sadka-jf-2010-01-07.pdf). [well-replicated, foundational]
- **Home/mechanism:** individual stocks, cross-section; same-half-hour return continuation persisting ~40 trading days — unexplained by volume/order-imbalance/volatility patterns; institutional habitat/liquidity-provision cycles suspected.
- **Window:** same clock half-hour, day over day (not same-day).
- **Edge:** significant continuation documented; authors note it is "more pronounced for, but not restricted to," the first/last half-hour. `docs/analysis/2026-09-15_second_window_research.md` (#10) independently judged it "uneconomic alone, ~3bps/half-hour ≈ cost-neutral — usable only as overlay/filter."
- **Transfers to:** our mega-caps directly (home class) | **Not to:** likely too small-edge standalone for leveraged/bond ETFs.
- **Status:** untested as a standalone signal (distinct from the time-of-day seasonality tested and killed in `docs/paper_trading_plan_2026-09.md` §13, which tested midday-fade-of-morning-move, not same-clock-day-over-day continuation). Good cheap pretest candidate.

### 7. Overnight vs. intraday return "tug of war"
- **Papers:** Lou, Polk & Skouras, "A Tug of War: Overnight versus Intraday Expected Returns," Journal of Financial Economics 2019 (https://personal.lse.ac.uk/loud/ATugofWar_appendix.pdf). [well-replicated, widely cited]
- **Home/mechanism:** individual stocks; retail investors trade near the open (sentiment-driven), institutions concentrate intraday — overnight winners predict overnight winners and intraday losers, and vice versa (cross-predictability, not same-period momentum).
- **Window:** split by overnight (close→open) vs. intraday (open→close) return.
- **Edge:** overnight winner-minus-loser decile: 1.25%/month three-factor alpha, t=4.28; intraday leg strongly negative in their sample's early years.
- **Transfers to:** any liquid single stock (home class) | **Not to:** cannot be fully harvested intraday-only — **the overnight leg requires an overnight hold, which this system structurally forbids** (CLAUDE.md: "intraday only — no overnight holds"). Usable only as a *signal* to bias next-session-open exposure, not a position carried overnight.
- **Status:** untested as this specific cross-predictability signal. `research/swing/2026-09-26_multiday_long_research.md` tested related multi-day/overnight ideas (dead except PEAD-hold-10, #14) but not this exact overnight→intraday predictor. Cheap pretest on daily bars; low priority given the no-overnight-hold constraint limits how we could act on it.

### 8. "Strikingly suspicious" overnight drift
- **Papers:** Knuteson, "Strikingly Suspicious Overnight and Intraday Returns," arXiv 2010.01727 (https://arxiv.org/abs/2010.01727), SSRN 3705017. **[not peer-reviewed; author's own causal theory (large quant funds manipulating the close) is disputed/unverified — treat as a documented empirical pattern only, not an endorsed mechanism]**
- **Home/mechanism:** major index ETFs/futures; overnight strongly positive, intraday strongly negative, persistently, for decades — cause contested.
- **Window:** close→open vs. open→close, index level.
- **Edge:** described qualitatively ("wildly positive" overnight, "disturbingly negative" intraday); no clean net-of-cost number in the source.
- **Transfers to:** SPY/QQQ (documented) | **Not to:** mechanism-agnostic claim, weak basis for extrapolating to single names or ETF subclasses.
- **Status:** untested. Also blocked by the no-overnight-hold constraint (#7). Low priority given the unverified/disputed mechanism — include only as a sanity check, not a strategy source.

### 9. Pre-FOMC announcement drift
- **Papers:** Lucca & Moench, "The Pre-FOMC Announcement Drift," Journal of Finance 2015 (NY Fed SR512; https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1923197). [well-replicated through ~2014; see #10 for later decay evidence]
- **Home/mechanism:** U.S. equity indices; >80% of the equity premium over 17 years earned in the 24h before scheduled FOMC announcements, at unusually low realized vol — monetary-policy-uncertainty resolution priced in ahead of time. The paper explicitly finds **no effect in Treasuries/money-market futures**.
- **Window:** 24h (effectively prior session's afternoon through announcement) before ~8 scheduled FOMC meetings/year.
- **Edge:** large per the original sample; "The Disappearing Pre-FOMC Announcement Drift" (cited in search results, Tandfonline 2024) reports it has weakened/become short-lived in recent years — **decayed**.
- **Transfers to:** SPY/QQQ, and plausibly mega-cap beta exposure | **Not to:** bond/gold ETFs — the paper's own null result.
- **Status:** untested. We already carry FOMC dates in-engine (`crates/indicators/src/custom/event_calendar.rs`, used in `research/crash_days/`), so this is a very cheap pretest (≈8 events/yr, need only daily or 1m bars already cached). Top pretest candidate.

### 10. FOMC-day intraday volatility compression / pinning
- **Papers:** practitioner/working-paper tier only — Song, "Pre-FOMC Uncertainty Accumulation: Evidence from 0DTE Options" (https://afajof.org/management/viewp.php?n=260400); BIS Working Paper 1079, "Volume dynamics around FOMC announcements" (https://www.bis.org/publ/work1079.pdf). **[single/recent working papers — not established the way #9 is]**
- **Home/mechanism:** SPX/SPY 0DTE options; realized vol compresses into the announcement as event variance dominates, then jumps — a derivatives-positioning effect layered on #9.
- **Window:** same-day, morning through the 2pm/14:00 ET announcement.
- **Edge:** described qualitatively; no net-of-cost equity trading rule given in the sources (these are variance/options papers, not direct directional-equity strategies).
- **Transfers to:** SPY/QQQ (home) | **Not to:** sector/leveraged ETFs or single names lacking daily-expiry options liquidity.
- **Status:** untested. True test needs options/gamma-exposure data we don't have (see process section); a crude realized-range proxy on existing bars is possible but approximate.

### 11. End-of-day / closing-auction price pressure and reversal
- **Papers:** Bogousslavsky, "The Cross-Section of Intraday and Overnight Returns" (ScienceDirect, https://www.sciencedirect.com/science/article/abs/pii/S0304405X21000854); Bogousslavsky & Muravyev on closing-auction price deviations reverting ("Who Trades at the Close?," https://www.researchgate.net/publication/371850619). [moderately replicated — closing-auction share of volume has risen from 3.1% (2010) to 7.5%+ (2018), making the mechanism stronger over time, not decaying]
- **Home/mechanism:** individual stocks; closing-auction price deviations from the pre-close continuous price tend to reverse quickly — infrequent-rebalancer price pressure, not information.
- **Window:** last minutes before and the few minutes after the closing auction print.
- **Edge:** "deviations... tend to reverse rapidly on average" — directional, magnitude not independently net-of-cost verified by us.
- **Transfers to:** liquid single stocks (home, includes our mega-caps) | **Not to:** instruments without a meaningful closing auction (most leveraged/sector ETFs trade MOC too, so plausible there as well).
- **Status:** untested. **Blocked on data:** we do not have a closing-auction imbalance feed (see process section) — the cheapest version we could run (comparing last-print vs. last-trade reversal on existing 1m bars) is a weak proxy for the real signal.

### 12. Leveraged-ETF / dealer-gamma rebalancing flows at the close
- **Papers:** Barbon, Beckmeyer, Buraschi & Moerke, "The Role of Leveraged ETFs and Option Market Imbalances on End-of-Day Price Dynamics," SSRN 3925725 (https://ssrn.com/abstract=3925725); Moerke, "Liquidity Provision to Leveraged ETFs and Equity Options Rebalancing Flows" (https://wp.lancs.ac.uk/fofi2022/files/2022/08/FoFI-2022-027-Mathis-Moerke.pdf). [well-replicated for the LETF leg; the paper reports LETF effects have *decreased* over time while gamma effects persist]
- **Home/mechanism:** SPY/QQQ and their 2–3x leveraged ETFs (SSO/SPXL/QLD/TQQQ etc.); daily rebalancing to a fixed leverage target forces same-direction-as-the-day flow into the close — a one-std-dev increase in LETF rebalancing flow raises the last-half-hour return by "403% of the average return in that window" per the paper.
- **Window:** last ~30 min of RTH, on days with a large enough daily move to force meaningful rebalancing.
- **Edge:** large per the paper for the underlying index ETFs; effect has shrunk over time as more liquidity providers front-run it.
- **Transfers to:** sector-ETF/leveraged-ETF pairs (SMH/SOXL, XLK/TECL) plausible; mega-caps with heavy options open interest (NVDA) plausible via the gamma leg | **Not to:** broad illiquid single stocks, bond ETFs (no comparable daily-rebalanced leveraged complex at scale).
- **Status:** untested. Needs a daily-move-magnitude filter (cheap, have the bars) but the full dealer-gamma leg needs options OI data we lack.

### 13. Leveraged VIX-product rebalancing flows
- **Papers:** "The Market Impact of Predictable Flows: Evidence from Leveraged VIX Products," ScienceDirect (https://www.sciencedirect.com/science/article/abs/pii/S0378426621002363). [single paper found this session]
- **Home/mechanism:** VIX futures and leveraged/inverse VIX ETNs (VXX, UVXY, SVXY); daily rebalancing of these products forces predictable late-day VIX-futures flow that "is completely reversed overnight" per the paper.
- **Window:** late-day, VIX futures curve.
- **Edge:** "economically significant" late-day impact, fully mean-reverting overnight.
- **Transfers to:** SPY/QQQ only indirectly (via the VIX-equity relationship) | **Not to:** single stocks, bond/gold ETFs — mechanism is specific to the VIX futures complex.
- **Status:** untested; we do not hold VIX futures data (see process section) — low priority without it.

### 14. Post-earnings-announcement drift (PEAD)
- **Papers:** Bernard & Thomas, Journal of Accounting Research 1989 [from memory — classic result, not re-verified by URL this session]; post-publication decay per McLean & Pontiff, "Does Academic Research Destroy Stock Return Predictability?," Journal of Finance 2016 (https://www.hec.ca/finance/Fichier/McLean.pdf) — average anomaly decays ~35% post-publication, PEAD specifically noted as one of the more durable ones in follow-on literature. [extremely well-replicated, one of the most robust anomalies in the literature]
- **Home/mechanism:** individual stocks around scheduled earnings releases; market under-reacts to the surprise, drift continues 60–90 days, larger for low-analyst-coverage names.
- **Window:** multi-day, starting the announcement day/next session (not purely intraday).
- **Edge:** classically several hundred bps per quarter for extreme-surprise deciles; "stocks followed by <5 analysts" show materially larger drift.
- **Transfers to:** any single stock with scheduled earnings (home, includes our mega-caps) | **Not to:** index/sector ETFs (no idiosyncratic earnings surprise).
- **Status:** tested correctly on the daily universe (59 names incl. our mega-caps) in `research/swing/2026-09-26_multiday_long_research.md` — **the one multi-day long edge that passed**: hold-10-sessions after >+3% earnings reaction, PF 1.55–1.78, +67bps/trade over unconditional hold, every year. **Parked** — needs 8–12 days of engine work to support persistent multi-day positions (conflicts with the current intraday-only single-position design).

### 15. Earnings-day gap: non-fundamental reversal vs. continuation
- **Papers:** Ben-Rephael, "Mind the Gap: The Non-Fundamental Role of Earnings Days" (https://haslam.utk.edu/wp-content/uploads/2024/11/Ben-Rephael-Paper.pdf). [single paper found this session, recent]
- **Home/mechanism:** individual stocks on earnings-reaction days; roughly half the earnings-day announcement return is non-fundamental and reverses slowly (up to ~3 years) — distinct from PEAD's continuation of the *surprise-driven* component.
- **Window:** the earnings-day gap itself, multi-year reversal (not intraday actionable as stated, but the *gap-day* positioning is).
- **Edge:** ~50% of the earnings-day effect reverses over time per the paper.
- **Transfers to:** single stocks (home, our mega-caps) | **Not to:** ETFs (no idiosyncratic earnings gap).
- **Status:** untested as this specific reversal rule. A *different* earnings-day rule (5-min opening-range breakout, i.e. continuation) was tested correctly on our mega-caps in `research/earnings/README.md` / `docs/paper_trading_plan_2026-09.md` §13.3 — **parked**: +23bps/trade (n=74, PF 1.38) but negative in 2 of 5 years. This entry's reversal mechanism is the untested complement.

### 16. Gap fade / gap fill
- **Papers:** practitioner-tier only — quantifiedstrategies.com (https://www.quantifiedstrategies.com/gap-fill-trading-strategies/), shareplanner SPY/QQQ gap study. **[no peer-reviewed primary source found; treat as folklore-with-some-public-backtest-evidence, flagged in `docs/analysis/2026-09-15_second_window_research.md` #5 as "plausible, recent futures evidence negative"]**
- **Home/mechanism:** ambiguous by instrument — claimed on both single stocks and index ETFs; news-driven gaps fade less, no-news/overnight-imbalance gaps fill more. On QQQ, cited fill rates ~45% for 1–2% gaps, ~30–33% for >2% gaps; on >2% index gaps, continuation reportedly wins (PF ~1.50 cited for QQQ).
- **Window:** open, same-day resolution.
- **Edge:** mixed/size-dependent; one source: "breaks even at best" for small gaps, loses more as gap grows, with continuation favored on large index gaps.
- **Transfers to:** SPY/QQQ and single stocks both claimed (home ambiguous) | **Not to:** no clear mechanism statement to rule anything out — treat any transfer as unverified.
- **Status:** untested on our instruments. Cheap pretest (daily open vs. prior close, already have both bar sets) but low rigor given the source quality.

### 17. Turn-of-month / first-trading-day-of-month flows
- **Papers:** Ariel 1987, Journal of Financial Economics [from memory — classic, foundational, not re-verified by URL]; McConnell & Xu, "Equity Returns at the Turn of the Month," Financial Analysts Journal 2008 (https://www.chesler.us/resources/academia/turn_of_the_month_stock_returns.pdf). [well-replicated internationally; post-ETF era concentrates the effect on the first trading day specifically per search-result summaries]
- **Home/mechanism:** broad equity indices; pension/mutual-fund month-end cash flows reinvested — though the pure flow-timing story is weakened by evidence of no corresponding net mutual-fund-flow pattern; more recent work attributes it to infrequent rebalancing + risk deferral.
- **Window:** last trading day of month through first ~3–4 of next month; ETF era concentrates it on day 1.
- **Edge:** ~10bps/day higher mean return across the 4-day window per multi-country study; all of the historical excess market return over some long samples occurred in this window.
- **Transfers to:** SPY/QQQ (home); plausibly sector ETFs and beta-weighted mega-caps; bond ETFs plausible (fixed-income funds have analogous month-end flow literature) | **Not to:** no strong reason to exclude any liquid instrument — weakest-mechanism entry in this catalog.
- **Status:** untested. Cheap pretest — pure calendar rule on daily or 1m bars we already have.

### 18. Index inclusion / rebalance effect
- **Papers:** Greenwood & Sammon, "The Disappearing Index Effect" (https://www.hbs.edu/ris/Publication%20Files/23-025_563e45c6-df92-4d9c-ae05-608d4d0acab1.pdf); NY Fed SR484, "Is There an S&P 500 Index Effect?" (https://www.newyorkfed.org/medialibrary/media/research/staff_reports/sr484.pdf). [well-replicated across 1989–2023; **actively decaying/largely gone 2016–2023**]
- **Home/mechanism:** the single stock being added to/removed from an index; index-fund buying/selling pressure around the announcement/effective date. Explicitly a per-event, per-stock effect, not an ETF-level one.
- **Window:** announcement date through effective inclusion date.
- **Edge:** was large and significant 1989–2006/2015; "became statistically insignificant from 2016 to 2023" per Greenwood & Sammon — **decayed to ~zero**.
- **Transfers to:** nothing in our tradeable set — our 4 mega-caps are long-standing index constituents, not names undergoing an inclusion event | **Not to:** our instrument universe structurally, regardless of decay.
- **Status:** not applicable to us by construction (mechanism requires a membership-change event; none of our names have one). Lowest priority in this catalog — included for completeness only.

### 19. 0DTE options / gamma pinning
- **Papers:** practitioner sources only — SpotGamma (https://spotgamma.com/0dte/), GEXBoard, MenthorQ. **[no peer-reviewed academic source found this session — treat as an informally-documented, high-plausibility-mechanism pattern, not a verified result]**
- **Home/mechanism:** SPX/SPY/QQQ index options; 0DTE now ~50–67% of SPX options volume; dealer long-gamma hedging dampens realized vol and can "pin" price near heavy-open-interest strikes, roughly 10:30am–2:30pm ET absent a catalyst.
- **Window:** midday, same-day-expiry days (now effectively every trading day for SPX/SPY).
- **Edge:** described qualitatively (vol compression, pinning); no net-of-cost directional-equity edge quantified in the sources.
- **Transfers to:** SPY/QQQ (home, daily listed expiries) | **Not to:** sector ETFs or single names without daily-expiry options liquidity at comparable scale (mega-caps have weekly, not daily — weak transfer at best).
- **Status:** untested. A crude realized-range-compression proxy is cheap on existing bars; the real gamma-exposure signal needs options OI data we lack.

### 20. Lunch-hour volume trough / U-shaped volume and volatility
- **Papers:** Admati & Pfleiderer, "A Theory of Intraday Patterns," Review of Financial Studies 1988 [from memory, foundational, not re-verified by URL]; supporting recent evidence via ResearchGate 46511595, "Are Intraday Volume and Volatility U-Shaped After Accounting for Public Information?" (https://www.researchgate.net/publication/46511595). [extremely well-replicated, textbook market-microstructure result]
- **Home/mechanism:** essentially universal across liquid equities/ETFs; information-driven trading clusters at the open, portfolio-rebalancing trading clusters at the close, leaving a midday volume/volatility trough.
- **Window:** roughly 12:00–1:30pm ET.
- **Edge:** "typical midday drops are in the tens of percent" vs. the morning peak — a liquidity/regime fact, not a directional edge by itself.
- **Transfers to:** all instrument classes (home is universal) | **Not to:** n/a — this is a filter/regime fact, not a standalone directional rule.
- **Status:** effectively already "built," not separately backtested as a standalone anomaly — `crates/indicators/src/custom/rvol.rs` ("relative volume vs. time-of-day average") already encodes a time-of-day volume baseline. No further pretest needed as a new strategy; relevant as a filter input only.

### 21. Monday / weekend effect
- **Papers:** French, "Stock Returns and the Weekend Effect," Journal of Financial Economics 1980 [from memory, foundational]; reassessed in "Weekends Can Be Rough," Boston Fed WP98-6 (https://www.bostonfed.org/-/media/Documents/Workingpapers/PDF/wp98_6.pdf); ASU 2017 news summary of work finding it gone in the US (https://news.asu.edu/20170201-discoveries-asu-research-debunks-myth-stock-market-weekend-effect). [well-replicated originally; **well-documented decay/disappearance in US large caps post-1975**, some persistence reported in emerging markets]
- **Home/mechanism:** broad US equities historically; individual-investor Friday-to-Monday sentiment/settlement-timing stories, none fully confirmed.
- **Window:** Friday close → Monday close.
- **Edge:** 1926–1974 Monday returns averaged down 18.1bps; 1975–2014 averaged down only ~5bps, not statistically significant per the cited reanalysis.
- **Transfers to:** nothing promising in US large caps today | **Not to:** our instrument set specifically, given the documented US decay.
- **Status:** untested by us and low priority — well-replicated as a *historical* effect but well-replicated as *decayed* in the modern US market we trade.

### 22. VWAP intraday mean reversion
- **Papers:** none rigorous found — `docs/analysis/2026-09-15_second_window_research.md` (#9) itself flags this as "folklore — no traceable primary source, only content-farm claims." **[not an academically documented anomaly; included because it is widely traded in practice and we built and tested it]**
- **Home/mechanism:** claimed on individual stocks; price reverting to session VWAP after a stretch away from it — plausible liquidity-provision story, no rigorous empirical source.
- **Window:** intraday, any stretch away from VWAP.
- **Edge:** no credible net-of-cost figure in the literature.
- **Transfers to:** unclear given no rigorous home instrument | **Not to:** n/a.
- **Status:** tested correctly on our mega-caps — `research/vwap_short/2026-09-27_vwap_anchored_short.md`: VWAP-anchored short retest on crash days **fails as specified** ("on a genuine down day names rarely return to VWAP once the market is visibly down"); best variant beats baseline only on tail days, loses on ordinary days. No pipeline patch written. A clean illustration of a folklore claim getting a fair, correctly-instrumented test and failing.

### 23. Short-term reversal (1–5 day), single names
- **Papers:** Jegadeesh, "Evidence of Predictable Behavior of Security Returns," Journal of Finance 1990 [from memory, foundational]; Lehmann, Quarterly Journal of Economics 1990 [from memory]; decomposition/caveat in NY Fed SR513, "Decomposing Short-Term Return Reversal" (https://www.newyorkfed.org/medialibrary/media/research/staff_reports/sr513.pdf) — flags bid-ask bounce and illiquidity price-pressure as confounds, not pure overreaction. [well-replicated, but economically murky — much of the "edge" may be microstructure noise, not a tradeable premium]
- **Home/mechanism:** individual stocks; week/month-level contrarian strategy on own-lagged return — overreaction or liquidity-provision compensation, contested.
- **Window:** 1-week to 1-month holding in the original papers; faster versions (1–5 day) inherit the same liquidity-confound concerns more acutely.
- **Edge:** Jegadeesh (1990) reports ~2%/month extra return 1934–1987; modern, cost-aware replications are scarcer and the bid-ask-bounce caveat is serious at short horizons.
- **Transfers to:** our mega-caps (home, individual stocks) | **Not to:** index/ETF level (diversification kills idiosyncratic reversal).
- **Status:** tested correctly (our mega-caps are part of the 59-name daily universe) in `research/swing/2026-09-26_multiday_long_research.md` — **reversal listed explicitly as dead**, alongside breakout momentum, same study.

### 24. Retail order flow / payment-for-order-flow-induced reversal
- **Papers:** Boehmer, Jones, Zhang & Zhang, "Tracking Retail Investor Activity," Journal of Finance 2021 (https://www.researchgate.net/publication/320938250_Tracking_Retail_Investor_Activity). [well-replicated — the sub-penny-print identification method is now standard in the market-microstructure literature]
- **Home/mechanism:** individual stocks; wholesalers internalize retail orders (identifiable by sub-penny execution prices); institutional order flow arrives opposite retail flow within 5 minutes, producing short-horizon reversal around retail prints.
- **Window:** minutes, around identified retail executions.
- **Edge:** "more than 400 institutional shares" opposite an unexpected 1,000-share retail order in the first 5 minutes, per the paper; directional, not independently net-of-cost quantified by us.
- **Transfers to:** our mega-caps (home, high retail-flow single names) | **Not to:** ETFs (retail internalization economics differ).
- **Status:** untested — **blocked on data:** requires tick-level, sub-penny-execution trade data to identify retail flow; we only have 1-minute bars. Lowest-priority "home" cell given the data gap.

---

## process: replicate on the home instrument first, then vary

- **Rule:** before testing any documented anomaly on our 4 mega-caps (or any instrument class), first check `research/edge_matrix.md` for its mechanism's documented "home" instrument class. If we haven't correctly tested it there yet, test there first — a proxy test on the wrong instrument (as in #2/#3's first pass) produces a false negative, not evidence the edge is dead.
- **Staging:** matrix cell flagged `home` + untested → cheap Python pre-test on cached bars (data we already have: 1m IEX bars 2022→ for the liquid names/ETFs, daily bars 2021→ for ~59 names) with a pre-registered kill criterion → if it clears, an engine study (`research/<topic>/`) → pipeline candidate (`scripts/pipeline/pipeline.py propose`) → shadow book → promotion proposal (human-gated).
- **Data we lack, and what it rules out for now:** no options/gamma-exposure or OI data (#10, #12's dealer-gamma leg, #19); no closing-auction imbalance feed (#11); no VIX futures data (#13); no tick-level sub-penny execution data (#24). These stay untested-by-necessity until/unless we add a data source — don't spend engine-study effort on them before then.
