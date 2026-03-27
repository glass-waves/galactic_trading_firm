"use client";

import { useState } from "react";
import { colors } from "@/lib/theme";

interface JsonTreeNodeProps {
  keyName: string;
  value: unknown;
  depth: number;
}

function JsonTreeNode({ keyName, value, depth }: JsonTreeNodeProps) {
  const [expanded, setExpanded] = useState(depth < 2);

  if (value === null || value === undefined) {
    return (
      <div style={{ paddingLeft: depth * 12 }} className="py-0.5">
        <span className="text-text-muted">{keyName}: </span>
        <span className="text-text-muted">null</span>
      </div>
    );
  }

  if (typeof value === "object" && !Array.isArray(value)) {
    const entries = Object.entries(value as Record<string, unknown>);
    return (
      <div style={{ paddingLeft: depth * 12 }}>
        <div
          className="py-0.5 cursor-pointer hover:text-text"
          onClick={() => setExpanded(!expanded)}
        >
          <span className="text-text-muted">{expanded ? "▾" : "▸"} </span>
          <span className="text-text-dim">{keyName}</span>
          <span className="text-text-muted"> {`{${entries.length}}`}</span>
        </div>
        {expanded &&
          entries.map(([k, v]) => (
            <JsonTreeNode key={k} keyName={k} value={v} depth={depth + 1} />
          ))}
      </div>
    );
  }

  if (Array.isArray(value)) {
    return (
      <div style={{ paddingLeft: depth * 12 }}>
        <div
          className="py-0.5 cursor-pointer hover:text-text"
          onClick={() => setExpanded(!expanded)}
        >
          <span className="text-text-muted">{expanded ? "▾" : "▸"} </span>
          <span className="text-text-dim">{keyName}</span>
          <span className="text-text-muted"> [{value.length}]</span>
        </div>
        {expanded &&
          value.map((item, i) => (
            <JsonTreeNode
              key={i}
              keyName={String(i)}
              value={item}
              depth={depth + 1}
            />
          ))}
      </div>
    );
  }

  // primitive
  let valueColor: string = colors.text;
  if (typeof value === "number") valueColor = colors.cyan;
  if (typeof value === "boolean") valueColor = colors.amber;
  if (typeof value === "string") valueColor = colors.textDim;

  return (
    <div style={{ paddingLeft: depth * 12 }} className="py-0.5">
      <span className="text-text-muted">{keyName}: </span>
      <span style={{ color: valueColor }}>{JSON.stringify(value)}</span>
    </div>
  );
}

export function JsonTree({ data }: { data: unknown }) {
  if (data === null || data === undefined) {
    return <div className="text-text-muted text-xs">null</div>;
  }

  if (typeof data === "object" && !Array.isArray(data)) {
    return (
      <div className="text-xs overflow-auto max-h-96">
        {Object.entries(data as Record<string, unknown>).map(([k, v]) => (
          <JsonTreeNode key={k} keyName={k} value={v} depth={0} />
        ))}
      </div>
    );
  }

  return (
    <pre className="text-xs text-text-dim overflow-auto max-h-96">
      {JSON.stringify(data, null, 2)}
    </pre>
  );
}
