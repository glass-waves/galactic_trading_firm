"""backtest validation via rust CLI binary.

runs proposed configs through the backtest engine and validates
results against thresholds before promoting.
"""

from __future__ import annotations

import json
import logging
import os
import subprocess
import tempfile
from typing import Any

from agents.models import ValidationThresholds

logger = logging.getLogger(__name__)

# default data directory for historical candle CSVs
DEFAULT_DATA_DIR = os.environ.get("BACKTEST_DATA_DIR", "")


def run_backtest_validation(
    proposed_config: dict[str, Any],
    data_path: str,
    ticker: str,
    timeout_seconds: int = 120,
) -> dict[str, Any] | None:
    """run the backtest CLI binary with a proposed config.

    writes config to a temp file, calls `cargo run -p backtest`,
    and parses the result JSON from stdout.

    returns the parsed BacktestResult dict, or None on error.
    """
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False
    ) as config_file:
        json.dump(proposed_config, config_file)
        config_path = config_file.name

    try:
        result = subprocess.run(
            [
                "cargo", "run", "-p", "backtest", "--",
                "--config", config_path,
                "--data", data_path,
                "--ticker", ticker,
            ],
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
        )

        if result.returncode != 0:
            logger.error(f"backtest failed (exit {result.returncode}): {result.stderr}")
            return None

        return json.loads(result.stdout)

    except subprocess.TimeoutExpired:
        logger.error(f"backtest timed out after {timeout_seconds}s")
        return None
    except json.JSONDecodeError as e:
        logger.error(f"failed to parse backtest output: {e}")
        return None
    except FileNotFoundError:
        logger.error("cargo binary not found — is rust toolchain installed?")
        return None
    finally:
        os.unlink(config_path)


def validate_backtest_result(
    current_metrics: dict[str, Any] | None,
    proposed_metrics: dict[str, Any],
    thresholds: ValidationThresholds | None = None,
) -> tuple[bool, list[str]]:
    """check whether proposed backtest metrics pass validation gates.

    returns (passed, reasons) where reasons lists any failures.
    """
    if thresholds is None:
        thresholds = ValidationThresholds()

    reasons: list[str] = []

    # min trades gate
    total_trades = proposed_metrics.get("total_trades", 0)
    if total_trades < thresholds.min_trades:
        reasons.append(
            f"insufficient trades: {total_trades} < {thresholds.min_trades}"
        )

    # min win rate gate
    win_rate = proposed_metrics.get("win_rate", 0.0)
    if win_rate < thresholds.min_win_rate:
        reasons.append(
            f"win rate too low: {win_rate:.2%} < {thresholds.min_win_rate:.2%}"
        )

    # max drawdown gate
    max_dd = proposed_metrics.get("max_drawdown_pct", 0.0)
    if max_dd > thresholds.max_drawdown_pct:
        reasons.append(
            f"max drawdown too high: {max_dd:.2%} > {thresholds.max_drawdown_pct:.2%}"
        )

    # sharpe degradation gate (only if we have current metrics to compare)
    if current_metrics is not None:
        current_sharpe = current_metrics.get("sharpe_ratio", 0.0)
        proposed_sharpe = proposed_metrics.get("sharpe_ratio", 0.0)
        degradation = current_sharpe - proposed_sharpe
        if degradation > thresholds.max_sharpe_degradation:
            reasons.append(
                f"sharpe degradation too large: {degradation:.3f} > {thresholds.max_sharpe_degradation:.3f}"
            )

    passed = len(reasons) == 0
    return passed, reasons
