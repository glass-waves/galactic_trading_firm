import { NextResponse } from "next/server";
import { getTradeStats } from "@/lib/data/sources/trades";
import { getCsvTradeStats } from "@/lib/data/sources/csv";

export async function GET() {
  try {
    const stats = await getTradeStats();
    if (stats.totalTrades > 0) return NextResponse.json(stats);
    return NextResponse.json(getCsvTradeStats());
  } catch {
    // postgres down — serve from CSV
    try {
      return NextResponse.json(getCsvTradeStats());
    } catch (csvErr) {
      const message = csvErr instanceof Error ? csvErr.message : "unknown error";
      return NextResponse.json({ error: message }, { status: 500 });
    }
  }
}
