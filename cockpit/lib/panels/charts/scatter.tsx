"use client";

import { useRef, useEffect, useState } from "react";
import * as d3 from "d3";
import { registerPanel } from "../registry";
import type { PanelProps } from "../types";
import { colors } from "@/lib/theme";
import type { Trade } from "@/lib/types";

function TradeScatterComponent({ data }: PanelProps<Trade[]>) {
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(800);

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

    const height = 240;
    const margin = { top: 8, right: 8, bottom: 24, left: 56 };
    const innerW = width - margin.left - margin.right;
    const innerH = height - margin.top - margin.bottom;

    svg.attr("width", width).attr("height", height);

    const g = svg
      .append("g")
      .attr("transform", `translate(${margin.left},${margin.top})`);

    const sorted = [...data].sort(
      (a, b) =>
        new Date(a.exit_fill_at).getTime() - new Date(b.exit_fill_at).getTime()
    );

    const x = d3
      .scaleTime()
      .domain(
        d3.extent(sorted, (d) => new Date(d.exit_fill_at)) as [Date, Date]
      )
      .range([0, innerW]);

    const maxAbs = d3.max(sorted, (d) => Math.abs(d.pnl_dollars)) ?? 1;
    const y = d3
      .scaleLinear()
      .domain([-maxAbs * 1.1, maxAbs * 1.1])
      .range([innerH, 0]);

    // zero line
    g.append("line")
      .attr("x1", 0)
      .attr("x2", innerW)
      .attr("y1", y(0))
      .attr("y2", y(0))
      .attr("stroke", colors.border)
      .attr("stroke-dasharray", "2,2");

    // dots
    g.selectAll("circle")
      .data(sorted)
      .enter()
      .append("circle")
      .attr("cx", (d) => x(new Date(d.exit_fill_at)))
      .attr("cy", (d) => y(d.pnl_dollars))
      .attr("r", 2)
      .attr("fill", (d) => (d.pnl_dollars >= 0 ? colors.green : colors.red))
      .attr("opacity", 0.6);

    // axes
    g.append("g")
      .attr("transform", `translate(0,${innerH})`)
      .call(
        d3
          .axisBottom(x)
          .ticks(6)
          .tickFormat((d) => d3.timeFormat("%Y-%m")(d as Date))
      )
      .call((g) => g.select(".domain").attr("stroke", colors.border))
      .call((g) =>
        g
          .selectAll(".tick text")
          .attr("fill", colors.textMuted)
          .attr("font-size", "9px")
          .attr("font-family", "inherit")
      )
      .call((g) => g.selectAll(".tick line").attr("stroke", colors.border));

    g.append("g")
      .call(
        d3
          .axisLeft(y)
          .ticks(5)
          .tickFormat((d) => `$${(d as number).toFixed(0)}`)
      )
      .call((g) => g.select(".domain").attr("stroke", colors.border))
      .call((g) =>
        g
          .selectAll(".tick text")
          .attr("fill", colors.textMuted)
          .attr("font-size", "9px")
          .attr("font-family", "inherit")
      )
      .call((g) => g.selectAll(".tick line").attr("stroke", colors.border));

    // tooltip on hover
    const tooltip = svg
      .append("text")
      .attr("x", margin.left + 4)
      .attr("y", margin.top + 12)
      .attr("fill", colors.textDim)
      .attr("font-size", "10px")
      .attr("font-family", "inherit")
      .attr("visibility", "hidden");

    g.selectAll("circle")
      .on("mouseenter", function (_event, d) {
        const trade = d as Trade;
        d3.select(this).attr("r", 4).attr("opacity", 1);
        tooltip
          .text(
            `${trade.ticker} ${trade.exit_fill_at.slice(0, 10)} ${trade.pnl_dollars >= 0 ? "+" : ""}$${trade.pnl_dollars.toFixed(2)} ${trade.exit_reason}`
          )
          .attr("visibility", "visible");
      })
      .on("mouseleave", function () {
        d3.select(this).attr("r", 2).attr("opacity", 0.6);
        tooltip.attr("visibility", "hidden");
      });
  }, [data, width]);

  if (data.length === 0) {
    return <div className="text-text-muted text-xs">no trades</div>;
  }

  return (
    <div ref={containerRef} className="w-full">
      <svg ref={svgRef} style={{ height: 240 }} className="w-full" />
    </div>
  );
}

registerPanel<Trade[]>({
  id: "trade-scatter",
  name: "trade scatter",
  component: TradeScatterComponent,
  defaultConfig: {},
  fetchData: async () => {
    const res = await fetch("/api/trades?limit=5000");
    if (!res.ok) throw new Error("failed to fetch trades");
    return res.json();
  },
});
