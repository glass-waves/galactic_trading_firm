"use client";

import { useState, useMemo } from "react";
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  flexRender,
  type ColumnDef,
  type SortingState,
} from "@tanstack/react-table";

export interface DataTableProps<T> {
  data: T[];
  columns: ColumnDef<T, unknown>[];
  pageSize?: number;
  onRowClick?: (row: T) => void;
}

export function DataTable<T>({
  data,
  columns,
  pageSize = 25,
  onRowClick,
}: DataTableProps<T>) {
  const [sorting, setSorting] = useState<SortingState>([]);
  const [globalFilter, setGlobalFilter] = useState("");

  const table = useReactTable({
    data,
    columns,
    state: { sorting, globalFilter },
    onSortingChange: setSorting,
    onGlobalFilterChange: setGlobalFilter,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    initialState: { pagination: { pageSize } },
  });

  return (
    <div>
      <div className="mb-2">
        <input
          type="text"
          value={globalFilter}
          onChange={(e) => setGlobalFilter(e.target.value)}
          placeholder="filter..."
          className="bg-bg border border-border px-2 py-1 text-xs text-text w-48 focus:border-border-alt focus:outline-none"
        />
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-xs">
          <thead>
            {table.getHeaderGroups().map((hg) => (
              <tr key={hg.id} className="border-b border-border">
                {hg.headers.map((header) => (
                  <th
                    key={header.id}
                    className="text-left text-text-muted px-2 py-1 cursor-pointer select-none uppercase tracking-wide font-normal"
                    onClick={header.column.getToggleSortingHandler()}
                  >
                    {flexRender(
                      header.column.columnDef.header,
                      header.getContext()
                    )}
                    {{ asc: " ↑", desc: " ↓" }[
                      header.column.getIsSorted() as string
                    ] ?? ""}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <tr
                key={row.id}
                className={`border-b border-border hover:bg-surface ${
                  onRowClick ? "cursor-pointer" : ""
                }`}
                onClick={() => onRowClick?.(row.original)}
              >
                {row.getVisibleCells().map((cell) => (
                  <td key={cell.id} className="px-2 py-1 whitespace-nowrap">
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="flex items-center gap-3 mt-2 text-xs text-text-dim">
        <button
          onClick={() => table.previousPage()}
          disabled={!table.getCanPreviousPage()}
          className="disabled:text-text-muted hover:text-text"
        >
          ← prev
        </button>
        <span>
          {table.getState().pagination.pageIndex + 1} / {table.getPageCount()}
        </span>
        <button
          onClick={() => table.nextPage()}
          disabled={!table.getCanNextPage()}
          className="disabled:text-text-muted hover:text-text"
        >
          next →
        </button>
        <span className="ml-auto">
          {table.getFilteredRowModel().rows.length} rows
        </span>
      </div>
    </div>
  );
}
