import csv
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import metrics  # noqa: E402

TRADE_COLS = ["row_type", "date", "ticker", "direction", "entry_time", "exit_time", "entry_price",
              "exit_price", "size", "pnl", "pnl_pct", "hold_duration_ms", "exit_reason", "entry_reason"]


def write_trades(path: Path, rows: list[dict]) -> None:
    with open(path, "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=TRADE_COLS)
        w.writeheader()
        for r in rows:
            row = {c: "" for c in TRADE_COLS}
            row["row_type"] = "trade"
            row.update(r)
            w.writerow(row)


def trade(date, ticker, direction, entry_time, pnl, entry_reason="window:x", exit_time=None):
    row = {"date": date, "ticker": ticker, "direction": direction, "entry_time": entry_time,
           "pnl": pnl, "entry_reason": entry_reason}
    if exit_time is not None:
        row["exit_time"] = exit_time
    return row


class SplitMarginal(unittest.TestCase):
    def test_identical_trades_are_base_the_rest_is_added(self):
        base_rows = [trade("2022-01-04", "NVDA", "Short", "2022-01-04T15:01:00+00:00", 100.0)]
        cand_rows = list(base_rows) + [trade("2022-01-05", "AAPL", "Short", "2022-01-05T15:01:00+00:00", 50.0)]
        base, added = metrics.split_marginal(cand_rows, base_rows)
        self.assertEqual(len(base), 1)
        self.assertEqual(len(added), 1)
        self.assertEqual(added[0]["pnl"], 50.0)

    def test_matches_on_identity_not_pnl(self):
        # same (date, ticker, entry_time, direction) but a different fill/pnl still counts as base:
        # the point is whether the trade fired, not whether its price drifted
        base_rows = [trade("2022-01-04", "NVDA", "Short", "2022-01-04T15:01:00+00:00", 100.0)]
        cand_rows = [trade("2022-01-04", "NVDA", "Short", "2022-01-04T15:01:00+00:00", 99.5)]
        base, added = metrics.split_marginal(cand_rows, base_rows)
        self.assertEqual(len(base), 1)
        self.assertEqual(len(added), 0)

    def test_direction_and_ticker_distinguish_same_minute_trades(self):
        base_rows = [trade("2022-01-04", "NVDA", "Short", "2022-01-04T15:01:00+00:00", 100.0)]
        cand_rows = [trade("2022-01-04", "NVDA", "Long", "2022-01-04T15:01:00+00:00", -10.0)]
        base, added = metrics.split_marginal(cand_rows, base_rows)
        self.assertEqual(len(base), 0)
        self.assertEqual(len(added), 1)

    def test_vacuous(self):
        self.assertEqual(metrics.split_marginal([], []), ([], []))
        base, added = metrics.split_marginal([trade("2022-01-04", "X", "Short", "t", 1.0)], [])
        self.assertEqual(base, [])
        self.assertEqual(len(added), 1)


class MarginalMetrics(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        for y in metrics.YEARS:
            write_trades(self.dir / f"base_{y}_trades.csv", [])
            write_trades(self.dir / f"cand_{y}_trades.csv", [])
        base_trades = [
            trade("2022-01-04", "NVDA", "Short", "2022-01-04T15:01:00+00:00", 100.0),
            trade("2022-01-05", "AAPL", "Short", "2022-01-05T15:01:00+00:00", -20.0),
        ]
        write_trades(self.dir / "base_2022_trades.csv", base_trades)
        write_trades(self.dir / "cand_2022_trades.csv", base_trades + [
            trade("2022-02-01", "MSFT", "Short", "2022-02-01T15:01:00+00:00", 40.0, "window:new"),
            trade("2022-02-02", "MSFT", "Short", "2022-02-02T15:01:00+00:00", -5.0, "window:new"),
        ])

    def tearDown(self):
        self.tmp.cleanup()

    def test_base_matches_baseline_exactly_when_untouched(self):
        mg = metrics.marginal_metrics("cand", "base", self.dir)
        self.assertEqual(mg["base"]["pnl"], 80.0)
        self.assertEqual(mg["base"]["n"], 2)

    def test_added_is_the_new_window_only(self):
        mg = metrics.marginal_metrics("cand", "base", self.dir)
        self.assertEqual(mg["added"]["pnl"], 35.0)
        self.assertEqual(mg["added"]["n"], 2)
        self.assertAlmostEqual(mg["added"]["pf"], 40.0 / 5.0)
        self.assertEqual(mg["added"]["worst_trade"], -5.0)
        self.assertEqual(mg["added"]["worst_day"], -5.0)
        self.assertEqual(mg["added"]["years"]["2022"]["pnl"], 35.0)
        # years with no added trades are 0, not missing — the floor check treats them as flat, not a loss
        self.assertEqual(mg["added"]["worst_year_pnl"], 0.0)

    def test_full_metrics_attaches_marginal_only_when_baseline_tag_given(self):
        m = metrics.full_metrics("cand", data_dir=self.dir)
        self.assertNotIn("marginal", m)
        m = metrics.full_metrics("cand", data_dir=self.dir, baseline_tag="base")
        self.assertIn("marginal", m)
        self.assertEqual(m["marginal"]["added"]["pnl"], 35.0)

    def test_combined_is_baseline_rows_plus_added_rows(self):
        # the additive-ticker gate's input: baseline_rows (80 pnl / 2 trades) + added_rows (35 / 2) —
        # exact concatenation, since a different-ticker candidate's trades never match a baseline key
        mg = metrics.marginal_metrics("cand", "base", self.dir)
        self.assertEqual(mg["combined"]["pnl"], 80.0 + 35.0)
        self.assertEqual(mg["combined"]["n"], 2 + 2)
        self.assertEqual(mg["combined"]["years"]["2022"]["pnl"], 80.0 + 35.0)
        self.assertIn("concurrency", mg)
        self.assertEqual(mg["concurrency"]["n_exceedance_days"], 0)  # no entry/exit_time in this fixture


class ConcurrencyExceedance(unittest.TestCase):
    """metrics.concurrency_exceedance(): info only (additive-ticker gate), never a pass/fail check."""

    def test_no_overlap_is_fine(self):
        rows = [
            trade("2024-01-02", "A", "Short", "2024-01-02T10:00:00+00:00", 1.0, exit_time="2024-01-02T10:05:00+00:00"),
            trade("2024-01-02", "B", "Short", "2024-01-02T10:10:00+00:00", 1.0, exit_time="2024-01-02T10:15:00+00:00"),
        ]
        out = metrics.concurrency_exceedance(rows, cap=3)
        self.assertEqual(out["n_exceedance_days"], 0)
        self.assertEqual(out["max_concurrent_observed"], 1)

    def test_four_concurrent_exceeds_cap_of_three(self):
        rows = [
            trade("2024-01-02", "A", "Short", "2024-01-02T10:00:00+00:00", 1.0, exit_time="2024-01-02T10:30:00+00:00"),
            trade("2024-01-02", "B", "Short", "2024-01-02T10:05:00+00:00", 1.0, exit_time="2024-01-02T10:25:00+00:00"),
            trade("2024-01-02", "C", "Short", "2024-01-02T10:10:00+00:00", 1.0, exit_time="2024-01-02T10:20:00+00:00"),
            trade("2024-01-02", "D", "Short", "2024-01-02T10:12:00+00:00", 1.0, exit_time="2024-01-02T10:15:00+00:00"),
        ]
        out = metrics.concurrency_exceedance(rows, cap=3)
        self.assertEqual(out["n_exceedance_days"], 1)
        self.assertEqual(out["exceedance_days"], ["2024-01-02"])
        self.assertEqual(out["max_concurrent_observed"], 4)

    def test_close_and_open_at_the_same_instant_do_not_double_count(self):
        # A/B/C open (3 concurrent); A closes exactly when D opens — a close frees the slot before
        # the open reuses it, so concurrency never ticks up to 4
        rows = [
            trade("2024-01-02", "A", "Short", "2024-01-02T09:55:00+00:00", 1.0, exit_time="2024-01-02T10:00:00+00:00"),
            trade("2024-01-02", "B", "Short", "2024-01-02T09:56:00+00:00", 1.0, exit_time="2024-01-02T10:05:00+00:00"),
            trade("2024-01-02", "C", "Short", "2024-01-02T09:57:00+00:00", 1.0, exit_time="2024-01-02T10:05:00+00:00"),
            trade("2024-01-02", "D", "Short", "2024-01-02T10:00:00+00:00", 1.0, exit_time="2024-01-02T10:05:00+00:00"),
        ]
        out = metrics.concurrency_exceedance(rows, cap=3)
        self.assertEqual(out["max_concurrent_observed"], 3)
        self.assertEqual(out["n_exceedance_days"], 0)

    def test_separate_days_counted_independently(self):
        rows = [
            trade("2024-01-02", "A", "Short", "2024-01-02T10:00:00+00:00", 1.0, exit_time="2024-01-02T10:30:00+00:00"),
            trade("2024-01-02", "B", "Short", "2024-01-02T10:05:00+00:00", 1.0, exit_time="2024-01-02T10:25:00+00:00"),
            trade("2024-01-02", "C", "Short", "2024-01-02T10:10:00+00:00", 1.0, exit_time="2024-01-02T10:20:00+00:00"),
            trade("2024-01-02", "D", "Short", "2024-01-02T10:12:00+00:00", 1.0, exit_time="2024-01-02T10:15:00+00:00"),
            trade("2024-01-03", "E", "Short", "2024-01-03T10:00:00+00:00", 1.0, exit_time="2024-01-03T10:05:00+00:00"),
        ]
        out = metrics.concurrency_exceedance(rows, cap=3)
        self.assertEqual(out["exceedance_days"], ["2024-01-02"])

    def test_rows_missing_entry_or_exit_time_are_skipped_not_fatal(self):
        rows = [trade("2024-01-02", "A", "Short", "2024-01-02T10:00:00+00:00", 1.0)]  # no exit_time
        out = metrics.concurrency_exceedance(rows, cap=3)
        self.assertEqual(out["max_concurrent_observed"], 0)
        self.assertEqual(out["n_exceedance_days"], 0)


class MaxPositionFraction(unittest.TestCase):
    def test_largest_wins(self):
        rows = [{"size": "30", "entry_price": "98.0"}, {"size": "37", "entry_price": "98.0"}, {"size": "10", "entry_price": "50.0"}]
        # 30*98/10000 = 0.294, 37*98/10000 = 0.3626, 10*50/10000 = 0.05
        self.assertAlmostEqual(metrics.max_position_fraction(rows), 0.3626)

    def test_empty_rows_is_zero(self):
        self.assertEqual(metrics.max_position_fraction([]), 0.0)

    def test_unparseable_rows_are_skipped_not_fatal(self):
        rows = [{"size": "", "entry_price": ""}, {"size": "35", "entry_price": "105.1135"}]
        self.assertAlmostEqual(metrics.max_position_fraction(rows), 35 * 105.1135 / 10000, places=4)

    def test_custom_capital(self):
        rows = [{"size": "100", "entry_price": "50.0"}]
        self.assertAlmostEqual(metrics.max_position_fraction(rows, capital=5000.0), 1.0)

    def test_summarize_folds_it_in(self):
        with tempfile.TemporaryDirectory() as d:
            d = Path(d)
            write_trades(d / "sz_2025_trades.csv", [
                {"date": "2025-01-02", "ticker": "AMZN", "direction": "Short", "entry_time": "t1", "pnl": 10.0, "size": "28", "entry_price": "105.1135"},
                {"date": "2025-01-03", "ticker": "NVDA", "direction": "Short", "entry_time": "t2", "pnl": -5.0, "size": "37", "entry_price": "98.1555"},
            ])
            m = metrics.summarize("sz", d)
            self.assertAlmostEqual(m["max_position_fraction"], 37 * 98.1555 / 10000, places=4)


if __name__ == "__main__":
    unittest.main()
