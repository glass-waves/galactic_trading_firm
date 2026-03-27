import { NextRequest, NextResponse } from "next/server";
import { getTrades, getRecentTrades, getTickerSummary } from "@/lib/data/sources/trades";
import {
  loadAllCsvTrades,
  getCsvRecentTrades,
  getCsvTickerSummary,
} from "@/lib/data/sources/csv";

export async function GET(request: NextRequest) {
  const { searchParams } = request.nextUrl;
  const view = searchParams.get("view");

  try {
    if (view === "recent") {
      const limit = Number(searchParams.get("limit") ?? 10);
      // try postgres first, fall back to CSV
      const dbTrades = await getRecentTrades(limit);
      if (dbTrades.length > 0) return NextResponse.json(dbTrades);
      return NextResponse.json(getCsvRecentTrades(limit));
    }

    if (view === "ticker-summary") {
      const dbSummary = await getTickerSummary();
      if (dbSummary.length > 0) return NextResponse.json(dbSummary);
      return NextResponse.json(getCsvTickerSummary());
    }

    // full trade list
    const ticker = searchParams.get("ticker") ?? undefined;
    const exitReason = searchParams.get("exit_reason") ?? undefined;
    const limit = Number(searchParams.get("limit") ?? 500);
    const offset = Number(searchParams.get("offset") ?? 0);

    const dbTrades = await getTrades({ ticker, exitReason, limit, offset });
    if (dbTrades.length > 0) return NextResponse.json(dbTrades);

    // CSV fallback with client-side filtering
    let trades = loadAllCsvTrades();
    if (ticker) trades = trades.filter((t) => t.ticker === ticker);
    if (exitReason) trades = trades.filter((t) => t.exit_reason === exitReason);
    return NextResponse.json(trades.slice(offset, offset + limit));
  } catch (err) {
    // if postgres is down, serve from CSV
    try {
      if (view === "recent") {
        return NextResponse.json(getCsvRecentTrades(10));
      }
      if (view === "ticker-summary") {
        return NextResponse.json(getCsvTickerSummary());
      }
      let trades = loadAllCsvTrades();
      const ticker = searchParams.get("ticker");
      const exitReason = searchParams.get("exit_reason");
      if (ticker) trades = trades.filter((t) => t.ticker === ticker);
      if (exitReason) trades = trades.filter((t) => t.exit_reason === exitReason);
      const limit = Number(searchParams.get("limit") ?? 500);
      const offset = Number(searchParams.get("offset") ?? 0);
      return NextResponse.json(trades.slice(offset, offset + limit));
    } catch {
      const message = err instanceof Error ? err.message : "unknown error";
      return NextResponse.json({ error: message }, { status: 500 });
    }
  }
}
