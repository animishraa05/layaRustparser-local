"use client";

import { TimeSeriesPoint } from "@/lib/types";
import { Code2 } from "lucide-react";

interface PerformanceChartProps {
  history: TimeSeriesPoint[];
  onToggleJson?: () => void;
  showJson?: boolean;
  status?: "LIVE" | "MOCK" | "OFFLINE" | "STALE";
}

export function PerformanceChart({
  history,
  onToggleJson,
  showJson = false,
  status = "LIVE",
}: PerformanceChartProps) {
  // SVG canvas dimensions
  const width = 800;
  const height = 160;

  // Compute points or use default trajectory if history is building up
  const points =
    history.length >= 2
      ? history
      : history.length === 1
      ? [{ ...history[0], time: "-10s" }, history[0]]
      : status === "MOCK"
      ? [
          { time: "-60s", eps: 141000, latency_p50: 1.35 },
          { time: "-50s", eps: 141500, latency_p50: 1.3 },
          { time: "-40s", eps: 142000, latency_p50: 1.34 },
          { time: "-30s", eps: 141800, latency_p50: 1.28 },
          { time: "-20s", eps: 142200, latency_p50: 1.26 },
          { time: "-10s", eps: 142400, latency_p50: 1.29 },
          { time: "0s", eps: 142500, latency_p50: 1.28 },
        ]
      : [
          { time: "-10s", eps: 0, latency_p50: 0 },
          { time: "0s", eps: 0, latency_p50: 0 },
        ];

  // Dynamic scale calculations for EPS:
  // Automatically adapts to steady demo rates (e.g. 100 - 1,000 EPS) or high-volume bursts (50k - 200k EPS)
  const nonZeroEps = points
    .map((p) => p.eps)
    .filter((v) => typeof v === "number" && v > 0);
  const peakEps = nonZeroEps.length > 0 ? Math.max(...nonZeroEps) : 142500;
  const lowestEps = nonZeroEps.length > 0 ? Math.min(...nonZeroEps) : 120000;

  let minEps: number;
  let maxEps: number;

  if (peakEps > 10000) {
    // High-speed mode (e.g. 100k - 200k EPS)
    minEps = Math.max(0, lowestEps * 0.85);
    maxEps = Math.max(peakEps * 1.15, minEps + 10000);
  } else if (peakEps > 0) {
    // Steady demo rate mode (e.g. 100 - 1,000 EPS)
    minEps = Math.max(0, lowestEps * 0.75);
    maxEps = Math.max(peakEps * 1.3, minEps + 50);
  } else {
    minEps = 0;
    maxEps = 1000;
  }
  const epsRange = Math.max(1, maxEps - minEps);

  // Dynamic scale calculations for Latency (range ~0.2 - 10.0 µs)
  const nonZeroLat = points
    .map((p) => p.latency_p50)
    .filter((v) => typeof v === "number" && v > 0);
  const peakLat = nonZeroLat.length > 0 ? Math.max(...nonZeroLat) : 2.5;
  const lowestLat = nonZeroLat.length > 0 ? Math.min(...nonZeroLat) : 0.5;
  const minLat = Math.max(0.1, lowestLat * 0.8);
  const maxLat = Math.max(minLat + 0.5, peakLat * 1.25);
  const latRange = Math.max(0.1, maxLat - minLat);

  const count = points.length;
  const stepX = width / Math.max(1, count - 1);

  const epsCoords = points.map((p, i) => {
    const x = i * stepX;
    // higher EPS -> lower Y
    const clampedEps = Math.max(minEps, Math.min(maxEps, p.eps || minEps));
    const y = height - ((clampedEps - minEps) / epsRange) * (height - 30) - 20;
    return { x, y };
  });

  const latCoords = points.map((p, i) => {
    const x = i * stepX;
    const clampedLat = Math.max(minLat, Math.min(maxLat, p.latency_p50 || minLat));
    const y = height - ((clampedLat - minLat) / latRange) * (height - 40) - 10;
    return { x, y };
  });

  const epsPolyline = epsCoords.map((c) => `${c.x.toFixed(1)},${c.y.toFixed(1)}`).join(" ");
  const epsPolygon = `0,${height} ${epsPolyline} ${width},${height}`;
  const latPolyline = latCoords.map((c) => `${c.x.toFixed(1)},${c.y.toFixed(1)}`).join(" ");

  const latestEpsCoord = epsCoords[epsCoords.length - 1] || { x: width, y: 44 };

  const formatEpsLabel = (val: number) => {
    if (val >= 1000) return `${(val / 1000).toFixed(0)}k EPS`;
    return `${Math.round(val)} EPS`;
  };

  return (
    <div
      data-tour="performance-chart"
      className="flex flex-col bg-white p-4 rounded-xl border border-[#E2E8F0] shadow-sm"
    >
      {/* Header */}
      <div className="flex flex-wrap items-center justify-between gap-2 pb-3">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-sm bg-[#006398]"></div>
          <span className="text-[1.125rem] text-[#1E293B] font-semibold">
            Real-Time Ingestion Throughput &amp; Latency Profile
          </span>
          <span className="font-mono text-[0.75rem] text-[#64748B] px-1.5 py-0.5 rounded bg-[#F0F3FF]">
            GET /metrics
          </span>
        </div>
        <div className="flex items-center gap-3">
          <span className="inline-flex items-center gap-1 font-mono text-[0.75rem] text-[#64748B]">
            <span className="w-2.5 h-2.5 rounded-sm bg-[#006398]"></span> Throughput (EPS)
          </span>
          <span className="inline-flex items-center gap-1 font-mono text-[0.75rem] text-[#64748B]">
            <span className="w-2.5 h-2.5 rounded-sm bg-[#009768]"></span> P50 (µs)
          </span>
        </div>
      </div>

      {/* SVG Canvas */}
      <div className="relative w-full h-64 bg-[#F0F3FF]/50 rounded-lg p-3 flex flex-col justify-between border border-[#E2E8F0]">
        {status === "OFFLINE" && (
          <div className="absolute inset-0 bg-white/85 backdrop-blur-[2px] rounded-lg flex flex-col items-center justify-center z-10 p-4">
            <span className="font-mono text-[0.875rem] font-semibold text-amber-800">
              Telemetry stream offline
            </span>
            <span className="font-mono text-[0.75rem] text-[#64748B] mt-1 text-center">
              Backend is not responding on /metrics
            </span>
          </div>
        )}
        <div className="flex justify-between text-[#64748B] font-mono text-[0.6875rem]">
          <span>{formatEpsLabel(maxEps)}</span>
          <span>Target SLA Plateau (Sub-µs Latency)</span>
          <span>{minLat.toFixed(1)} µs Latency</span>
        </div>

        <svg
          className="w-full h-44 overflow-visible"
          preserveAspectRatio="none"
          viewBox={`0 0 ${width} ${height}`}
        >
          <defs>
            <linearGradient id="epsGrad" x1="0" x2="0" y1="0" y2="1">
              <stop offset="0%" stopColor="#006398" stopOpacity="0.3"></stop>
              <stop offset="100%" stopColor="#006398" stopOpacity="0.0"></stop>
            </linearGradient>
            <linearGradient id="latencyGrad" x1="0" x2="0" y1="0" y2="1">
              <stop offset="0%" stopColor="#009768" stopOpacity="0.25"></stop>
              <stop offset="100%" stopColor="#009768" stopOpacity="0.0"></stop>
            </linearGradient>
          </defs>

          {/* Grid Lines */}
          <line
            stroke="#DEE8FF"
            strokeDasharray="4 4"
            strokeWidth="1"
            x1="0"
            x2={width}
            y1="40"
            y2="40"
          ></line>
          <line
            stroke="#DEE8FF"
            strokeDasharray="4 4"
            strokeWidth="1"
            x1="0"
            x2={width}
            y1="80"
            y2="80"
          ></line>
          <line
            stroke="#DEE8FF"
            strokeDasharray="4 4"
            strokeWidth="1"
            x1="0"
            x2={width}
            y1="120"
            y2="120"
          ></line>

          {/* EPS Area Fill & Line */}
          <polygon fill="url(#epsGrad)" points={epsPolygon}></polygon>
          <polyline
            fill="none"
            points={epsPolyline}
            stroke="#006398"
            strokeLinejoin="round"
            strokeWidth="2.5"
          ></polyline>

          {/* Latency Line */}
          <polyline
            fill="none"
            points={latPolyline}
            stroke="#009768"
            strokeLinejoin="round"
            strokeWidth="2"
          ></polyline>

          {/* Live Cursor Marker */}
          <circle
            className="animate-ping"
            cx={latestEpsCoord.x}
            cy={latestEpsCoord.y}
            fill="#006398"
            r="4"
          ></circle>
          <circle
            cx={latestEpsCoord.x}
            cy={latestEpsCoord.y}
            fill="#ffffff"
            r="3"
            stroke="#006398"
            strokeWidth="2"
          ></circle>
        </svg>

        {/* Time X-Axis */}
        <div className="flex justify-between text-[#64748B] font-mono text-[0.75rem] pt-1">
          <span>-60s (T-01:00)</span>
          <span>-45s</span>
          <span>-30s</span>
          <span>-15s</span>
          <span className="text-[#006398] font-semibold">
            {status === "LIVE"
              ? "T-00:00 (Live)"
              : status === "MOCK"
              ? "T-00:00 (Mock)"
              : status === "STALE"
              ? "T-00:00 (Stale)"
              : "T-00:00 (Offline)"}
          </span>
        </div>
      </div>

      {/* Footer info & Toggle */}
      <div className="flex flex-wrap items-center justify-between gap-2 pt-3 mt-1 border-t border-[#F1F5F9]">
        <span className="text-[0.75rem] text-[#64748B]">
          Throughput sustained above benchmark threshold with zero dropped frames.
        </span>
        {onToggleJson && (
          <button
            onClick={onToggleJson}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-mono text-[0.75rem] transition-colors cursor-pointer border ${
              showJson
                ? "bg-[#0284C7] text-white border-[#0284C7]"
                : "bg-[#F0F3FF] hover:bg-[#DEE8FF] text-[#1E293B] border-[#CBD5E1]"
            }`}
          >
            <Code2 className="w-4 h-4 text-[#0284C7]" />
            <span>{showJson ? "Hide API Payload JSON" : "View API Payload JSON"}</span>
          </button>
        )}
      </div>
    </div>
  );
}
