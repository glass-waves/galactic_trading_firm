"use client";

import { useRef, useEffect } from "react";
import * as d3 from "d3";
import { registerPanel } from "../registry";
import type { PanelProps } from "../types";
import { colors } from "@/lib/theme";

interface DayData {
  day: string;
  pnl: number;
}

function CalendarHeatmapComponent({ data }: PanelProps<DayData[]>) {
  const svgRef = useRef<SVGSVGElement>(null);

  useEffect(() => {
    if (!svgRef.current || data.length === 0) return;

    const svg = d3.select(svgRef.current);
    svg.selectAll("*").remove();

    const cellSize = 11;
    const gap = 1;
    const step = cellSize + gap;
    const marginLeft = 20;
    const marginTop = 14;

    // group by year
    const sorted = [...data].sort((a, b) => a.day.localeCompare(b.day));
    const years = [...new Set(sorted.map((d) => d.day.slice(0, 4)))].sort();

    // color scale: diverging red → dark → green
    const maxAbs = d3.max(sorted, (d) => Math.abs(d.pnl)) ?? 1;
    const colorScale = d3
      .scaleLinear<string>()
      .domain([-maxAbs, 0, maxAbs])
      .range([colors.red, colors.bg, colors.green]);

    // build a map for quick lookup
    const pnlMap = new Map(sorted.map((d) => [d.day, d.pnl]));

    let yOffset = 0;

    for (const year of years) {
      const yearStart = new Date(`${year}-01-01`);
      const yearEnd = new Date(`${year}-12-31`);

      // year label
      svg
        .append("text")
        .attr("x", marginLeft)
        .attr("y", yOffset + 10)
        .attr("fill", colors.textMuted)
        .attr("font-size", "9px")
        .attr("font-family", "inherit")
        .text(year);

      const startWeek = d3.timeWeek.count(d3.timeYear(yearStart), yearStart);

      // iterate all days in the year
      const allDays = d3.timeDays(yearStart, d3.timeDay.offset(yearEnd, 1));

      const g = svg
        .append("g")
        .attr("transform", `translate(${marginLeft}, ${yOffset + marginTop})`);

      g.selectAll("rect")
        .data(allDays)
        .enter()
        .append("rect")
        .attr("x", (d) => {
          const week = d3.timeWeek.count(d3.timeYear(d), d);
          return week * step;
        })
        .attr("y", (d) => d.getDay() * step)
        .attr("width", cellSize)
        .attr("height", cellSize)
        .attr("fill", (d) => {
          const key = d.toISOString().slice(0, 10);
          const pnl = pnlMap.get(key);
          if (pnl == null) return colors.surface;
          return colorScale(pnl);
        })
        .on("mouseenter", function (event, d) {
          const key = d.toISOString().slice(0, 10);
          const pnl = pnlMap.get(key);
          tooltip
            .text(
              `${key}: ${pnl != null ? (pnl >= 0 ? "+" : "") + "$" + pnl.toFixed(2) : "no trades"}`
            )
            .attr("visibility", "visible");
        })
        .on("mouseleave", function () {
          tooltip.attr("visibility", "hidden");
        });

      // ~53 weeks, 7 rows
      yOffset += 7 * step + marginTop + 4;
    }

    const totalHeight = yOffset + 16;
    const totalWidth = marginLeft + 53 * step + 10;
    svg.attr("width", totalWidth).attr("height", totalHeight);

    // tooltip
    const tooltip = svg
      .append("text")
      .attr("x", marginLeft)
      .attr("y", totalHeight - 2)
      .attr("fill", colors.textDim)
      .attr("font-size", "10px")
      .attr("font-family", "inherit")
      .attr("visibility", "hidden");
  }, [data]);

  if (data.length === 0) {
    return <div className="text-text-muted text-xs">no data</div>;
  }

  return (
    <div className="overflow-x-auto">
      <svg ref={svgRef} />
    </div>
  );
}

registerPanel<DayData[]>({
  id: "calendar-heatmap",
  name: "trade calendar",
  component: CalendarHeatmapComponent,
  defaultConfig: {},
  fetchData: async () => {
    // fetch all daily P&L (no limit)
    const res = await fetch("/api/daily/performance?days=9999");
    if (!res.ok) throw new Error("failed to fetch calendar data");
    return res.json();
  },
});
