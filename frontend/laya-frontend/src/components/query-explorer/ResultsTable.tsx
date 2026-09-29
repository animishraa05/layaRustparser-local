"use client";

import React from "react";
import { StoredRecordItem, RecordsQueryParams } from "@/lib/types";
import {
  Table as TableIcon,
  ChevronLeft,
  ChevronRight,
  Search,
  Eye,
  AlertCircle,
  WifiOff,
  RotateCcw,
  Layers,
  ArrowRight,
} from "lucide-react";

interface ResultsTableProps {
  records: StoredRecordItem[];
  selectedRecordId: string | null;
  onSelectRecord: (record: StoredRecordItem) => void;
  selectedBlockId: number | null;
  isLoading: boolean;
  requestState: "idle" | "loading" | "success" | "error" | "not_found" | "offline";
  errorMessage?: string;
  totalRecordsInBlock: number;
  filteredCount: number;
  offset: number;
  limit: number;
  filters?: RecordsQueryParams;
  isOffline?: boolean;
  onPageChange: (newOffset: number) => void;
  onLimitChange: (newLimit: number) => void;
  onRetry: () => void;
  onClearFilters: () => void;
}

export function ResultsTable({
  records,
  selectedRecordId,
  onSelectRecord,
  selectedBlockId,
  isLoading,
  requestState,
  errorMessage,
  totalRecordsInBlock,
  filteredCount,
  offset,
  limit,
  filters,
  isOffline = false,
  onPageChange,
  onLimitChange,
  onRetry,
  onClearFilters,
}: ResultsTableProps) {
  // Page bounds calculation
  const currentPage = Math.floor(offset / limit) + 1;
  const totalPages = Math.max(1, Math.ceil(filteredCount / limit));
  const startItem = filteredCount === 0 ? 0 : offset + 1;
  const endItem = Math.min(offset + limit, filteredCount);

  // Status badge styling
  const renderDispositionBadge = (disp?: string) => {
    const val = disp || "UNKNOWN";
    const lower = val.toLowerCase();

    if (lower.includes("allow") || lower.includes("open") || lower.includes("success")) {
      return (
        <span className="inline-flex items-center px-2 py-0.5 rounded text-[0.6875rem] font-semibold bg-[#10B981]/10 text-[#10B981] border border-[#10B981]/30">
          {val}
        </span>
      );
    }
    if (lower.includes("block") || lower.includes("deny")) {
      return (
        <span className="inline-flex items-center px-2 py-0.5 rounded text-[0.6875rem] font-semibold bg-[#FF5C5C]/10 text-[#FF5C5C] border border-[#FF5C5C]/30">
          {val}
        </span>
      );
    }
    if (lower.includes("drop")) {
      return (
        <span className="inline-flex items-center px-2 py-0.5 rounded text-[0.6875rem] font-semibold bg-amber-500/10 text-amber-600 border border-amber-500/30">
          {val}
        </span>
      );
    }

    return (
      <span className="inline-flex items-center px-2 py-0.5 rounded text-[0.6875rem] font-semibold bg-slate-100 text-[#64748B] border border-[#CBD5E1]">
        {val}
      </span>
    );
  };

  // State: No Block Selected
  if (selectedBlockId === null) {
    return (
      <div className="bg-white rounded-xl border border-[#E2E8F0] p-12 text-center flex flex-col items-center justify-center gap-3 shadow-sm">
        <div className="w-12 h-12 rounded-xl bg-slate-100 text-[#64748B] flex items-center justify-center">
          <Layers className="w-6 h-6 text-[#0284C7]" />
        </div>
        <div className="flex flex-col gap-1 max-w-md">
          <h3 className="text-base font-semibold text-[#1E293B]">No block selected</h3>
          <p className="text-xs text-[#64748B]">
            Select a Parquet block from the ledger above to begin searching and investigating columnar log evidence.
          </p>
        </div>
      </div>
    );
  }

  // Preserve loaded data: Only show full-screen error/offline if no records are available
  if (records.length === 0) {
    // State: Offline
    if (requestState === "offline" || isOffline) {
      return (
        <div className="bg-white rounded-xl border border-[#E2E8F0] p-8 text-center flex flex-col items-center justify-center gap-3 shadow-sm">
          <div className="w-12 h-12 rounded-xl bg-rose-50 text-[#FF5C5C] flex items-center justify-center">
            <WifiOff className="w-6 h-6" />
          </div>
          <div className="flex flex-col gap-1 max-w-md">
            <h3 className="text-base font-semibold text-[#1E293B]">Backend Offline</h3>
            <p className="text-xs text-[#64748B]">
              The investigation backend could not be reached. Ensure the Rust backend is running and retry.
            </p>
          </div>
          <button
            onClick={onRetry}
            className="mt-2 flex items-center gap-1.5 px-3 py-1.5 text-xs font-semibold text-white bg-[#1A1D20] hover:bg-[#2E343A] rounded-lg transition-colors"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            Retry Connection
          </button>
        </div>
      );
    }

    // State: 404 Not Found
    if (requestState === "not_found") {
      return (
        <div className="bg-white rounded-xl border border-[#E2E8F0] p-8 text-center flex flex-col items-center justify-center gap-3 shadow-sm">
          <div className="w-12 h-12 rounded-xl bg-amber-50 text-amber-600 flex items-center justify-center">
            <AlertCircle className="w-6 h-6" />
          </div>
          <div className="flex flex-col gap-1 max-w-md">
            <h3 className="text-base font-semibold text-[#1E293B]">Block not found</h3>
            <p className="text-xs text-[#64748B]">
              The selected Parquet block #{selectedBlockId} is not available on disk or has not been archived yet.
            </p>
          </div>
          <button
            onClick={onRetry}
            className="mt-2 flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-[#1E293B] bg-white border border-[#E2E8F0] hover:bg-[#F3F3F3] rounded-lg transition-colors"
          >
            <RotateCcw className="w-3.5 h-3.5 text-[#64748B]" />
            Retry
          </button>
        </div>
      );
    }

    // State: Error 500
    if (requestState === "error") {
      return (
        <div className="bg-white rounded-xl border border-[#E2E8F0] p-8 text-center flex flex-col items-center justify-center gap-3 shadow-sm">
          <div className="w-12 h-12 rounded-xl bg-rose-50 text-[#FF5C5C] flex items-center justify-center">
            <AlertCircle className="w-6 h-6" />
          </div>
          <div className="flex flex-col gap-1 max-w-md">
            <h3 className="text-base font-semibold text-[#1E293B]">Unable to load records</h3>
            <p className="text-xs text-[#64748B]">
              {errorMessage || "The backend encountered an error while reading the selected Parquet block."}
            </p>
          </div>
          <button
            onClick={onRetry}
            className="mt-2 flex items-center gap-1.5 px-3 py-1.5 text-xs font-semibold text-white bg-[#1A1D20] hover:bg-[#2E343A] rounded-lg transition-colors"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            Retry Query
          </button>
        </div>
      );
    }
  }

  const hasActiveFilters = Boolean(
    filters && (filters.vendor || filters.disposition || filters.ip || filters.query)
  );

  return (
    <div className="bg-white rounded-xl border border-[#E2E8F0] flex flex-col shadow-sm overflow-hidden">
      {/* Offline/Error Warning Banner when data is preserved */}
      {records.length > 0 && (requestState === "offline" || isOffline) && (
        <div className="bg-amber-50 border-b border-amber-200 px-4 py-2.5 flex items-center justify-between text-xs text-amber-800">
          <div className="flex items-center gap-2">
            <WifiOff className="w-4 h-4 text-amber-600 shrink-0" />
            <span>
              <strong>Backend Offline — Showing previously loaded data:</strong> Displaying {records.length} cached records (STALE). Live queries, Merkle proofs, and bundle exports are paused until backend reconnects.
            </span>
          </div>
          <button
            onClick={onRetry}
            className="text-xs font-semibold underline hover:text-amber-900 ml-2 shrink-0"
          >
            Retry Connection
          </button>
        </div>
      )}

      {records.length > 0 && requestState === "error" && (
        <div className="bg-rose-50 border-b border-rose-200 px-4 py-2 flex items-center justify-between text-xs text-[#FF5C5C]">
          <div className="flex items-center gap-2">
            <AlertCircle className="w-4 h-4 text-[#FF5C5C] shrink-0" />
            <span>
              <strong>Query Warning:</strong> {errorMessage || "Failed to refresh records from backend."} Displaying previously loaded data.
            </span>
          </div>
          <button
            onClick={onRetry}
            className="text-xs font-semibold underline hover:text-rose-800 ml-2"
          >
            Retry
          </button>
        </div>
      )}

      {/* Table Header Bar */}
      <div className="p-4 border-b border-[#E2E8F0] flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-lg bg-[#1A1D20] text-white flex items-center justify-center shrink-0">
            <TableIcon className="w-4 h-4 text-[#0284C7]" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h3 className="text-base font-semibold text-[#1E293B]">Columnar Query Results</h3>
              <span className="font-mono text-xs text-[#64748B]">
                ({filteredCount} of {totalRecordsInBlock} total)
              </span>
              {isOffline && records.length > 0 && (
                <span className="font-mono text-[0.6875rem] font-bold px-2 py-0.5 rounded bg-amber-100 text-amber-800 border border-amber-300">
                  STALE CACHE
                </span>
              )}
            </div>
            <p className="text-xs text-[#64748B]">
              Click any record row to inspect side-by-side Raw Log, OCSF v1.3 normalization, and cryptographic hash
            </p>
          </div>
        </div>

        {/* Page Limit Selector */}
        <div className="flex items-center gap-2 text-xs">
          <span className="text-[#64748B] font-mono">Rows per page:</span>
          <select
            value={limit}
            onChange={(e) => onLimitChange(Number(e.target.value))}
            className="font-mono bg-white border border-[#CBD5E1] rounded px-2 py-1 text-[#1E293B] focus:ring-1 focus:ring-[#0284C7] focus:outline-none"
          >
            <option value={25}>25</option>
            <option value={50}>50</option>
            <option value={100}>100</option>
          </select>
        </div>
      </div>

      {/* Relevant search/filter summary banner */}
      {hasActiveFilters && (
        <div className="px-4 py-2 bg-[#F8FAFC] border-b border-[#E2E8F0] flex flex-wrap items-center justify-between gap-2 text-xs">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-[0.6875rem] font-mono uppercase text-[#64748B] font-semibold">Active Filter Scope:</span>
            {filters?.vendor && (
              <span className="px-2 py-0.5 rounded font-mono text-[0.6875rem] bg-[#0284C7]/10 text-[#0284C7] border border-[#0284C7]/30">
                vendor = {filters.vendor}
              </span>
            )}
            {filters?.disposition && (
              <span className="px-2 py-0.5 rounded font-mono text-[0.6875rem] bg-slate-100 text-slate-700 border border-[#CBD5E1]">
                disposition = {filters.disposition}
              </span>
            )}
            {filters?.ip && (
              <span className="px-2 py-0.5 rounded font-mono text-[0.6875rem] bg-slate-100 text-slate-700 border border-[#CBD5E1]">
                ip = {filters.ip}
              </span>
            )}
            {filters?.query && (
              <span className="px-2 py-0.5 rounded font-mono text-[0.6875rem] bg-slate-100 text-slate-700 border border-[#CBD5E1]">
                query = &quot;{filters.query}&quot;
              </span>
            )}
          </div>
          <button
            onClick={onClearFilters}
            className="text-[0.6875rem] font-mono text-[#0284C7] hover:underline"
          >
            Clear Filters
          </button>
        </div>
      )}

      {/* Main Table View */}
      <div className="overflow-x-auto w-full">
        <table className="w-full text-left text-xs border-collapse">
          <thead>
            <tr className="bg-[#F8FAFC] text-[#64748B] font-mono uppercase tracking-wider text-[0.6875rem] border-b border-[#E2E8F0]">
              <th className="py-2.5 px-3 font-semibold">Block #</th>
              <th className="py-2.5 px-3 font-semibold">Leaf #</th>
              <th className="py-2.5 px-3 font-semibold">Timestamp (UTC)</th>
              <th className="py-2.5 px-3 font-semibold">Event ID</th>
              <th className="py-2.5 px-3 font-semibold">Vendor</th>
              <th className="py-2.5 px-3 font-semibold">Disposition</th>
              <th className="py-2.5 px-3 font-semibold">Raw Hash (SHA-256)</th>
              <th className="py-2.5 px-3 font-semibold">Endpoints (Src → Dst)</th>
              <th className="py-2.5 px-3 font-semibold text-right">Action</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[#F1F5F9]">
            {isLoading ? (
              <tr>
                <td colSpan={9} className="py-12 text-center text-[#64748B]">
                  <div className="flex flex-col items-center justify-center gap-2">
                    <div className="w-5 h-5 border-2 border-[#0284C7] border-t-transparent rounded-full animate-spin"></div>
                    <span className="font-mono text-xs">Searching records...</span>
                  </div>
                </td>
              </tr>
            ) : records.length === 0 ? (
              <tr>
                <td colSpan={9} className="py-12 text-center text-[#64748B]">
                  <div className="flex flex-col items-center justify-center gap-2 max-w-sm mx-auto">
                    <Search className="w-8 h-8 text-[#CBD5E1]" />
                    <h4 className="text-sm font-semibold text-[#1E293B]">No matching records</h4>
                    <p className="text-xs text-[#64748B]">
                      Try clearing a filter or changing the investigation query.
                    </p>
                    <button
                      onClick={onClearFilters}
                      className="mt-1 text-xs text-[#0284C7] hover:underline font-medium"
                    >
                      Clear active filters
                    </button>
                  </div>
                </td>
              </tr>
            ) : (
              records.map((r) => {
                const isSelected = r.event_id === selectedRecordId;
                const srcIp = r.ocsf?.src_endpoint?.ip;
                const srcPort = r.ocsf?.src_endpoint?.port;
                const dstIp = r.ocsf?.dst_endpoint?.ip;
                const dstPort = r.ocsf?.dst_endpoint?.port;
                const disposition = r.ocsf?.disposition;

                return (
                  <tr
                    key={r.event_id}
                    onClick={() => onSelectRecord(r)}
                    className={`cursor-pointer transition-colors ${
                      isSelected
                        ? "bg-[#0284C7]/10 border-l-4 border-l-[#0284C7] font-medium"
                        : "hover:bg-[#F8FAFC]"
                    }`}
                  >
                    {/* Block ID */}
                    <td className="py-2.5 px-3 font-mono text-[#64748B] whitespace-nowrap">
                      #{String(r.block_id).padStart(5, "0")}
                    </td>

                    {/* Leaf Index */}
                    <td className="py-2.5 px-3 font-mono font-bold text-[#1E293B] whitespace-nowrap">
                      #{r.leaf_index}
                    </td>

                    {/* Timestamp */}
                    <td className="py-2.5 px-3 font-mono text-[#64748B] whitespace-nowrap">
                      {new Date(r.timestamp).toLocaleTimeString("en-US", { timeZone: "UTC" })}
                    </td>

                    {/* Event ID */}
                    <td className="py-2.5 px-3 font-mono text-[#0284C7] truncate max-w-[130px]" title={r.event_id}>
                      {r.event_id}
                    </td>

                    {/* Vendor */}
                    <td className="py-2.5 px-3 text-[#1E293B] font-medium whitespace-nowrap">
                      <span className="px-2 py-0.5 rounded bg-slate-100 font-mono text-[0.6875rem] text-slate-700">
                        {r.vendor}
                      </span>
                    </td>

                    {/* Disposition */}
                    <td className="py-2.5 px-3 whitespace-nowrap">
                      {renderDispositionBadge(disposition)}
                    </td>

                    {/* Raw Hash */}
                    <td className="py-2.5 px-3 font-mono text-[0.6875rem] text-[#64748B] truncate max-w-[110px]" title={r.raw_hash}>
                      {r.raw_hash ? `${r.raw_hash.slice(0, 10)}...` : "—"}
                    </td>

                    {/* Endpoints */}
                    <td className="py-2.5 px-3 font-mono text-[0.6875rem] text-[#64748B] whitespace-nowrap">
                      {srcIp ? (
                        <div className="flex items-center gap-1">
                          <span className="text-[#1E293B]">
                            {srcIp}{srcPort ? `:${srcPort}` : ""}
                          </span>
                          {dstIp && (
                            <>
                              <ArrowRight className="w-3 h-3 text-[#CBD5E1]" />
                              <span className="text-[#1E293B]">
                                {dstIp}{dstPort ? `:${dstPort}` : ""}
                              </span>
                            </>
                          )}
                        </div>
                      ) : (
                        <span className="text-[#94A3B8] italic">No endpoint data</span>
                      )}
                    </td>

                    {/* Action */}
                    <td className="py-2.5 px-3 text-right whitespace-nowrap">
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          onSelectRecord(r);
                        }}
                        className={`inline-flex items-center gap-1 px-2.5 py-1 rounded text-xs transition-colors ${
                          isSelected
                            ? "bg-[#0284C7] text-white font-medium"
                            : "bg-white border border-[#CBD5E1] text-[#1E293B] hover:bg-[#F3F3F3]"
                        }`}
                      >
                        <Eye className="w-3 h-3" />
                        Inspect
                      </button>
                    </td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>

      {/* Pagination Footer */}
      {filteredCount > 0 && (
        <div className="p-3 border-t border-[#E2E8F0] flex flex-wrap items-center justify-between gap-3 text-xs bg-[#F8FAFC]">
          <span className="font-mono text-[#64748B]">
            Showing <strong className="text-[#1E293B]">{startItem}</strong> -{" "}
            <strong className="text-[#1E293B]">{endItem}</strong> of{" "}
            <strong className="text-[#1E293B]">{filteredCount}</strong> results
          </span>

          <div className="flex items-center gap-2">
            <button
              onClick={() => onPageChange(Math.max(0, offset - limit))}
              disabled={offset === 0 || isLoading}
              className="flex items-center gap-1 px-2.5 py-1 rounded border border-[#CBD5E1] bg-white text-[#1E293B] hover:bg-[#F3F3F3] disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              <ChevronLeft className="w-3.5 h-3.5" />
              Previous
            </button>

            <span className="font-mono text-[#64748B] px-1">
              Page <strong className="text-[#1E293B]">{currentPage}</strong> of {totalPages}
            </span>

            <button
              onClick={() => onPageChange(offset + limit)}
              disabled={offset + limit >= filteredCount || isLoading}
              className="flex items-center gap-1 px-2.5 py-1 rounded border border-[#CBD5E1] bg-white text-[#1E293B] hover:bg-[#F3F3F3] disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              Next
              <ChevronRight className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
