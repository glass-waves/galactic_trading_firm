import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import gates  # noqa: E402

YEARS = ["2022", "2023", "2024", "2025", "2026"]


def mk(pnl=2154, n=407, pf=1.48, years=(1565, -181, -93, 451, 413), stress=None, ordinary=None):
    ys = {y: {"pnl": float(p), "n": 10, "pf": 1.0, "dd": 0.0} for y, p in zip(YEARS, years)}
    m = {"pnl": float(pnl), "n": n, "pf": pf, "years": ys,
         "years_positive": sum(1 for p in years if p > 0), "min_year_pnl": float(min(years)), "pnl_2026": float(years[-1])}
    if stress is not None:
        m["stress"] = {"stress_days": 60, "stress_pnl": stress[0], "stress_pnl_per_day": stress[0] / 60, "stress_worst_day": stress[1],
                       "ordinary_pnl": ordinary if ordinary is not None else pnl - stress[0], "stress_trades": 30, "stress_pf": 1.5}
    return m


BASE = mk()


def names(res):
    return gates.failed_names(res)


class DefaultTicker(unittest.TestCase):
    def test_boundaries_pass_exactly_at_threshold(self):
        m = mk(pnl=500, n=100, pf=1.3, years=(100, -300, 100, 100, 100))
        r = gates.default_ticker(m)
        self.assertTrue(r["pass"], names(r))
        self.assertTrue(gates.default_ticker(mk(pf=2, n=100, years=(1, 1, 1, 1, 0)))["pass"])  # 2026 flat is allowed

    def test_each_check_fails_alone(self):
        self.assertEqual(names(gates.default_ticker(mk(pf=1.299, n=100, years=(1, 1, 1, 1, 0)))), ["pf_5y"])
        self.assertEqual(names(gates.default_ticker(mk(pf=2, n=99, years=(1, 1, 1, 1, 0)))), ["trades_5y"])
        self.assertEqual(names(gates.default_ticker(mk(pf=2, n=100, years=(1, 1, -1, -1, 0)))), ["years_positive"])
        self.assertEqual(names(gates.default_ticker(mk(pf=2, n=100, years=(-301, 1, 1, 1, 1)))), ["min_year_pnl"])
        self.assertEqual(names(gates.default_ticker(mk(pf=2, n=100, years=(1, 1, 1, 1, -1)))), ["pnl_2026"])

    def test_amd_like_fails(self):
        r = gates.default_ticker(mk(pf=1.25, n=300, years=(400, -100, -50, 200, 100)))
        self.assertFalse(r["pass"])
        self.assertEqual(set(names(r)), {"pf_5y", "years_positive"})


class VolumeConfig(unittest.TestCase):
    def test_needs_baseline(self):
        r = gates.volume_config(BASE, None)
        self.assertFalse(r["pass"])
        self.assertEqual(names(r), ["baseline"])

    def test_boundaries(self):
        m = mk(pnl=2154 * 0.9, n=407, pf=1.3, years=(1, 1, 1, -300, 1))
        self.assertTrue(gates.volume_config(m, BASE)["pass"])
        self.assertEqual(names(gates.volume_config(mk(n=406, pf=2, years=(1, 1, 1, 1, 1)), BASE)), ["trades_5y"])
        self.assertEqual(names(gates.volume_config(mk(pnl=2154 * 0.9 - 1, pf=2, years=(1, 1, 1, 1, 1)), BASE)), ["pnl_5y"])
        self.assertEqual(names(gates.volume_config(mk(pf=1.29, years=(1, 1, 1, 1, 1)), BASE)), ["pf_5y"])

    def test_negative_baseline_floor_is_below_it(self):
        b = mk(pnl=-100)
        self.assertTrue(gates.volume_config(mk(pnl=-105, pf=2, years=(1, 1, 1, 1, 1)), b)["pass"])
        self.assertIn("pnl_5y", names(gates.volume_config(mk(pnl=-111, pf=2, years=(1, 1, 1, 1, 1)), b)))


