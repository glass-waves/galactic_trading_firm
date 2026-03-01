"""config diff computation.

compares two config blobs field by field and produces atomic change
dicts suitable for insertion into the config_changelog table.
"""

from __future__ import annotations

from typing import Any


# valid change categories matching the config_changelog.change_category enum
CHANGE_CATEGORIES = {
    "knob_tuned",
    "tool_enabled",
    "tool_disabled",
    "weight_adjusted",
    "threshold_adjusted",
    "session_rule_changed",
    "scoring_changed",
}


def compute_config_diff(
    old_config: dict[str, Any],
    new_config: dict[str, Any],
) -> list[dict[str, Any]]:
    """compare two config blobs and produce a list of atomic changes.

    each change dict has keys: change_category, target_timescale,
    target_tool_id, target_tool_type, target_param, old_value,
    new_value, reason.
    """
    changes: list[dict[str, Any]] = []

    # compare tickers
    old_tickers = set(old_config.get("tickers", []))
    new_tickers = set(new_config.get("tickers", []))
    if old_tickers != new_tickers:
        changes.append(_change(
            category="session_rule_changed",
            param="tickers",
            old_value=sorted(old_tickers),
            new_value=sorted(new_tickers),
        ))

    # compare scoring
    changes.extend(_diff_scoring(
        old_config.get("scoring", {}),
        new_config.get("scoring", {}),
    ))

    # compare session rules
    changes.extend(_diff_session(
        old_config.get("session", {}),
        new_config.get("session", {}),
    ))

    # compare indicators
    changes.extend(_diff_tools(
        old_config.get("indicators", []),
        new_config.get("indicators", []),
        tool_kind="indicator",
    ))

    # compare actions
    changes.extend(_diff_tools(
        old_config.get("actions", []),
        new_config.get("actions", []),
        tool_kind="action",
    ))

    return changes


def _change(
    category: str,
    param: str,
    old_value: Any = None,
    new_value: Any = None,
    timescale: str | None = None,
    tool_id: str | None = None,
    tool_type: str | None = None,
    reason: str = "config mutation by PM agent",
) -> dict[str, Any]:
    return {
        "change_category": category,
        "target_timescale": timescale,
        "target_tool_id": tool_id,
        "target_tool_type": tool_type,
        "target_param": param,
        "old_value": old_value,
        "new_value": new_value,
        "reason": reason,
    }


def _diff_scoring(old: dict, new: dict) -> list[dict[str, Any]]:
    changes: list[dict[str, Any]] = []

    # thresholds
    for key in ("entry_threshold", "exit_threshold"):
        old_val = old.get(key)
        new_val = new.get(key)
        if old_val != new_val:
            changes.append(_change(
                category="threshold_adjusted",
                param=key,
                old_value=old_val,
                new_value=new_val,
            ))

    # aggregation method
    if old.get("aggregation") != new.get("aggregation"):
        changes.append(_change(
            category="scoring_changed",
            param="aggregation",
            old_value=old.get("aggregation"),
            new_value=new.get("aggregation"),
        ))

    # hard gate timescales
    old_gates = old.get("hard_gate_timescales", [])
    new_gates = new.get("hard_gate_timescales", [])
    if sorted(str(g) for g in old_gates) != sorted(str(g) for g in new_gates):
        changes.append(_change(
            category="scoring_changed",
            param="hard_gate_timescales",
            old_value=old_gates,
            new_value=new_gates,
        ))

    # timescale weights
    old_weights = old.get("timescale_weights", {})
    new_weights = new.get("timescale_weights", {})
    all_timescales = set(list(old_weights.keys()) + list(new_weights.keys()))
    for ts in sorted(all_timescales):
        old_w = old_weights.get(ts)
        new_w = new_weights.get(ts)
        if old_w != new_w:
            changes.append(_change(
                category="weight_adjusted",
                param="timescale_weight",
                timescale=ts,
                old_value=old_w,
                new_value=new_w,
            ))

    return changes


def _diff_session(old: dict, new: dict) -> list[dict[str, Any]]:
    changes: list[dict[str, Any]] = []
    all_keys = set(list(old.keys()) + list(new.keys()))
    for key in sorted(all_keys):
        if old.get(key) != new.get(key):
            changes.append(_change(
                category="session_rule_changed",
                param=key,
                old_value=old.get(key),
                new_value=new.get(key),
            ))
    return changes


def _diff_tools(
    old_tools: list[dict],
    new_tools: list[dict],
    tool_kind: str,
) -> list[dict[str, Any]]:
    """diff indicator or action tool lists by instance_id."""
    changes: list[dict[str, Any]] = []

    old_by_id = {t["instance_id"]: t for t in old_tools if "instance_id" in t}
    new_by_id = {t["instance_id"]: t for t in new_tools if "instance_id" in t}

    all_ids = set(list(old_by_id.keys()) + list(new_by_id.keys()))

    for tool_id in sorted(all_ids):
        old_tool = old_by_id.get(tool_id)
        new_tool = new_by_id.get(tool_id)

        if old_tool is None and new_tool is not None:
            # new tool added (enabled)
            changes.append(_change(
                category="tool_enabled",
                param="enabled",
                tool_id=tool_id,
                tool_type=new_tool.get(f"{tool_kind}_type", new_tool.get("action_type", new_tool.get("indicator_type"))),
                timescale=new_tool.get("timescale"),
                old_value=None,
                new_value=True,
            ))
            continue

        if old_tool is not None and new_tool is None:
            # tool removed (disabled)
            changes.append(_change(
                category="tool_disabled",
                param="enabled",
                tool_id=tool_id,
                tool_type=old_tool.get(f"{tool_kind}_type", old_tool.get("action_type", old_tool.get("indicator_type"))),
                timescale=old_tool.get("timescale"),
                old_value=True,
                new_value=None,
            ))
            continue

        # both exist — check for changes
        assert old_tool is not None and new_tool is not None

        tool_type = new_tool.get(f"{tool_kind}_type", new_tool.get("action_type", new_tool.get("indicator_type")))
        timescale = new_tool.get("timescale")

        # enabled/disabled toggle
        old_enabled = old_tool.get("enabled", True)
        new_enabled = new_tool.get("enabled", True)
        if old_enabled != new_enabled:
            cat = "tool_enabled" if new_enabled else "tool_disabled"
            changes.append(_change(
                category=cat,
                param="enabled",
                tool_id=tool_id,
                tool_type=tool_type,
                timescale=timescale,
                old_value=old_enabled,
                new_value=new_enabled,
            ))

        # weight change (indicators only)
        if "weight" in old_tool or "weight" in new_tool:
            old_w = old_tool.get("weight")
            new_w = new_tool.get("weight")
            if old_w != new_w:
                changes.append(_change(
                    category="weight_adjusted",
                    param="weight",
                    tool_id=tool_id,
                    tool_type=tool_type,
                    timescale=timescale,
                    old_value=old_w,
                    new_value=new_w,
                ))

        # params diff
        old_params = old_tool.get("params", {})
        new_params = new_tool.get("params", {})
        all_param_keys = set(list(old_params.keys()) + list(new_params.keys()))
        for pkey in sorted(all_param_keys):
            if old_params.get(pkey) != new_params.get(pkey):
                changes.append(_change(
                    category="knob_tuned",
                    param=pkey,
                    tool_id=tool_id,
                    tool_type=tool_type,
                    timescale=timescale,
                    old_value=old_params.get(pkey),
                    new_value=new_params.get(pkey),
                ))

    return changes
