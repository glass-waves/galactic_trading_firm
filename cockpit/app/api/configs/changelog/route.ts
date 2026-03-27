import { NextRequest, NextResponse } from "next/server";
import { getConfigChangelog } from "@/lib/data/sources/configs";

export async function GET(request: NextRequest) {
  const configId = request.nextUrl.searchParams.get("config_version_id");

  try {
    const changelog = await getConfigChangelog(
      configId ? Number(configId) : undefined
    );
    return NextResponse.json(changelog);
  } catch (err) {
    const message = err instanceof Error ? err.message : "unknown error";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
