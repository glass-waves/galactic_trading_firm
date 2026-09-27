import csv
import datetime as dt
import sys
import tempfile
import unittest
from pathlib import Path
from zoneinfo import ZoneInfo

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import metrics  # noqa: E402
import shadow  # noqa: E402

ET = ZoneInfo("America/New_York")
D = dt.date


class Due(unittest.TestCase):
    def test_min_sessions_and_trades(self):
        self.assertFalse(shadow.due(19, 100, 20, 15)[0])
        self.assertFalse(shadow.due(20, 14, 20, 15)[0])
        self.assertTrue(shadow.due(20, 15, 20, 15)[0])

    def test_time_limit_regardless_of_trades(self):
        ok, why = shadow.due(60, 0, 20, 15)
        self.assertTrue(ok)
        self.assertIn("time limit", why)
        self.assertFalse(shadow.due(59, 0, 20, 15)[0])


class NotHosted(unittest.TestCase):
    def test_weekday_fallback(self):
        # started friday 2026-09-25; mon 28, tue 29, wed 30 are three trading days
        self.assertEqual(shadow.trading_days_between(D(2026, 9, 25), D(2026, 9, 30)), 3)
        self.assertEqual(shadow.not_hosted(D(2026, 9, 25), [], D(2026, 9, 29)), (False, 2))
        self.assertEqual(shadow.not_hosted(D(2026, 9, 25), [], D(2026, 9, 30)), (True, 3))

    def test_calendar_from_primary_sessions_is_exact(self):
        cal = [D(2026, 9, 28), D(2026, 9, 30), D(2026, 10, 2)]  # the trader was down on the 29th and 1st
        self.assertEqual(shadow.trading_days_between(D(2026, 9, 25), D(2026, 10, 1), cal), 2)
        self.assertFalse(shadow.not_hosted(D(2026, 9, 25), [], D(2026, 10, 1), cal)[0])
        self.assertTrue(shadow.not_hosted(D(2026, 9, 25), [], D(2026, 10, 2), cal)[0])

    def test_hosted_book_never_flags(self):
        self.assertFalse(shadow.not_hosted(D(2026, 9, 1), [D(2026, 9, 2)], D(2026, 12, 1))[0])


class Parity(unittest.TestCase):
    def t(self, pnl, price=100.0, size=10):
        return {"pnl": pnl, "entry_price": price, "size": size}

    def test_count_tolerance(self):
        r = shadow.parity([self.t(1)] * 7, [self.t(1)] * 10)
        self.assertTrue(r["count_ok"])
        r = shadow.parity([self.t(1)] * 6, [self.t(1)] * 10)
        self.assertFalse(r["count_ok"])
        self.assertFalse(r["ok"])

    def test_bps_tolerance(self):
        # 10 bps on a $1000 notional = $1
        r = shadow.parity([self.t(2.0)], [self.t(1.0)])
        self.assertTrue(r["bps_ok"])
        r = shadow.parity([self.t(2.1)], [self.t(1.0)])
        self.assertFalse(r["bps_ok"])

    def test_vacuous(self):
        self.assertTrue(shadow.parity([], [])["ok"])
        self.assertTrue(shadow.parity([self.t(1)], [])["ok"])
        self.assertFalse(shadow.parity([self.t(1), self.t(1)], [])["ok"])


class Edge(unittest.TestCase):
    def t(self, pnl, price=100.0, size=10):
        return {"pnl": pnl, "entry_price": price, "size": size}

    def test_ticker_kind(self):
        r = shadow.edge("ticker", [self.t(3), self.t(-2)], [self.t(1)])
        self.assertTrue(r["pass"], r)
        r = shadow.edge("ticker", [self.t(1), self.t(-2)], [self.t(1)])
        self.assertIn("shadow_pf", [c["name"] for c in r["checks"] if not c["ok"]])

    def test_config_kind_vs_primary(self):
        r = shadow.edge("config", [self.t(3), self.t(-2)], [self.t(0.5)], primary_pnl=1.25)
        self.assertTrue(r["pass"], r)
        r = shadow.edge("config", [self.t(3), self.t(-2)], [self.t(0.5)], primary_pnl=1.26)
        self.assertEqual([c["name"] for c in r["checks"] if not c["ok"]], ["shadow_pnl_vs_primary"])


