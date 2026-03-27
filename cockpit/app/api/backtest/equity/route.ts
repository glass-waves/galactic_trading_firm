import { NextRequest, NextResponse } from "next/server";
import { loadAllCsvTrades } from "@/lib/data/sources/csv";

export async function GET(request: NextRequest) {
  const initialCapital = Number(
    request.nextUrl.searchParams.get("capital") ?? 10000
  );

  try {
    const trades = loadAllCsvTrades();

    // sort chronologically for equity curve
    const sorted = [...trades].sort(
      (a, b) =>
        new Date(a.exit_fill_at).getTime() - new Date(b.exit_fill_at).getTime()
    );

    // build equity curve
    let equity = initialCapital;
    let peak = equity;
    const curve: { time: string; equity: number; drawdown_pct: number }[] = [
      { time: sorted[0]?.exit_fill_at ?? "", equity, drawdown_pct: 0 },
    ];

    for (const trade of sorted) {
      equity += trade.pnl_dollars;
      if (equity > peak) peak = equity;
      const drawdown_pct = peak > 0 ? ((peak - equity) / peak) * 100 : 0;
      curve.push({
        time: trade.exit_fill_at,
        equity: Math.round(equity * 100) / 100,
        drawdown_pct: Math.round(drawdown_pct * 100) / 100,
      });
    }

    return NextResponse.json(curve);
  } catch (err) {
    const message = err instanceof Error ? err.message : "unknown error";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