class QualityConfig(unittest.TestCase):
    def test_boundaries(self):
        ok = mk(pnl=100, n=int(0.6 * 407) + 1, pf=1.53, years=(1, 1, 1, 1, 1))
        self.assertTrue(gates.quality_config(ok, BASE)["pass"], names(gates.quality_config(ok, BASE)))
        self.assertEqual(names(gates.quality_config(mk(pf=1.529, n=300, years=(1, 1, 1, 1, 1)), BASE)), ["pf_5y"])
        self.assertEqual(names(gates.quality_config(mk(pf=1.6, n=244, years=(1, 1, 1, 1, 1)), BASE)), ["trades_5y"])
        self.assertTrue(gates.quality_config(mk(pf=1.6, n=245, years=(1, 1, 1, 1, 1)), BASE)["pass"])

    def test_vpin_quality_candidate_passes_quality_not_volume(self):
        m = mk(pnl=2000, n=306, pf=1.58, years=(1, 1, 1, 1, 1))
        self.assertTrue(gates.quality_config(m, BASE)["pass"])
        self.assertIn("trades_5y", names(gates.volume_config(m, BASE)))


class StressMode(unittest.TestCase):
    def setUp(self):
        self.b = mk(pnl=2154, n=407, stress=(258, -120), ordinary=1896)

    def test_passes_at_boundaries(self):
        # trades 0.9x, stress pnl 3x, per day exactly 40, worst -300, ordinary within 10 %
        m = mk(pnl=2154 * 0.9 + 3 * 258, n=367, pf=1.3, years=(1, 1, 1, 1, 1), stress=(2400, -300), ordinary=1896 * 0.9)
        r = gates.stress_mode(m, self.b)
        self.assertTrue(r["pass"], names(r))
        self.assertEqual(r["gate"], "stress-mode")

    def test_each_stress_check(self):
        good = dict(pnl=5000, n=400, pf=2.0, years=(1, 1, 1, 1, 1))
        self.assertEqual(names(gates.stress_mode(mk(**good, stress=(773, -10), ordinary=1896), self.b)), ["stress_pnl", "stress_pnl_per_day"])
        self.assertEqual(names(gates.stress_mode(mk(**good, stress=(2400, -301), ordinary=1896), self.b)), ["stress_worst_day"])
        self.assertEqual(names(gates.stress_mode(mk(**good, stress=(2400, -10), ordinary=1896 * 1.11), self.b)), ["ordinary_pnl"])
        self.assertEqual(names(gates.stress_mode(mk(**good, stress=(2400, -10), ordinary=1896 * 0.89), self.b)), ["ordinary_pnl"])
        self.assertEqual(names(gates.stress_mode(mk(pnl=5000, n=366, pf=2.0, years=(1, 1, 1, 1, 1), stress=(2400, -10), ordinary=1896), self.b)), ["trades_5y"])

    def test_missing_buckets(self):
        r = gates.stress_mode(mk(pnl=5000, n=400, pf=2.0, years=(1, 1, 1, 1, 1)), self.b)
        self.assertFalse(r["pass"])
        self.assertIn("stress_buckets", names(r))


class Resolve(unittest.TestCase):
    def test_names(self):
        self.assertEqual(gates.resolve("default", "ticker"), "default-ticker")
        self.assertEqual(gates.resolve("default", "config"), "volume-config")
        self.assertEqual(gates.resolve("default-config", "config"), "volume-config")
        self.assertEqual(gates.resolve("stress-mode", "config"), "stress-mode")
        with self.assertRaises(KeyError):
            gates.resolve("bogus", "config")

    def test_result_shape(self):
        r = gates.run_gate("default-ticker", BASE)
        self.assertEqual(set(r), {"gate", "pass", "checks"})
        for c in r["checks"]:
            self.assertEqual(set(c), {"name", "value", "threshold", "ok"})


if __name__ == "__main__":
    unittest.main()
