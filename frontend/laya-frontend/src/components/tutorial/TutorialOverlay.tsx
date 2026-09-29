"use client";

import React, { useEffect, useState, useCallback, useRef } from "react";
import {
  Sparkles,
  ChevronRight,
  ChevronLeft,
  X,
  Radio,
  CheckCircle2,
  Terminal,
  ShieldAlert,
  Gauge,
  Activity,
  Layers,
  Compass,
} from "lucide-react";

export interface TutorialStep {
  id: string;
  targetSelector: string;
  title: string;
  subtitle: string;
  category: string;
  icon: typeof Sparkles;
  details: string[];
  preferredPlacement?: "bottom" | "top" | "left" | "right";
}

const TUTORIAL_STEPS: TutorialStep[] = [
  {
    id: "header",
    targetSelector: '[data-tour="header"]',
    title: "SOC Command Bar & Live Ingestion Status",
    subtitle: "Wire connectivity, EPS telemetry, and dual API operation",
    category: "INGESTION PLANE",
    icon: Activity,
    details: [
      "Real-Time Telemetry: Continuously monitors packet intake across UDP and TCP multi-threaded socket pools on port 5140.",
      "Dual-Mode Engine: Seamlessly toggle between LIVE mode (polling the out-of-band Axum REST API on port 8080) and MOCK test fixtures.",
      "Time Window Aggregator: Switch between 15m, 1h, and 24h rolling analysis frames.",
    ],
    preferredPlacement: "bottom",
  },
  {
    id: "command-ribbon",
    targetSelector: '[data-tour="command-ribbon"]',
    title: "Tactical Command & Evaluation Ribbon",
    subtitle: "Rapid operational triggers and forensic demonstration tools",
    category: "CONTROL PLANE",
    icon: Compass,
    details: [
      "Live Traffic Generator: Blast 10,000 to 50,000 multi-vendor Syslog packets on demand directly into the ingestion engine.",
      "Adversarial Tamper Simulation: Stealthily alter archived bytes to demonstrate instantaneous RFC 6962 Merkle tree tamper detection.",
      "1-Click Regex Onboarding: Synthesize strict regex parsers from 3-5 sample lines for unknown firewall appliances without restarting.",
    ],
    preferredPlacement: "bottom",
  },
  {
    id: "kpi-cards",
    targetSelector: '[data-tour="kpi-cards"]',
    title: "High-Assurance Data Plane KPIs",
    subtitle: "Microsecond latencies, throughput rates, and cache efficiency",
    category: "CORE PERFORMANCE",
    icon: Gauge,
    details: [
      "Throughput (EPS): Line-rate multi-threaded ingestion capable of absorbing > 1.70M EPS in benchmark mode.",
      "Median Latency (p50): Sub-microsecond processing (~1.28 µs) powered by zero-copy byte slicing directly from packet buffers.",
      "RingBuffer Depth: Non-blocking bounded channel preventing packet drops or hot-path degradation during traffic surges.",
      "Tier-1 LRU Hit Rate: Lock-free 64-bit structural signature cache achieving > 96% direct fast-path classification.",
    ],
    preferredPlacement: "bottom",
  },
  {
    id: "performance-chart",
    targetSelector: '[data-tour="performance-chart"]',
    title: "Real-Time Ingestion & Latency Profile",
    subtitle: "Dual-axis SVG telemetry tracking microsecond performance",
    category: "TELEMETRY",
    icon: Layers,
    details: [
      "Blue Area Curve: Visualizes dynamic throughput spikes as high-volume syslog bursts arrive from perimeter firewalls.",
      "Latency Curves: Real-time tracking of median (p50) and tail (p99) microsecond processing times.",
      "Polled directly from the sovereign Rust backend via GET /metrics with zero third-party telemetry.",
    ],
    preferredPlacement: "bottom",
  },
  {
    id: "alert-feed",
    targetSelector: '[data-tour="alert-feed"]',
    title: "Security & Forensic Tamper Alert Feed",
    subtitle: "Immediate cryptographic alarms and pattern drift detection",
    category: "FORENSIC INTEGRITY",
    icon: ShieldAlert,
    details: [
      "Critical Tamper Alarms: Mathematically proves bit-flip tampering at exact Parquet leaf indices using RFC 6962 Merkle audits.",
      "Parser Drift Alarms: Fixed-depth (d=4) DrainMiner prefix tree flags novel structural evasion patterns or zero-day log formats.",
      "Action Inviolability: Deterministic anchor tokens enforce that ALLOW and DENY actions are never merged.",
    ],
    preferredPlacement: "bottom",
  },
  {
    id: "ocsf-table",
    targetSelector: '[data-tour="ocsf-table"]',
    title: "Canonical OCSF 1.3 Event Ingestion Ledger",
    subtitle: "Monotonically ordered lossless evidence records",
    category: "OCSF 1.3 NORMALIZATION",
    icon: Radio,
    details: [
      "Class UID 4001 NetworkActivity: Normalizes Cisco ASA, FortiGate, Palo Alto, Suricata, and pfSense into standardized schemas.",
      "Monotonic UUIDv7: Guaranteed chronological order across distributed ingestion threads.",
      "Lossless Forensic Provenance: 100% complete raw syslog preserved byte-for-byte alongside its SHA-256 raw_hash for legal admissibility.",
    ],
    preferredPlacement: "top",
  },
  {
    id: "contract-inspector",
    targetSelector: '[data-tour="contract-inspector"]',
    title: "Wire Protocol & REST Contract Inspector",
    subtitle: "Live transparent inspection of sovereign backend APIs",
    category: "PROTOCOL AUDIT",
    icon: Terminal,
    details: [
      "Direct Inspection: Live viewer displaying raw JSON emitted by the native Axum HTTP backend on port 8080.",
      "Air-Gapped Guarantee: Validates that the system makes zero external internet calls or cloud telemetry pings.",
      "Copyable CLI commands for immediate terminal-based debugging.",
    ],
    preferredPlacement: "top",
  },
  {
    id: "sidebar-nav",
    targetSelector: '[data-tour="sidebar-nav"]',
    title: "Navigation Hub & Specialized Modules",
    subtitle: "Multi-page forensic investigation and analysis suite",
    category: "NAVIGATION",
    icon: Sparkles,
    details: [
      "Threat Intelligence: Deep-dive multi-vendor distribution and disposition breakdown (Allowed vs Blocked).",
      "SIEM Alerts: Dedicated incident queue and courtroom evidentiary bundle export.",
      "Query Explorer: SQL console over columnar Apache Parquet files with live RFC 6962 inclusion proof visualizers.",
    ],
    preferredPlacement: "right",
  },
];

