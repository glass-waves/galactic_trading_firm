import type { PanelDefinition } from "./types";

// global panel registry — mirrors the rust indicator registry pattern
const panelRegistry = new Map<string, PanelDefinition<any, any>>();

export function registerPanel<TData, TConfig = Record<string, unknown>>(
  panel: PanelDefinition<TData, TConfig>
): void {
  if (panelRegistry.has(panel.id)) {
    console.warn(`panel "${panel.id}" already registered, overwriting`);
  }
  panelRegistry.set(panel.id, panel);
}

export function getPanel(id: string): PanelDefinition | undefined {
  return panelRegistry.get(id);
}

export function listPanels(): PanelDefinition[] {
  return Array.from(panelRegistry.values());
}

export function hasPanelRegistered(id: string): boolean {
  return panelRegistry.has(id);
}
