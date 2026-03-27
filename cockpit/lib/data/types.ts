// data source abstraction — independent of panels, shareable across multiple panels
export interface DataSource<TParams = void, TResult = unknown> {
  id: string;
  query: (params: TParams) => Promise<TResult>;
}

// common filter types used across data sources
export interface DateRange {
  start: string; // YYYY-MM-DD
  end: string;
}

export interface TradeFilters {
  ticker?: string;
  exitReason?: string;
  configVersionId?: number;
  dateRange?: DateRange;
  limit?: number;
  offset?: number;
}

export interface ConfigFilters {
  status?: string;
  limit?: number;
}
