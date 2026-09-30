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


def trade(date, ticker, direction, entry_time, pnl, entry_reason="window:x"):
    return {"date": date, "ticker": ticker, "direction": direction, "entry_time": entry_time,
            "pnl": pnl, "entry_reason": entry_reason}


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


if __name__ == "__main__":
    unittest.main()
