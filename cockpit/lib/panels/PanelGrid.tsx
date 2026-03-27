"use client";

import { Panel } from "./Panel";
import type { PanelSlot } from "./types";

interface PanelGridProps {
  cols?: number;
  panels: PanelSlot[];
  className?: string;
}

export function PanelGrid({ cols = 2, panels, className }: PanelGridProps) {
  return (
    <div
      className={`grid gap-px ${className ?? ""}`}
      style={{
        gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))`,
      }}
    >
      {panels.map((slot) => (
        <Panel
          key={slot.id}
          id={slot.id}
          config={slot.config}
          className={
            (slot.span && slot.span > 1 ? `col-span-${slot.span}` : "") +
            (slot.rowSpan && slot.rowSpan > 1
              ? ` row-span-${slot.rowSpan}`
              : "")
          }
        />
      ))}
    </div>
  );
}
