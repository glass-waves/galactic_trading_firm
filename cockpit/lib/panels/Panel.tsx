"use client";

import { useEffect, useState } from "react";
import { getPanel } from "./registry";

interface PanelWrapperProps {
  id: string;
  config?: Record<string, unknown>;
  className?: string;
}

export function Panel({ id, config, className }: PanelWrapperProps) {
  const panel = getPanel(id);
  const [data, setData] = useState<unknown>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const mergedConfig = { ...panel?.defaultConfig, ...config };

  useEffect(() => {
    if (!panel) return;
    let cancelled = false;

    setLoading(true);
    setError(null);

    panel
      .fetchData(mergedConfig)
      .then((result) => {
        if (!cancelled) {
          setData(result);
          setLoading(false);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id, JSON.stringify(mergedConfig)]);

  if (!panel) {
    return (
      <div className={`border border-red bg-surface p-3 ${className ?? ""}`}>
        <span className="text-red">panel not found: {id}</span>
      </div>
    );
  }

  if (loading) {
    return (
      <div className={`border border-border bg-surface p-3 ${className ?? ""}`}>
        <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
          {panel.name}
        </div>
        <span className="text-text-dim">loading...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className={`border border-red bg-surface p-3 ${className ?? ""}`}>
        <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
          {panel.name}
        </div>
        <span className="text-red">{error}</span>
      </div>
    );
  }

  const Component = panel.component;
  return (
    <div className={`border border-border bg-surface p-3 ${className ?? ""}`}>
      <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
        {panel.name}
      </div>
      <Component data={data} config={mergedConfig} />
    </div>
  );
}
