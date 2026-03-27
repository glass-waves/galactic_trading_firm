"use client";

import { useRef, useEffect, useState } from "react";
import * as d3 from "d3";
import { registerPanel } from "../registry";
import type { PanelProps } from "../types";
import { colors } from "@/lib/theme";
import type { Trade } from "@/lib/types";

interface ExitBreakdownRow {
  reason: string;
  count: number;
  total_pnl: number;
  win_rate: number;
}

function ExitBreakdownComponent({ data }: PanelProps<ExitBreakdownRow[]>) {
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(400);

  useEffect(() => {
    if (!containerRef.current) return;
    const obs = new ResizeObserver((entries) => {
      setWidth(entries[0].contentRect.width);
    });
    obs.observe(containerRef.current);
    return () => obs.disconnect();
  }, []);

  useEffect(() => {
    if (!svgRef.current || data.length === 0) return;

    const svg = d3.select(svgRef.current);
    svg.selectAll("*").remove();

    const rowHeight = 22;
    const height = data.length * rowHeight + 4;
    const margin = { top: 0, right: 8, bottom: 0, left: 120 };
    const innerW = width - margin.left - margin.right;

    svg.attr("width", width).attr("height", height);

    const totalTrades = d3.sum(data, (d) => d.count);
    const maxCount = d3.max(data, (d) => d.count) ?? 1;

    const x = d3.scaleLinear().domain([0, maxCount]).range([0, innerW]);

    const g = svg
      .append("g")
      .attr("transform", `translate(${margin.left},${margin.top})`);

    data.forEach((d, i) => {
      const y = i * rowHeight + 2;

      // label
      svg
        .append("text")
        .attr("x", margin.left - 4)
        .attr("y", y + 14)
        .attr("fill", colors.textDim)
        .attr("font-size", "10px")
        .attr("font-family", "inherit")
        .attr("text-anchor", "end")
        .text(d.reason);

      // bar
      g.append("rect")
        .attr("x", 0)
        .attr("y", y)
        .attr("width", x(d.count))
        .attr("height", rowHeight - 4)
        .attr("fill", d.total_pnl >= 0 ? colors.green : colors.red)
        .attr("opacity", 0.3);

      // count + pnl text inside bar
      g.append("text")
        .attr("x", x(d.count) + 4)
        .attr("y", y + 13)
        .attr("fill", colors.textDim)
        .attr("font-size", "9px")
        .attr("font-family", "inherit")
        .text(
          `${d.count} (${((d.count / totalTrades) * 100).toFixed(0)}%)  ${d.total_pnl >= 0 ? "+" : ""}$${d.total_pnl.toFixed(2)}  ${d.win_rate.toFixed(0)}% win`
        );
    });
  }, [data, width]);

  if (data.length === 0) {
    return <div className="text-text-muted text-xs">no data</div>;
  }

  return (
    <div ref={containerRef} className="w-full">
      <svg ref={svgRef} className="w-full" />
    </div>
  );
}

registerPanel<ExitBreakdownRow[]>({
  id: "exit-breakdown",
  name: "exit reason breakdown",
  component: ExitBreakdownComponent,
  defaultConfig: {},
  fetchData: async () => {
    const res = await fetch("/api/trades?limit=5000");
    if (!res.ok) throw new Error("failed to fetch");
    const trades: Trade[] = await res.json();

    const byReason = new Map<
      string,
      { count: number; pnl: number; wins: number }
    >();

    for (const t of trades) {
      const entry = byReason.get(t.exit_reason) ?? {
        count: 0,
        pnl: 0,
        wins: 0,
      };
      entry.count++;
      entry.pnl += t.pnl_dollars;
      if (t.pnl_dollars > 0) entry.wins++;
      byReason.set(t.exit_reason, entry);
    }

    return Array.from(byReason.entries())
      .map(([reason, v]) => ({
        reason,
        count: v.count,
        total_pnl: Math.round(v.pnl * 100) / 100,
        win_rate: v.count > 0 ? (v.wins / v.count) * 100 : 0,
      }))
      .sort((a, b) => b.count - a.count);
  },
});
