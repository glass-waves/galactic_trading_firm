import { NextRequest, NextResponse } from "next/server";
import { getDailyPnl } from "@/lib/data/sources/trades";
import { getCsvDailyPnl } from "@/lib/data/sources/csv";

export async function GET(request: NextRequest) {
  const days = Number(request.nextUrl.searchParams.get("days") ?? 30);

  try {
    const data = await getDailyPnl(days);
    if (data.length > 0) return NextResponse.json(data);
    return NextResponse.json(getCsvDailyPnl(days));
  } catch {
    try {
      return NextResponse.json(getCsvDailyPnl(days));
    } catch (csvErr) {
      const message = csvErr instanceof Error ? csvErr.message : "unknown error";
      return NextResponse.json({ error: message }, { status: 500 });
    }
  }
}
