"use client";

import { useRef, useEffect, useState } from "react";
import * as d3 from "d3";
import { registerPanel } from "../registry";
import type { PanelProps } from "../types";
import { colors } from "@/lib/theme";
import type { Trade } from "@/lib/types";

interface HistogramData {
  winners: number[];
  losers: number[];
  label: string;
}

function ScoreHistogramComponent({ data }: PanelProps<HistogramData>) {
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
    if (!svgRef.current) return;

    const svg = d3.select(svgRef.current);
    svg.selectAll("*").remove();

    const height = 180;
    const margin = { top: 16, right: 8, bottom: 24, left: 32 };
    const innerW = width - margin.left - margin.right;
    const innerH = height - margin.top - margin.bottom;

    svg.attr("width", width).attr("height", height);

    const g = svg
      .append("g")
      .attr("transform", `translate(${margin.left},${margin.top})`);

    const allValues = [...data.winners, ...data.losers].filter(
      (v) => v != null && !isNaN(v)
    );
    if (allValues.length === 0) return;

    const x = d3
      .scaleLinear()
      .domain([
        d3.min(allValues) ?? -1,
        d3.max(allValues) ?? 1,
      ])
      .range([0, innerW]);

    const bins = 25;

    const winnerBins = d3
      .bin()
      .domain(x.domain() as [number, number])
      .thresholds(bins)(data.winners);

    const loserBins = d3
      .bin()
      .domain(x.domain() as [number, number])
      .thresholds(bins)(data.losers);

    const maxCount = d3.max(
      [...winnerBins, ...loserBins],
      (d) => d.length
    ) ?? 1;

    const y = d3.scaleLinear().domain([0, maxCount]).range([innerH, 0]);

    // winner bars (green outline, no fill)
    g.selectAll(".winner")
      .data(winnerBins)
      .enter()
      .append("rect")
      .attr("class", "winner")
      .attr("x", (d) => x(d.x0 ?? 0))
      .attr("y", (d) => y(d.length))
      .attr("width", (d) => Math.max(0, x(d.x1 ?? 0) - x(d.x0 ?? 0) - 1))
      .attr("height", (d) => innerH - y(d.length))
      .attr("fill", "none")
      .attr("stroke", colors.green)
      .attr("stroke-width", 1);

    // loser bars (red outline)
    g.selectAll(".loser")
      .data(loserBins)
      .enter()
      .append("rect")
      .attr("class", "loser")
      .attr("x", (d) => x(d.x0 ?? 0))
      .attr("y", (d) => y(d.length))
      .attr("width", (d) => Math.max(0, x(d.x1 ?? 0) - x(d.x0 ?? 0) - 1))
      .attr("height", (d) => innerH - y(d.length))
      .attr("fill", "none")
      .attr("stroke", colors.red)
      .attr("stroke-width", 1);

    // axes
    g.append("g")
      .attr("transform", `translate(0,${innerH})`)
      .call(d3.axisBottom(x).ticks(6))
      .call((g) => g.select(".domain").attr("stroke", colors.border))
      .call((g) =>
        g
          .selectAll(".tick text")
          .attr("fill", colors.textMuted)
          .attr("font-size", "9px")
          .attr("font-family", "inherit")
      )
      .call((g) => g.selectAll(".tick line").attr("stroke", colors.border));

    // label
    svg
      .append("text")
      .attr("x", margin.left)
      .attr("y", 10)
      .attr("fill", colors.textDim)
      .attr("font-size", "10px")
      .attr("font-family", "inherit")
      .text(data.label);

    // legend
    svg
      .append("text")
      .attr("x", width - margin.right - 80)
      .attr("y", 10)
      .attr("fill", colors.green)
      .attr("font-size", "9px")
      .attr("font-family", "inherit")
      .text(`win: ${data.winners.length}`);

    svg
      .append("text")
      .attr("x", width - margin.right - 20)
      .attr("y", 10)
      .attr("fill", colors.red)
      .attr("font-size", "9px")
      .attr("font-family", "inherit")
      .text(`loss: ${data.losers.length}`);
  }, [data, width]);

  return (
    <div ref={containerRef} className="w-full">
      <svg ref={svgRef} style={{ height: 180 }} className="w-full" />
    </div>
  );
}

// composite score distribution
registerPanel<HistogramData>({
  id: "score-distribution-composite",
  name: "entry composite distribution",
  component: ScoreHistogramComponent,
  defaultConfig: {},
  fetchData: async () => {
    const res = await fetch("/api/trades?limit=5000");
    if (!res.ok) throw new Error("failed to fetch");
    const trades: Trade[] = await res.json();
    return {
      winners: trades
        .filter((t) => t.pnl_dollars > 0 && t.entry_score_composite != null)
        .map((t) => t.entry_score_composite!),
      losers: trades
        .filter((t) => t.pnl_dollars <= 0 && t.entry_score_composite != null)
        .map((t) => t.entry_score_composite!),
      label: "composite",
    };
  },
});

// 5-min score distribution
registerPanel<HistogramData>({
  id: "score-distribution-5m",
  name: "entry 5min distribution",
  component: ScoreHistogramComponent,
  defaultConfig: {},
  fetchData: async () => {
    const res = await fetch("/api/trades?limit=5000");
    if (!res.ok) throw new Error("failed to fetch");
    const trades: Trade[] = await res.json();
    return {
      winners: trades
        .filter((t) => t.pnl_dollars > 0 && t.entry_score_5min != null)
        .map((t) => t.entry_score_5min!),
      losers: trades
        .filter((t) => t.pnl_dollars <= 0 && t.entry_score_5min != null)
        .map((t) => t.entry_score_5min!),
      label: "5min",
    };
  },
});

// hourly score distribution
registerPanel<HistogramData>({
  id: "score-distribution-1h",
  name: "entry hourly distribution",
  component: ScoreHistogramComponent,
  defaultConfig: {},
  fetchData: async () => {
    const res = await fetch("/api/trades?limit=5000");
    if (!res.ok) throw new Error("failed to fetch");
    const trades: Trade[] = await res.json();
    return {
      winners: trades
        .filter((t) => t.pnl_dollars > 0 && t.entry_score_hourly != null)
        .map((t) => t.entry_score_hourly!),
      losers: trades
        .filter((t) => t.pnl_dollars <= 0 && t.entry_score_hourly != null)
        .map((t) => t.entry_score_hourly!),
      label: "hourly",
    };
  },
});
