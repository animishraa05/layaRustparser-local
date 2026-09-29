"use client";

import { useState } from "react";
import { CheckCircle2, RefreshCw } from "lucide-react";

interface CommandRibbonProps {
  onSync: () => void;
  isPolling?: boolean;
  isLive?: boolean;
  status?: "LIVE" | "MOCK" | "OFFLINE" | "STALE";
}

export function CommandRibbon({
  onSync,
  isPolling = true,
  isLive = true,
  status,
}: CommandRibbonProps) {
  const [syncing, setSyncing] = useState(false);
  const currentStatus = status ?? (isLive ? "LIVE" : "MOCK");

  const handleSync = async () => {
    setSyncing(true);
    await onSync();
    setTimeout(() => setSyncing(false), 600);
  };

  return (
    <div
      data-tour="command-ribbon"
      className="flex flex-col xl:flex-row xl:items-center justify-between gap-4 bg-white p-4 rounded-xl border border-[#E2E8F0] shadow-sm"
    >
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-1.5 text-[#64748B] font-mono text-[0.75rem] uppercase tracking-wide">
          <span>SOC</span>
          <span>/</span>
          <span>Operations</span>
          <span>/</span>
          <span className="text-[#0284C7] font-semibold">
            Analyst Command Center & Live Telemetry
          </span>
        </div>
        <div className="flex flex-wrap items-center gap-2 pt-0.5">
          <span className="text-[1.25rem] text-[#1E293B] font-semibold">
            ULPF Ingestion Node #01
          </span>
          <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-[#F0F3FF] text-[#009768] font-mono text-[0.75rem] border border-[#CBD5E1]/40">
            <span
              className={`w-2 h-2 rounded-full ${
                isPolling && currentStatus !== "OFFLINE"
                  ? "bg-[#009768] animate-pulse"
                  : "bg-[#64748B]"
              }`}
            ></span>
            GET /metrics Polling: {isPolling ? (currentStatus === "OFFLINE" ? "RETRYING" : "ACTIVE") : "PAUSED"} (Interval: 1,000ms)
          </span>
          <span
            className={`inline-flex items-center px-2 py-0.5 rounded font-mono text-[0.6875rem] font-semibold border ${
              currentStatus === "LIVE"
                ? "bg-emerald-50 text-emerald-700 border-emerald-300"
                : currentStatus === "MOCK"
                ? "bg-sky-50 text-sky-700 border-sky-300"
                : "bg-amber-50 text-amber-800 border-amber-300"
            }`}
          >
            {currentStatus === "LIVE"
              ? "LIVE STREAM"
              : currentStatus === "MOCK"
              ? "MOCK"
              : currentStatus === "STALE"
              ? "STALE"
              : "OFFLINE"}
          </span>
        </div>
      </div>

      {/* Right Side Telemetry Badges */}
      <div className="flex flex-wrap items-center gap-2 bg-[#F0F3FF] p-1.5 rounded-lg border border-[#CBD5E1]/40">
        <div className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-white shadow-sm border border-[#E2E8F0]">
          <CheckCircle2 className="w-3.5 h-3.5 text-[#0284C7]" />
          <span className="font-mono text-[0.6875rem] text-[#1E293B] uppercase tracking-wider font-semibold">
            ZERO_DROP_GUARANTEE
          </span>
        </div>
        <div className="flex items-center gap-1 px-2.5 py-1 rounded bg-white shadow-sm border border-[#E2E8F0]">
          <span className="font-mono text-[0.6875rem] text-[#64748B] uppercase">UPTIME:</span>
          <span className="font-mono text-[0.75rem] text-[#1E293B] font-semibold">3,600s</span>
        </div>
        <div className="flex items-center gap-1 px-2.5 py-1 rounded bg-white shadow-sm border border-[#E2E8F0]">
          <span className="font-mono text-[0.6875rem] text-[#64748B] uppercase">WORM:</span>
          <span className="font-mono text-[0.75rem] text-[#0284C7] font-semibold">
            Parquet Snappy
          </span>
        </div>
        <button
          onClick={handleSync}
          disabled={syncing}
          className="flex items-center gap-1.5 px-3 py-1 rounded bg-[#1A1D20] text-white hover:bg-[#2E343A] transition-colors font-mono text-[0.75rem] shadow-sm cursor-pointer ml-1 disabled:opacity-50"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${syncing ? "animate-spin" : ""}`} />
          {syncing ? "Syncing..." : "Sync State"}
        </button>
      </div>
    </div>
  );
}
