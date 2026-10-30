import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import report  # noqa: E402


class ProvenanceOf(unittest.TestCase):
    def test_reads_provenance_from_patch(self):
        c = {"patch": {"_provenance": {"cells_scanned": 20, "study": "x.md", "loyo": "stable"}}}
        self.assertEqual(report.provenance_of(c), {"cells_scanned": 20, "study": "x.md", "loyo": "stable"})

    def test_no_patch_is_empty(self):
        self.assertEqual(report.provenance_of({}), {})
        self.assertEqual(report.provenance_of({"patch": None}), {})
        self.assertEqual(report.provenance_of({"patch": {}}), {})

    def test_patch_without_provenance_key_is_empty(self):
        self.assertEqual(report.provenance_of({"patch": {"disable": ["x"]}}), {})


class StatsSuffix(unittest.TestCase):
    """report._stats_suffix(): the 'PF 1.46 [1.31-1.63] · p 0.02 · best of 20 cells' tail appended
    right after the PF figure in _bt_cell() — purely descriptive, never a gate input."""

    def test_no_stats_block_is_empty(self):
        self.assertEqual(report._stats_suffix(None, {}), "")
        self.assertEqual(report._stats_suffix({}, {}), "")

    def test_ci_only_no_provenance(self):
        stats = {"pf_ci90": [1.31, 1.63], "signflip_p": None, "n_trades": 460}
        self.assertEqual(report._stats_suffix(stats, {}), " [1.31-1.63]")

    def test_ci_and_p_no_provenance(self):
        stats = {"pf_ci90": [1.31, 1.63], "signflip_p": 0.02, "n_trades": 460}
        self.assertEqual(report._stats_suffix(stats, {}), " [1.31-1.63] · sign-flip p 0.02")

    def test_full_line_matches_the_documented_example(self):
        stats = {"pf_ci90": [1.31, 1.63], "signflip_p": 0.02, "n_trades": 460}
        prov = {"cells_scanned": 20, "study": "research/trigger/2026-10-02_entry_trigger_study.md", "loyo": "stable"}
        self.assertEqual(report._stats_suffix(stats, prov), " [1.31-1.63] · sign-flip p 0.02 · best of 20 cells")

    def test_cells_scanned_of_one_is_not_a_selection_discount(self):
        # a single-cell "grid" is not a best-of-N search; no "best of 1 cells" noise
        stats = {"pf_ci90": [1.0, 1.0], "signflip_p": 0.5, "n_trades": 10}
        out = report._stats_suffix(stats, {"cells_scanned": 1})
        self.assertNotIn("best of", out)

    def test_missing_ci_still_shows_p_and_best_of(self):
        stats = {"signflip_p": 0.1}
        out = report._stats_suffix(stats, {"cells_scanned": 5})
        self.assertEqual(out, " · sign-flip p 0.10 · best of 5 cells")


class BtCellStatsLine(unittest.TestCase):
    """_bt_cell() wires the stats suffix in right after the PF number, before the per-year split."""

    def _candidate(self, stats=None, prov=None):
        m = {"pnl": 2195.0, "n": 460, "pf": 1.46, "years": {"2022": {"pnl": 1144.0}}}
        if stats is not None:
            m["stats"] = stats
        c = {"backtest_result": {"metrics": m}}
        if prov is not None:
            c["patch"] = {"_provenance": prov}
        return c

    def test_with_stats_and_provenance(self):
        stats = {"pf_ci90": [1.31, 1.63], "signflip_p": 0.02, "n_trades": 460}
        prov = {"cells_scanned": 20}
        line = report._bt_cell(self._candidate(stats, prov))
        self.assertIn("PF 1.46 [1.31-1.63] · sign-flip p 0.02 · best of 20 cells", line)
        self.assertTrue(line.startswith("+2195 / 460 / PF 1.46 [1.31-1.63]"))

    def test_without_stats_falls_back_to_plain_pf(self):
        line = report._bt_cell(self._candidate())
        self.assertIn("PF 1.46 ·", line)
        self.assertNotIn("[", line)

    def test_no_metrics_at_all(self):
        self.assertEqual(report._bt_cell({}), "—")
        self.assertEqual(report._bt_cell({"backtest_result": {"blocked": "coverage"}}), "blocked: coverage")


if __name__ == "__main__":
    unittest.main()
