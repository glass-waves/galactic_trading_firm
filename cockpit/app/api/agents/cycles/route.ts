import { NextRequest, NextResponse } from "next/server";
import { getEvolutionCycles } from "@/lib/data/sources/agents";

export async function GET(request: NextRequest) {
  const limit = Number(request.nextUrl.searchParams.get("limit") ?? 50);
  try {
    const cycles = await getEvolutionCycles(limit);
    return NextResponse.json(cycles);
  } catch (err) {
    return NextResponse.json([], { status: 200 });
  }
}
