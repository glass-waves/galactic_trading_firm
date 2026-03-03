/**
 * log reader tool for PM agent.
 *
 * reads structured log files written by paper_trader, backtest, or agents.
 * supports filtering by level and ticker, with tail-based line limiting.
 */

import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

function getLogDir(): string {
  return process.env.LOG_DIR ?? "logs";
}

export interface ReadLogsParams {
  date: string;
  source: "paper_trader" | "backtest" | "agents";
  ticker?: string;
  level?: string;
  tail?: number;
}

interface ReadLogsResult {
  file: string;
  total_lines: number;
  filtered_lines: number;
  returned_lines: number;
  lines: string[];
}

interface ReadLogsError {
  error: string;
  available_files?: string[];
}

export function readLogs(params: ReadLogsParams): string {
  const { date, source, ticker, level, tail = 200 } = params;

  const logDir = getLogDir();
  const prefix = `${source}_${date}`;

  if (!existsSync(logDir)) {
    return JSON.stringify({ error: `log directory not found: ${logDir}`, available_files: [] } satisfies ReadLogsError);
  }

  // find matching files
  const allFiles = readdirSync(logDir).filter((f) => f.endsWith(".log"));
  const matchingFiles = allFiles.filter((f) => f.startsWith(prefix));

  if (matchingFiles.length === 0) {
    return JSON.stringify({
      error: `no log files matching ${prefix}*.log`,
      available_files: allFiles,
    } satisfies ReadLogsError);
  }

  // read the first matching file (there should typically be one per source+date)
  const fileName = matchingFiles[0];
  const filePath = join(logDir, fileName);
  const content = readFileSync(filePath, "utf-8");
  const allLines = content.split("\n").filter((l) => l.length > 0);

  // filter lines
  let filtered = allLines;

  if (level || ticker) {
    filtered = allLines.filter((line) => {
      try {
        const parsed = JSON.parse(line) as Record<string, unknown>;
        if (level && String(parsed.level ?? "").toUpperCase() !== level.toUpperCase()) {
          return false;
        }
        if (ticker) {
          // check common ticker field locations
          const lineStr = JSON.stringify(parsed);
          if (!lineStr.includes(ticker)) {
            return false;
          }
        }
        return true;
      } catch {
        // plain text line — fall back to string matching
        if (level && !line.toUpperCase().includes(level.toUpperCase())) {
          return false;
        }
        if (ticker && !line.includes(ticker)) {
          return false;
        }
        return true;
      }
    });
  }

  // take last N lines
  const returned = filtered.slice(-tail);

  const result: ReadLogsResult = {
    file: fileName,
    total_lines: allLines.length,
    filtered_lines: filtered.length,
    returned_lines: returned.length,
    lines: returned,
  };

  return JSON.stringify(result);
}
