import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import pipeline  # noqa: E402


class VerifyMaterialization(unittest.TestCase):
    """candidate #28 (size-vpin26-x1.25) plumbing fix: the verify step must run the materialized-row
    replay and the base+--patch-json replay with NO --sizing-fraction / --max-position-pct of any
    kind on either side - neither the standard sizing args nor the candidate's own `_sweep_args`.

    the literal first attempt at this fix (append `_sweep_args` to both sides, same as the real gate
    sweep) was tried and does NOT make the two sides identical for a sizing-config candidate: confirmed
    by running `backtest --config-id <materialized>` vs `backtest --config-id <base> --patch-json
    <patch>` by hand for candidate #28 (research/sizing/size_vpin26_v1.patch.json, which both adds a
    new Sizing action and sets its own `session.max_position_pct`). crates/backtest/src/main.rs's
    apply() applies a bare --max-position-pct/--sizing-fraction CLI override to config.actions/
    config.session *before* the --patch-json loop runs, so the override lands permanently on the
    materialized side (the field already exists when the override runs) but gets overwritten right
    back by the patch's own session/action values on the --patch-json side (applied after). with
    `_sweep_args` ("--max-position-pct 0.45") appended to both sides as literally instructed, the two
    sides still diverged: materialized ended at 0.45, patch-json at 0.40 (the patch's own session
    value) - same divergence as before the fix, just at different numbers. only dropping every sizing
    CLI flag from the verify step entirely makes the two sides byte-identical, which is in fact the
    check's actual job (does materialize.py mirror --patch-json), independent of whatever cap the gate
    separately sweeps at."""

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.enterContext(mock.patch.object(pipeline, "LOGS", self.tmp))
        self.enterContext(mock.patch.object(pipeline, "verify_dates", return_value=["2025-01-01", "2025-01-02"]))

    def _fake_run_backtest_day(self, calls):
        def fake(date, extra, sizing, capital=10000.0):
            calls.append((list(extra), sizing))
            return ([], "", "")
        return fake

    def test_no_sizing_args_on_either_side_even_with_sweep_args_override(self):
        calls = []
        cand = {"id": 28, "patch": {"disable": ["sizing_fixed"], "session": {"max_position_pct": 0.40},
                                     "_sweep_args": "--max-position-pct 0.45"}}
        with mock.patch.object(pipeline, "run_backtest_day", side_effect=self._fake_run_backtest_day(calls)):
            out = pipeline.verify_materialization(cand, base_id=11, mid=42, sweep_args_override="--max-position-pct 0.45")

        self.assertEqual(out["args"], [])  # this check never applies a sizing override, by design
        self.assertEqual(out["sweep_args_override"], "--max-position-pct 0.45")  # recorded for visibility only
        self.assertEqual(len(calls), 4)  # (materialized, patch) x 2 verify dates
        for extra, sizing in calls:
            self.assertFalse(sizing)
            self.assertNotIn("--max-position-pct", extra)
            self.assertNotIn("--sizing-fraction", extra)

    def test_both_sides_get_the_same_treatment_regardless_of_override(self):
        """materialized-row and base+patch-json run with identical argv shapes (modulo --config-id /
        --patch-json themselves) whether or not the candidate declared a `_sweep_args` override."""
        for override in ("--max-position-pct 0.45", None):
            with self.subTest(override=override):
                calls = []
                cand = {"id": 28, "patch": {"_sweep_args": override} if override else {}}
                with mock.patch.object(pipeline, "run_backtest_day", side_effect=self._fake_run_backtest_day(calls)):
                    pipeline.verify_materialization(cand, base_id=11, mid=42, sweep_args_override=override)
                a, b = calls[0], calls[1]
                self.assertEqual(a, (["--config-id", "42"], False))
                self.assertEqual(b, (["--config-id", "11", "--patch-json", str(self.tmp / "pipeline" / "cand_28_patch.json")], False))

    def test_patch_file_written_includes_meta_key_harmlessly(self):
        """_sweep_args rides along in the written patch file (materialize.py / the rust --patch-json
        reader both ignore unknown keys - docs/pipeline.md), it just never reaches the CLI argv here."""
        calls = []
        cand = {"id": 31, "patch": {"disable": ["x"], "_sweep_args": "--max-position-pct 0.45"}}
        with mock.patch.object(pipeline, "run_backtest_day", side_effect=self._fake_run_backtest_day(calls)):
            pipeline.verify_materialization(cand, base_id=11, mid=45, sweep_args_override="--max-position-pct 0.45")
        import json
        written = json.loads((self.tmp / "pipeline" / "cand_31_patch.json").read_text())
        self.assertEqual(written["_sweep_args"], "--max-position-pct 0.45")

    def test_mismatched_trades_marked_not_identical(self):
        def fake(date, extra, sizing, capital=10000.0):
            # materialized side (--config-id only) sees one trade, patch side sees none
            return (["trade,row1"], "", "") if "--patch-json" not in extra else ([], "", "")

        cand = {"id": 30, "patch": {}}
        with mock.patch.object(pipeline, "run_backtest_day", side_effect=fake):
            out = pipeline.verify_materialization(cand, base_id=11, mid=44, sweep_args_override=None)
        self.assertFalse(out["ok"])
        for d in out["dates"].values():
            self.assertFalse(d["identical"])

    def test_matching_trades_marked_identical(self):
        def fake(date, extra, sizing, capital=10000.0):
            return (["trade,row1", "trade,row2"], "", "")

        cand = {"id": 32, "patch": {}}
        with mock.patch.object(pipeline, "run_backtest_day", side_effect=fake):
            out = pipeline.verify_materialization(cand, base_id=11, mid=46, sweep_args_override=None)
        self.assertTrue(out["ok"])
        for d in out["dates"].values():
            self.assertTrue(d["identical"])


if __name__ == "__main__":
    unittest.main()
