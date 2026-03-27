"use client";

import { useRef, useEffect, useState } from "react";
import * as d3 from "d3";
import { registerPanel } from "../registry";
import type { PanelProps } from "../types";
import { colors } from "@/lib/theme";

interface EquityPoint {
  time: string;
  equity: number;
  drawdown_pct: number;
}

function EquityCurveComponent({ data }: PanelProps<EquityPoint[]>) {
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [dimensions, setDimensions] = useState({ width: 800, height: 300 });

  useEffect(() => {
    if (!containerRef.current) return;
    const obs = new ResizeObserver((entries) => {
      const { width } = entries[0].contentRect;
      setDimensions({ width, height: 300 });
    });
    obs.observe(containerRef.current);
    return () => obs.disconnect();
  }, []);

  useEffect(() => {
    if (!svgRef.current || data.length < 2) return;

    const svg = d3.select(svgRef.current);
    svg.selectAll("*").remove();

    const { width, height } = dimensions;
    const margin = { top: 8, right: 8, bottom: 24, left: 56 };
    const innerW = width - margin.left - margin.right;
    const innerH = height - margin.top - margin.bottom;

    svg.attr("width", width).attr("height", height);

    const g = svg
      .append("g")
      .attr("transform", `translate(${margin.left},${margin.top})`);

    // scales
    const parseTime = (d: EquityPoint) => new Date(d.time);
    const x = d3
      .scaleTime()
      .domain(d3.extent(data, parseTime) as [Date, Date])
      .range([0, innerW]);

    const yExtent = d3.extent(data, (d) => d.equity) as [number, number];
    const yPad = (yExtent[1] - yExtent[0]) * 0.05;
    const y = d3
      .scaleLinear()
      .domain([yExtent[0] - yPad, yExtent[1] + yPad])
      .range([innerH, 0]);

    // drawdown area (inverted, red fill under equity line)
    const maxDD = d3.max(data, (d) => d.drawdown_pct) ?? 1;
    const yDD = d3.scaleLinear().domain([0, maxDD]).range([0, innerH * 0.3]);

    const ddArea = d3
      .area<EquityPoint>()
      .x((d) => x(parseTime(d)))
      .y0(innerH)
      .y1((d) => innerH - yDD(d.drawdown_pct));

    g.append("path")
      .datum(data)
      .attr("d", ddArea)
      .attr("fill", colors.red)
      .attr("opacity", 0.15);

    // equity line
    const line = d3
      .line<EquityPoint>()
      .x((d) => x(parseTime(d)))
      .y((d) => y(d.equity));

    g.append("path")
      .datum(data)
      .attr("d", line)
      .attr("fill", "none")
      .attr("stroke", colors.text)
      .attr("stroke-width", 1.5);

    // axes
    const xAxis = d3
      .axisBottom(x)
      .ticks(6)
      .tickFormat((d) => d3.timeFormat("%Y-%m")(d as Date));

    g.append("g")
      .attr("transform", `translate(0,${innerH})`)
      .call(xAxis)
      .call((g) => g.select(".domain").attr("stroke", colors.border))
      .call((g) =>
        g
          .selectAll(".tick text")
          .attr("fill", colors.textMuted)
          .attr("font-size", "9px")
          .attr("font-family", "inherit")
      )
      .call((g) =>
        g.selectAll(".tick line").attr("stroke", colors.border)
      );

    const yAxis = d3
      .axisLeft(y)
      .ticks(5)
      .tickFormat((d) => `$${(d as number).toFixed(0)}`);

    g.append("g")
      .call(yAxis)
      .call((g) => g.select(".domain").attr("stroke", colors.border))
      .call((g) =>
        g
          .selectAll(".tick text")
          .attr("fill", colors.textMuted)
          .attr("font-size", "9px")
          .attr("font-family", "inherit")
      )
      .call((g) =>
        g.selectAll(".tick line").attr("stroke", colors.border)
      );

    // crosshair + tooltip
    const crosshair = g
      .append("line")
      .attr("y1", 0)
      .attr("y2", innerH)
      .attr("stroke", colors.borderAlt)
      .attr("stroke-dasharray", "2,2")
      .attr("visibility", "hidden");

    const tooltip = svg
      .append("text")
      .attr("x", margin.left + 4)
      .attr("y", margin.top + 12)
      .attr("fill", colors.textDim)
      .attr("font-size", "10px")
      .attr("font-family", "inherit")
      .attr("visibility", "hidden");

    // overlay for mouse events
    g.append("rect")
      .attr("width", innerW)
      .attr("height", innerH)
      .attr("fill", "transparent")
      .on("mousemove", (event: MouseEvent) => {
        const [mx] = d3.pointer(event);
        const date = x.invert(mx);
        const bisect = d3.bisector<EquityPoint, Date>(
          (d) => new Date(d.time)
        ).left;
        const idx = bisect(data, date);
        const d = data[Math.min(idx, data.length - 1)];
        if (!d) return;

        crosshair
          .attr("x1", x(new Date(d.time)))
          .attr("x2", x(new Date(d.time)))
          .attr("visibility", "visible");

        tooltip
          .text(
            `${d.time.slice(0, 10)}  $${d.equity.toFixed(2)}  dd:${d.drawdown_pct.toFixed(1)}%`
          )
          .attr("visibility", "visible");
      })
      .on("mouseleave", () => {
        crosshair.attr("visibility", "hidden");
        tooltip.attr("visibility", "hidden");
      });

    // zoom
    const zoom = d3
      .zoom<SVGSVGElement, unknown>()
      .scaleExtent([1, 20])
      .translateExtent([
        [0, 0],
        [width, height],
      ])
      .on("zoom", (event) => {
        const newX = event.transform.rescaleX(x);
        // re-render line
        g.select<SVGPathElement>("path")
          .datum(data)
          .attr(
            "d",
            d3
              .line<EquityPoint>()
              .x((d) => newX(parseTime(d)))
              .y((d) => y(d.equity))
          );
        // update x axis
        g.select<SVGGElement>("g").call(
          d3
            .axisBottom(newX)
            .ticks(6)
            .tickFormat((d) => d3.timeFormat("%Y-%m")(d as Date))
        );
      });

    svg.call(zoom);
  }, [data, dimensions]);

  if (data.length < 2) {
    return <div className="text-text-muted text-xs">insufficient data</div>;
  }

  return (
    <div ref={containerRef} className="w-full">
      <svg ref={svgRef} style={{ height: 300 }} className="w-full" />
    </div>
  );
}

registerPanel<EquityPoint[]>({
  id: "equity-curve",
  name: "equity curve",
  component: EquityCurveComponent,
  defaultConfig: {},
  fetchData: async () => {
    const res = await fetch("/api/backtest/equity?capital=10000");
    if (!res.ok) throw new Error("failed to fetch equity curve");
    return res.json();
  },
});
