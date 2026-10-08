---
name: ideate
description: Weekly ideation check-in (runs Sunday 18:00 PT). Reads the edge catalog, the edge matrix and the pretest summary, and proposes up to three new stage-0 pretest cells for documented-but-untested home-instrument anomalies (or variations on a cell that already passed pretest). Writes new cells only — never edits an existing cell, the harness, or any strategy config.
---

# weekly ideation

you are the weekly idea generator for the research pipeline (`CLAUDE.md` "the wheel",
`docs/edge_catalog.md`, `research/edge_matrix.md`, `research/pretest/README.md`). you have
**authority to add new stage-0 pretest cells only** — nothing you do touches the engine,
the live config, or a cell that already exists.

## hard rules

- no engine sweeps, no backtest runs, no `scripts/pipeline/` proposals.
- you may only **add** entries to `research/pretest/cells.py`'s `CELLS` list. never edit an
  existing cell, `research/pretest/harness.py`, or any file outside the three named below.
- one commit, touching only: `research/pretest/cells.py`, `research/edge_matrix.md`,
  `docs/ideation/<date>.md`.
- if fewer than 3 good candidates exist, propose fewer — do not pad the list.

## 1. read

- `docs/edge_catalog.md` — the 24 documented anomalies (process section at the bottom).
- `research/edge_matrix.md` — the anomaly x instrument matrix and its ranked top-10 list.
- `research/pretest/results/summary.md` — current cell verdicts, so you don't re-propose
  something already `dead` or already `pass-pretest`.
- `docs/pipeline/status.md` — candidates already in the live research/promotion lane; don't
  duplicate one.
- the last 5 `docs/reports/*.md` (`ls -t docs/reports/*.md | head -5`) for anything the eod
  routine flagged about an anomaly's live relevance or decay.

## 2. pick

up to 3 candidates, each one of:
- an untested `home` cell from the matrix (instrument marked `home`, no `dead`/`done`/
  `pass-pretest`/`needs-product` superscript), preferring the matrix's own ranked list; or
- a variation on an existing `pass-pretest` cell (a different grid point, a stricter filter,
  a nearby instrument) worth its own fresh kill criterion.

skip anything the catalog's process section marks data-blocked (#10, #11, #12's gamma leg,
#13, #19, #24) — no new data source exists yet, so a pretest there can't be cheap.

## 3. write

for each candidate:
- **a new cell** in `research/pretest/cells.py`: append a dict to `CELLS` following the
  existing `Rule`/cell pattern (id, anomaly, instruments, variants with a rule function,
  a **pre-registered kill criterion** written into `kill_text` before anything is run,
  `verdict_fn`/`verdict`). reuse `standard_verdict(years_pos, pf)` / `cell_verdict` unless the
  cell genuinely needs its own. do not run `harness.py` yourself — `pretest.timer` runs it
  next and will report the verdict in the changelog.
- **a dated note** appended to `research/edge_matrix.md`'s "top 10" ranked list (or just below
  it if the list already has 10): one line, citing the catalog anomaly number and why it's
  worth a pretest now.
- **`docs/ideation/<date>.md`** (≤ 60 lines): what you proposed and why — one paragraph per
  candidate citing the catalog entry and matrix row — plus, if you proposed fewer than 3, one
  sentence on why the rest didn't clear the bar.

## 4. commit

one commit, message `ideate: <date> — N new cell(s)` (or `ideate: <date> — no new cells`),
touching only the three files above. **do not `git push`** (matches `eod-review.service`'s
`--disallowedTools`) — the next nightly research/pipeline/pretest job pushes it along with
its own commit.

## output

end with one line: `ideate: proposed N cell(s): <ids>` (or `ideate: no candidates this week`).

## verdict standard (binding for every cell you add)

a pre-test cell is a NEIGHBOURHOOD, not a point: pre-register in the cell a small grid around the
paper's rule (holding window 2–3 values, threshold/decile cut 2–3 values, time of day where
relevant, and the other instruments in the home class). the harness stores every grid cell. the
verdict vocabulary is: dead (no gross edge anywhere), sub-cost (gross edge exists, net < costs —
state the gross bps), decayed (positive early years, ≤ 0 in the last two), fragile (passes but
concentrated in one name or the top-3 trades), pass. never retire an idea on one point estimate.
