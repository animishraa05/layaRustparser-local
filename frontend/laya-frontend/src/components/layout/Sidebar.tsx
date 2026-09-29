"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  LayoutGrid,
  Globe,
  Sliders,
  AlertTriangle,
  Terminal,
  Lock,
  Cpu,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { startTutorialTour } from "@/components/tutorial/TutorialOverlay";

interface NavItem {
  name: string;
  href: string;
  icon: typeof LayoutGrid;
  badge?: string;
  badgeColor?: string;
  activePath: string;
  disabled?: boolean;
}

const navItems: NavItem[] = [
  {
    name: "Overview",
    href: "/",
    icon: LayoutGrid,
    activePath: "/",
  },
  {
    name: "Threat Intel",
    href: "/threat-intelligence",
    icon: Globe,
    badge: "OCSF",
    badgeColor: "bg-[#0284C7]/10 text-[#0284C7] border-[#0284C7]/30",
    activePath: "/threat-intelligence",
  },
  {
    name: "Parsers/Norm",
    href: "/parsers-norm",
    icon: Sliders,
    badge: "1.3",
    badgeColor: "bg-white text-[#64748B] border-[#CBD5E1]",
    activePath: "/parsers-norm",
    disabled: false,
  },
  {
    name: "SIEM Alerts",
    href: "/siem-alerting",
    icon: AlertTriangle,
    badge: "2",
    badgeColor: "bg-[#FF5C5C] text-white font-semibold",
    activePath: "/siem-alerting",
  },
  {
    name: "Query Explorer",
    href: "/query-explorer",
    icon: Terminal,
    badge: "Parquet",
    badgeColor: "bg-white text-[#0284C7] border-[#CBD5E1]",
    activePath: "/query-explorer",
    disabled: false,
  },
  {
    name: "Crypto Vault",
    href: "/crypto-vault",
    icon: Lock,
    badge: "Audit",
    badgeColor: "bg-[#10B981]/10 text-[#10B981] border-[#10B981]/30",
    activePath: "/crypto-vault",
    disabled: false,
  },
  {
    name: "System Health",
    href: "/system-health",
    icon: Cpu,
    badge: "99.9%",
    badgeColor: "bg-[#10B981]/10 text-[#10B981] border-[#10B981]/30",
    activePath: "/system-health",
    disabled: false,
  },
];

export function Sidebar() {
  const pathname = usePathname();

  return (
    <aside className="fixed left-0 top-0 h-full w-[250px] bg-[#E2E4E8] border-r border-[#CBD5E1] z-50 flex flex-col justify-between select-none text-[#475569]">
      <div className="flex flex-col">
        {/* Header Logo */}
        <div className="h-16 px-4 border-b border-[#CBD5E1] flex items-center gap-2.5 bg-[#E2E4E8]">
          <div className="w-8 h-8 rounded-lg bg-[#1A1D20] text-white flex items-center justify-center shrink-0 shadow-sm">
            <ShieldCheck className="w-5 h-5 text-[#0284C7]" />
          </div>
          <div className="flex flex-col min-w-0">
            <span className="text-[1.125rem] text-[#1E293B] tracking-tight truncate font-semibold">
              ULPF SIEM
            </span>
            <span className="text-[0.75rem] font-mono text-[#64748B] truncate">
              Log Engine v2.4
            </span>
          </div>
        </div>

        {/* Section Label */}
        <div className="px-4 py-2.5">
          <span className="text-[0.6875rem] font-mono text-[#64748B] uppercase tracking-wider font-semibold">
            Navigation Engine
          </span>
        </div>

        {/* Nav Links */}
        <nav data-tour="sidebar-nav" className="px-2 flex flex-col gap-1">
          {navItems.map((item) => {
            const Icon = item.icon;
            const isActive =
              item.href === "/"
                ? pathname === "/"
                : item.href !== "#" && pathname.startsWith(item.href);

            if (item.disabled) {
              return (
                <div
                  key={item.name}
                  title="Scheduled for later issues (#14/#15)"
                  className="group flex items-center justify-between px-3 py-2 rounded text-[#94A3B8] cursor-not-allowed opacity-75"
                >
                  <div className="flex items-center gap-2.5">
                    <Icon className="w-4 h-4 text-[#94A3B8]" />
                    <span className="text-[0.875rem]">{item.name}</span>
                  </div>
                  {item.badge && (
                    <span
                      className={`text-[0.6875rem] font-mono px-1.5 py-0.5 rounded border ${item.badgeColor || "bg-white text-[#64748B] border-[#CBD5E1]"
                        }`}
                    >
                      {item.badge}
                    </span>
                  )}
                </div>
              );
            }

            return (
              <Link
                key={item.name}
                href={item.href}
                className={`group flex items-center justify-between px-3 py-2 rounded transition-colors ${isActive
                    ? "bg-white text-[#0284C7] font-semibold border border-[#CBD5E1] shadow-sm"
                    : "text-[#475569] hover:bg-[#D8DCE2] hover:text-[#1E293B]"
                  }`}
              >
                <div className="flex items-center gap-2.5">
                  <Icon
                    className={`w-4 h-4 ${isActive ? "text-[#0284C7]" : "text-[#64748B] group-hover:text-[#1E293B]"
                      }`}
                  />
                  <span
                    className={`text-[0.875rem] ${isActive ? "text-[#1E293B] font-semibold" : ""
                      }`}
                  >
                    {item.name}
                  </span>
                </div>
                {item.badge && (
                  <span
                    className={`text-[0.6875rem] font-mono px-1.5 py-0.5 rounded border ${item.badgeColor || "bg-white text-[#0284C7] border-[#CBD5E1]"
                      }`}
                  >
                    {item.badge}
                  </span>
                )}
              </Link>
            );
          })}
        </nav>
      </div>

      {/* Footer Info */}
      <div className="p-3 border-t border-[#CBD5E1] flex flex-col gap-2 bg-[#D8DCE2]">
        <button
          onClick={startTutorialTour}
          className="flex items-center justify-center gap-2 px-2.5 py-1.5 rounded bg-gradient-to-r from-[#0284C7] to-[#0284C7]/90 hover:from-[#0369A1] hover:to-[#0284C7] text-white text-[0.75rem] font-mono font-semibold shadow-sm transition-all cursor-pointer"
        >
          <Sparkles className="w-3.5 h-3.5 text-white animate-pulse" />
          <span>Interactive Tutorial</span>
        </button>

        <div className="flex items-center gap-2 px-2.5 py-1.5 rounded bg-white border border-[#CBD5E1] shadow-sm">
          <span className="w-2 h-2 rounded-full bg-[#10B981] ring-2 ring-emerald-500/30 animate-pulse"></span>
          <span className="text-[0.75rem] font-mono text-[#1E293B] font-medium truncate">
            ulpf-engine:stable
          </span>
        </div>
      </div>
    </aside>
  );
}
