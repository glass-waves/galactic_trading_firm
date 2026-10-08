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
SWEEP_125 = "--max-position-pct 0.45"  # candidate #28-style override: effective cap 0.45, tolerance 0.459


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


def mk_ticker_marginal(added_pnl, added_n, added_pf, combined_pf, combined_years,
                        added_worst_year=0.0, n_exceedance_days=0, baseline_tag="iex_v18"):
    combined_years_dict = {y: {"pnl": float(p)} for y, p in zip(YEARS, combined_years)}
    return {
        "baseline_tag": baseline_tag,
        "added": {"pnl": added_pnl, "n": added_n, "pf": added_pf, "worst_year_pnl": added_worst_year, "worst_day": -50.0},
        "combined": {"pf": combined_pf, "years": combined_years_dict},
        "concurrency": {"cap": 3, "n_exceedance_days": n_exceedance_days,
                        "max_concurrent_observed": 4 if n_exceedance_days else 3, "exceedance_days": []},
    }


class AdditiveTicker(unittest.TestCase):
    """baseline is the live book itself (iex_v18): +1,728 / 436 / PF 1.37, years +991 -3 -20 +332
    +429. a ticker candidate is judged on its MARGINAL effect on that book (its own sweep is a
    single new ticker, concatenated with the baseline's trades — metrics.marginal_metrics()
    against baseline_tag='iex_v18'), not its stand-alone five-year record (default_ticker())."""

    def setUp(self):
        self.b = mk(pnl=1728, n=436, pf=1.37, years=(991, -3, -20, 332, 429))

    def test_needs_baseline(self):
        r = gates.additive_ticker(BASE, None)
        self.assertFalse(r["pass"])
        self.assertEqual(names(r), ["baseline"])

    def test_needs_marginal_split(self):
        m = mk(pnl=2000, n=460, pf=1.4, years=(1, 1, 1, 1, 1))
        r = gates.additive_ticker(m, self.b)
        self.assertFalse(r["pass"])
        self.assertEqual(names(r), ["marginal"])

    def test_passes_at_boundaries(self):
        # added trades exactly 40, added PF exactly 1.15, combined PF exactly baseline - 0.02,
        # every combined year exactly baseline's year - 50, added worst year exactly -150
        combined_years = (991 - 50, -3 - 50, -20 - 50, 332 - 50, 429 - 50)
        m = mk(pnl=2000, n=476, pf=1.3, years=combined_years)
        m["marginal"] = mk_ticker_marginal(added_pnl=50.0, added_n=40, added_pf=1.15, combined_pf=1.35,
                                            combined_years=combined_years, added_worst_year=-150.0)
        r = gates.additive_ticker(m, self.b)
        self.assertTrue(r["pass"], names(r))
        self.assertEqual(r["gate"], "additive-ticker")

    def test_added_trades_threshold(self):
        combined_years = (991, -3, -20, 332, 429)
        m = mk(pnl=1800, n=456, pf=1.4, years=combined_years)
        m["marginal"] = mk_ticker_marginal(added_pnl=50.0, added_n=39, added_pf=2.0, combined_pf=1.4, combined_years=combined_years)
        self.assertIn("added_trades", names(gates.additive_ticker(m, self.b)))
        m["marginal"] = mk_ticker_marginal(added_pnl=50.0, added_n=40, added_pf=2.0, combined_pf=1.4, combined_years=combined_years)
        self.assertNotIn("added_trades", names(gates.additive_ticker(m, self.b)))

    def test_added_pnl_must_be_positive(self):
        combined_years = (991, -3, -20, 332, 429)
        m = mk(pnl=1728, n=476, pf=1.3, years=combined_years)
        m["marginal"] = mk_ticker_marginal(added_pnl=0.0, added_n=40, added_pf=2.0, combined_pf=1.4, combined_years=combined_years)
        self.assertIn("added_pnl", names(gates.additive_ticker(m, self.b)))
        m["marginal"] = mk_ticker_marginal(added_pnl=0.01, added_n=40, added_pf=2.0, combined_pf=1.4, combined_years=combined_years)
        self.assertNotIn("added_pnl", names(gates.additive_ticker(m, self.b)))

    def test_added_pf_threshold(self):
        combined_years = (991, -3, -20, 332, 429)
        m = mk(pnl=1800, n=476, pf=1.4, years=combined_years)
        m["marginal"] = mk_ticker_marginal(added_pnl=50.0, added_n=40, added_pf=1.149, combined_pf=1.4, combined_years=combined_years)
        self.assertIn("added_pf", names(gates.additive_ticker(m, self.b)))
        m["marginal"] = mk_ticker_marginal(added_pnl=50.0, added_n=40, added_pf=1.15, combined_pf=1.4, combined_years=combined_years)
        self.assertNotIn("added_pf", names(gates.additive_ticker(m, self.b)))

    def test_combined_pf_not_dilutive_boundary(self):
        combined_years = (991, -3, -20, 332, 429)
        common = dict(added_pnl=50.0, added_n=40, added_pf=2.0, combined_years=combined_years)
        m = mk(pnl=1800, n=476, pf=1.35, years=combined_years)
        m["marginal"] = mk_ticker_marginal(combined_pf=1.35, **common)
        self.assertNotIn("combined_pf", names(gates.additive_ticker(m, self.b)))
        m2 = mk(pnl=1800, n=476, pf=1.349, years=combined_years)
        m2["marginal"] = mk_ticker_marginal(combined_pf=1.349, **common)
        self.assertIn("combined_pf", names(gates.additive_ticker(m2, self.b)))

    def test_combined_years_not_worse_is_relative_to_baseline_year(self):
        # baseline 2023 = -3; combined 2023 = -54 -> margin -51, just past the -50 floor
        combined_years = (991, -54, -20, 332, 429)
        m = mk(pnl=1800, n=476, pf=1.4, years=combined_years)
        m["marginal"] = mk_ticker_marginal(added_pnl=50.0, added_n=40, added_pf=2.0, combined_pf=1.4, combined_years=combined_years)
        self.assertIn("combined_years_not_worse", names(gates.additive_ticker(m, self.b)))
        # -53 -> margin exactly -50: passes
        combined_years2 = (991, -53, -20, 332, 429)
        m2 = mk(pnl=1800, n=476, pf=1.4, years=combined_years2)
        m2["marginal"] = mk_ticker_marginal(added_pnl=50.0, added_n=40, added_pf=2.0, combined_pf=1.4, combined_years=combined_years2)
        self.assertNotIn("combined_years_not_worse", names(gates.additive_ticker(m2, self.b)))

    def test_added_worst_year_threshold(self):
        combined_years = (991, -3, -20, 332, 429)
        m = mk(pnl=1800, n=476, pf=1.4, years=combined_years)
        m["marginal"] = mk_ticker_marginal(added_pnl=50.0, added_n=40, added_pf=2.0, combined_pf=1.4,
                                            combined_years=combined_years, added_worst_year=-150.01)
        self.assertIn("added_worst_year", names(gates.additive_ticker(m, self.b)))
        m["marginal"] = mk_ticker_marginal(added_pnl=50.0, added_n=40, added_pf=2.0, combined_pf=1.4,
                                            combined_years=combined_years, added_worst_year=-150.0)
        self.assertNotIn("added_worst_year", names(gates.additive_ticker(m, self.b)))

    def test_concurrency_is_info_not_a_gating_check(self):
        # a candidate with heavy concurrency exceedance still passes if every real check clears —
        # the live 3-position cap is reported, never gated
        combined_years = (991, -3, -20, 332, 429)
        m = mk(pnl=2000, n=476, pf=1.4, years=combined_years)
        m["marginal"] = mk_ticker_marginal(added_pnl=100.0, added_n=40, added_pf=2.0, combined_pf=1.4,
                                            combined_years=combined_years, n_exceedance_days=37)
        r = gates.additive_ticker(m, self.b)
        self.assertTrue(r["pass"], names(r))
        info = [c for c in r["checks"] if c["name"] == "concurrency_info"][0]
        self.assertTrue(info["ok"])
        self.assertEqual(info["value"], 37)

    def test_amzn_amd_like_fails_on_everything(self):
        # modeled on a weak candidate: too few trades, loses money, drags PF below the baseline
        combined_years = (900, -60, -70, 300, 400)
        m = mk(pnl=1600, n=460, pf=1.2, years=combined_years)
        m["marginal"] = mk_ticker_marginal(added_pnl=-128.0, added_n=24, added_pf=0.74, combined_pf=1.2,
                                            combined_years=combined_years, added_worst_year=-200.0)
        r = gates.additive_ticker(m, self.b)
        self.assertFalse(r["pass"])
        failed = set(names(r))
        self.assertIn("added_trades", failed)
        self.assertIn("added_pnl", failed)
        self.assertIn("added_pf", failed)
        self.assertIn("combined_pf", failed)
        self.assertIn("added_worst_year", failed)


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
        # dd exactly x1.3, every year exactly -50 vs baseline, fraction exactly the _sweep_args cap
        # (0.45) x 1.02 tolerance
        m = mk(pnl=2785 * 1.10, n=510, pf=1.62, years=(450, 450, 450, 735, 450), dd=-780, max_position_fraction=0.45 * 1.02)
        m["marginal"] = mk_marginal(base_pnl=2785 * 1.10, base_n=475, added_pnl=0, added_n=35, added_pf=1.0)
        r = gates.sizing_config(m, self.b, sweep_args_override=SWEEP_125)
        self.assertTrue(r["pass"], names(r))
        self.assertEqual(r["gate"], "sizing-config")

    def test_same_trade_count_boundary(self):
        common = dict(pnl=3500, pf=1.7, years=(1, 1, 1, 1, 1), dd=-600, max_position_fraction=0.4)
        m = mk(n=510, **common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=10, added_pf=1.0)
        self.assertNotIn("same_trade_count", names(gates.sizing_config(m, self.b, sweep_args_override=SWEEP_125)))
        m2 = mk(n=511, **common)
        m2["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=11, added_pf=1.0)
        self.assertIn("same_trade_count", names(gates.sizing_config(m2, self.b, sweep_args_override=SWEEP_125)))

    def test_same_trade_set_boundary(self):
        common = dict(pnl=3500, n=500, pf=1.7, years=(1, 1, 1, 1, 1), dd=-600, max_position_fraction=0.4)
        m = mk(**common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=475, added_pnl=0, added_n=25, added_pf=1.0)
        self.assertNotIn("same_trade_set", names(gates.sizing_config(m, self.b, sweep_args_override=SWEEP_125)))
        m2 = mk(**common)
        m2["marginal"] = mk_marginal(base_pnl=3500, base_n=474, added_pnl=0, added_n=26, added_pf=1.0)
        self.assertIn("same_trade_set", names(gates.sizing_config(m2, self.b, sweep_args_override=SWEEP_125)))

    def test_pnl_threshold(self):
        common = dict(n=500, pf=1.7, years=(1, 1, 1, 1, 1), dd=-600, max_position_fraction=0.4)
        m = mk(pnl=2785 * 1.10 - 1, **common)
        m["marginal"] = mk_marginal(base_pnl=2785 * 1.10 - 1, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertIn("pnl_5y", names(gates.sizing_config(m, self.b, sweep_args_override=SWEEP_125)))
        m2 = mk(pnl=2785 * 1.10, **common)
        m2["marginal"] = mk_marginal(base_pnl=2785 * 1.10, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertNotIn("pnl_5y", names(gates.sizing_config(m2, self.b, sweep_args_override=SWEEP_125)))

    def test_pf_threshold(self):
        common = dict(pnl=3500, n=500, years=(1, 1, 1, 1, 1), dd=-600, max_position_fraction=0.4)
        m = mk(pf=1.619, **common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertIn("pf_5y", names(gates.sizing_config(m, self.b, sweep_args_override=SWEEP_125)))
        m2 = mk(pf=1.62, **common)
        m2["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertNotIn("pf_5y", names(gates.sizing_config(m2, self.b, sweep_args_override=SWEEP_125)))

    def test_max_drawdown_threshold(self):
        common = dict(pnl=3500, n=500, pf=1.7, years=(1, 1, 1, 1, 1), max_position_fraction=0.4)
        m = mk(dd=-780.01, **common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertIn("max_drawdown", names(gates.sizing_config(m, self.b, sweep_args_override=SWEEP_125)))
        m2 = mk(dd=-780.0, **common)
        m2["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertNotIn("max_drawdown", names(gates.sizing_config(m2, self.b, sweep_args_override=SWEEP_125)))

    def test_years_not_worse_boundary(self):
        # baseline's 3rd year (2024) = 500; margin exactly -50 passes, -51 fails
        common = dict(pnl=3500, n=500, pf=1.7, dd=-600, max_position_fraction=0.4)
        m = mk(years=(500, 500, 450, 785, 500), **common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertNotIn("years_not_worse", names(gates.sizing_config(m, self.b, sweep_args_override=SWEEP_125)))
        m2 = mk(years=(500, 500, 449, 785, 500), **common)
        m2["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertIn("years_not_worse", names(gates.sizing_config(m2, self.b, sweep_args_override=SWEEP_125)))

    def test_max_position_fraction_cap_standard(self):
        # no _sweep_args override -> cap is the standard 0.36, tolerance 0.36 x 1.02 = 0.3672
        common = dict(pnl=3500, n=500, pf=1.7, years=(1, 1, 1, 1, 1), dd=-600)
        threshold = 0.36 * gates.MAX_POSITION_FRACTION_TOLERANCE
        m = mk(max_position_fraction=threshold, **common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertNotIn("max_position_fraction", names(gates.sizing_config(m, self.b)))
        m2 = mk(max_position_fraction=threshold + 0.0001, **common)
        m2["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertIn("max_position_fraction", names(gates.sizing_config(m2, self.b)))

    def test_max_position_fraction_cap_follows_sweep_args_override(self):
        # candidate #28 size-vpin26-x1.25: _sweep_args raises the sweep's own clamp to 0.45, so the
        # gate's cap check follows it too (x 1.02 = 0.459), not the standard 0.36's 0.3672
        common = dict(pnl=3500, n=500, pf=1.7, years=(1, 1, 1, 1, 1), dd=-600)
        m = mk(max_position_fraction=0.456, **common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertIn("max_position_fraction", names(gates.sizing_config(m, self.b)))  # fails the standard 0.3672 cap
        self.assertNotIn("max_position_fraction", names(gates.sizing_config(m, self.b, sweep_args_override=SWEEP_125)))
        m2 = mk(max_position_fraction=0.4591, **common)
        m2["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertIn("max_position_fraction", names(gates.sizing_config(m2, self.b, sweep_args_override=SWEEP_125)))


def mk_standalone(daily_corr=0.0, combined_pf=1.5, combined_dd=-400.0, combined_active_days=500,
                   baseline_active_days=325, candidate_active_days=275, ex_best_year_pf=1.3,
                   excluded_year="2022", n_exceedance_days=0, combined_years=None, baseline_tag="iex_v18"):
    combined = {"pf": combined_pf, "dd": combined_dd}
    if combined_years is not None:
        combined["years"] = {y: {"pnl": float(p)} for y, p in zip(YEARS, combined_years)}
    return {
        "baseline_tag": baseline_tag,
        "daily_corr": daily_corr,
        "combined": combined,
        "baseline_active_days": baseline_active_days,
        "candidate_active_days": candidate_active_days,
        "combined_active_days": combined_active_days,
        "ex_best_year": {"pf": ex_best_year_pf, "excluded_year": excluded_year, "excluded_year_pnl": 600.0},
        "concurrency": {"cap": 3, "n_exceedance_days": n_exceedance_days,
                        "max_concurrent_observed": 4 if n_exceedance_days else 2, "exceedance_days": []},
    }


class StandaloneStrategy(unittest.TestCase):
    """baseline modeled on iex_v18 (the live book): +1,728 / 436 / PF 1.37, dd -469, 325 active
    days. the candidate (modeled on #47 qqq-noise-pm-vol): standalone +1,095 / 275 / PF 1.50,
    4/5 years, corr -0.03, combined with v18 PF 1.41 / dd -390 / 524 active days
    (research/index_momentum/2026-10-07_index_intraday_momentum.md §4,7)."""

    def setUp(self):
        self.b = mk(pnl=1728, n=436, pf=1.37, years=(991, -3, -20, 332, 429), dd=-469)

    def test_needs_baseline(self):
        r = gates.standalone_strategy(BASE, None)
        self.assertFalse(r["pass"])
        self.assertEqual(names(r), ["baseline"])

    def test_needs_standalone_metrics(self):
        m = mk(pnl=1095, n=275, pf=1.50, years=(600, 168, -22, 227, 122))
        r = gates.standalone_strategy(m, self.b)
        self.assertFalse(r["pass"])
        self.assertEqual(names(r), ["standalone"])

    def test_study_numbers_pass(self):
        # exact figures from the index-momentum study: standalone PF 1.50, 275 trades, 4/5 years,
        # corr -0.03, combined PF 1.41 (> baseline 1.37), dd -390 (>= -469 x 1.25 = -586.25),
        # active days 524 (>= 325 x 1.25 = 406.25), ex-2022 PF ~1.4
        m = mk(pnl=1095, n=275, pf=1.50, years=(600, 168, -22, 227, 122))
        m["standalone"] = mk_standalone(daily_corr=-0.03, combined_pf=1.41, combined_dd=-390.0,
                                         combined_active_days=524, baseline_active_days=325,
                                         candidate_active_days=275, ex_best_year_pf=1.4, excluded_year="2022")
        r = gates.standalone_strategy(m, self.b)
        self.assertTrue(r["pass"], names(r))
        self.assertEqual(r["gate"], "standalone-strategy")

    def test_standalone_pf_boundary(self):
        m = mk(pnl=1095, n=275, pf=1.4, years=(600, 168, -22, 227, 122))
        m["standalone"] = mk_standalone(ex_best_year_pf=1.4)
        self.assertNotIn("standalone_pf", names(gates.standalone_strategy(m, self.b)))
        m2 = mk(pnl=1095, n=275, pf=1.399, years=(600, 168, -22, 227, 122))
        m2["standalone"] = mk_standalone(ex_best_year_pf=1.4)
        self.assertIn("standalone_pf", names(gates.standalone_strategy(m2, self.b)))

    def test_standalone_years_positive_boundary(self):
        m = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -22, -5, 122))  # 3 of 5 positive
        m["standalone"] = mk_standalone(ex_best_year_pf=1.4)
        self.assertIn("standalone_years_positive", names(gates.standalone_strategy(m, self.b)))
        m2 = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -22, 227, 122))  # 4 of 5
        m2["standalone"] = mk_standalone(ex_best_year_pf=1.4)
        self.assertNotIn("standalone_years_positive", names(gates.standalone_strategy(m2, self.b)))

    def test_standalone_trades_boundary(self):
        m = mk(pnl=1095, n=149, pf=1.5, years=(600, 168, -22, 227, 122))
        m["standalone"] = mk_standalone(ex_best_year_pf=1.4)
        self.assertIn("standalone_trades", names(gates.standalone_strategy(m, self.b)))
        m2 = mk(pnl=1095, n=150, pf=1.5, years=(600, 168, -22, 227, 122))
        m2["standalone"] = mk_standalone(ex_best_year_pf=1.4)
        self.assertNotIn("standalone_trades", names(gates.standalone_strategy(m2, self.b)))

    def test_standalone_worst_year_boundary(self):
        m = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -301, 227, 122))
        m["standalone"] = mk_standalone(ex_best_year_pf=1.4)
        self.assertIn("standalone_worst_year", names(gates.standalone_strategy(m, self.b)))
        m2 = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -300, 227, 122))
        m2["standalone"] = mk_standalone(ex_best_year_pf=1.4)
        self.assertNotIn("standalone_worst_year", names(gates.standalone_strategy(m2, self.b)))

    def test_daily_corr_boundary(self):
        m = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -22, 227, 122))
        m["standalone"] = mk_standalone(daily_corr=0.30, ex_best_year_pf=1.4)
        self.assertNotIn("daily_corr_with_baseline", names(gates.standalone_strategy(m, self.b)))
        m2 = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -22, 227, 122))
        m2["standalone"] = mk_standalone(daily_corr=0.301, ex_best_year_pf=1.4)
        self.assertIn("daily_corr_with_baseline", names(gates.standalone_strategy(m2, self.b)))

    def test_combined_pf_not_dilutive_boundary(self):
        m = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -22, 227, 122))
        m["standalone"] = mk_standalone(combined_pf=1.37, ex_best_year_pf=1.4)
        self.assertNotIn("combined_pf", names(gates.standalone_strategy(m, self.b)))
        m2 = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -22, 227, 122))
        m2["standalone"] = mk_standalone(combined_pf=1.369, ex_best_year_pf=1.4)
        self.assertIn("combined_pf", names(gates.standalone_strategy(m2, self.b)))

    def test_combined_max_dd_boundary(self):
        # baseline dd -469 x 1.25 = -586.25
        m = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -22, 227, 122))
        m["standalone"] = mk_standalone(combined_dd=-586.25, ex_best_year_pf=1.4)
        self.assertNotIn("combined_max_dd", names(gates.standalone_strategy(m, self.b)))
        m2 = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -22, 227, 122))
        m2["standalone"] = mk_standalone(combined_dd=-586.26, ex_best_year_pf=1.4)
        self.assertIn("combined_max_dd", names(gates.standalone_strategy(m2, self.b)))

    def test_combined_active_days_boundary(self):
        # baseline active days 325 x 1.25 = 406.25
        m = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -22, 227, 122))
        m["standalone"] = mk_standalone(combined_active_days=407, baseline_active_days=325, ex_best_year_pf=1.4)
        self.assertNotIn("combined_active_days", names(gates.standalone_strategy(m, self.b)))
        m2 = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -22, 227, 122))
        m2["standalone"] = mk_standalone(combined_active_days=406, baseline_active_days=325, ex_best_year_pf=1.4)
        self.assertIn("combined_active_days", names(gates.standalone_strategy(m2, self.b)))

    def test_ex_best_year_pf_boundary(self):
        m = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -22, 227, 122))
        m["standalone"] = mk_standalone(ex_best_year_pf=1.2)
        self.assertNotIn("ex_best_year_pf", names(gates.standalone_strategy(m, self.b)))
        m2 = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -22, 227, 122))
        m2["standalone"] = mk_standalone(ex_best_year_pf=1.199)
        self.assertIn("ex_best_year_pf", names(gates.standalone_strategy(m2, self.b)))

    def test_concurrency_is_info_not_a_gating_check(self):
        m = mk(pnl=1095, n=275, pf=1.5, years=(600, 168, -22, 227, 122))
        m["standalone"] = mk_standalone(ex_best_year_pf=1.4, n_exceedance_days=12)
        r = gates.standalone_strategy(m, self.b)
        self.assertTrue(r["pass"], names(r))
        info = [c for c in r["checks"] if c["name"] == "concurrency_info"][0]
        self.assertTrue(info["ok"])
        self.assertEqual(info["value"], 12)

    def test_one_year_wonder_like_candidate_fails_ex_best_year(self):
        # almost all of the P&L is 2022; ex-2022 the strategy is a loser
        m = mk(pnl=1095, n=275, pf=1.5, years=(1050, 10, -22, 30, 27))
        m["standalone"] = mk_standalone(ex_best_year_pf=0.8, excluded_year="2022")
        r = gates.standalone_strategy(m, self.b)
        self.assertFalse(r["pass"])
        self.assertIn("ex_best_year_pf", names(r))

    def test_weak_candidate_fails_on_everything(self):
        m = mk(pnl=200, n=80, pf=1.1, years=(100, 50, -301, 50, -50))  # 3/5, under trades, under worst year
        m["standalone"] = mk_standalone(daily_corr=0.5, combined_pf=1.2, combined_dd=-600.0,
                                         combined_active_days=350, baseline_active_days=325, ex_best_year_pf=0.9)
        r = gates.standalone_strategy(m, self.b)
        self.assertFalse(r["pass"])
        failed = set(names(r))
        for name in ("standalone_pf", "standalone_years_positive", "standalone_trades", "standalone_worst_year",
                     "daily_corr_with_baseline", "combined_pf", "combined_max_dd", "combined_active_days", "ex_best_year_pf"):
            self.assertIn(name, failed, name)


