"use client";

import { useState, useEffect, useRef } from "react";
import * as d3 from "d3";
import { colors } from "@/lib/theme";

interface IndicatorNode {
  instance_id: string;
  indicator_type: string;
  timescale: string;
  weight: number;
  enabled: boolean;
}

interface ScoringConfig {
  timescale_weights: Record<string, number>;
  entry_threshold: number;
  exit_threshold: number;
  aggregation: string;
  hard_gate_timescales: string[];
}

export function PipelineContent() {
  const [indicators, setIndicators] = useState<IndicatorNode[]>([]);
  const [scoring, setScoring] = useState<ScoringConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const svgRef = useRef<SVGSVGElement>(null);

  useEffect(() => {
    fetch("/api/configs?view=latest-promoted")
      .then((res) => (res.ok ? res.json() : null))
      .then((config) => {
        if (config?.config_blob) {
          setIndicators(config.config_blob.indicators ?? []);
          setScoring(config.config_blob.scoring ?? null);
        }
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  useEffect(() => {
    if (!svgRef.current || indicators.length === 0 || !scoring) return;

    const svg = d3.select(svgRef.current);
    svg.selectAll("*").remove();

    const enabledIndicators = indicators.filter((i) => i.enabled);

    // layout: 4 columns
    // col 0: indicators
    // col 1: timescale aggregation
    // col 2: gates
    // col 3: composite output
    const colX = [40, 280, 480, 640];
    const nodeHeight = 18;
    const nodeGap = 2;

    // group by timescale
    const timescaleOrder = ["OneMinute", "FiveMinute", "OneHour", "OneDay", "OneMonth"];
    const byTs = new Map<string, IndicatorNode[]>();
    for (const ind of enabledIndicators) {
      if (!byTs.has(ind.timescale)) byTs.set(ind.timescale, []);
      byTs.get(ind.timescale)!.push(ind);
    }

    const sortedTs = [...byTs.keys()].sort(
      (a, b) => timescaleOrder.indexOf(a) - timescaleOrder.indexOf(b)
    );

    // compute Y positions for indicators
    type NodePos = { x: number; y: number; w: number; h: number; label: string };
    const indNodes: (NodePos & { timescale: string; weight: number })[] = [];
    const tsNodes: (NodePos & { timescale: string })[] = [];

    let yOffset = 30;
    const tsYMap = new Map<string, number>();

    for (const ts of sortedTs) {
      const inds = byTs.get(ts) ?? [];
      const tsStartY = yOffset;

      for (const ind of inds) {
        indNodes.push({
          x: colX[0],
          y: yOffset,
          w: 200,
          h: nodeHeight,
          label: `${ind.instance_id} (${ind.weight.toFixed(2)})`,
          timescale: ind.timescale,
          weight: ind.weight,
        });
        yOffset += nodeHeight + nodeGap;
      }

      const tsY = tsStartY + ((yOffset - nodeGap - tsStartY) / 2) - nodeHeight / 2;
      const tsW = scoring.timescale_weights[ts];
      tsNodes.push({
        x: colX[1],
        y: tsY,
        w: 160,
        h: nodeHeight,
        label: `${ts} (w=${tsW?.toFixed(2) ?? "?"})`,
        timescale: ts,
      });
      tsYMap.set(ts, tsY + nodeHeight / 2);

      yOffset += 12; // gap between timescale groups
    }

    // gate node
    const gateY = (yOffset - 30) / 2;
    const isHardGate = scoring.hard_gate_timescales.length > 0;

    // composite node
    const compositeY = gateY;

    const totalHeight = yOffset + 20;
    const totalWidth = colX[3] + 160;
    svg.attr("width", totalWidth).attr("height", totalHeight);

    // draw links: indicators → timescales
    for (const ind of indNodes) {
      const tsY = tsYMap.get(ind.timescale);
      if (tsY == null) continue;

      svg
        .append("path")
        .attr(
          "d",
          `M${ind.x + ind.w},${ind.y + ind.h / 2} C${ind.x + ind.w + 30},${ind.y + ind.h / 2} ${colX[1] - 30},${tsY} ${colX[1]},${tsY}`
        )
        .attr("fill", "none")
        .attr("stroke", colors.border)
        .attr("stroke-width", Math.max(1, ind.weight * 4))
        .attr("opacity", 0.5);
    }

    // draw links: timescales → gate
    for (const tsNode of tsNodes) {
      const tsCenter = tsNode.y + tsNode.h / 2;
      svg
        .append("path")
        .attr(
          "d",
          `M${tsNode.x + tsNode.w},${tsCenter} C${tsNode.x + tsNode.w + 30},${tsCenter} ${colX[2] - 30},${gateY + nodeHeight / 2} ${colX[2]},${gateY + nodeHeight / 2}`
        )
        .attr("fill", "none")
        .attr("stroke", colors.border)
        .attr("stroke-width", 2)
        .attr("opacity", 0.5);
    }

    // gate → composite
    svg
      .append("path")
      .attr(
        "d",
        `M${colX[2] + 120},${gateY + nodeHeight / 2} L${colX[3]},${compositeY + nodeHeight / 2}`
      )
      .attr("fill", "none")
      .attr("stroke", colors.border)
      .attr("stroke-width", 2)
      .attr("opacity", 0.5);

    // draw indicator nodes
    for (const ind of indNodes) {
      svg
        .append("rect")
        .attr("x", ind.x)
        .attr("y", ind.y)
        .attr("width", ind.w)
        .attr("height", ind.h)
        .attr("fill", colors.surface)
        .attr("stroke", colors.borderAlt);

      svg
        .append("text")
        .attr("x", ind.x + 4)
        .attr("y", ind.y + 12)
        .attr("fill", colors.textDim)
        .attr("font-size", "9px")
        .attr("font-family", "inherit")
        .text(ind.label);
    }

    // draw timescale nodes
    for (const tsNode of tsNodes) {
      svg
        .append("rect")
        .attr("x", tsNode.x)
        .attr("y", tsNode.y)
        .attr("width", tsNode.w)
        .attr("height", tsNode.h)
        .attr("fill", colors.surface)
        .attr("stroke", colors.text)
        .attr("stroke-width", 1);

      svg
        .append("text")
        .attr("x", tsNode.x + 4)
        .attr("y", tsNode.y + 12)
        .attr("fill", colors.text)
        .attr("font-size", "10px")
        .attr("font-family", "inherit")
        .text(tsNode.label);
    }

    // gate node
    svg
      .append("rect")
      .attr("x", colX[2])
      .attr("y", gateY)
      .attr("width", 120)
      .attr("height", nodeHeight)
      .attr("fill", colors.surface)
      .attr("stroke", isHardGate ? colors.amber : colors.borderAlt);

    svg
      .append("text")
      .attr("x", colX[2] + 4)
      .attr("y", gateY + 12)
      .attr("fill", isHardGate ? colors.amber : colors.textDim)
      .attr("font-size", "10px")
      .attr("font-family", "inherit")
      .text(
        isHardGate
          ? `gates: ${scoring.hard_gate_timescales.join(", ")}`
          : "no gates"
      );

    // composite node
    svg
      .append("rect")
      .attr("x", colX[3])
      .attr("y", compositeY)
      .attr("width", 130)
      .attr("height", nodeHeight)
      .attr("fill", colors.surface)
      .attr("stroke", colors.text);

    svg
      .append("text")
      .attr("x", colX[3] + 4)
      .attr("y", compositeY + 12)
      .attr("fill", colors.text)
      .attr("font-size", "10px")
      .attr("font-family", "inherit")
      .text(`composite ≥ ${scoring.entry_threshold}`);

    // column headers
    const headers = ["indicators", "timescale agg", "gates", "output"];
    headers.forEach((h, i) => {
      svg
        .append("text")
        .attr("x", colX[i])
        .attr("y", 14)
        .attr("fill", colors.textMuted)
        .attr("font-size", "9px")
        .attr("font-family", "inherit")
        .attr("text-transform", "uppercase")
        .text(h);
    });
  }, [indicators, scoring]);

  if (loading) return <div className="text-text-dim text-xs">loading...</div>;

  if (indicators.length === 0) {
    return <div className="text-text-muted text-xs">no config data available</div>;
  }

  return (
    <div className="space-y-4">
      <div className="border border-border bg-surface p-3 overflow-x-auto">
        <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
          scoring pipeline flow
        </div>
        <svg ref={svgRef} />
      </div>

      {/* summary stats */}
      {scoring && (
        <div className="border border-border bg-surface p-3">
          <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
            pipeline summary
          </div>
          <div className="grid grid-cols-4 gap-4 text-xs">
            <div>
              <span className="text-text-muted">indicators </span>
              <span className="text-text">
                {indicators.filter((i) => i.enabled).length} enabled /{" "}
                {indicators.length} total
              </span>
            </div>
            <div>
              <span className="text-text-muted">timescales </span>
              <span className="text-text">
                {new Set(indicators.filter((i) => i.enabled).map((i) => i.timescale)).size}
              </span>
            </div>
            <div>
              <span className="text-text-muted">aggregation </span>
              <span className="text-text">{scoring.aggregation}</span>
            </div>
            <div>
              <span className="text-text-muted">thresholds </span>
              <span className="text-text">
                entry:{scoring.entry_threshold} exit:{scoring.exit_threshold}
              </span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