def write_bars(path: Path, days: list[dt.date], bars_per_day: int = 2, start_price: float = 100.0, close_delta: float = 0.0):
    with open(path, "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["timestamp", "open", "high", "low", "close", "volume"])
        for d in days:
            t0 = dt.datetime.combine(d, dt.time(9, 30), tzinfo=ET)
            for i in range(bars_per_day):
                ts = int((t0 + dt.timedelta(minutes=i)).timestamp())
                close = start_price + (close_delta if i == bars_per_day - 1 else 0)
                w.writerow([ts, start_price, start_price, start_price, close, 100])


def weekdays(start: dt.date, n: int) -> list[dt.date]:
    out, d = [], start
    while len(out) < n:
        if d.weekday() < 5:
            out.append(d)
        d += dt.timedelta(days=1)
    return out


class Coverage(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        # SPY: 250 sessions in 2025 (full year) + 40 in 2026 (partial)
        self.spy_days = weekdays(D(2025, 1, 2), 250) + weekdays(D(2026, 1, 2), 40)
        write_bars(self.dir / "SPY.csv", self.spy_days)

    def tearDown(self):
        self.tmp.cleanup()

    def test_sessions_are_eastern_dates(self):
        self.assertEqual(len(metrics.sessions(self.dir / "SPY.csv")), 290)

    def test_full_coverage_ok(self):
        write_bars(self.dir / "X.csv", self.spy_days)
        c = metrics.coverage("X", self.dir)
        self.assertTrue(c["ok"], c["problems"])
        self.assertEqual(c["years"]["2025"]["required"], 240)
        self.assertEqual(c["years"]["2026"]["required"], 38)  # ceil(0.95 * 40)

    def test_full_year_below_240_fails(self):
        write_bars(self.dir / "X.csv", self.spy_days[:239] + self.spy_days[250:])
        c = metrics.coverage("X", self.dir)
        self.assertFalse(c["ok"])
        self.assertTrue(any("2025" in p for p in c["problems"]))

    def test_partial_year_95pct_rule(self):
        write_bars(self.dir / "X.csv", self.spy_days[:250] + self.spy_days[250:287])  # 37 of 40 < 38
        c = metrics.coverage("X", self.dir)
        self.assertIn("2026: 37 sessions < 38 required", c["problems"])

    def test_hole_detection(self):
        days = self.spy_days[:100] + self.spy_days[106:]  # 6 consecutive missing, counts still >= thresholds
        write_bars(self.dir / "X.csv", days)
        c = metrics.coverage("X", self.dir)
        self.assertEqual(c["max_gap"], 6)
        self.assertTrue(any(p.startswith("hole") for p in c["problems"]))
        days = self.spy_days[:100] + self.spy_days[105:]  # 5 missing is tolerated
        write_bars(self.dir / "X.csv", days)
        self.assertTrue(metrics.coverage("X", self.dir)["ok"])

    def test_stale_tail(self):
        write_bars(self.dir / "X.csv", self.spy_days[:-1])
        c = metrics.coverage("X", self.dir)
        self.assertTrue(any(p.startswith("stale") for p in c["problems"]))

    def test_missing_file(self):
        c = metrics.coverage("NOPE", self.dir)
        self.assertFalse(c["ok"])
        self.assertFalse(c["exists"])


class SpyBuckets(unittest.TestCase):
    def test_day_return_and_buckets(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "SPY.csv"
            days = [D(2025, 3, 3), D(2025, 3, 4), D(2025, 3, 5)]
            with open(p, "w", newline="") as fh:
                w = csv.writer(fh)
                w.writerow(["timestamp", "open", "high", "low", "close", "volume"])
                for d, (o, c) in zip(days, [(100, 98.5), (100, 100.5), (100, 99.2)]):
                    t0 = dt.datetime.combine(d, dt.time(9, 30), tzinfo=ET)
                    w.writerow([int(t0.timestamp()), o, o, o, o, 1])
                    w.writerow([int((t0 + dt.timedelta(hours=6)).timestamp()), o, o, o, c, 1])
            r = metrics.spy_day_returns(p)
            self.assertAlmostEqual(r["2025-03-03"], -0.015)
            self.assertAlmostEqual(r["2025-03-05"], -0.008)
            rows = [{"date": "2025-03-03", "pnl": -50.0}, {"date": "2025-03-03", "pnl": 20.0}, {"date": "2025-03-04", "pnl": 10.0}]
            b = metrics.spy_buckets(rows, r, -0.01, D(2025, 1, 1), D(2025, 12, 31))
            self.assertEqual(b["stress_days"], 1)
            self.assertEqual(b["stress_trades"], 2)
            self.assertEqual(b["stress_pnl"], -30.0)
            self.assertEqual(b["stress_worst_day"], -30.0)
            self.assertEqual(b["ordinary_pnl"], 10.0)


class Stats(unittest.TestCase):
    def test_summarize_maths(self):
        rows = [{"pnl": 10.0, "date": "2025-01-02"}, {"pnl": -5.0, "date": "2025-01-03"}, {"pnl": -10.0, "date": "2025-01-03"}]
        s = metrics.stats(rows)
        self.assertEqual(s["pnl"], -5.0)
        self.assertEqual(s["n"], 3)
        self.assertAlmostEqual(s["pf"], 10 / 15, places=3)
        self.assertEqual(s["dd"], -15.0)
        self.assertEqual(metrics.stats([])["n"], 0)
        self.assertEqual(metrics.stats([{"pnl": 1.0, "date": "x"}])["pf"], metrics.PF_CAP)


if __name__ == "__main__":
    unittest.main()
