"use client";

import React, { useState, useEffect, useCallback, useRef, useSyncExternalStore } from "react";
import { AppShell } from "@/components/layout/AppShell";
import { BlockSelector } from "@/components/query-explorer/BlockSelector";
import { QueryConsole } from "@/components/query-explorer/QueryConsole";
import { ResultsTable } from "@/components/query-explorer/ResultsTable";
import { RecordInspector } from "@/components/query-explorer/RecordInspector";
import {
  getBlocks,
  getBlockRecords,
  getMetrics,
  getApiMode,
  getBackendStatus,
  subscribeBackendStatus,
  checkBackendReachable,
  ApiError,
  ApiMode,
  BackendStatus,
} from "@/lib/api";
import { BlockItem, StoredRecordItem, RecordsQueryParams, MetricsResponse } from "@/lib/types";
import { Terminal, ArrowRight, WifiOff, RotateCcw } from "lucide-react";

export default function QueryExplorerPage() {
  // Shared global API status - single source of truth across Header and Pages
  const backendStatus = useSyncExternalStore<BackendStatus>(
    subscribeBackendStatus,
    () => getBackendStatus(),
    () => "LIVE"
  );
  const apiMode = useSyncExternalStore<ApiMode>(
    subscribeBackendStatus,
    () => getApiMode(),
    () => "LIVE"
  );

  // Blocks state
  const [blocks, setBlocks] = useState<BlockItem[]>([]);
  const [metrics, setMetrics] = useState<MetricsResponse | null>(null);
  const [blocksLoading, setBlocksLoading] = useState(false);
  const [isLiveBlocks, setIsLiveBlocks] = useState(false);

  // Selected block
  const [selectedBlockId, setSelectedBlockId] = useState<number | null>(null);

  // Records state
  const [records, setRecords] = useState<StoredRecordItem[]>([]);
  const [totalRecordsInBlock, setTotalRecordsInBlock] = useState(0);
  const [filteredCount, setFilteredCount] = useState(0);
  const [recordsLoading, setRecordsLoading] = useState(false);

  // Filter & Pagination state
  const [filters, setFilters] = useState<RecordsQueryParams>({});
  const [offset, setOffset] = useState(0);
  const [limit, setLimit] = useState(50);

  // Request & Error state
  const [requestState, setRequestState] = useState<
    "idle" | "loading" | "success" | "error" | "not_found" | "offline"
  >("idle");
  const [errorMessage, setErrorMessage] = useState<string | undefined>(undefined);

  // Selected record for deep inspection
  const [selectedRecord, setSelectedRecord] = useState<StoredRecordItem | null>(null);

  const isMountedRef = useRef(true);
  const activeRequestIdRef = useRef<number>(0);
  const lastFetchedQueryRef = useRef<string>("");
  const prevApiModeRef = useRef<ApiMode | null>(null);
  const prevBackendStatusRef = useRef<BackendStatus | null>(null);

  // Derive consolidated offline / mock flags
  const isOffline = backendStatus === "OFFLINE" || requestState === "offline";
  const isMock = apiMode === "MOCK";
  const effectiveIsLiveBlocks = !isMock && !isOffline && isLiveBlocks;

  // 1. Fetch records for selected block
  const fetchRecords = useCallback(
    async (
      blockId: number,
      queryFilters: RecordsQueryParams,
      pageOffset: number,
      pageLimit: number,
      force: boolean = false
    ) => {
      const mode = getApiMode();
      const queryKey = `${mode}-${blockId}-${JSON.stringify(queryFilters)}-${pageOffset}-${pageLimit}`;
      if (!force && lastFetchedQueryRef.current === queryKey) {
        return;
      }

      const reqId = ++activeRequestIdRef.current;
      setRecordsLoading(true);
      setRequestState("loading");
      setErrorMessage(undefined);

      try {
        const res = await getBlockRecords(blockId, {
          ...queryFilters,
          offset: pageOffset,
          limit: pageLimit,
        });

        if (!isMountedRef.current || activeRequestIdRef.current !== reqId) return;

        lastFetchedQueryRef.current = queryKey;
        setRecords(res.data);
        setTotalRecordsInBlock(res.total);
        setFilteredCount(res.filteredCount);
        setRequestState("success");

        // Keep or select the first record if none selected
        setSelectedRecord((prev) => {
          if (prev && res.data.some((r) => r.event_id === prev.event_id)) {
            return res.data.find((r) => r.event_id === prev.event_id) || prev;
          }
          return res.data[0] || null;
        });
      } catch (err: unknown) {
        if (!isMountedRef.current || activeRequestIdRef.current !== reqId) return;

        if (err instanceof ApiError) {
          if (err.isOffline) {
            setRequestState("offline");
            setErrorMessage("Backend Offline: Unable to reach investigation service.");
          } else if (err.status === 404) {
            setRequestState("not_found");
            setErrorMessage(err.message);
          } else {
            setRequestState("error");
            setErrorMessage(err.message);
          }
        } else {
          setRequestState("error");
          setErrorMessage(err instanceof Error ? err.message : "Failed to load records");
        }
      } finally {
        if (isMountedRef.current && activeRequestIdRef.current === reqId) {
          setRecordsLoading(false);
        }
      }
    },
    []
  );

  // 2. Fetch available blocks ledger and synchronize records
  const loadBlocks = useCallback(
    async (preferredBlockId?: number, shouldFetchRecords: boolean = true) => {
      setBlocksLoading(true);
      try {
        const [res, metricsRes] = await Promise.all([
          getBlocks(),
          getMetrics().catch((err) => {
            console.warn("Failed to fetch /metrics for query explorer:", err);
            return null;
          }),
        ]);
        if (!isMountedRef.current) return;
        setBlocks(res.data);
        if (metricsRes) {
          setMetrics(metricsRes.data);
        }
        setIsLiveBlocks(res.isLive);

        if (res.data.length > 0) {
          let chosenBlockId: number;
          if (preferredBlockId !== undefined && res.data.some((b) => b.block_id === preferredBlockId)) {
            chosenBlockId = preferredBlockId;
          } else if (selectedBlockId !== null && res.data.some((b) => b.block_id === selectedBlockId)) {
            chosenBlockId = selectedBlockId;
          } else {
            const block1 = res.data.find((b) => b.block_id === 1);
            chosenBlockId = block1 ? block1.block_id : res.data[0].block_id;
          }

          setSelectedBlockId(chosenBlockId);

          if (shouldFetchRecords) {
            await fetchRecords(chosenBlockId, filters, offset, limit, true);
          }
        } else {
          setSelectedBlockId(null);
          setRecords([]);
        }
      } catch {
        if (!isMountedRef.current) return;
        // In offline/error case, preserve empty or previous blocks
        setIsLiveBlocks(false);
      } finally {
        if (isMountedRef.current) {
          setBlocksLoading(false);
        }
      }
    },
    [selectedBlockId, filters, offset, limit, fetchRecords]
  );

  // Available vendors derived dynamically from records
  const availableVendors = React.useMemo(() => {
    const set = new Set<string>();
    records.forEach((r) => {
      if (r.vendor) set.add(r.vendor);
    });
    return Array.from(set);
  }, [records]);

  // Synchronize on mount, API mode change (LIVE <-> MOCK), or recovery from OFFLINE
  useEffect(() => {
    isMountedRef.current = true;

    const isInitialMount = prevApiModeRef.current === null;
    const modeChanged = prevApiModeRef.current !== null && prevApiModeRef.current !== apiMode;
    const recoveredOnline = prevBackendStatusRef.current === "OFFLINE" && backendStatus === "LIVE";

    prevApiModeRef.current = apiMode;
    prevBackendStatusRef.current = backendStatus;

    if (isInitialMount || modeChanged || recoveredOnline) {
      if (modeChanged || recoveredOnline) {
        lastFetchedQueryRef.current = "";
      }
      const timer = setTimeout(() => {
        loadBlocks(undefined, true);
      }, 0);

      return () => {
        isMountedRef.current = false;
        clearTimeout(timer);
      };
    }

    return () => {
      isMountedRef.current = false;
    };
  }, [apiMode, backendStatus, loadBlocks]);

  // Load records whenever block, filters, or pagination changes
  useEffect(() => {
    if (selectedBlockId !== null) {
      const timer = setTimeout(() => {
        fetchRecords(selectedBlockId, filters, offset, limit);
      }, 0);
      return () => {
        clearTimeout(timer);
      };
    }
  }, [selectedBlockId, filters, offset, limit, fetchRecords]);

  // Handle block selection change
  const handleSelectBlock = (newBlockId: number) => {
    if (newBlockId === selectedBlockId) return;
    setSelectedBlockId(newBlockId);
    setOffset(0);
  };

  // Handle filter application from console
  const handleApplyFilters = (newFilters: RecordsQueryParams) => {
    setFilters(newFilters);
    setOffset(0);
  };

  // Handle clear filters
  const handleClearFilters = () => {
    setFilters({});
    setOffset(0);
  };

  // Handle pivot action from record inspector
  const handlePivot = (field: "ip" | "vendor" | "disposition", value: string) => {
    setFilters((prev) => ({ ...prev, [field]: value }));
    setOffset(0);
  };

  // Handle refresh / retry
  const handleRefresh = useCallback(() => {
    lastFetchedQueryRef.current = "";
    loadBlocks(selectedBlockId ?? undefined, true);
  }, [loadBlocks, selectedBlockId]);

  const handleRetry = async () => {
    await checkBackendReachable();
    lastFetchedQueryRef.current = "";
    loadBlocks(selectedBlockId ?? undefined, true);
  };

  return (
    <AppShell currentSection="QUERY_EXPLORER" eps={metrics?.eps}>
      <div className="flex flex-col gap-6 max-w-[1600px] mx-auto pb-12">
        {/* Workspace Title & Workflow Breadcrumb */}
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div className="flex flex-col gap-1">
            <div className="flex items-center gap-2">
              <h1 className="text-xl font-bold tracking-tight text-[#1E293B] flex items-center gap-2">
                <Terminal className="w-5 h-5 text-[#0284C7]" />
                Query Explorer & Investigation Workspace
              </h1>

              {/* Page-level status badge */}
              {isOffline ? (
                <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded font-mono text-xs font-bold bg-amber-500/10 text-amber-800 border border-amber-500/30">
                  <span className="w-2 h-2 rounded-full bg-amber-500" />
                  OFFLINE (STALE DATA)
                </span>
              ) : isMock ? (
                <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded font-mono text-xs font-bold bg-sky-500/10 text-sky-700 border border-sky-500/30">
                  <span className="w-2 h-2 rounded-full bg-sky-500" />
                  MOCK FIXTURES
                </span>
              ) : (
                <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded font-mono text-xs font-bold bg-[#10B981]/10 text-[#10B981] border border-[#10B981]/30">
                  <span className="w-2 h-2 rounded-full bg-[#10B981] animate-pulse" />
                  LIVE API
                </span>
              )}
            </div>
            <p className="text-xs text-[#64748B]">
              Forensic investigation lifecycle: Alert → Search → Inspect → Pivot → Verify → Export
            </p>
          </div>

          {/* Workflow Step Tracker */}
          <div className="hidden xl:flex items-center gap-1.5 text-[0.6875rem] font-mono bg-white px-3 py-1.5 rounded-lg border border-[#E2E8F0] shadow-sm">
            <span className={selectedBlockId !== null ? "text-[#0284C7] font-bold" : "text-[#64748B]"}>
              1. Block
            </span>
            <ArrowRight className="w-3 h-3 text-[#CBD5E1]" />
            <span className="text-[#0284C7] font-bold">2. Search</span>
            <ArrowRight className="w-3 h-3 text-[#CBD5E1]" />
            <span className={selectedRecord ? "text-[#0284C7] font-bold" : "text-[#64748B]"}>
              3. Inspect
            </span>
            <ArrowRight className="w-3 h-3 text-[#CBD5E1]" />
            <span className="text-[#64748B]">4. Pivot</span>
            <ArrowRight className="w-3 h-3 text-[#CBD5E1]" />
            <span className="text-[#64748B]">5. Verify Proof</span>
            <ArrowRight className="w-3 h-3 text-[#CBD5E1]" />
            <span className="text-[#64748B]">6. Export</span>
          </div>
        </div>

        {/* Page-level Offline / Stale Data Banner */}
        {isOffline && (
          <div className="bg-amber-50 border border-amber-300 rounded-xl p-4 flex flex-wrap items-center justify-between gap-3 shadow-sm">
            <div className="flex items-center gap-3">
              <div className="w-9 h-9 rounded-xl bg-amber-100 text-amber-700 flex items-center justify-center shrink-0">
                <WifiOff className="w-5 h-5 text-amber-600" />
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <h3 className="text-sm font-bold text-amber-900">
                    Backend Offline — Showing previously loaded data
                  </h3>
                  <span className="text-[0.6875rem] font-mono font-bold px-2 py-0.5 rounded bg-amber-200 text-amber-900 border border-amber-300">
                    STALE CACHE
                  </span>
                </div>
                <p className="text-xs text-amber-700 mt-0.5">
                  The Rust backend is unreachable. Displaying cached {records.length} records and {blocks.length} ledger blocks for offline reference. Actions requiring backend connectivity (live queries, proofs, exports) are paused.
                </p>
              </div>
            </div>
            <button
              onClick={handleRetry}
              disabled={recordsLoading || blocksLoading}
              className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-semibold text-white bg-[#1A1D20] hover:bg-[#2E343A] rounded-lg transition-colors shadow-sm disabled:opacity-50"
            >
              <RotateCcw className={`w-3.5 h-3.5 text-[#0284C7] ${recordsLoading || blocksLoading ? "animate-spin" : ""}`} />
              Retry Connection
            </button>
          </div>
        )}

        {/* 1. Block Selector */}
        <section>
          <BlockSelector
            blocks={blocks}
            selectedBlockId={selectedBlockId}
            onSelectBlock={handleSelectBlock}
            isLoading={blocksLoading}
            onRefresh={handleRefresh}
            isLive={effectiveIsLiveBlocks}
            isOffline={isOffline}
          />
        </section>

        {/* 2. SQL & Quick Query Console */}
        <section>
          <QueryConsole
            selectedBlockId={selectedBlockId}
            filters={filters}
            onApplyFilters={handleApplyFilters}
            onClearFilters={handleClearFilters}
            isLoading={recordsLoading}
            filteredCount={filteredCount}
            totalCount={totalRecordsInBlock}
            availableVendors={availableVendors}
            isOffline={isOffline}
          />
        </section>

        {/* 3. Columnar Results Table */}
        <section>
          <ResultsTable
            records={records}
            selectedRecordId={selectedRecord?.event_id ?? null}
            onSelectRecord={(r) => setSelectedRecord(r)}
            selectedBlockId={selectedBlockId}
            isLoading={recordsLoading}
            requestState={requestState}
            errorMessage={errorMessage}
            totalRecordsInBlock={totalRecordsInBlock}
            filteredCount={filteredCount}
            offset={offset}
            limit={limit}
            filters={filters}
            isOffline={isOffline}
            onPageChange={(newOffset) => setOffset(newOffset)}
            onLimitChange={(newLimit) => {
              setLimit(newLimit);
              setOffset(0);
            }}
            onRetry={handleRetry}
            onClearFilters={handleClearFilters}
          />
        </section>

        {/* 4. Record Deep Inspector, Provenance, Merkle Proof, and Courtroom Bundle Export */}
        <section>
          <RecordInspector
            record={selectedRecord}
            onPivot={handlePivot}
            isOffline={isOffline}
          />
        </section>
      </div>
    </AppShell>
  );
}
