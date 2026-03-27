// semantic color constants — functional only, no decoration
export const colors = {
  bg: "#0A0A0A",
  surface: "#141414",
  border: "#1E1E1E",
  borderAlt: "#2A2A2A",
  text: "#E0E0E0",
  textDim: "#808080",
  textMuted: "#4A4A4A",
  green: "#00CC66",
  red: "#CC3333",
  amber: "#CC9900",
  cyan: "#4499CC",
} as const;

export type ThemeColor = keyof typeof colors;

// P&L color helper
export function pnlColor(value: number): string {
  if (value > 0) return colors.green;
  if (value < 0) return colors.red;
  return colors.textDim;
}

// status color mapping
export function statusColor(
  status: string
): string {
  switch (status) {
    case "promoted":
      return colors.green;
    case "rejected":
      return colors.red;
    case "proposed":
    case "backtesting":
      return colors.amber;
    case "validated":
      return colors.cyan;
    default:
      return colors.textMuted;
  }
}