export function startTutorialTour() {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("ulpf:start-tutorial"));
  }
}

export function TutorialOverlay() {
  const [isOpen, setIsOpen] = useState(false);
  const [currentStep, setCurrentStep] = useState(0);
  const [targetRect, setTargetRect] = useState<DOMRect | null>(null);
  const [tooltipPos, setTooltipPos] = useState<{ top: number; left: number }>({
    top: 0,
    left: 0,
  });
  const tooltipRef = useRef<HTMLDivElement>(null);

  const activeStep = TUTORIAL_STEPS[currentStep];

  const updateTargetPosition = useCallback(() => {
    if (!activeStep) return;
    const el = document.querySelector(activeStep.targetSelector);
    if (el) {
      const rect = el.getBoundingClientRect();
      setTargetRect(rect);

      // Calculate tooltip position
      const padding = 16;
      const tooltipWidth = 440;
      const tooltipHeight = 340;

      let top = 0;
      let left = 0;

      // Position logic based on preferred placement and screen boundaries
      if (activeStep.preferredPlacement === "right") {
        left = rect.right + padding;
        top = Math.max(
          padding,
          Math.min(
            rect.top + rect.height / 2 - tooltipHeight / 2,
            window.innerHeight - tooltipHeight - padding
          )
        );
      } else if (activeStep.preferredPlacement === "top") {
        top = Math.max(padding, rect.top - tooltipHeight - padding);
        left = Math.max(
          padding,
          Math.min(
            rect.left + rect.width / 2 - tooltipWidth / 2,
            window.innerWidth - tooltipWidth - padding
          )
        );
      } else {
        // Default "bottom"
        top = rect.bottom + padding;
        if (top + tooltipHeight > window.innerHeight) {
          top = Math.max(padding, rect.top - tooltipHeight - padding);
        }
        left = Math.max(
          padding,
          Math.min(
            rect.left + rect.width / 2 - tooltipWidth / 2,
            window.innerWidth - tooltipWidth - padding
          )
        );
      }

      // Ensure tooltip does not overflow viewport horizontally
      left = Math.max(padding, Math.min(left, window.innerWidth - tooltipWidth - padding));

      setTooltipPos({ top, left });
    }
  }, [activeStep]);

  // Scroll into view on step change
  useEffect(() => {
    if (!isOpen || !activeStep) return;

    const el = document.querySelector(activeStep.targetSelector);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
      const timer = setTimeout(updateTargetPosition, 300);
      return () => clearTimeout(timer);
    }
  }, [isOpen, currentStep, activeStep, updateTargetPosition]);

  // Handle window resize and scroll
  useEffect(() => {
    if (!isOpen) return;

    const handleUpdate = () => {
      updateTargetPosition();
    };

    window.addEventListener("resize", handleUpdate);
    window.addEventListener("scroll", handleUpdate, true);

    return () => {
      window.removeEventListener("resize", handleUpdate);
      window.removeEventListener("scroll", handleUpdate, true);
    };
  }, [isOpen, updateTargetPosition]);

  // Keyboard navigation
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setIsOpen(false);
      } else if (e.key === "ArrowRight" || e.key === "Enter") {
        if (currentStep < TUTORIAL_STEPS.length - 1) {
          setCurrentStep((prev) => prev + 1);
        } else {
          setIsOpen(false);
        }
      } else if (e.key === "ArrowLeft") {
        if (currentStep > 0) {
          setCurrentStep((prev) => prev - 1);
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, currentStep]);

  // Listen for custom trigger event
  useEffect(() => {
    const handleStart = () => {
      setCurrentStep(0);
      setIsOpen(true);
    };

    window.addEventListener("ulpf:start-tutorial", handleStart);
    return () => window.removeEventListener("ulpf:start-tutorial", handleStart);
  }, []);

  if (!isOpen || !activeStep || !targetRect) return null;

  const progressPercent = ((currentStep + 1) / TUTORIAL_STEPS.length) * 100;
  const StepIcon = activeStep.icon;

  return (
    <div className="fixed inset-0 z-[100] pointer-events-auto select-none">
      {/* Spotlight cutout element with immense box-shadow to dim & blur the surrounding UI */}
      <div
        style={{
          top: targetRect.top - 6,
          left: targetRect.left - 6,
          width: targetRect.width + 12,
          height: targetRect.height + 12,
          boxShadow: "0 0 0 9999px rgba(15, 23, 42, 0.72)",
        }}
        className="fixed rounded-xl ring-4 ring-[#0284C7] ring-offset-2 ring-offset-slate-900 pointer-events-none transition-all duration-300 z-[101] shadow-[0_0_50px_rgba(2,132,199,0.6)]"
      />

      {/* Floating Interactive Tooltip Card */}
      <div
        ref={tooltipRef}
        style={{
          top: tooltipPos.top,
          left: tooltipPos.left,
          width: 440,
        }}
        className="fixed z-[102] bg-[#0F172A] border border-[#334155] rounded-2xl shadow-[0_25px_60px_rgba(0,0,0,0.6)] text-white p-5 flex flex-col gap-4 animate-in fade-in zoom-in-95 duration-200"
      >
        {/* Progress Bar & Header */}
        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between text-[0.6875rem] font-mono tracking-wider text-[#94A3B8]">
            <span className="flex items-center gap-1.5 text-[#38BDF8] font-semibold uppercase">
              <StepIcon className="w-3.5 h-3.5" />
              {activeStep.category}
            </span>
            <span>
              STEP {currentStep + 1} OF {TUTORIAL_STEPS.length}
            </span>
          </div>

          {/* Progress bar track */}
          <div className="w-full h-1.5 bg-[#1E293B] rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-[#0284C7] to-[#38BDF8] rounded-full transition-all duration-300"
              style={{ width: `${progressPercent}%` }}
            />
          </div>
        </div>

        {/* Title & Close */}
        <div className="flex items-start justify-between gap-3">
          <div className="flex flex-col">
            <h3 className="text-[1.125rem] font-semibold text-white leading-snug">
              {activeStep.title}
            </h3>
            <p className="text-[0.8125rem] text-[#94A3B8] font-normal mt-0.5">
              {activeStep.subtitle}
            </p>
          </div>
          <button
            onClick={() => setIsOpen(false)}
            className="p-1 rounded-lg text-[#94A3B8] hover:text-white hover:bg-[#1E293B] transition-colors"
            title="Close Tutorial (Esc)"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Explanatory Bullet Points */}
        <div className="flex flex-col gap-2.5 py-1">
          {activeStep.details.map((detail, idx) => (
            <div key={idx} className="flex items-start gap-2.5 text-[0.8125rem] leading-relaxed text-[#CBD5E1]">
              <CheckCircle2 className="w-4 h-4 text-[#38BDF8] shrink-0 mt-0.5" />
              <span>{detail}</span>
            </div>
          ))}
        </div>

        {/* Action Controls */}
        <div className="flex items-center justify-between pt-2 border-t border-[#1E293B] mt-1">
          <button
            onClick={() => setIsOpen(false)}
            className="text-[0.75rem] font-mono text-[#94A3B8] hover:text-white transition-colors"
          >
            Skip Tour (Esc)
          </button>

          <div className="flex items-center gap-2">
            {currentStep > 0 && (
              <button
                onClick={() => setCurrentStep((prev) => prev - 1)}
                className="flex items-center gap-1 px-3 py-1.5 rounded-lg text-[0.75rem] font-medium bg-[#1E293B] hover:bg-[#334155] text-white transition-colors"
              >
                <ChevronLeft className="w-3.5 h-3.5" />
                Back
              </button>
            )}

            <button
              onClick={() => {
                if (currentStep < TUTORIAL_STEPS.length - 1) {
                  setCurrentStep((prev) => prev + 1);
                } else {
                  setIsOpen(false);
                }
              }}
              className="flex items-center gap-1.5 px-4 py-1.5 rounded-lg text-[0.75rem] font-semibold bg-gradient-to-r from-[#0284C7] to-[#0284C7]/80 hover:from-[#0369A1] hover:to-[#0284C7] text-white shadow-md shadow-[#0284C7]/20 transition-all cursor-pointer"
            >
              <span>{currentStep === TUTORIAL_STEPS.length - 1 ? "Finish Tour" : "Next"}</span>
              <ChevronRight className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
