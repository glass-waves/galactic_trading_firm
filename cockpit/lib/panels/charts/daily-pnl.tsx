"use client";

import { useRef, useEffect } from "react";
import * as d3 from "d3";
import { registerPanel } from "../registry";
import type { PanelProps } from "../types";
import { colors } from "@/lib/theme";

interface DailyPnlPoint {
  day: string;
  pnl: number;
}

function DailyPnlComponent({ data }: PanelProps<DailyPnlPoint[]>) {
  const svgRef = useRef<SVGSVGElement>(null);

  useEffect(() => {
    if (!svgRef.current || data.length === 0) return;

    const sorted = [...data].sort(
      (a, b) => new Date(a.day).getTime() - new Date(b.day).getTime()
    );

    const svg = d3.select(svgRef.current);
    svg.selectAll("*").remove();

    const width = svgRef.current.clientWidth;
    const height = 120;
    const margin = { top: 4, right: 4, bottom: 16, left: 4 };
    const innerW = width - margin.left - margin.right;
    const innerH = height - margin.top - margin.bottom;

    svg.attr("width", width).attr("height", height);

    const g = svg
      .append("g")
      .attr("transform", `translate(${margin.left},${margin.top})`);

    const x = d3
      .scaleBand()
      .domain(sorted.map((d) => d.day))
      .range([0, innerW])
      .padding(0.2);

    const maxAbs = d3.max(sorted, (d) => Math.abs(d.pnl)) ?? 1;
    const y = d3.scaleLinear().domain([-maxAbs, maxAbs]).range([innerH, 0]);

    // zero line
    g.append("line")
      .attr("x1", 0)
      .attr("x2", innerW)
      .attr("y1", y(0))
      .attr("y2", y(0))
      .attr("stroke", colors.border)
      .attr("stroke-width", 1);

    // bars
    g.selectAll("rect")
      .data(sorted)
      .enter()
      .append("rect")
      .attr("x", (d) => x(d.day) ?? 0)
      .attr("y", (d) => (d.pnl >= 0 ? y(d.pnl) : y(0)))
      .attr("width", x.bandwidth())
      .attr("height", (d) => Math.abs(y(0) - y(d.pnl)))
      .attr("fill", (d) => (d.pnl >= 0 ? colors.green : colors.red));

    // tooltip
    g.selectAll("rect")
      .on("mouseenter", function (_event, d) {
        const datum = d as DailyPnlPoint;
        d3.select(this).attr("opacity", 0.7);
        tooltip
          .text(`${datum.day}: ${datum.pnl >= 0 ? "+" : ""}$${datum.pnl.toFixed(2)}`)
          .attr("visibility", "visible");
      })
      .on("mouseleave", function () {
        d3.select(this).attr("opacity", 1);
        tooltip.attr("visibility", "hidden");
      });

    const tooltip = svg
      .append("text")
      .attr("x", margin.left + 4)
      .attr("y", height - 2)
      .attr("fill", colors.textDim)
      .attr("font-size", "10px")
      .attr("font-family", "inherit")
      .attr("visibility", "hidden");
  }, [data]);

  if (data.length === 0) {
    return <div className="text-text-muted text-xs">no data</div>;
  }

  return <svg ref={svgRef} className="w-full" style={{ height: 120 }} />;
}

registerPanel<DailyPnlPoint[]>({
  id: "daily-pnl",
  name: "daily P&L",
  component: DailyPnlComponent,
  defaultConfig: {},
  fetchData: async () => {
    const res = await fetch("/api/daily/performance?days=30");
    if (!res.ok) throw new Error("failed to fetch daily P&L");
    return res.json();
  },
});
