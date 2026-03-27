import { NextRequest, NextResponse } from "next/server";
import { getBeliefs, getActiveBeliefsCount } from "@/lib/data/sources/agents";

export async function GET(request: NextRequest) {
  const view = request.nextUrl.searchParams.get("view");

  try {
    if (view === "count") {
      const count = await getActiveBeliefsCount();
      return NextResponse.json({ count });
    }

    const status = request.nextUrl.searchParams.get("status") ?? undefined;
    const beliefs = await getBeliefs(status);
    return NextResponse.json(beliefs);
  } catch (err) {
    const message = err instanceof Error ? err.message : "unknown error";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
