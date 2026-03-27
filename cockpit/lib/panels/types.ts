import type { ComponentType } from "react";

// every visualization is a PanelDefinition — self-contained with data + rendering
export interface PanelDefinition<TData = unknown, TConfig = Record<string, unknown>> {
  id: string;
  name: string;
  // fetch data client-side. receives panel config, returns typed data.
  fetchData: (config: TConfig) => Promise<TData>;
  // react component that renders the data. receives data + config as props.
  component: ComponentType<PanelProps<TData, TConfig>>;
  // default config for this panel
  defaultConfig: TConfig;
}

export interface PanelProps<TData = unknown, TConfig = Record<string, unknown>> {
  data: TData;
  config: TConfig;
}

// panel instance config passed from page layouts
export interface PanelSlot {
  id: string;
  config?: Record<string, unknown>;
  span?: number; // grid column span (default 1)
  rowSpan?: number; // grid row span (default 1)
}
