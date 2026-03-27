import { NextRequest, NextResponse } from "next/server";
import { getAgentMemos } from "@/lib/data/sources/agents";

export async function GET(request: NextRequest) {
  const limit = Number(request.nextUrl.searchParams.get("limit") ?? 50);
  try {
    const memos = await getAgentMemos(limit);
    return NextResponse.json(memos);
  } catch (err) {
    return NextResponse.json([], { status: 200 });
  }
}
