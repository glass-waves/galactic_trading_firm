# signal research patches

each file is a `--patch-json` for the backtest: it disables the promoted config's entry
windows and gates, adds zero-weight indicators, and adds one candidate long window, so the
candidate is measured in isolation. run a candidate over a date range with

    ./scripts/backtest_range.sh sig_<name> 2026-01-02 2026-09-10 \
        --sizing-fraction 0.36 --max-position-pct 0.36 --patch-json research/signals/<name>.json

and compare with `scripts/report_backtest.py --year 2026 --compare sig_longbook_control sig_<name>`.
the bar to clear (long book alone, 36 %): 2026 +$2,095, 2025 −$2,635.
