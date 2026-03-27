import { NextRequest, NextResponse } from "next/server";
import { getConfigVersions, getLatestPromoted } from "@/lib/data/sources/configs";

export async function GET(request: NextRequest) {
  const { searchParams } = request.nextUrl;
  const view = searchParams.get("view");

  try {
    if (view === "latest-promoted") {
      const config = await getLatestPromoted();
      return NextResponse.json(config);
    }

    const status = searchParams.get("status") ?? undefined;
    const configs = await getConfigVersions(status);
    return NextResponse.json(configs);
  } catch (err) {
    const message = err instanceof Error ? err.message : "unknown error";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
