"""tests for backtest validation logic (pure python, no subprocess)."""

from agents.models import ValidationThresholds
from agents.tools.backtest_runner import validate_backtest_result


def test_validation_passes():
    current = {"sharpe_ratio": 1.5, "win_rate": 0.55, "total_trades": 50}
    proposed = {
        "sharpe_ratio": 1.4,
        "win_rate": 0.52,
        "max_drawdown_pct": 0.08,
        "total_trades": 40,
    }
    passed, reasons = validate_backtest_result(current, proposed)
    assert passed is True
    assert reasons == []


def test_sharpe_degradation_fails():
    current = {"sharpe_ratio": 2.0}
    proposed = {
        "sharpe_ratio": 1.0,
        "win_rate": 0.50,
        "max_drawdown_pct": 0.05,
        "total_trades": 20,
    }
    passed, reasons = validate_backtest_result(current, proposed)
    assert passed is False
    assert any("sharpe degradation" in r for r in reasons)


def test_low_win_rate_fails():
    proposed = {
        "sharpe_ratio": 1.0,
        "win_rate": 0.20,
        "max_drawdown_pct": 0.05,
        "total_trades": 20,
    }
    passed, reasons = validate_backtest_result(None, proposed)
    assert passed is False
    assert any("win rate" in r for r in reasons)


def test_high_drawdown_fails():
    proposed = {
        "sharpe_ratio": 1.0,
        "win_rate": 0.50,
        "max_drawdown_pct": 0.25,
        "total_trades": 20,
    }
    passed, reasons = validate_backtest_result(None, proposed)
    assert passed is False
    assert any("drawdown" in r for r in reasons)


def test_insufficient_trades_fails():
    proposed = {
        "sharpe_ratio": 1.0,
        "win_rate": 0.50,
        "max_drawdown_pct": 0.05,
        "total_trades": 2,
    }
    passed, reasons = validate_backtest_result(None, proposed)
    assert passed is False
    assert any("insufficient trades" in r for r in reasons)


def test_custom_thresholds():
    thresholds = ValidationThresholds(
        max_sharpe_degradation=0.1,
        min_win_rate=0.60,
        max_drawdown_pct=0.05,
        min_trades=10,
    )
    proposed = {
        "sharpe_ratio": 1.0,
        "win_rate": 0.55,
        "max_drawdown_pct": 0.06,
        "total_trades": 8,
    }
    passed, reasons = validate_backtest_result(None, proposed, thresholds)
    assert passed is False
    # should have multiple failures
    assert len(reasons) >= 2


def test_no_current_metrics_skips_sharpe():
    """when current_metrics is None, sharpe degradation check is skipped."""
    proposed = {
        "sharpe_ratio": 0.1,
        "win_rate": 0.50,
        "max_drawdown_pct": 0.05,
        "total_trades": 20,
    }
    passed, reasons = validate_backtest_result(None, proposed)
    assert passed is True
    assert not any("sharpe" in r for r in reasons)
