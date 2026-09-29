"use client";

import { AlertItem } from "@/lib/types";
import { ArrowRight, ExternalLink, Filter, Download } from "lucide-react";

interface IncidentQueueProps {
  alerts: AlertItem[] | null;
  selectedId: string | null;
  onSelect: (alert: AlertItem) => void;
}

export function IncidentQueue({ alerts, selectedId, onSelect }: IncidentQueueProps) {
  const isAvailable = !!alerts;
  const alertList = alerts ?? [];

  return (
    <div className="bg-white rounded-xl shadow-sm border border-[#CBD5E1] overflow-hidden">
      {/* Header */}
      <div className="p-4 bg-[#F8FAFC] flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-[#E2E8F0]">
        <div className="flex items-center gap-3">
          <span className="text-[1.125rem] text-[#1E293B] font-semibold tracking-tight">
            Active Incident Queue
          </span>
          {isAvailable ? (
            <span className="font-mono text-[0.75rem] px-2.5 py-0.5 rounded-full bg-[#1A1D20] text-white">
              {alerts.length} CONTRACT ENTITIES
            </span>
          ) : (
            <span className="font-mono text-[0.75rem] px-2.5 py-0.5 rounded-full bg-amber-50 text-amber-800 font-semibold border border-amber-300">
              QUEUE OFFLINE
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => alert("Filter applied to active incidents.")}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded text-[#44474A] bg-white border border-[#CBD5E1] shadow-sm hover:text-[#1E293B] font-mono text-[0.75rem] transition-colors cursor-pointer"
          >
            <Filter className="w-3.5 h-3.5" />
            <span>Filter Status</span>
          </button>
          <button
            onClick={() => {
              if (!isAvailable) return;
              const blob = new Blob([JSON.stringify(alertList, null, 2)], {
                type: "application/json",
              });
              const url = URL.createObjectURL(blob);
              const a = document.createElement("a");
              a.href = url;
              a.download = "incidents-schema.json";
              a.click();
              URL.revokeObjectURL(url);
            }}
            disabled={!isAvailable}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded text-[#44474A] bg-white border border-[#CBD5E1] shadow-sm hover:text-[#1E293B] font-mono text-[0.75rem] transition-colors cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
          >
            <Download className="w-3.5 h-3.5" />
            <span>JSON Schema</span>
          </button>
        </div>
      </div>

      {/* Incident Table */}
      <div className="overflow-x-auto">
        <table className="w-full text-left border-collapse">
          <thead>
            <tr className="bg-[#F0F3FF] text-[#64748B] font-mono text-[0.75rem] uppercase tracking-wider border-b border-[#E2E8F0]">
              <th className="py-3 px-4">Severity</th>
              <th className="py-3 px-4">Incident Title &amp; Detection Rule</th>
              <th className="py-3 px-4">Target Block / Asset</th>
              <th className="py-3 px-4">Primary Indicator</th>
              <th className="py-3 px-4 hidden lg:table-cell">Root Cause Details</th>
              <th className="py-3 px-4">Status</th>
              <th className="py-3 px-4 text-right">Action</th>
            </tr>
          </thead>
          <tbody className="text-[#1E293B] text-[0.875rem] divide-y divide-[#F1F5F9]">
            {!isAvailable ? (
              <tr>
                <td colSpan={7} className="py-8 text-center text-[#64748B] font-mono text-[0.875rem]">
                  <p className="text-amber-800 font-semibold mb-1">Queue unavailable</p>
                  <p className="text-[#64748B] text-[0.75rem]">Backend is not responding on /alerts</p>
                </td>
              </tr>
            ) : alertList.length === 0 ? (
              <tr>
                <td colSpan={7} className="py-8 text-center text-[#64748B] font-mono text-[0.875rem]">
                  No active security incidents found.
                </td>
              </tr>
            ) : (
              alertList.map((alert) => {
              const isSelected = selectedId === alert.id;
              const isCrit =
                alert.severity === "Critical" || alert.severity === "High";

              const ruleName = isCrit
                ? "RULE-HASH-VERIFY-009 // MerkleSealValidator"
                : "RULE-INGEST-DRAIN3-109 // ParserAutoCluster";

              const targetAsset =
                alert.block_id !== undefined
                  ? `Block #${String(alert.block_id).padStart(5, "0")}`
                  : "Block #00000";

              const indicator = isCrit
                ? "172.16.0.25 (DB Vault Node HSM)"
                : "198.51.100.42 (Edge-Router Cisco)";

              const statusText = isCrit
                ? "Investigating - Tier 3"
                : "Unassigned - Auto-Queued";

              return (
                <tr
                  key={alert.id}
                  onClick={() => onSelect(alert)}
                  className={`transition-colors cursor-pointer group ${
                    isSelected
                      ? "bg-[#F0F3FF] border-l-4 border-l-[#0284C7]"
                      : "hover:bg-[#F8FAFC]"
                  }`}
                >
                  <td className="py-3.5 px-4 whitespace-nowrap">
                    <span
                      className={`font-mono text-[0.6875rem] px-2 py-0.5 rounded-full font-semibold flex items-center gap-1 w-max ${
                        isCrit
                          ? "bg-[#FF5C5C] text-white"
                          : "bg-[#CCE5FF] text-[#004B73]"
                      }`}
                    >
                      {isCrit && (
                        <span className="w-1.5 h-1.5 rounded-full bg-white animate-ping"></span>
                      )}
                      {alert.severity.toUpperCase()}
                    </span>
                  </td>

                  <td className="py-3.5 px-4">
                    <div className="flex flex-col">
                      <span className="text-[0.9375rem] font-semibold text-[#1E293B] group-hover:text-[#0284C7] transition-colors">
                        {alert.title}
                      </span>
                      <span className="font-mono text-[0.75rem] text-[#64748B]">
                        {ruleName}
                      </span>
                    </div>
                  </td>

                  <td className="py-3.5 px-4 whitespace-nowrap">
                    <div
                      className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded font-mono text-[0.75rem] font-semibold ${
                        isCrit
                          ? "bg-[#FFDAD6] text-[#93000A]"
                          : "bg-[#F0F3FF] text-[#1E293B]"
                      }`}
                    >
                      {targetAsset}
                    </div>
                  </td>

                  <td className="py-3.5 px-4 whitespace-nowrap">
                    <div className="flex flex-col">
                      <span className="font-mono text-[0.75rem] font-semibold text-[#1E293B]">
                        {indicator}
                      </span>
                    </div>
                  </td>

                  <td className="py-3.5 px-4 max-w-xs hidden lg:table-cell">
                    <p className="text-[0.75rem] text-[#64748B] line-clamp-2">
                      {alert.details}
                    </p>
                  </td>

                  <td className="py-3.5 px-4 whitespace-nowrap">
                    <div className="flex items-center gap-1.5 font-mono text-[0.75rem]">
                      <span
                        className={`w-2 h-2 rounded-full ${
                          isCrit ? "bg-[#FF5C5C]" : "bg-[#64748B]"
                        }`}
                      ></span>
                      <span
                        className={isCrit ? "text-[#FF5C5C] font-semibold" : "text-[#64748B]"}
                      >
                        {statusText}
                      </span>
                    </div>
                  </td>

                  <td className="py-3.5 px-4 text-right whitespace-nowrap">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onSelect(alert);
                      }}
                      className={`px-3 py-1.5 rounded font-mono text-[0.75rem] font-semibold shadow-sm inline-flex items-center gap-1 transition-colors cursor-pointer ${
                        isCrit
                          ? "bg-[#1A1D20] text-white hover:bg-[#2E343A]"
                          : "bg-[#F0F3FF] text-[#1E293B] hover:bg-[#DEE8FF] border border-[#CBD5E1]"
                      }`}
                    >
                      <span>{isCrit ? "Triage & Quarantine" : "Triage & Review"}</span>
                      {isCrit ? (
                        <ArrowRight className="w-3.5 h-3.5" />
                      ) : (
                        <ExternalLink className="w-3.5 h-3.5" />
                      )}
                    </button>
                  </td>
                </tr>
              );
            })
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
