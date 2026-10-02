import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import gates  # noqa: E402

YEARS = ["2022", "2023", "2024", "2025", "2026"]


def mk(pnl=2154, n=407, pf=1.48, years=(1565, -181, -93, 451, 413), stress=None, ordinary=None, dd=0.0, max_position_fraction=0.0):
    ys = {y: {"pnl": float(p), "n": 10, "pf": 1.0, "dd": 0.0} for y, p in zip(YEARS, years)}
    m = {"pnl": float(pnl), "n": n, "pf": pf, "dd": float(dd), "max_position_fraction": float(max_position_fraction), "years": ys,
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


def mk_marginal(base_pnl, base_n, added_pnl, added_n, added_pf, added_worst_year=-50.0, added_worst_day=-100.0, baseline_tag="base"):
    return {"baseline_tag": baseline_tag, "base": {"pnl": base_pnl, "n": base_n},
            "added": {"pnl": added_pnl, "n": added_n, "pf": added_pf, "worst_year_pnl": added_worst_year, "worst_day": added_worst_day}}


class AdditiveConfig(unittest.TestCase):
    """baseline modeled on iex_v18: +1,728 / 436 / PF 1.37, years +991 -3 -20 +332 +429 (only 3 of 5
    positive) — an additive candidate can never clear quality-config's years_positive on this baseline,
    which is exactly the gate additive-config exists to replace for a purely-additive candidate."""

    def setUp(self):
        self.b = mk(pnl=1728, n=436, pf=1.37, years=(991, -3, -20, 332, 429))

    def test_needs_baseline(self):
        r = gates.additive_config(BASE, None)
        self.assertFalse(r["pass"])
        self.assertEqual(names(r), ["baseline"])

    def test_needs_marginal_split(self):
        m = mk(pnl=2003, n=459, pf=1.414, years=(1226, -3, -24, 375, 429))
        r = gates.additive_config(m, self.b)
        self.assertFalse(r["pass"])
        self.assertEqual(names(r), ["marginal"])

    def test_stress_s15_core_like_candidate_passes(self):
        # modeled on candidate #6 stress-s15-core: base subset untouched, added window +274 on 23
        # trades (PF 3.3): 2022 +235/18, 2024 -4/2, 2025 +43/3, none in 2023/2026
        years = (991 + 235, -3, -20 - 4, 332 + 43, 429)
        m = mk(pnl=1728 + 274, n=436 + 23, pf=1.414, years=years)
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=436, added_pnl=274, added_n=23, added_pf=3.3, added_worst_year=-4.0, added_worst_day=-4.0)
        r = gates.additive_config(m, self.b)
        self.assertTrue(r["pass"], names(r))
        self.assertEqual(r["gate"], "additive-config")

    def test_base_unchanged_pnl_boundary(self):
        m = mk(pnl=1728 * 0.98 + 100, n=456, pf=1.4, years=(1, 1, 1, 1, 1))
        m["marginal"] = mk_marginal(base_pnl=1728 * 0.98, base_n=436, added_pnl=100, added_n=20, added_pf=2.0)
        self.assertNotIn("base_unchanged_pnl", names(gates.additive_config(m, self.b)))
        m["marginal"] = mk_marginal(base_pnl=1728 * 0.98 - 1, base_n=436, added_pnl=100, added_n=20, added_pf=2.0)
        self.assertIn("base_unchanged_pnl", names(gates.additive_config(m, self.b)))

    def test_base_unchanged_trades_boundary(self):
        lo = 0.99 * 436
        m = mk(pnl=1728 + 100, n=456, pf=1.4, years=(1, 1, 1, 1, 1))
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=lo, added_pnl=100, added_n=20, added_pf=2.0)
        self.assertNotIn("base_unchanged_trades", names(gates.additive_config(m, self.b)))
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=lo - 1, added_pnl=100, added_n=20, added_pf=2.0)
        self.assertIn("base_unchanged_trades", names(gates.additive_config(m, self.b)))

    def test_added_pnl_must_be_positive(self):
        m = mk(pnl=1728, n=456, pf=1.3, years=(1, 1, 1, 1, 1))
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=436, added_pnl=0, added_n=20, added_pf=2.0)
        self.assertIn("added_pnl", names(gates.additive_config(m, self.b)))

    def test_added_pf_threshold(self):
        m = mk(pnl=1728 + 50, n=456, pf=1.3, years=(1, 1, 1, 1, 1))
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=436, added_pnl=50, added_n=20, added_pf=1.299)
        self.assertIn("added_pf", names(gates.additive_config(m, self.b)))
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=436, added_pnl=50, added_n=20, added_pf=1.3)
        self.assertNotIn("added_pf", names(gates.additive_config(m, self.b)))

    def test_added_trades_threshold(self):
        m = mk(pnl=1728 + 50, n=450, pf=1.4, years=(1, 1, 1, 1, 1))
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=436, added_pnl=50, added_n=14, added_pf=2.0)
        self.assertIn("added_trades", names(gates.additive_config(m, self.b)))
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=436, added_pnl=50, added_n=15, added_pf=2.0)
        self.assertNotIn("added_trades", names(gates.additive_config(m, self.b)))

    def test_added_worst_year_threshold(self):
        m = mk(pnl=1728 + 50, n=450, pf=1.4, years=(1, 1, 1, 1, 1))
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=436, added_pnl=50, added_n=20, added_pf=2.0, added_worst_year=-101.0)
        self.assertIn("added_worst_year", names(gates.additive_config(m, self.b)))
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=436, added_pnl=50, added_n=20, added_pf=2.0, added_worst_year=-100.0)
        self.assertNotIn("added_worst_year", names(gates.additive_config(m, self.b)))

    def test_added_worst_day_threshold(self):
        m = mk(pnl=1728 + 50, n=450, pf=1.4, years=(1, 1, 1, 1, 1))
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=436, added_pnl=50, added_n=20, added_pf=2.0, added_worst_day=-301.0)
        self.assertIn("added_worst_day", names(gates.additive_config(m, self.b)))
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=436, added_pnl=50, added_n=20, added_pf=2.0, added_worst_day=-300.0)
        self.assertNotIn("added_worst_day", names(gates.additive_config(m, self.b)))

    def test_combined_years_not_worse_is_relative_to_baseline_year(self):
        # baseline 2023 = -3; combined candidate 2023 = -54 -> margin -51, just past the -50 floor
        m = mk(pnl=1728 + 50, n=450, pf=1.4, years=(991, -54, -20, 332, 429))
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=436, added_pnl=50, added_n=20, added_pf=2.0)
        self.assertIn("combined_years_not_worse", names(gates.additive_config(m, self.b)))
        # -53 -> margin exactly -50: passes
        m2 = mk(pnl=1728 + 50, n=450, pf=1.4, years=(991, -53, -20, 332, 429))
        m2["marginal"] = mk_marginal(base_pnl=1728, base_n=436, added_pnl=50, added_n=20, added_pf=2.0)
        self.assertNotIn("combined_years_not_worse", names(gates.additive_config(m2, self.b)))

    def test_trend_day_ride_like_candidate_fails(self):
        # modeled on candidate #7 trend-day-ride: the added window loses money overall (2023/2024 drag)
        m = mk(pnl=1728 - 40, n=436 + 30, pf=1.2, years=(991 + 50, -3 - 30, -20 - 10, 332 + 10, 429 + 10))
        m["marginal"] = mk_marginal(base_pnl=1728, base_n=436, added_pnl=-40, added_n=30, added_pf=0.7, added_worst_year=-30.0, added_worst_day=-25.0)
        r = gates.additive_config(m, self.b)
        self.assertFalse(r["pass"])
        self.assertIn("added_pnl", names(r))
        self.assertIn("added_pf", names(r))


