import type { NextConfig } from "next";

const BACKEND_URL = process.env.BACKEND_URL || "http://127.0.0.1:8080";

const nextConfig: NextConfig = {
  async rewrites() {
    return [
      {
        source: "/metrics",
        destination: `${BACKEND_URL}/metrics`,
      },
      {
        source: "/alerts",
        destination: `${BACKEND_URL}/alerts`,
      },
      {
        source: "/blocks",
        destination: `${BACKEND_URL}/blocks`,
      },
      {
        source: "/blocks/:path*",
        destination: `${BACKEND_URL}/blocks/:path*`,
      },
      {
        source: "/prove/:path*",
        destination: `${BACKEND_URL}/prove/:path*`,
      },
      {
        source: "/export/bundle/:path*",
        destination: `${BACKEND_URL}/export/bundle/:path*`,
      },
      {
        source: "/parsers",
        destination: `${BACKEND_URL}/parsers`,
      },
      {
        source: "/parsers/:path*",
        destination: `${BACKEND_URL}/parsers/:path*`,
      },
      {
        source: "/onboard",
        destination: `${BACKEND_URL}/onboard`,
      },
      {
        source: "/system",
        destination: `${BACKEND_URL}/system`,
      },
      {
        source: "/tamper/:path*",
        destination: `${BACKEND_URL}/tamper/:path*`,
      },
    ];
  },
};

export default nextConfig;

