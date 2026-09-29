"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import { AppShell } from "@/components/layout/AppShell";
import { CommandRibbon } from "@/components/dashboard/CommandRibbon";
import { KpiTelemetryCards } from "@/components/dashboard/KpiTelemetryCards";
import { PerformanceChart } from "@/components/dashboard/PerformanceChart";
import { RestContractInspector } from "@/components/dashboard/RestContractInspector";
import { AlertFeed } from "@/components/dashboard/AlertFeed";
import { OcsfStreamTable } from "@/components/dashboard/OcsfStreamTable";
import { TutorialOverlay } from "@/components/tutorial/TutorialOverlay";
import { getMetrics, getAlerts, getBlockRecords, getApiMode } from "@/lib/api";
import {
  MetricsResponse,
  AlertItem,
  StoredRecordItem,
  TimeSeriesPoint,
} from "@/lib/types";
import { mockMetrics, mockAlerts, mockRecords } from "@/lib/mock-data";

export type DashboardStatus = "LIVE" | "MOCK" | "OFFLINE" | "STALE";

export default function AnalystDashboardPage() {
  const [metrics, setMetrics] = useState<MetricsResponse | null>(null);
  const [alerts, setAlerts] = useState<AlertItem[] | null>(null);
  const [records, setRecords] = useState<StoredRecordItem[]>([]);
  const [status, setStatus] = useState<DashboardStatus>("OFFLINE");
  const [isPolling] = useState<boolean>(true);
  const [showJsonInspector, setShowJsonInspector] = useState<boolean>(true);

  // Time series buffer for live SVG performance chart
  const [history, setHistory] = useState<TimeSeriesPoint[]>([]);

  // Buffer of stored canonical frames for continuous demo looping on EC2
  const streamBufferRef = useRef<StoredRecordItem[]>([]);
  const loopIndexRef = useRef<number>(0);
  const [demoLoopActive] = useState<boolean>(true);

  const inFlightRef = useRef<boolean>(false);
  const latestPollIdRef = useRef<number>(0);

  const pollData = useCallback(async () => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    const pollId = ++latestPollIdRef.current;
    const mode = getApiMode();

    if (mode === "MOCK") {
      setMetrics(mockMetrics);
      setAlerts(mockAlerts);
      setRecords(mockRecords);
      streamBufferRef.current = mockRecords;
      setStatus("MOCK");
      setHistory((prev) =>
        prev.length >= 2
          ? prev
          : [
              { time: "-60s", eps: 141000, latency_p50: 1.35 },
              { time: "-50s", eps: 141500, latency_p50: 1.3 },
              { time: "-40s", eps: 142000, latency_p50: 1.34 },
              { time: "-30s", eps: 141800, latency_p50: 1.28 },
              { time: "-20s", eps: 142200, latency_p50: 1.26 },
              { time: "-10s", eps: 142400, latency_p50: 1.29 },
              { time: "0s", eps: 142500, latency_p50: 1.28 },
            ]
      );
      inFlightRef.current = false;
      return;
    }

    // LIVE mode: serialized poll across all endpoints
    try {
      const metricsRes = await getMetrics();
      const totalBlocks = metricsRes.data.total_blocks ?? 0;
      // Target the newest block dynamically or block 1
      const targetBlock = totalBlocks > 0 ? (totalBlocks > 1 ? totalBlocks - 1 : 1) : 1;

      const [alertsRes, recordsRes] = await Promise.allSettled([
        getAlerts(),
        getBlockRecords(targetBlock, { limit: 50 }),
      ]);

      // If a newer poll was initiated, do not overwrite state with stale results
      if (pollId !== latestPollIdRef.current) return;

      setMetrics(metricsRes.data);
      if (alertsRes.status === "fulfilled") {
        setAlerts(alertsRes.value.data);
      }
      if (
        recordsRes.status === "fulfilled" &&
        recordsRes.value.data &&
        recordsRes.value.data.length > 0
      ) {
        const newRecords = recordsRes.value.data;
        const existingIds = new Set(streamBufferRef.current.map((r) => r.event_id));
        const merged = [...streamBufferRef.current];
        for (const rec of newRecords) {
          if (!existingIds.has(rec.event_id)) {
            merged.push(rec);
            existingIds.add(rec.event_id);
          }
        }
        streamBufferRef.current = merged.slice(-100);

        // Seed visible records if currently empty
        setRecords((prev) => (prev.length === 0 ? newRecords.slice(0, 10) : prev));
      }
      setStatus("LIVE");

      // Append new time series point from actual response
      const currentEps =
        typeof metricsRes.data.eps === "number" && metricsRes.data.eps > 0
          ? metricsRes.data.eps
          : (history.length > 0 ? history[history.length - 1].eps : 142500);
      const currentLat =
        typeof metricsRes.data.latency_p50_micros === "number" && metricsRes.data.latency_p50_micros > 0
          ? metricsRes.data.latency_p50_micros
          : (history.length > 0 ? history[history.length - 1].latency_p50 : 1.28);

      const newPoint: TimeSeriesPoint = {
        time: new Date().toLocaleTimeString().slice(-5),
        eps: currentEps,
        latency_p50: currentLat,
      };

      setHistory((prev) => {
        const next = [...prev.slice(-15), newPoint];
        return next;
      });
    } catch {
      // Backend failed or unreachable in LIVE mode
      if (pollId !== latestPollIdRef.current) return;

      // Do NOT fall back to mock fixtures!
      // Clear displayed metrics, alerts, and records when LIVE request fails
      setStatus("OFFLINE");
      setMetrics(null);
      setAlerts(null);
      setRecords([]);
      streamBufferRef.current = [];
    } finally {
      inFlightRef.current = false;
    }
  }, [history]);

  // Serialized polling requirement:
  // poll -> await all required requests -> update state -> wait 1 second -> poll again
  useEffect(() => {
    let active = true;
    let timerId: NodeJS.Timeout | null = null;

    async function pollLoop() {
      if (!active) return;
      await pollData();
      if (!active) return;
      if (isPolling) {
        timerId = setTimeout(pollLoop, 1000);
      }
    }

    pollLoop();

    return () => {
      active = false;
      if (timerId) {
        clearTimeout(timerId);
      }
    };
  }, [pollData, isPolling]);

  // Respond immediately to mode changes (LIVE <-> MOCK toggle)
  useEffect(() => {
    const handleModeChange = () => {
      latestPollIdRef.current += 1;
      const mode = getApiMode();
      if (mode === "LIVE") {
        // When switching to LIVE, clear state immediately until next successful poll
        setMetrics(null);
        setAlerts(null);
        setRecords([]);
        setHistory([]);
        setStatus("OFFLINE");
      } else {
        setMetrics(mockMetrics);
        setAlerts(mockAlerts);
        setRecords(mockRecords);
        setStatus("MOCK");
        setHistory((prev) =>
          prev.length >= 2
            ? prev
            : [
                { time: "-60s", eps: 141000, latency_p50: 1.35 },
                { time: "-50s", eps: 141500, latency_p50: 1.3 },
                { time: "-40s", eps: 142000, latency_p50: 1.34 },
                { time: "-30s", eps: 141800, latency_p50: 1.28 },
                { time: "-20s", eps: 142200, latency_p50: 1.26 },
                { time: "-10s", eps: 142400, latency_p50: 1.29 },
                { time: "0s", eps: 142500, latency_p50: 1.28 },
              ]
        );
      }
      pollData();
    };

    window.addEventListener("ulpf_api_mode_change", handleModeChange);
    return () => {
      window.removeEventListener("ulpf_api_mode_change", handleModeChange);
    };
  }, [pollData]);

  // Continuous Demo Looping Ticker:
  // Designed specifically for continuous presentation on EC2.
  // Every 1.5s, cycle to the next record in the stream buffer,
  // prepend it to displayed records with an updated timestamp,
  // and smoothly wrap around to index 0 when reaching the end.
  useEffect(() => {
    if (!demoLoopActive || status === "OFFLINE") return;

    const intervalId = setInterval(() => {
      const buffer = streamBufferRef.current;
      if (buffer.length === 0) return;

      const idx = loopIndexRef.current % buffer.length;
      loopIndexRef.current = (loopIndexRef.current + 1) % buffer.length;

      const base = buffer[idx];
      const liveFrame: StoredRecordItem = {
        ...base,
        timestamp: Date.now(),
      };

      setRecords((prev) => {
        const next = [liveFrame, ...prev.filter((r) => r.event_id !== liveFrame.event_id).slice(0, 9)];
        return next;
      });
    }, 1500);

    return () => clearInterval(intervalId);
  }, [demoLoopActive, status]);

  const isLive = status === "LIVE";
  const epsHistory = history.map((h) => h.eps);

  return (
    <AppShell currentSection="LIVE_STREAM" eps={metrics?.eps}>
      <div className="flex flex-col w-full gap-5">
        {/* Top Command & Telemetry Ribbon */}
        <CommandRibbon
          onSync={pollData}
          isPolling={isPolling}
          isLive={isLive}
          status={status}
        />

        {/* 4 KPI Telemetry Cards */}
        <KpiTelemetryCards
          metrics={metrics}
          epsHistory={epsHistory}
          status={status}
        />

        {/* Primary Visual Panel: Live Stream Latency & Throughput Profile + REST Inspector */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-5">
          <div className={showJsonInspector ? "lg:col-span-8" : "lg:col-span-12"}>
            <PerformanceChart
              history={history}
              onToggleJson={() => setShowJsonInspector((prev) => !prev)}
              showJson={showJsonInspector}
              status={status}
            />
          </div>

          {showJsonInspector && (
            <div className="lg:col-span-4">
              <RestContractInspector metrics={metrics} status={status} />
            </div>
          )}
        </div>

        {/* Active Security Alerts Feed (GET /alerts) */}
        <AlertFeed alerts={alerts} onRefresh={pollData} status={status} />

        {/* Real-Time Ingested Events Stream Table (OCSF Canonical) */}
        <OcsfStreamTable
          records={records}
          status={status}
          demoLoopActive={demoLoopActive}
        />

        {/* Interactive Guided UI Tutorial */}
        <TutorialOverlay />
      </div>
    </AppShell>
  );
}
