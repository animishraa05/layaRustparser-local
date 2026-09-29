"use client";

import { useState } from "react";
import Link from "next/link";
import { AlertItem, AlertSeverity } from "@/lib/types";
import { AlertTriangle, RefreshCw, ArrowRight } from "lucide-react";

interface AlertFeedProps {
  alerts: AlertItem[] | null;
  onRefresh?: () => void;
  status?: "LIVE" | "MOCK" | "OFFLINE" | "STALE";
}

export function AlertFeed({ alerts, onRefresh, status = "LIVE" }: AlertFeedProps) {
  const [filterSeverity, setFilterSeverity] = useState<AlertSeverity | "ALL">("ALL");

  const isOffline = alerts === null || status === "OFFLINE";
  const alertList = alerts ?? [];

  const filteredAlerts =
    filterSeverity === "ALL"
      ? alertList
      : alertList.filter((a) => a.severity.toLowerCase() === filterSeverity.toLowerCase());

  return (
    <div
      data-tour="alert-feed"
      className="flex flex-col bg-white p-4 rounded-xl border border-[#E2E8F0] shadow-sm"
    >
      {/* Header */}
      <div className="flex flex-wrap items-center justify-between gap-3 pb-3 border-b border-[#F1F5F9]">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-[#FFDAD6] text-[#93000A] flex items-center justify-center shadow-sm">
            <AlertTriangle className="w-5 h-5 text-[#BA1A1A]" />
          </div>
          <div className="flex flex-col">
            <div className="flex items-center gap-2">
              <span className="text-[1.125rem] text-[#1E293B] font-semibold">
                Active Security Alerts Feed
              </span>
              {isOffline ? (
                <span className="font-mono text-[0.6875rem] px-2 py-0.5 rounded-full bg-amber-50 text-amber-800 font-semibold border border-amber-300">
                  FEED OFFLINE
                </span>
              ) : (
                <span className="font-mono text-[0.6875rem] px-2 py-0.5 rounded-full bg-[#BA1A1A] text-white font-semibold">
                  {alertList.length} In Flight
                </span>
              )}
            </div>
            <span className="font-mono text-[0.75rem] text-[#64748B]">
              Mapped contract: Endpoint GET /alerts (Evaluation Cycle: 100ms)
            </span>
          </div>
        </div>

        {/* Controls */}
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1 bg-[#F0F3FF] p-1 rounded-lg border border-[#CBD5E1]/40">
            {(["ALL", "Critical", "Medium"] as const).map((sev) => (
              <button
                key={sev}
                onClick={() => setFilterSeverity(sev)}
                className={`px-2.5 py-1 rounded text-[0.75rem] font-mono transition-colors ${filterSeverity === sev
                    ? "bg-white text-[#1E293B] font-semibold shadow-sm"
                    : "text-[#64748B] hover:text-[#1E293B]"
                  }`}
              >
                {sev}
              </button>
            ))}
          </div>

          {onRefresh && (
            <button
              onClick={onRefresh}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#1A1D20] text-white hover:bg-[#2E343A] font-mono text-[0.75rem] transition-colors cursor-pointer shadow-sm"
            >
              <RefreshCw className="w-3.5 h-3.5" />
              <span>Trigger Poll</span>
            </button>
          )}
        </div>
      </div>

      {/* Alert Items Container */}
      <div className="flex flex-col gap-3 mt-3">
        {isOffline ? (
          <div className="p-8 text-center text-[#64748B] font-mono text-[0.875rem] bg-[#F8FAFC] rounded-lg border border-dashed border-[#CBD5E1]">
            <p className="text-amber-800 font-semibold mb-1">Alerts unavailable</p>
            <p className="text-[#64748B] text-[0.75rem]">Backend is not responding on /alerts</p>
          </div>
        ) : filteredAlerts.length === 0 ? (
          <div className="p-8 text-center text-[#64748B] font-mono text-[0.875rem] bg-[#F8FAFC] rounded-lg border border-dashed border-[#CBD5E1]">
            No alerts detected matching current filter criteria.
          </div>
        ) : (
          filteredAlerts.map((alert) => {
            const isCritical =
              alert.severity === "Critical" || alert.severity === "High";
            const isMedium = alert.severity === "Medium";

            return (
              <div
                key={alert.id}
                className={`flex flex-col xl:flex-row xl:items-center justify-between gap-4 p-4 rounded-xl border transition-colors ${isCritical
                    ? "bg-[#FFDAD6]/20 hover:bg-[#FFDAD6]/30 border-[#FECACA]"
                    : isMedium
                      ? "bg-[#F0F3FF]/60 hover:bg-[#F0F3FF]/80 border-[#CBD5E1]"
                      : "bg-[#F8FAFC] hover:bg-[#F1F5F9] border-[#E2E8F0]"
                  }`}
              >
                <div className="flex items-start gap-3">
                  <span
                    className={`inline-flex items-center px-2.5 py-1 rounded font-mono text-[0.6875rem] font-bold uppercase tracking-wider shrink-0 mt-0.5 ${isCritical
                        ? "bg-[#BA1A1A] text-white"
                        : isMedium
                          ? "bg-[#CCE5FF] text-[#004B73]"
                          : "bg-[#E2E4E8] text-[#1E293B]"
                      }`}
                  >
                    {alert.severity}
                  </span>

                  <div className="flex flex-col gap-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="text-[1rem] text-[#1E293B] font-semibold">
                        {alert.title}
                      </span>
                      {alert.block_id !== undefined && (
                        <span
                          className={`font-mono text-[0.75rem] font-medium ${isCritical ? "text-[#BA1A1A]" : "text-[#006398]"
                            }`}
                        >
                          [Block #{String(alert.block_id).padStart(5, "0")}, Leaf{" "}
                          {alert.leaf_index ?? 0}]
                        </span>
                      )}
                    </div>
                    <p className="text-[0.875rem] text-[#44474A] max-w-4xl leading-normal">
                      {alert.details}
                    </p>
                    <div className="flex flex-wrap items-center gap-3 pt-1 text-[#64748B] font-mono text-[0.75rem]">
                      <span className="text-[#1E293B] font-medium">
                        Source: {alert.alert_type}
                      </span>
                      <span>•</span>
                      <span suppressHydrationWarning>
                        Time: {new Date(alert.timestamp).toLocaleTimeString("en-US")}
                      </span>
                      <span>•</span>
                      <span
                        className={
                          isCritical
                            ? "text-[#BA1A1A] font-semibold"
                            : "text-[#009768] font-medium"
                        }
                      >
                        {isCritical
                          ? "Tainted State Identified"
                          : "Drain3 Auto-Clustered"}
                      </span>
                    </div>
                  </div>
                </div>

                <div className="flex flex-wrap items-center gap-2 shrink-0">
                  <Link
                    href={`/siem-alerting?selected=${alert.id}`}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#1A1D20] text-white hover:bg-[#2E343A] font-mono text-[0.75rem] font-semibold transition-colors cursor-pointer shadow-sm"
                  >
                    <span>Triage Incident</span>
                    <ArrowRight className="w-3.5 h-3.5" />
                  </Link>

                  {isCritical ? (
                    <button
                      onClick={() =>
                        window.alert(
                          `Quarantine action dispatched for Block #${alert.block_id ?? 0}.`
                        )
                      }
                      className="px-3 py-1.5 rounded-lg bg-[#BA1A1A] text-white hover:bg-[#93000A] font-mono text-[0.75rem] font-semibold transition-colors cursor-pointer shadow-sm"
                    >
                      Quarantine Block
                    </button>
                  ) : (
                    <Link
                      href="/siem-alerting"
                      className="px-3 py-1.5 rounded-lg bg-white border border-[#CBD5E1] text-[#006398] hover:bg-[#F0F3FF] font-mono text-[0.75rem] font-semibold transition-colors cursor-pointer shadow-sm"
                    >
                      Review in SIEM
                    </Link>
                  )}
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
