---
name: recession-watch
description: Daily recession-watch check-in (runs ~05:45 PT Mon-Fri, after scripts/macro/fetch_indicators.py). Reads today's rubric BASE score, searches recent news for qualitative macro stress, and writes docs/macro/<date>.md with a FINAL 0-10 score. Information only — never touches config, migrations, crates, or the trader.
---

# recession watch (daily, information only)

> this score is a disciplined checklist, not a forecast. yield-curve, credit, labor, and
> equity-stress indicators have historically led recessions by months (sometimes over a year)
> and have produced real false positives. **nothing you write here may be read into trading
> config, gates, or the trader** — this is a human-facing number only. you may only edit files
> under `docs/macro/` and `data/macro/`.

## 1. read today's base score

read `data/macro/<today YYYY-MM-DD>.json` (the systemd unit ran `fetch_indicators.py` first).
if it's missing, run `python3 scripts/macro/fetch_indicators.py` yourself, then read it. note
`swing_cache_as_of` and `series_failed` — mention any stale/missing inputs in the report.

compare today's `base` and component scores to yesterday's row in `data/macro/history.csv`
(and the `momentum` field: base 5 and 20 sessions ago) — this is your "what changed" section.

## 2. qualitative read — up to 8 web searches/fetches, last 24 hours only

search for genuine recent (last 24h) developments in these seven areas — one or two searches
each, stop once you have enough to judge, never exceed 8 total:

1. credit / banking stress (bank failures, funding-market stress, large spread moves)
2. sovereign or large-issuer defaults
3. oil / energy price shocks
4. geopolitical escalation with a stated market impact
5. broad earnings-guidance cuts (multiple large companies, not one name)
6. Fed / rates surprises (emergency moves, surprise statements)
7. major data releases today (payrolls, CPI, ISM) and what they showed

**cite every claim** — every qualitative statement in the report needs a source URL from
WebSearch/WebFetch. assign a qualitative score **0-2**:
- 0: nothing notable beyond the base indicators
- 1: one credible, cited stress signal (e.g. a guidance-cut wave, a surprise hot CPI)
- 2: multiple corroborating stress signals, or one severe one (e.g. a large-issuer default,
  a credit-market freeze headline)

## 3. compute FINAL and write the report

`FINAL = min(10, base + qualitative)`.

write `docs/macro/<date>.md`:
- header stating plainly: this is a disciplined checklist, not a forecast; these indicators
  lead by months and have false positives; information only, never fed into trading config.
- **score**: `FINAL/10 (base B + qualitative Q)`
- **components table**: component, score, value, one-line detail (from the json)
- **what changed vs yesterday**: base delta, which component(s) moved, momentum (5/20 session)
- **qualitative evidence**: one bullet per area searched, with URL citations (or "nothing
  notable, searched: <areas>" if none found)
- **what would move it up / down**: 2-4 concrete bullets (e.g. "HY OAS crossing 400bps",
  "a second consecutive >=20% claims y/y reading", "SPY closing below its 200dma")
- **REQUIRED when FINAL >= 6**: a plain-language paragraph explaining why, for a non-technical
  reader — which components fired and what the cited news means in practice

## 4. update history.csv (upsert — do not duplicate the row)

`data/macro/history.csv` has one row per date (`fetch_indicators.py` already wrote today's
row with `base` + components). read the file, find the row where `date` == today, set its
`qualitative` and `final` columns, and rewrite the file with every other row unchanged (python
`csv` module, in a `Bash(python3 scripts/macro/*)` call, or `Edit`). if today's row is somehow
missing, append one with `base`/components blank and only `date`/`qualitative`/`final` set,
and say so plainly in the report — that is a plumbing problem to flag, not to fix here.

## 5. notify

- FINAL <= 5: `./scripts/notify.sh info "recession watch <FINAL>/10 (base <B>): <one line>"`
- FINAL == 6: `./scripts/notify.sh warning "recession watch 6/10: <reasons, one line>"`
- FINAL >= 7: `./scripts/notify.sh critical "recession watch <FINAL>/10: <reasons, one line>"`

## 6. commit

one commit, touching only `docs/macro/<date>.md` and `data/macro/history.csv`:
`git add docs/macro/<date>.md data/macro/history.csv && git commit -m "recession-watch: <date> — <FINAL>/10 (base <B> + qual <Q>)"`.
**do not `git push`** — the next nightly job pushes it. do not touch anything outside
`docs/macro/` and `data/macro/`: never config, migrations, crates, `scripts/pipeline/`, or any
other doc (`CLAUDE.md`, `docs/routines/eod-report.md` already link here; don't re-edit them).

## output

end with one line: `recession-watch: <FINAL>/10 (base <B> + qual <Q>)`.
