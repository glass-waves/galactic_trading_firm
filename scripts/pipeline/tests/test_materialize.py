import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import materialize  # noqa: E402

BASE = {
    "schema_version": "0.2", "config_id": 18, "created_by": "human", "parent_config_id": 17,
    "tickers": ["AMZN", "AAPL", "NVDA", "MSFT"],
    "indicators": [{"indicator_type": "rsi", "instance_id": "rsi_7_1min", "timescale": "OneMinute", "enabled": True, "weight": 0.3, "params": {}}],
    "actions": [
        {"action_type": "entry_window", "instance_id": "window_5m_thrust", "phase": "Entry", "enabled": True, "priority": 10, "params": {}},
        {"action_type": "entry_window", "instance_id": "window_strong_core_short", "phase": "Entry", "enabled": True, "priority": 20, "params": {}},
        {"action_type": "session_close", "instance_id": "exit_session", "phase": "Exit", "enabled": True, "priority": 5, "params": {"force_exit_by": "11:55"}},
        {"action_type": "session_close", "instance_id": "exit_session_b", "phase": "Exit", "enabled": False, "priority": 6, "params": {"force_exit_by": "11:55"}},
    ],
    "scoring": {"entry_threshold": 0.58},
    "session": {"force_exit_by": "11:55", "no_new_entries_after": "11:30", "max_position_pct": 0.30},
}


class ApplyPatch(unittest.TestCase):
    def test_disable_only_touches_named_actions(self):
        out = materialize.apply_patch(BASE, {"disable": ["window_5m_thrust", "nope"]})
        by = {a["instance_id"]: a for a in out["actions"]}
        self.assertFalse(by["window_5m_thrust"]["enabled"])
        self.assertTrue(by["window_strong_core_short"]["enabled"])
        self.assertEqual(len(out["actions"]), 4)

    def test_indicators_and_actions_append_in_order(self):
        ind = {"indicator_type": "event_calendar", "instance_id": "cal_sma50", "timescale": "OneMinute", "weight": 0.0, "enabled": True, "params": {"dates": []}}
        act = {"action_type": "entry_window", "instance_id": "window_x", "phase": "Entry", "enabled": True, "priority": 10, "params": {}}
        out = materialize.apply_patch(BASE, {"indicators": [ind], "actions": [act]})
        self.assertEqual([i["instance_id"] for i in out["indicators"]], ["rsi_7_1min", "cal_sma50"])
        self.assertEqual(out["actions"][-1]["instance_id"], "window_x")
        # appended, never replaced, even when the id already exists (mirrors the CLI)
        dup = dict(act, instance_id="window_5m_thrust")
        out2 = materialize.apply_patch(BASE, {"actions": [dup]})
        self.assertEqual(sum(1 for a in out2["actions"] if a["instance_id"] == "window_5m_thrust"), 2)

    def test_session_shallow_merge_and_session_close_mirror(self):
        out = materialize.apply_patch(BASE, {"session": {"no_new_entries_after": "15:30", "force_exit_by": "15:55"}})
        self.assertEqual(out["session"], {"force_exit_by": "15:55", "no_new_entries_after": "15:30", "max_position_pct": 0.30})
        for a in out["actions"]:
            if a["action_type"] == "session_close":
                self.assertEqual(a["params"]["force_exit_by"], "15:55")  # every session_close, enabled or not
            else:
                self.assertNotIn("force_exit_by", a["params"])

    def test_session_without_force_exit_by_leaves_actions_alone(self):
        out = materialize.apply_patch(BASE, {"session": {"max_position_pct": 0.36}})
        self.assertEqual(out["session"]["max_position_pct"], 0.36)
        self.assertEqual(out["session"]["force_exit_by"], "11:55")
        self.assertEqual(out["actions"][2]["params"]["force_exit_by"], "11:55")

    def test_mirror_reaches_appended_session_close(self):
        extra = {"action_type": "session_close", "instance_id": "exit_session_pm", "phase": "Exit", "enabled": True, "priority": 7, "params": {"force_exit_by": "12:00"}}
        out = materialize.apply_patch(BASE, {"actions": [extra], "session": {"force_exit_by": "15:55"}})
        self.assertEqual(out["actions"][-1]["params"]["force_exit_by"], "15:55")

    def test_tickers_replace(self):
        out = materialize.apply_patch(BASE, {"tickers": ["AMD"]})
        self.assertEqual(out["tickers"], ["AMD"])

    def test_pure_and_order(self):
        base = copy.deepcopy(BASE)
        patch = {"disable": ["exit_session"], "actions": [{"action_type": "session_close", "instance_id": "s2", "phase": "Exit", "enabled": True, "priority": 1, "params": {}}],
                 "session": {"force_exit_by": "15:55"}, "tickers": ["X"]}
        out = materialize.apply_patch(base, patch)
        self.assertEqual(base, BASE)  # input untouched
        by = {a["instance_id"]: a for a in out["actions"]}
        self.assertFalse(by["exit_session"]["enabled"])
        self.assertEqual(by["exit_session"]["params"]["force_exit_by"], "15:55")  # disable then mirror
        self.assertEqual(by["s2"]["params"]["force_exit_by"], "15:55")

    def test_unknown_key_rejected(self):
        with self.assertRaises(materialize.PatchError):
            materialize.apply_patch(BASE, {"indicator": []})
        self.assertTrue(materialize.validate_patch({"disable": "x"}))
        self.assertTrue(materialize.validate_patch({"actions": [{"instance_id": "a"}]}))
        self.assertEqual(materialize.validate_patch({"disable": [], "indicators": [], "actions": [], "session": {}, "tickers": []}), [])


class MergePatches(unittest.TestCase):
    def test_merge_equals_sequential_application(self):
        p1 = {"indicators": [{"indicator_type": "event_calendar", "instance_id": "cal", "timescale": "OneMinute", "weight": 0, "enabled": True, "params": {}}], "actions": []}
        p2 = {"disable": ["window_5m_thrust"], "indicators": [], "actions": [{"action_type": "entry_window", "instance_id": "w", "phase": "Entry", "enabled": True, "priority": 1, "params": {}}],
              "session": {"force_exit_by": "15:55"}}
        p3 = {"session": {"no_new_entries_after": "15:30"}, "tickers": ["A"]}
        merged = materialize.merge_patches(p1, p2, p3)
        seq = materialize.apply_patch(materialize.apply_patch(materialize.apply_patch(BASE, p1), p2), p3)
        self.assertEqual(materialize.apply_patch(BASE, merged), seq)
        self.assertEqual(merged["session"], {"force_exit_by": "15:55", "no_new_entries_after": "15:30"})
        self.assertNotIn("disable", materialize.merge_patches(p1))

    def test_stamp_blob(self):
        out = materialize.stamp_blob(materialize.apply_patch(BASE, {}), BASE, "x")
        self.assertEqual(out["parent_config_id"], 18)
        self.assertEqual(out["created_by"], "pipeline")
        self.assertTrue(out["created_at"].endswith("Z"))


if __name__ == "__main__":
    unittest.main()
