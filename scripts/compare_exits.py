#!/usr/bin/env python3
"""compare exit strategy backtest results across variants."""
import csv
import sys
from collections import defaultdict

def load_trades(filepath):
    """load trade rows from CSV, skip summary rows."""
    trades = []
    with open(filepath) as f:
        reader = csv.DictReader(f)
        for row in reader:
            if row.get('row_type') == 'trade':
                trades.append(row)
    return trades

def analyze(trades, label):
    """compute key metrics from trade list."""
    if not trades:
        print(f"\n{label}: NO TRADES")
        return {}

    pnls = [float(t['pnl']) for t in trades]
    winners = [p for p in pnls if p > 0]
    losers = [p for p in pnls if p <= 0]
    total_pnl = sum(pnls)
    gross_profit = sum(winners) if winners else 0
    gross_loss = abs(sum(losers)) if losers else 0.001
    pf = gross_profit / gross_loss if gross_loss > 0 else 999

    # exit reason breakdown
    by_reason = defaultdict(list)
    for t in trades:
        by_reason[t['exit_reason']].append(float(t['pnl']))

    # hold durations
    hold_durations = [int(t['hold_duration_ms']) / 60000 for t in trades]

    win_rate = len(winners) / len(pnls) * 100
    avg_winner = sum(winners) / len(winners) if winners else 0
    avg_loser = sum(losers) / len(losers) if losers else 0

    print(f"\n{'='*60}")
    print(f"  {label}")
    print(f"{'='*60}")
    print(f"  trades: {len(trades)}  |  P&L: ${total_pnl:.2f}  |  PF: {pf:.2f}")
    print(f"  win rate: {win_rate:.1f}%  |  avg win: ${avg_winner:.2f}  |  avg loss: ${avg_loser:.2f}")
    print(f"  W/L ratio: {avg_winner / abs(avg_loser) if avg_loser != 0 else 999:.2f}")
    print(f"  avg hold: {sum(hold_durations)/len(hold_durations):.0f} min  |  max hold: {max(hold_durations):.0f} min")

    print(f"\n  exit breakdown:")
    for reason, reason_pnls in sorted(by_reason.items(), key=lambda x: -len(x[1])):
        r_wins = [p for p in reason_pnls if p > 0]
        r_total = sum(reason_pnls)
        r_wr = len(r_wins) / len(reason_pnls) * 100
        r_avg = r_total / len(reason_pnls)
        print(f"    {reason:20s}  n={len(reason_pnls):3d}  P&L=${r_total:7.2f}  WR={r_wr:5.1f}%  avg=${r_avg:6.2f}")

    return {
        'label': label, 'trades': len(trades), 'pnl': total_pnl,
        'pf': pf, 'wr': win_rate, 'avg_win': avg_winner, 'avg_loss': avg_loser,
    }

if __name__ == '__main__':
    files = sys.argv[1:]
    if not files:
        print("usage: python3 compare_exits.py file1.csv [label1] file2.csv [label2] ...")
        sys.exit(1)

    # parse args as file [label] pairs
    results = []
    i = 0
    while i < len(files):
        filepath = files[i]
        if i + 1 < len(files) and not files[i + 1].endswith('.csv'):
            label = files[i + 1]
            i += 2
        else:
            label = filepath.split('/')[-1].replace('_trades.csv', '')
            i += 1
        trades = load_trades(filepath)
        r = analyze(trades, label)
        if r:
            results.append(r)

    if len(results) > 1:
        print(f"\n{'='*60}")
        print(f"  COMPARISON SUMMARY")
        print(f"{'='*60}")
        base = results[0]
        print(f"  {'variant':30s} {'trades':>6s} {'P&L':>10s} {'PF':>6s} {'WR':>6s} {'delta':>10s}")
        for r in results:
            delta = r['pnl'] - base['pnl']
            delta_str = f"{'+'if delta>=0 else ''}{delta:.2f}"
            print(f"  {r['label']:30s} {r['trades']:6d} ${r['pnl']:9.2f} {r['pf']:6.2f} {r['wr']:5.1f}% {delta_str:>10s}")
