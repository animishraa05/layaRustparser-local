"use client";

import { useState } from "react";
import { MetricsResponse } from "@/lib/types";
import { Terminal, Copy, Check } from "lucide-react";

interface RestContractInspectorProps {
  metrics: MetricsResponse | null;
  status?: "LIVE" | "MOCK" | "OFFLINE" | "STALE";
}

export function RestContractInspector({ metrics, status = "OFFLINE" }: RestContractInspectorProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    const origin = typeof window !== "undefined" ? window.location.origin : "http://127.0.0.1:8080";
    navigator.clipboard.writeText(`curl -s ${origin}/metrics`);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div
      data-tour="contract-inspector"
      className="flex flex-col bg-white p-4 rounded-xl border border-[#E2E8F0] shadow-sm justify-between"
    >
      <div className="flex items-center justify-between pb-2">
        <div className="flex items-center gap-2">
          <Terminal className="w-4 h-4 text-[#0284C7]" />
          <span className="text-[1.125rem] text-[#1E293B] font-semibold">
            REST Contract Inspector
          </span>
        </div>
        <span
          className={`font-mono text-[0.6875rem] px-2 py-0.5 rounded font-semibold border uppercase ${
            status === "LIVE"
              ? "bg-[#F0F3FF] text-[#009768] border-emerald-200"
              : status === "MOCK"
              ? "bg-sky-50 text-sky-700 border-sky-200"
              : "bg-amber-50 text-amber-800 border-amber-200"
          }`}
        >
          {status === "LIVE"
            ? "HTTP 200 OK"
            : status === "MOCK"
            ? "MOCK FIXTURE"
            : status === "STALE"
            ? "STALE CACHE"
            : "OFFLINE"}
        </span>
      </div>

      <div className="bg-[#1A1D20] text-[#E7EEFF] p-3 rounded-lg font-mono text-[0.75rem] my-2 overflow-x-auto shadow-inner border border-[#313540]">
        <div className="flex items-center justify-between text-[#828589] pb-2 mb-2 border-b border-[#313540]/60">
          <span>{"// GET /metrics"}</span>
          <span
            className={
              status === "LIVE"
                ? "text-[#4EDEA3]"
                : status === "MOCK"
                ? "text-[#5BB8FE]"
                : "text-[#FFB4AB]"
            }
          >
            {status === "LIVE"
              ? "live: true"
              : status === "MOCK"
              ? "mode: MOCK"
              : status === "STALE"
              ? "status: STALE"
              : "status: OFFLINE"}
          </span>
        </div>
        {!metrics || status === "OFFLINE" ? (
          <div className="py-8 px-4 text-center font-mono text-[0.75rem] text-[#94A3B8]">
            <p className="text-[#FFB4AB] font-semibold mb-1">Metrics unavailable</p>
            <p className="text-[#828589]">Backend is not responding on /metrics</p>
          </div>
        ) : (
          <pre className="leading-relaxed text-[0.75rem]">
            <span className="text-[#5BB8FE]">&quot;eps&quot;</span>:{" "}
            <span className="text-[#6FFBBE]">{metrics.eps}</span>,{"\n"}
            <span className="text-[#5BB8FE]">&quot;latency_p50_micros&quot;</span>:{" "}
            <span className="text-[#6FFBBE]">{metrics.latency_p50_micros}</span>,{"\n"}
            <span className="text-[#5BB8FE]">&quot;latency_p99_micros&quot;</span>:{" "}
            <span className="text-[#6FFBBE]">{metrics.latency_p99_micros}</span>,{"\n"}
            <span className="text-[#5BB8FE]">&quot;lru_hit_rate&quot;</span>:{" "}
            <span className="text-[#6FFBBE]">{metrics.lru_hit_rate}</span>,{"\n"}
            <span className="text-[#5BB8FE]">&quot;queue_depth&quot;</span>:{" "}
            <span className="text-[#6FFBBE]">{metrics.queue_depth}</span>,{"\n"}
            <span className="text-[#5BB8FE]">&quot;queue_capacity&quot;</span>:{" "}
            <span className="text-[#6FFBBE]">{metrics.queue_capacity}</span>,{"\n"}
            <span className="text-[#5BB8FE]">&quot;total_ingested&quot;</span>:{" "}
            <span className="text-[#6FFBBE]">{metrics.total_ingested}</span>,{"\n"}
            <span className="text-[#5BB8FE]">&quot;vendor_mix&quot;</span>:{" "}
            <span className="text-[#D8E3FB]">
              {JSON.stringify(metrics.vendor_mix)}
            </span>
            ,{"\n"}
            <span className="text-[#5BB8FE]">&quot;disposition_breakdown&quot;</span>:{" "}
            <span className="text-[#D8E3FB]">
              {JSON.stringify(metrics.disposition_breakdown)}
            </span>
            ,{"\n"}
            <span className="text-[#5BB8FE]">&quot;status&quot;</span>:{" "}
            <span className="text-[#93CCFF]">&quot;{metrics.status}&quot;</span>
          </pre>
        )}
      </div>

      <div className="flex items-center justify-between text-[#64748B] font-mono text-[0.75rem] pt-2 border-t border-[#F1F5F9]">
        <span className="flex items-center gap-1.5">
          <span className="w-2 h-2 rounded-full bg-[#009768]"></span> Zero-Copy Deserialization
        </span>
        <button
          onClick={handleCopy}
          className="flex items-center gap-1 text-[#0284C7] hover:underline cursor-pointer"
        >
          {copied ? (
            <>
              <Check className="w-3.5 h-3.5 text-emerald-600" />
              <span className="text-emerald-600">Copied!</span>
            </>
          ) : (
            <>
              <Copy className="w-3.5 h-3.5" />
              <span>Copy Curl</span>
            </>
          )}
        </button>
      </div>
    </div>
  );
}
