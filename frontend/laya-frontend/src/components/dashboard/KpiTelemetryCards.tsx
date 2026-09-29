"use client";

import { MetricsResponse } from "@/lib/types";
import { TrendingUp, ArrowDown } from "lucide-react";

interface KpiTelemetryCardsProps {
  metrics: MetricsResponse | null;
  epsHistory?: number[];
  status?: "LIVE" | "MOCK" | "OFFLINE" | "STALE";
}

export function KpiTelemetryCards({
  metrics,
  epsHistory = [],
  status = "LIVE",
}: KpiTelemetryCardsProps) {
  if (!metrics || status === "OFFLINE") {
    return (
      <div data-tour="kpi-cards" className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4">
        {/* Card 1: Total EPS */}
        <div className="flex flex-col justify-between bg-white p-4 rounded-xl border border-[#E2E8F0] shadow-sm">
          <div className="flex items-start justify-between">
            <div className="flex flex-col">
              <span className="font-mono text-[0.6875rem] text-[#64748B] uppercase tracking-wider">
                TOTAL EPS (Throughput)
              </span>
              <div className="flex items-baseline gap-2 mt-1">
                <span className="text-[2.25rem] text-[#94A3B8] font-semibold leading-tight">
                  —
                </span>
                <span className="font-mono text-[0.75rem] text-[#94A3B8]">EPS</span>
              </div>
            </div>
            <span className="inline-flex items-center px-1.5 py-0.5 rounded bg-amber-50 text-amber-800 font-mono text-[0.6875rem] font-semibold border border-amber-200">
              OFFLINE
            </span>
          </div>

          <div className="my-2 py-1 flex items-center justify-center h-8 bg-[#F8FAFC] rounded border border-dashed border-[#CBD5E1]">
            <span className="font-mono text-[0.75rem] text-[#64748B]">Metrics unavailable</span>
          </div>

          <div className="flex items-center justify-between text-[#64748B] font-mono text-[0.75rem] pt-1 border-t border-[#F1F5F9]">
            <span>Backend is not responding</span>
            <span className="text-amber-700 font-medium">DISCONNECTED</span>
          </div>
        </div>

        {/* Card 2: Latency P50 */}
        <div className="flex flex-col justify-between bg-white p-4 rounded-xl border border-[#E2E8F0] shadow-sm">
          <div className="flex items-start justify-between">
            <div className="flex flex-col">
              <span className="font-mono text-[0.6875rem] text-[#64748B] uppercase tracking-wider">
                LATENCY P50 (Engine)
              </span>
              <div className="flex items-baseline gap-2 mt-1">
                <span className="text-[2.25rem] text-[#94A3B8] font-semibold leading-tight">
                  —
                </span>
                <span className="font-mono text-[0.75rem] text-[#94A3B8]">µs</span>
              </div>
            </div>
            <span className="inline-flex items-center px-1.5 py-0.5 rounded bg-amber-50 text-amber-800 font-mono text-[0.6875rem] font-semibold border border-amber-200">
              OFFLINE
            </span>
          </div>

          <div className="my-2 py-1 flex items-center justify-center h-8 bg-[#F8FAFC] rounded border border-dashed border-[#CBD5E1]">
            <span className="font-mono text-[0.75rem] text-[#64748B]">Metrics unavailable</span>
          </div>

          <div className="flex items-center justify-between text-[#64748B] font-mono text-[0.75rem] pt-1 border-t border-[#F1F5F9]">
            <span>Target: &lt;50.0 µs</span>
            <span className="text-[#94A3B8] font-medium">P99: —</span>
          </div>
        </div>

        {/* Card 3: LRU Cache Hit Rate */}
        <div className="flex flex-col justify-between bg-white p-4 rounded-xl border border-[#E2E8F0] shadow-sm">
          <div className="flex items-start justify-between">
            <div className="flex flex-col">
              <span className="font-mono text-[0.6875rem] text-[#64748B] uppercase tracking-wider">
                LRU CACHE HIT RATE
              </span>
              <div className="flex items-baseline gap-2 mt-1">
                <span className="text-[2.25rem] text-[#94A3B8] font-semibold leading-tight">
                  —
                </span>
                <span className="font-mono text-[0.75rem] text-[#94A3B8]">%</span>
              </div>
            </div>
            <span className="inline-flex items-center px-1.5 py-0.5 rounded bg-amber-50 text-amber-800 font-mono text-[0.6875rem] font-semibold border border-amber-200">
              OFFLINE
            </span>
          </div>

          <div className="my-2 py-1 flex items-center justify-center h-8 bg-[#F8FAFC] rounded border border-dashed border-[#CBD5E1]">
            <span className="font-mono text-[0.75rem] text-[#64748B]">Metrics unavailable</span>
          </div>

          <div className="flex items-center justify-between text-[#64748B] font-mono text-[0.75rem] pt-1 border-t border-[#F1F5F9]">
            <span>Drain3 Cluster Hits</span>
            <span className="text-[#94A3B8] font-medium">Evictions: —</span>
          </div>
        </div>

        {/* Card 4: Ring Queue Depth */}
        <div className="flex flex-col justify-between bg-white p-4 rounded-xl border border-[#E2E8F0] shadow-sm">
          <div className="flex items-start justify-between">
            <div className="flex flex-col">
              <span className="font-mono text-[0.6875rem] text-[#64748B] uppercase tracking-wider">
                QUEUE DEPTH (RingBuffer)
              </span>
              <div className="flex items-baseline gap-2 mt-1">
                <span className="text-[2.25rem] text-[#94A3B8] font-semibold leading-tight">
                  —
                </span>
                <span className="font-mono text-[0.75rem] text-[#94A3B8]">
                  / —
                </span>
              </div>
            </div>
            <span className="inline-flex items-center px-1.5 py-0.5 rounded bg-amber-50 text-amber-800 font-mono text-[0.6875rem] font-semibold border border-amber-200">
              OFFLINE
            </span>
          </div>

          <div className="my-2 py-1 flex items-center justify-center h-8 bg-[#F8FAFC] rounded border border-dashed border-[#CBD5E1]">
            <span className="font-mono text-[0.75rem] text-[#64748B]">Metrics unavailable</span>
          </div>

          <div className="flex items-center justify-between text-[#64748B] font-mono text-[0.75rem] pt-1 border-t border-[#F1F5F9]">
            <span>RingBuffer: Offline</span>
            <span className="text-amber-700 font-medium">DISCONNECTED</span>
          </div>
        </div>
      </div>
    );
  }

  // Sparkline coordinates from history
  const validHistory = epsHistory.filter((v) => typeof v === "number" && v > 0);
  const points =
    validHistory.length >= 2
      ? validHistory.slice(-10)
      : validHistory.length === 1
      ? [validHistory[0], validHistory[0]]
      : typeof metrics.eps === "number" && metrics.eps > 0
      ? [metrics.eps, metrics.eps]
      : [0, 0];
  const minVal = Math.min(...points) * 0.98;
  const maxVal = Math.max(...points) * 1.02;
  const range = maxVal - minVal || 1;
  const width = 160;
  const height = 28;

  const sparkCoords = points.map((val, idx) => {
    const x = (idx / (points.length - 1)) * width;
    const y = height - ((val - minVal) / range) * (height - 6) - 3;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  const pathD = `M${sparkCoords.join(" L")}`;
  const areaD = `M${sparkCoords[0]} L${sparkCoords.join(" L")} L${width},${height} L0,${height} Z`;

  const queuePct =
    typeof metrics.queue_capacity === "number" &&
    typeof metrics.queue_depth === "number" &&
    metrics.queue_capacity > 0
      ? Math.min(100, Math.round((metrics.queue_depth / metrics.queue_capacity) * 100))
      : 0;

  return (
    <div data-tour="kpi-cards" className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4">
      {/* Card 1: Total EPS */}
      <div className="flex flex-col justify-between bg-white p-4 rounded-xl border border-[#E2E8F0] shadow-sm hover:shadow-md transition-shadow">
        <div className="flex items-start justify-between">
          <div className="flex flex-col">
            <span className="font-mono text-[0.6875rem] text-[#64748B] uppercase tracking-wider">
              TOTAL EPS (Throughput)
            </span>
            <div className="flex items-baseline gap-2 mt-1">
              <span
                className="text-[2.25rem] text-[#1E293B] font-semibold leading-tight"
                suppressHydrationWarning
              >
                {typeof metrics.eps === "number" ? metrics.eps.toLocaleString("en-US") : "—"}
              </span>
              <span className="font-mono text-[0.75rem] text-[#64748B]">EPS</span>
            </div>
          </div>
          <div className="flex items-center gap-1.5">
            {status === "MOCK" && (
              <span className="font-mono text-[0.6875rem] px-2 py-0.5 rounded font-semibold border bg-sky-50 text-sky-700 border-sky-300">
                MOCK
              </span>
            )}
            {status === "LIVE" && (
              <span className="font-mono text-[0.6875rem] px-2 py-0.5 rounded font-semibold border bg-emerald-50 text-emerald-700 border-emerald-300">
                LIVE
              </span>
            )}
            <span className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded bg-[#F0F3FF] text-[#009768] font-mono text-[0.6875rem] font-semibold border border-emerald-200">
              <TrendingUp className="w-3 h-3 text-[#009768]" />
              +12.4% vs avg
            </span>
          </div>
        </div>

        {/* Mini Sparkline SVG */}
        <div className="my-2 pt-1">
          <svg
            className="w-full h-8 text-[#006398] overflow-visible"
            fill="none"
            preserveAspectRatio="none"
            viewBox="0 0 160 28"
          >
            <path
              d={pathD}
              stroke="currentColor"
              strokeLinecap="round"
              strokeWidth="2"
            />
            <path
              d={areaD}
              fill="currentColor"
              fillOpacity="0.08"
            />
          </svg>
        </div>

        <div className="flex items-center justify-between text-[#64748B] font-mono text-[0.75rem] pt-1 border-t border-[#F1F5F9]">
          <span>Ingestion: 50k Buffer</span>
          <span className="text-[#1E293B] font-medium">Cap: 250k</span>
        </div>
      </div>

      {/* Card 2: Latency P50 */}
      <div className="flex flex-col justify-between bg-white p-4 rounded-xl border border-[#E2E8F0] shadow-sm hover:shadow-md transition-shadow">
        <div className="flex items-start justify-between">
          <div className="flex flex-col">
            <span className="font-mono text-[0.6875rem] text-[#64748B] uppercase tracking-wider">
              LATENCY P50 (Engine)
            </span>
            <div className="flex items-baseline gap-2 mt-1">
              <span className="text-[2.25rem] text-[#1E293B] font-semibold leading-tight">
                {typeof metrics.latency_p50_micros === "number"
                  ? metrics.latency_p50_micros.toFixed(2)
                  : "—"}
              </span>
              <span className="font-mono text-[0.75rem] text-[#64748B]">µs</span>
            </div>
          </div>
          <span className="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded bg-[#F0F3FF] text-[#009768] font-mono text-[0.6875rem] font-semibold border border-emerald-200">
            <ArrowDown className="w-3 h-3 text-[#009768]" />
            -64% vs SLA
          </span>
        </div>

        {/* Mini SLA distribution indicator */}
        <div className="my-2 pt-2">
          <div className="w-full bg-[#F0F3FF] h-2 rounded-full overflow-hidden flex">
            <div
              className="bg-[#006398] h-full rounded-full transition-all duration-300"
              style={{
                width: `${Math.min(
                  100,
                  Math.max(5, ((metrics.latency_p50_micros ?? 1.28) / 50.0) * 100)
                )}%`,
              }}
            ></div>
          </div>
        </div>

        <div className="flex items-center justify-between text-[#64748B] font-mono text-[0.75rem] pt-1 border-t border-[#F1F5F9]">
          <span>Target: &lt;50.0 µs</span>
          <span className="text-[#1E293B] font-medium">
            P99: {typeof metrics.latency_p99_micros === "number" ? `${metrics.latency_p99_micros.toFixed(2)} µs` : "—"}
          </span>
        </div>
      </div>

      {/* Card 3: LRU Cache Hit Rate */}
      <div className="flex flex-col justify-between bg-white p-4 rounded-xl border border-[#E2E8F0] shadow-sm hover:shadow-md transition-shadow">
        <div className="flex items-start justify-between">
          <div className="flex flex-col">
            <span className="font-mono text-[0.6875rem] text-[#64748B] uppercase tracking-wider">
              LRU CACHE HIT RATE
            </span>
            <div className="flex items-baseline gap-2 mt-1">
              <span className="text-[2.25rem] text-[#1E293B] font-semibold leading-tight">
                {typeof metrics.lru_hit_rate === "number"
                  ? (metrics.lru_hit_rate * 100).toFixed(1)
                  : "—"}
              </span>
              <span className="font-mono text-[0.75rem] text-[#64748B]">%</span>
            </div>
          </div>
          <span className="inline-flex items-center px-1.5 py-0.5 rounded bg-[#CCE5FF] text-[#004B73] font-mono text-[0.6875rem] font-semibold">
            Fast-Path Active
          </span>
        </div>

        {/* Histogram bars */}
        <div className="my-2 flex items-end gap-1.5 h-8 pt-1">
          <div className="flex-1 bg-[#E7EEFF] h-4 rounded-sm"></div>
          <div className="flex-1 bg-[#E7EEFF] h-5 rounded-sm"></div>
          <div className="flex-1 bg-[#E7EEFF] h-6 rounded-sm"></div>
          <div className="flex-1 bg-[#5BB8FE] h-7 rounded-sm"></div>
          <div className="flex-1 bg-[#006398] h-8 rounded-sm"></div>
        </div>

        <div className="flex items-center justify-between text-[#64748B] font-mono text-[0.75rem] pt-1 border-t border-[#F1F5F9]">
          <span>Drain3 Cluster Hits</span>
          <span className="text-[#1E293B] font-medium">Evictions: 0</span>
        </div>
      </div>

      {/* Card 4: Ring Queue Depth */}
      <div className="flex flex-col justify-between bg-white p-4 rounded-xl border border-[#E2E8F0] shadow-sm hover:shadow-md transition-shadow">
        <div className="flex items-start justify-between">
          <div className="flex flex-col">
            <span className="font-mono text-[0.6875rem] text-[#64748B] uppercase tracking-wider">
              QUEUE DEPTH (RingBuffer)
            </span>
            <div className="flex items-baseline gap-2 mt-1">
              <span className="text-[2.25rem] text-[#1E293B] font-semibold leading-tight">
                {typeof metrics.queue_depth === "number" ? metrics.queue_depth : "—"}
              </span>
              <span
                className="font-mono text-[0.75rem] text-[#64748B]"
                suppressHydrationWarning
              >
                / {typeof metrics.queue_capacity === "number" ? metrics.queue_capacity.toLocaleString("en-US") : "—"}
              </span>
            </div>
          </div>
          <span className="inline-flex items-center px-1.5 py-0.5 rounded bg-[#F0F3FF] text-[#006398] font-mono text-[0.6875rem] font-semibold border border-blue-200">
            Air-Gapped: true
          </span>
        </div>

        <div className="my-2 pt-2">
          <div className="w-full bg-[#F0F3FF] h-2 rounded-full overflow-hidden">
            <div
              className="bg-[#009768] h-full rounded-full transition-all duration-300"
              style={{ width: `${Math.max(1, queuePct)}%` }}
            ></div>
          </div>
        </div>

        <div className="flex items-center justify-between text-[#64748B] font-mono text-[0.75rem] pt-1 border-t border-[#F1F5F9]">
          <span>Status: Nominal ({queuePct}% fill)</span>
          <span className="text-[#009768] font-medium">Loss Rate: 0.00%</span>
        </div>
      </div>
    </div>
  );
}