class EffectiveMaxPositionPct(unittest.TestCase):
    def test_no_override_is_standard(self):
        self.assertEqual(gates.effective_max_position_pct(None), 0.36)
        self.assertEqual(gates.effective_max_position_pct(""), 0.36)

    def test_parses_flag_from_sweep_args(self):
        self.assertEqual(gates.effective_max_position_pct("--max-position-pct 0.45"), 0.45)
        self.assertEqual(gates.effective_max_position_pct("--sizing-fraction 0.30 --max-position-pct 0.45"), 0.45)
        self.assertEqual(gates.effective_max_position_pct("--max-position-pct=0.45"), 0.45)

    def test_malformed_or_missing_flag_falls_back(self):
        self.assertEqual(gates.effective_max_position_pct("--sizing-fraction 0.30"), 0.36)
        self.assertEqual(gates.effective_max_position_pct("--max-position-pct notanumber"), 0.36)


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

    def test_run_gate_forwards_sweep_args_override_to_sizing_config(self):
        b = mk(pnl=2785, n=500, pf=1.64, years=(500, 500, 500, 785, 500), dd=-600, max_position_fraction=0.36)
        common = dict(pnl=3500, n=500, pf=1.7, years=(1, 1, 1, 1, 1), dd=-600, max_position_fraction=0.4)
        m = mk(**common)
        m["marginal"] = mk_marginal(base_pnl=3500, base_n=500, added_pnl=0, added_n=0, added_pf=1.0)
        self.assertIn("max_position_fraction", names(gates.run_gate("sizing-config", m, b)))
        self.assertNotIn("max_position_fraction", names(gates.run_gate("sizing-config", m, b, sweep_args_override=SWEEP_125)))
        # other gates ignore the kwarg silently
        self.assertEqual(gates.run_gate("default-ticker", BASE, sweep_args_override=SWEEP_125),
                          gates.run_gate("default-ticker", BASE))


if __name__ == "__main__":
    unittest.main()