class SizingConfig(unittest.TestCase):
    """baseline modeled on iex_v18 at 36 % research sizing: +2,785 / 500 / PF 1.64, dd -600, own max
    position fraction 0.36 (fixed_fractional, no tiering) — a sizing-only candidate (e.g. size up
    the top VPIN tier) must keep the same trades and not get materially worse on any other axis."""

    def setUp(self):
        self.b = mk(pnl=2785, n=500, pf=1.64, years=(500, 500, 500, 785, 500), dd=-600, max_position_fraction=0.36)

    def test_needs_baseline(self):
        r = gates.sizing_config(BASE, None)
        self.assertFalse(r["pass"])
        self.assertEqual(names(r), ["baseline"])

    def test_needs_marginal_split(self):
        m = mk(pnl=3200, n=500, pf=1.64, years=(1, 1, 1, 1, 1), dd=-600, max_position_fraction=0.4)
        r = gates.sizing_config(m, self.b)
        self.assertFalse(r["pass"])
        self.assertEqual(names(r), ["marginal"])

    def test_passes_at_boundaries(self):
        # trades exactly +2 %, 95 % of baseline's keys matched, P&L exactly x1.10, PF exactly -0.02,
        # dd exactly x1.3, every year exactly -50 vs baseline, fraction exactly the 0.45 cap
        m = mk(pnl=2785 * 1.10, n=510, pf=1.62, years=(450, 450, 450, 735, 450), dd=-780, max_position_fraction=0.45)
        m["marginal"] = mk_marginal(base_pnl=2785 * 1.10, base_n=475, added_pnl=0, added_n=35, added_pf=1.0)
        r = gates.sizing_config(m, self.b)
        self.assertTrue(r["pass"], names(r))
        self.assertEqual(r["gate"], "sizing-config")

    def test_same_trade_count_boundary(self):
        common = dict(pnl=3500, pf=1.7, years=(1, 1, 1, 1, 1), dd=-600, max_position_fraction=0.4)
        m = mk(n=510, **common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=10, added_pf=1.0)
        self.assertNotIn("same_trade_count", names(gates.sizing_config(m, self.b)))
        m2 = mk(n=511, **common)
        m2["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=11, added_pf=1.0)
        self.assertIn("same_trade_count", names(gates.sizing_config(m2, self.b)))

    def test_same_trade_set_boundary(self):
        common = dict(pnl=3500, n=500, pf=1.7, years=(1, 1, 1, 1, 1), dd=-600, max_position_fraction=0.4)
        m = mk(**common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=475, added_pnl=0, added_n=25, added_pf=1.0)
        self.assertNotIn("same_trade_set", names(gates.sizing_config(m, self.b)))
        m2 = mk(**common)
        m2["marginal"] = mk_marginal(base_pnl=3500, base_n=474, added_pnl=0, added_n=26, added_pf=1.0)
        self.assertIn("same_trade_set", names(gates.sizing_config(m2, self.b)))

    def test_pnl_threshold(self):
        common = dict(n=500, pf=1.7, years=(1, 1, 1, 1, 1), dd=-600, max_position_fraction=0.4)
        m = mk(pnl=2785 * 1.10 - 1, **common)
        m["marginal"] = mk_marginal(base_pnl=2785 * 1.10 - 1, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertIn("pnl_5y", names(gates.sizing_config(m, self.b)))
        m2 = mk(pnl=2785 * 1.10, **common)
        m2["marginal"] = mk_marginal(base_pnl=2785 * 1.10, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertNotIn("pnl_5y", names(gates.sizing_config(m2, self.b)))

    def test_pf_threshold(self):
        common = dict(pnl=3500, n=500, years=(1, 1, 1, 1, 1), dd=-600, max_position_fraction=0.4)
        m = mk(pf=1.619, **common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertIn("pf_5y", names(gates.sizing_config(m, self.b)))
        m2 = mk(pf=1.62, **common)
        m2["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertNotIn("pf_5y", names(gates.sizing_config(m2, self.b)))

    def test_max_drawdown_threshold(self):
        common = dict(pnl=3500, n=500, pf=1.7, years=(1, 1, 1, 1, 1), max_position_fraction=0.4)
        m = mk(dd=-780.01, **common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertIn("max_drawdown", names(gates.sizing_config(m, self.b)))
        m2 = mk(dd=-780.0, **common)
        m2["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertNotIn("max_drawdown", names(gates.sizing_config(m2, self.b)))

    def test_years_not_worse_boundary(self):
        # baseline's 3rd year (2024) = 500; margin exactly -50 passes, -51 fails
        common = dict(pnl=3500, n=500, pf=1.7, dd=-600, max_position_fraction=0.4)
        m = mk(years=(500, 500, 450, 785, 500), **common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertNotIn("years_not_worse", names(gates.sizing_config(m, self.b)))
        m2 = mk(years=(500, 500, 449, 785, 500), **common)
        m2["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertIn("years_not_worse", names(gates.sizing_config(m2, self.b)))

    def test_max_position_fraction_cap(self):
        common = dict(pnl=3500, n=500, pf=1.7, years=(1, 1, 1, 1, 1), dd=-600)
        m = mk(max_position_fraction=0.45, **common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertNotIn("max_position_fraction", names(gates.sizing_config(m, self.b)))
        m2 = mk(max_position_fraction=0.4501, **common)
        m2["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertIn("max_position_fraction", names(gates.sizing_config(m2, self.b)))


class Resolve(unittest.TestCase):
    def test_names(self):
        self.assertEqual(gates.resolve("default", "ticker"), "default-ticker")
        self.assertEqual(gates.resolve("default", "config"), "volume-config")
        self.assertEqual(gates.resolve("default-config", "config"), "volume-config")
        self.assertEqual(gates.resolve("stress-mode", "config"), "stress-mode")
        self.assertEqual(gates.resolve("additive-config", "config"), "additive-config")
        self.assertEqual(gates.resolve("sizing-config", "config"), "sizing-config")
        with self.assertRaises(KeyError):
            gates.resolve("bogus", "config")

    def test_result_shape(self):
        r = gates.run_gate("default-ticker", BASE)
        self.assertEqual(set(r), {"gate", "pass", "checks"})
        for c in r["checks"]:
            self.assertEqual(set(c), {"name", "value", "threshold", "ok"})


if __name__ == "__main__":
    unittest.main()
