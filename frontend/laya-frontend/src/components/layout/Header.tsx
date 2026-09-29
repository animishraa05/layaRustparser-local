"use client";

import { useEffect, useState, useSyncExternalStore } from "react";
import { Search, Bell, User, RefreshCw, Sparkles } from "lucide-react";
import { startTutorialTour } from "@/components/tutorial/TutorialOverlay";
import {
  getApiMode,
  setApiMode,
  checkBackendReachable,
  isBackendOnline,
  subscribeBackendStatus,
} from "@/lib/api";

interface HeaderProps {
  currentSection?: string;
  eps?: number | null;
}

export function Header({
  currentSection = "LIVE_OVERVIEW",
  eps,
}: HeaderProps) {
  const mode = useSyncExternalStore(
    subscribeBackendStatus,
    () => getApiMode(),
    () => "LIVE"
  );

  const backendOnline = useSyncExternalStore(
    subscribeBackendStatus,
    () => isBackendOnline(),
    () => null
  );

  const [timeRange, setTimeRange] = useState<"15m" | "1h" | "24h">("1h");
  const mounted = useSyncExternalStore(
    () => () => { },
    () => true,
    () => false
  );

  useEffect(() => {
    if (!mounted) return;

    let active = true;
    let timerId: ReturnType<typeof setTimeout> | null = null;

    async function probeLoop() {
      if (!active) return;

      try {
        await checkBackendReachable();
      } catch {
        // Handled in checkBackendReachable
      }

      if (!active) return;

      timerId = setTimeout(probeLoop, 3000);
    }

    probeLoop();

    return () => {
      active = false;

      if (timerId !== null) {
        clearTimeout(timerId);
      }
    };
  }, [mounted]);

  /*
   * During SSR and the initial hydration render:
   * - do not claim LIVE
   * - do not claim MOCK
   * - do not show real EPS
   *
   * After mount, the actual API mode + backend reachability determine
   * the displayed state.
   */
  const displayMode = mounted ? mode : "LIVE";

  const isLiveActive =
    mounted &&
    displayMode === "LIVE" &&
    (eps != null || backendOnline === true);

  const isMock = mounted && displayMode === "MOCK";


  const toggleMode = () => {
    const next = mode === "LIVE" ? "MOCK" : "LIVE";
    setApiMode(next);
  };

  return (
    <header
      data-tour="header"
      className="fixed top-0 left-[250px] right-0 h-16 bg-white/95 backdrop-blur-md border-b border-[#E2E8F0] z-40 px-6 flex items-center justify-between"
    >
      {/* Left: Breadcrumbs & Backend Status */}
      <div className="flex items-center gap-4">
        <nav className="flex items-center gap-1.5 text-[#64748B] font-mono text-[0.75rem] uppercase tracking-wider">
          <span className="hover:text-[#1E293B] transition-colors cursor-pointer">
            SOC
          </span>
          <span>/</span>
          <span className="hover:text-[#1E293B] transition-colors cursor-pointer">
            ULPF_CORE
          </span>
          <span>/</span>
          <span className="text-[#1E293B] font-semibold">
            {currentSection}
          </span>
        </nav>

        <div className="hidden xl:flex items-center gap-2 px-2.5 py-1 rounded bg-[#F0F3FF] border border-[#C5C6CA]/30">
          <span
            className={`w-2 h-2 rounded-full ${isLiveActive
              ? "bg-[#10B981] animate-pulse"
              : isMock
                ? "bg-sky-500"
                : "bg-amber-500"
              }`}
          />

          <span className="font-mono text-[0.75rem] text-[#1E293B] font-medium">
            {!mounted
              ? "BACKEND OFFLINE"
              : isMock
                ? "MOCK MODE"
                : isLiveActive
                  ? `INGESTING: ${(eps ?? 0).toLocaleString("en-US")} EPS`
                  : "BACKEND OFFLINE"}
          </span>
        </div>
      </div>

      {/* Right: Actions, Filters, Mode & User */}
      <div className="flex items-center gap-3">
        {/* Interactive Tutorial Walkthrough Button */}
        <button
          onClick={startTutorialTour}
          title="Start interactive dashboard walkthrough"
          className="flex items-center gap-1.5 px-3 py-1 rounded text-[0.75rem] font-mono font-semibold transition-all border border-[#0284C7]/30 bg-gradient-to-r from-[#0284C7]/10 via-[#38BDF8]/10 to-[#0284C7]/10 text-[#0284C7] hover:bg-[#0284C7]/20 shadow-sm cursor-pointer"
        >
          <Sparkles className="w-3.5 h-3.5 text-[#0284C7] animate-pulse" />
          <span>Tutorial</span>
        </button>

        {/* Live vs Mock Mode Switcher */}
        <button
          onClick={toggleMode}
          title={`Click to switch to ${mode === "LIVE" ? "MOCK" : "LIVE"
            } mode`}
          className={`flex items-center gap-1.5 px-2.5 py-1 rounded text-[0.75rem] font-mono font-semibold transition-all border shadow-sm ${!mounted
            ? "bg-amber-50 text-amber-800 border-amber-300"
            : mode === "LIVE"
              ? isLiveActive
                ? "bg-emerald-50 text-emerald-700 border-emerald-300 hover:bg-emerald-100"
                : "bg-amber-50 text-amber-800 border-amber-300 hover:bg-amber-100"
              : "bg-sky-50 text-sky-700 border-sky-300 hover:bg-sky-100"
            }`}
        >
          <span
            className={`w-2 h-2 rounded-full ${!mounted
              ? "bg-amber-500"
              : mode === "LIVE"
                ? isLiveActive
                  ? "bg-emerald-500 animate-pulse"
                  : "bg-amber-500"
                : "bg-sky-500"
              }`}
          />

          <span>
            {!mounted
              ? "CONNECTING..."
              : mode === "LIVE"
                ? isLiveActive
                  ? "LIVE API"
                  : "OFFLINE"
                : "MOCK"}
          </span>

          <RefreshCw className="w-3 h-3 ml-0.5 opacity-60" />
        </button>

        {/* Time Filter */}
        <div className="hidden md:flex items-center bg-[#F0F3FF] border border-[#C5C6CA]/40 rounded px-1 py-0.5">
          {(["15m", "1h", "24h"] as const).map((t) => (
            <button
              key={t}
              onClick={() => setTimeRange(t)}
              className={`px-2 py-0.5 font-mono text-[0.75rem] rounded transition-colors ${timeRange === t
                ? "bg-white text-[#1E293B] font-semibold shadow-[0_1px_2px_rgba(0,0,0,0.04)]"
                : "text-[#64748B] hover:text-[#1E293B]"
                }`}
            >
              {t}
            </button>
          ))}
        </div>

        {/* Search Input */}
        <div className="relative hidden lg:block w-52">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-[#64748B]" />

          <input
            type="text"
            placeholder="Filter stream (regex, OCSF)..."
            className="w-full h-8 pl-8 pr-2.5 bg-[#F0F3FF] border border-[#C5C6CA]/40 rounded text-[#1E293B] text-[0.75rem] placeholder:text-[#64748B]/70 focus:outline-none focus:border-[#0284C7]"
          />
        </div>

        {/* Notifications */}
        <div className="relative flex items-center justify-center p-1.5 rounded text-[#64748B] hover:text-[#1E293B] hover:bg-[#E2E4E8] cursor-pointer">
          <Bell className="w-5 h-5" />

          <span className="absolute top-1 right-1 w-2 h-2 bg-[#FF5C5C] rounded-full ring-2 ring-white" />
        </div>

        {/* User Avatar */}
        <div className="w-8 h-8 rounded-full bg-[#1A1D20] text-white flex items-center justify-center font-medium shadow-sm">
          <User className="w-4 h-4 text-white" />
        </div>
      </div>
    </header>
  );
}