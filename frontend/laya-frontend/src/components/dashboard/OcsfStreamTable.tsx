"use client";

import { useState } from "react";
import { StoredRecordItem } from "@/lib/types";
import { Database, Radio, X, Copy, Check } from "lucide-react";
import Link from "next/link";

interface OcsfStreamTableProps {
  records: StoredRecordItem[];
  status?: "LIVE" | "MOCK" | "OFFLINE" | "STALE";
  demoLoopActive?: boolean;
}

export function OcsfStreamTable({
  records,
  status = "LIVE",
  demoLoopActive = false,
}: OcsfStreamTableProps) {
  const [selectedRecord, setSelectedRecord] = useState<StoredRecordItem | null>(null);
  const [copied, setCopied] = useState(false);
  const isOffline = status === "OFFLINE";

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div
      data-tour="ocsf-table"
      className="flex flex-col bg-white p-4 rounded-xl border border-[#E2E8F0] shadow-sm"
    >
      {/* Header */}
      <div className="flex flex-wrap items-center justify-between gap-2 pb-3 border-b border-[#F1F5F9]">
        <div className="flex items-center gap-2.5">
          <Radio className="w-5 h-5 text-[#0284C7]" />
          <div className="flex flex-col">
            <span className="text-[1.125rem] text-[#1E293B] font-semibold">
              Active Event Ingestion Stream (OCSF v1.3 Canonical)
            </span>
            <span className="font-mono text-[0.75rem] text-[#64748B]">
              Live telemetry directly routed through Zero-Copy RingBuffer
            </span>
          </div>
        </div>
        <div className="flex items-center gap-2.5">
          {demoLoopActive && !isOffline && (
            <span className="inline-flex items-center gap-1.5 font-mono text-[0.75rem] text-[#00875A] font-semibold px-2.5 py-0.5 rounded-full bg-[#E3FCEF] border border-[#ABF5D1]">
              <span className="w-2 h-2 rounded-full bg-[#00875A] animate-ping" />
              DEMO AUTO-LOOP ACTIVE
            </span>
          )}
          <span className="font-mono text-[0.75rem] text-[#64748B]">
            {isOffline && records.length === 0
              ? "Stream disconnected"
              : `Showing latest ${records.length} buffered frames`}
          </span>
          <span
            className={`w-2 h-2 rounded-full ${
              isOffline ? "bg-amber-500" : "bg-[#009768] animate-ping"
            }`}
          ></span>
        </div>
      </div>

      {/* Table Canvas */}
      <div className="overflow-x-auto w-full mt-2">
        <table className="w-full text-left text-[0.875rem]">
          <thead>
            <tr className="bg-[#F0F3FF] text-[#64748B] font-mono text-[0.75rem] uppercase tracking-wider border-b border-[#CBD5E1]">
              <th className="py-2.5 px-4 rounded-l-lg">Timestamp (UTC)</th>
              <th className="py-2.5 px-4">UUIDv7 Identifier</th>
              <th className="py-2.5 px-4">OCSF Class</th>
              <th className="py-2.5 px-4">Source Node / IP</th>
              <th className="py-2.5 px-4">Disposition</th>
              <th className="py-2.5 px-4 text-right rounded-r-lg">Raw Payload</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[#F1F5F9]">
            {isOffline && records.length === 0 ? (
              <tr>
                <td
                  colSpan={6}
                  className="py-8 text-center text-[#64748B] font-mono text-[0.75rem]"
                >
                  <p className="text-amber-800 font-semibold mb-1">Stream unavailable</p>
                  <p className="text-[#64748B]">Backend is not responding</p>
                </td>
              </tr>
            ) : records.length === 0 ? (
              <tr>
                <td
                  colSpan={6}
                  className="py-8 text-center text-[#64748B] font-mono text-[0.75rem]"
                >
                  No buffered frames available.
                </td>
              </tr>
            ) : (
              records.map((rec, index) => {
              const dateStr = new Date(rec.timestamp).toLocaleTimeString("en-US", { timeZone: "UTC" });
              const disposition = rec.ocsf?.disposition || "ALLOWED";
              const isAllowed =
                disposition.toUpperCase() === "ALLOWED" ||
                disposition.toUpperCase() === "SUCCESS";
              const isDrop =
                disposition.toUpperCase() === "DROPPED" ||
                disposition.toUpperCase().includes("DROP");

              const srcIp =
                rec.ocsf?.src_endpoint?.ip
                  ? `${rec.ocsf.src_endpoint.ip}:${rec.ocsf.src_endpoint.port || 0}`
                  : "internal-gateway";

              const classUid = rec.ocsf?.class_uid || 4001;
              const className =
                classUid === 4001
                  ? "Network Activity (4001)"
                  : classUid === 3002
                  ? "Authentication (3002)"
                  : classUid === 2001
                  ? "Security Finding (2001)"
                  : `System Activity (${classUid})`;

              return (
                <tr
                  key={`${rec.event_id}-${rec.timestamp}-${index}`}
                  onClick={() => setSelectedRecord(rec)}
                  className={`cursor-pointer transition-colors duration-300 ${
                    index === 0 && demoLoopActive
                      ? "bg-[#F0FDF4]/90 font-medium"
                      : "hover:bg-[#F0F3FF]/50"
                  }`}
                >
                  <td
                    className="py-3 px-4 font-mono text-[0.75rem] text-[#1E293B] whitespace-nowrap"
                    suppressHydrationWarning
                  >
                    {dateStr}
                  </td>
                  <td className="py-3 px-4 font-mono text-[0.75rem] text-[#006398] whitespace-nowrap">
                    {rec.event_id}
                  </td>
                  <td className="py-3 px-4 whitespace-nowrap">
                    <span className="inline-flex items-center px-2 py-0.5 rounded bg-[#E7EEFF] text-[#1E293B] font-mono text-[0.6875rem] font-semibold">
                      {className}
                    </span>
                  </td>
                  <td className="py-3 px-4 font-mono text-[0.75rem] text-[#44474A] whitespace-nowrap">
                    {srcIp}
                  </td>
                  <td className="py-3 px-4 whitespace-nowrap">
                    <span
                      className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded font-mono text-[0.6875rem] font-semibold ${
                        isAllowed
                          ? "bg-emerald-50 text-emerald-800 border border-emerald-200"
                          : isDrop
                          ? "bg-red-50 text-red-800 border border-red-200"
                          : "bg-amber-50 text-amber-800 border border-amber-200"
                      }`}
                    >
                      <span
                        className={`w-1.5 h-1.5 rounded-full ${
                          isAllowed
                            ? "bg-emerald-600"
                            : isDrop
                            ? "bg-red-600"
                            : "bg-amber-600"
                        }`}
                      ></span>
                      {disposition.toUpperCase()}
                    </span>
                  </td>
                  <td className="py-3 px-4 text-right whitespace-nowrap">
                    <button
                      onClick={() => setSelectedRecord(rec)}
                      className="px-2.5 py-1 rounded bg-[#F0F3FF] hover:bg-[#DEE8FF] text-[#1E293B] font-mono text-[0.75rem] transition-colors cursor-pointer border border-[#CBD5E1]"
                    >
                      Inspect JSON
                    </button>
                  </td>
                </tr>
              );
            })
          )}
          </tbody>
        </table>
      </div>

      {/* Table Footer */}
      <div className="flex flex-wrap items-center justify-between gap-2 pt-3 mt-2 border-t border-[#F1F5F9] text-[#64748B] font-mono text-[0.75rem]">
        <div className="flex items-center gap-2">
          <Database className="w-4 h-4 text-[#006398]" />
          <span>Storage engine: Zero-Copy Parquet Partitioned by Epoch Hour</span>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-[#1E293B] font-medium">Batch Commit: 5,000 / block</span>
          <Link
            href="/threat-intelligence"
            className="text-[#0284C7] hover:underline cursor-pointer font-semibold"
          >
            Open Stream Analytics →
          </Link>
        </div>
      </div>

      {/* Inspect JSON Modal */}
      {selectedRecord && (
        <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="bg-white rounded-xl max-w-2xl w-full max-h-[85vh] flex flex-col border border-[#CBD5E1] shadow-2xl">
            <div className="flex items-center justify-between p-4 border-b border-[#E2E8F0]">
              <div className="flex flex-col">
                <span className="text-[1.125rem] font-semibold text-[#1E293B]">
                  OCSF Event Inspection
                </span>
                <span className="font-mono text-[0.75rem] text-[#006398]">
                  ID: {selectedRecord.event_id}
                </span>
              </div>
              <button
                onClick={() => setSelectedRecord(null)}
                className="p-1.5 rounded-lg hover:bg-[#F0F3FF] text-[#64748B] transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <div className="p-4 overflow-y-auto flex flex-col gap-3 font-mono text-[0.75rem]">
              <div>
                <span className="font-semibold text-[#1E293B] block mb-1">
                  Byte-for-Byte Raw Log:
                </span>
                <pre className="p-3 bg-[#F0F3FF] rounded border border-[#CBD5E1] text-[#1E293B] whitespace-pre-wrap break-all text-[0.75rem]">
                  {selectedRecord.raw_log}
                </pre>
              </div>

              <div>
                <span className="font-semibold text-[#1E293B] block mb-1">
                  SHA-256 Provenance Hash:
                </span>
                <code className="p-2 bg-[#F8FAFC] block rounded border border-[#E2E8F0] text-[#006398] text-[0.75rem]">
                  {selectedRecord.raw_hash}
                </code>
              </div>

              <div>
                <span className="font-semibold text-[#1E293B] block mb-1">
                  Canonical OCSF v1.3 JSON:
                </span>
                <pre className="p-3 bg-[#1A1D20] text-[#6FFBBE] rounded border border-[#313540] overflow-x-auto text-[0.75rem]">
                  {JSON.stringify(selectedRecord.ocsf, null, 2)}
                </pre>
              </div>
            </div>

            <div className="flex items-center justify-between p-3 border-t border-[#E2E8F0] bg-[#F8FAFC] rounded-b-xl">
              <button
                onClick={() =>
                  handleCopy(JSON.stringify(selectedRecord.ocsf, null, 2))
                }
                className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-white border border-[#CBD5E1] text-[#1E293B] hover:bg-[#F0F3FF] text-[0.75rem] font-mono font-semibold"
              >
                {copied ? (
                  <>
                    <Check className="w-3.5 h-3.5 text-emerald-600" />
                    <span>Copied JSON</span>
                  </>
                ) : (
                  <>
                    <Copy className="w-3.5 h-3.5" />
                    <span>Copy JSON</span>
                  </>
                )}
              </button>
              <button
                onClick={() => setSelectedRecord(null)}
                className="px-4 py-1.5 rounded bg-[#1A1D20] text-white hover:bg-[#2E343A] text-[0.75rem] font-mono font-semibold"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
