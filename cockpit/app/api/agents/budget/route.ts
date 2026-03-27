import { NextRequest, NextResponse } from "next/server";
import { getDailyBudgetHistory } from "@/lib/data/sources/agents";

export async function GET(request: NextRequest) {
  const days = Number(request.nextUrl.searchParams.get("days") ?? 30);
  try {
    const budget = await getDailyBudgetHistory(days);
    return NextResponse.json(budget);
  } catch (err) {
    return NextResponse.json([], { status: 200 });
  }
}
