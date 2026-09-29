import {
  MetricsResponse,
  AlertItem,
  StoredRecordItem,
  BlockRecordsResponse,
  BlockItem,
  InclusionProofResponse,
  ApiErrorResponse,
  RecordsQueryParams,
  ParserItem,
  ParserTestRequest,
  ParserTestResponse,
  OnboardRequest,
  OnboardResponse,
  TamperDrillRequest,
  TamperDrillResponse,
  SystemResponse,
} from "./types";
import {
  mockMetrics,
  mockAlerts,
  mockRecords,
  mockBlocks,
  mockProve501,
  mockProveLive,
  mockParsers,
  mockParserTest,
  mockOnboardPreview,
  mockOnboardHotLoaded,
  mockSystem,
  mockTamperDrillPreview,
  mockTamperDrillSuccess,
} from "./mock-data";

export class ApiError extends Error {
  status: number;
  data?: ApiErrorResponse;
  isOffline: boolean;

  constructor(
    message: string,
    status: number = 0,
    data?: ApiErrorResponse,
    isOffline: boolean = false
  ) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.data = data;
    this.isOffline = isOffline;
  }
}

// In the browser, API calls use relative paths so Next.js proxies them directly to the backend.
// In SSR, API calls connect directly to the internal backend URL.
const API_BASE =
  typeof window !== "undefined"
    ? ""
    : process.env.INTERNAL_BACKEND_URL || "http://127.0.0.1:8080";

export type ApiMode = "LIVE" | "MOCK";
export type BackendStatus = "LIVE" | "OFFLINE" | "MOCK";

// Global mode state stored in memory and local storage
let currentMode: ApiMode = "LIVE";
let currentBackendOnline: boolean | null = null;
const statusListeners = new Set<() => void>();

function notifyStatusChange() {
  statusListeners.forEach((fn) => {
    try {
      fn();
    } catch {
      // ignore
    }
  });
  if (typeof window !== "undefined") {
    window.dispatchEvent(new Event("ulpf_backend_status_change"));
  }
}

export function setBackendOnline(online: boolean) {
  if (currentBackendOnline !== online) {
    currentBackendOnline = online;
    notifyStatusChange();
  }
}

export function isBackendOnline(): boolean | null {
  return currentBackendOnline;
}

export function getBackendStatus(): BackendStatus {
  const mode = getApiMode();
  if (mode === "MOCK") return "MOCK";
  if (currentBackendOnline === false) return "OFFLINE";
  return "LIVE";
}

export function subscribeBackendStatus(listener: () => void): () => void {
  statusListeners.add(listener);
  const handleWindow = () => listener();
  if (typeof window !== "undefined") {
    window.addEventListener("ulpf_backend_status_change", handleWindow);
    window.addEventListener("ulpf_api_mode_change", handleWindow);
  }
  return () => {
    statusListeners.delete(listener);
    if (typeof window !== "undefined") {
      window.removeEventListener("ulpf_backend_status_change", handleWindow);
      window.removeEventListener("ulpf_api_mode_change", handleWindow);
    }
  };
}

export function getApiMode(): ApiMode {
  if (typeof window !== "undefined") {
    const saved = localStorage.getItem("ulpf_api_mode") as ApiMode | null;
    if (saved === "LIVE" || saved === "MOCK") {
      currentMode = saved;
    }
  }
  return currentMode;
}

export function setApiMode(mode: ApiMode) {
  currentMode = mode;
  if (typeof window !== "undefined") {
    localStorage.setItem("ulpf_api_mode", mode);
    window.dispatchEvent(new Event("ulpf_api_mode_change"));
  }
  notifyStatusChange();
}

/**
 * Checks if the ULPF backend is responding on API_BASE
 */
export async function checkBackendReachable(): Promise<boolean> {
  try {
    const res = await fetch(`${API_BASE}/metrics`, {
      method: "GET",
      cache: "no-store",
      headers: { Accept: "application/json" },
      signal: AbortSignal.timeout(1500),
    });
    const ok = res.ok;
    setBackendOnline(ok);
    return ok;
  } catch {
    setBackendOnline(false);
    return false;
  }
}

/**
 * Fetches metrics from GET /metrics.
 * In LIVE mode: calls backend and returns real metrics, or throws on failure.
 * In MOCK mode: returns fixture data.
 */
export async function getMetrics(): Promise<{
  data: MetricsResponse;
  isLive: boolean;
}> {
  const mode = getApiMode();

  if (mode === "LIVE") {
    const res = await fetch(`${API_BASE}/metrics`, {
      method: "GET",
      cache: "no-store",
      headers: { Accept: "application/json" },
      signal: AbortSignal.timeout(2000),
    });

    if (!res.ok) {
      throw new Error(`Failed to fetch metrics: HTTP ${res.status}`);
    }

    const json: MetricsResponse = await res.json();
    return { data: json, isLive: true };
  }

  return { data: mockMetrics, isLive: false };
}

/**
 * Fetches active alerts from GET /alerts.
 * In LIVE mode: calls backend and returns real alerts, or throws on failure.
 * In MOCK mode: returns fixture data.
 */
export async function getAlerts(): Promise<{
  data: AlertItem[];
  isLive: boolean;
}> {
  const mode = getApiMode();

  if (mode === "LIVE") {
    const res = await fetch(`${API_BASE}/alerts`, {
      method: "GET",
      cache: "no-store",
      headers: { Accept: "application/json" },
      signal: AbortSignal.timeout(2000),
    });

    if (!res.ok) {
      throw new Error(`Failed to fetch alerts: HTTP ${res.status}`);
    }

    const json: AlertItem[] = await res.json();
    return { data: json, isLive: true };
  }

  return { data: mockAlerts, isLive: false };
}

/**
 * Fetches stored records for a block from GET /blocks/:id/records.
 * In LIVE mode: calls backend and returns real records, or throws on failure.
 * In MOCK mode: returns fixture data filtered by criteria.
 */
export async function getBlockRecords(
  blockId: number = 1,
  params?: RecordsQueryParams
): Promise<{
  data: StoredRecordItem[];
  total: number;
  filteredCount: number;
  offset: number;
  limit: number;
  response: BlockRecordsResponse;
  isLive: boolean;
}> {
  const mode = getApiMode();

  if (mode === "LIVE") {
    const query = new URLSearchParams();
    if (params?.offset !== undefined) query.set("offset", params.offset.toString());
    if (params?.limit !== undefined) query.set("limit", params.limit.toString());
    if (params?.vendor) query.set("vendor", params.vendor);
    if (params?.disposition) query.set("disposition", params.disposition);
    if (params?.ip) query.set("ip", params.ip);
    if (params?.query) query.set("query", params.query);

    const qs = query.toString() ? `?${query.toString()}` : "";
    let res: Response;
    try {
      res = await fetch(`${API_BASE}/blocks/${blockId}/records${qs}`, {
        method: "GET",
        cache: "no-store",
        headers: { Accept: "application/json" },
        signal: AbortSignal.timeout(5000),
      });
      setBackendOnline(true);
    } catch {
      setBackendOnline(false);
      throw new ApiError(
        "Backend Offline: Unable to reach investigation service.",
        0,
        undefined,
        true
      );
    }

    if (!res.ok) {
      let errorData: ApiErrorResponse | undefined;
      try {
        errorData = await res.json();
      } catch {
        // non-json response
      }

      if (res.status === 404) {
        throw new ApiError(
          errorData?.message || `Parquet block #${blockId} does not exist.`,
          404,
          errorData,
          false
        );
      }
      if (res.status === 500) {
        throw new ApiError(
          errorData?.message || `Failed reading Parquet block #${blockId}.`,
          500,
          errorData,
          false
        );
      }
      throw new ApiError(
        errorData?.message || `Failed to fetch records: HTTP ${res.status}`,
        res.status,
        errorData,
        false
      );
    }

    const json: BlockRecordsResponse = await res.json();
    return {
      data: json.records,
      total: json.total_records_in_block,
      filteredCount: json.filtered_records_count,
      offset: json.offset,
      limit: json.limit,
      response: json,
      isLive: true,
    };
  }

  // MOCK mode: Filter mock records according to query parameters
  const targetBlock = mockBlocks.find((b) => b.block_id === blockId);
  if (targetBlock && !targetBlock.file_exists) {
    throw new ApiError(
      `Parquet block #${blockId} does not exist.`,
      404,
      {
        error: "Not Found",
        code: 404,
        message: `Parquet block #${blockId} does not exist on disk`,
        block_id: blockId,
        leaf_index: null,
      },
      false
    );
  }

  const offset = params?.offset ?? 0;
  const limit = params?.limit ?? 50;

  const filtered = mockRecords.filter((r) => {
    if (params?.vendor && r.vendor.toLowerCase() !== params.vendor.toLowerCase()) {
      return false;
    }
    if (
      params?.disposition &&
      r.ocsf?.disposition?.toLowerCase() !== params.disposition.toLowerCase()
    ) {
      return false;
    }
    if (params?.ip) {
      const inRaw = r.raw_log.includes(params.ip);
      const inSrc = r.ocsf?.src_endpoint?.ip?.includes(params.ip);
      const inDst = r.ocsf?.dst_endpoint?.ip?.includes(params.ip);
      if (!inRaw && !inSrc && !inDst) return false;
    }
    if (params?.query) {
      const q = params.query.toLowerCase();
      const inRaw = r.raw_log.toLowerCase().includes(q);
      const inId = r.event_id.toLowerCase().includes(q);
      const inHash = r.raw_hash.toLowerCase().includes(q);
      if (!inRaw && !inId && !inHash) return false;
    }
    return true;
  });

  const page = filtered.slice(offset, offset + limit);
  const response: BlockRecordsResponse = {
    block_id: blockId,
    total_records_in_block: mockRecords.length,
    filtered_records_count: filtered.length,
    offset,
    limit,
    records: page,
  };

  return {
    data: page,
    total: mockRecords.length,
    filteredCount: filtered.length,
    offset,
    limit,
    response,
    isLive: false,
  };
}

/**
 * Fetches blocks ledger from GET /blocks.
 * In LIVE mode: calls backend and returns real blocks, or throws on failure.
 * In MOCK mode: returns fixture data.
 */
export async function getBlocks(): Promise<{
  data: BlockItem[];
  isLive: boolean;
}> {
  const mode = getApiMode();

  if (mode === "LIVE") {
    let res: Response;
    try {
      res = await fetch(`${API_BASE}/blocks`, {
        method: "GET",
        cache: "no-store",
        headers: { Accept: "application/json" },
        signal: AbortSignal.timeout(3000),
      });
      setBackendOnline(true);
    } catch {
      setBackendOnline(false);
      throw new ApiError(
        "Backend Offline: Unable to reach blocks service.",
        0,
        undefined,
        true
      );
    }

    if (!res.ok) {
      throw new ApiError(
        `Failed to fetch blocks: HTTP ${res.status}`,
        res.status,
        undefined,
        false
      );
    }

    const json: BlockItem[] = await res.json();
    return { data: json, isLive: true };
  }

  return {
    data: mockBlocks,
    isLive: false,
  };
}

/**
 * Requests Merkle inclusion proof for a given block and leaf index: GET /prove/:block/:leaf
 * Supports ?live=true for RFC 6962 audit path calculation.
 * Preserves 501 Not Implemented semantics distinctly from network failure.
 */
export async function getProveInclusion(
  blockId: number,
  leafIndex: number,
  live?: boolean
): Promise<{
  status: number;
  data?: InclusionProofResponse;
  error?: ApiErrorResponse;
  isLive: boolean;
}> {
  const mode = getApiMode();

  if (mode === "LIVE") {
    const url = `${API_BASE}/prove/${blockId}/${leafIndex}${live ? "?live=true" : ""}`;
    let res: Response;
    try {
      res = await fetch(url, {
        method: "GET",
        cache: "no-store",
        headers: { Accept: "application/json" },
        signal: AbortSignal.timeout(4000),
      });
      setBackendOnline(true);
    } catch {
      setBackendOnline(false);
      throw new ApiError(
        "Backend Offline: Unable to reach proof service.",
        0,
        undefined,
        true
      );
    }

    if (res.status === 501) {
      const errJson: ApiErrorResponse = await res.json().catch(() => ({
        error: "Not Implemented",
        code: 501,
        message:
          "Merkle inclusion proof endpoint is stubbed pending completion of #5. Pass '?live=true' to execute live computation.",
        block_id: blockId,
        leaf_index: leafIndex,
      }));
      return { status: 501, error: errJson, isLive: true };
    }

    if (!res.ok) {
      let errJson: ApiErrorResponse | undefined;
      try {
        errJson = await res.json();
      } catch {
        // non-json
      }
      throw new ApiError(
        errJson?.message || `Proof request failed with HTTP ${res.status}`,
        res.status,
        errJson,
        false
      );
    }

    const proofJson: InclusionProofResponse = await res.json();
    return { status: 200, data: proofJson, isLive: true };
  }

  // MOCK mode
  if (!live) {
    return {
      status: 501,
      error: {
        ...mockProve501,
        block_id: blockId,
        leaf_index: leafIndex,
      },
      isLive: false,
    };
  }

  return {
    status: 200,
    data: {
      ...mockProveLive,
      block_id: blockId,
      leaf_index: leafIndex,
    },
    isLive: false,
  };
}

/**
 * Downloads courtroom evidence bundle: GET /export/bundle/:id
 * Authoritative backend artifact (.tar.gz). Never generates fake client archives.
 */
export async function exportEvidenceBundle(blockId: number): Promise<{
  filename: string;
}> {
  const mode = getApiMode();

  if (mode === "MOCK") {
    // Check if live backend happens to be reachable, otherwise fail honestly per integrity rules
    const isReachable = await checkBackendReachable();
    if (!isReachable) {
      throw new ApiError(
        "Authoritative courtroom evidence export requires a connected backend service. Client-side fake archive generation is prohibited.",
        400,
        undefined,
        false
      );
    }
  }

  let res: Response;
  try {
    res = await fetch(`${API_BASE}/export/bundle/${blockId}`, {
      method: "GET",
      cache: "no-store",
      signal: AbortSignal.timeout(10000),
    });
    setBackendOnline(true);
  } catch {
    setBackendOnline(false);
    throw new ApiError(
      "Backend Offline: Unable to reach evidence export service.",
      0,
      undefined,
      true
    );
  }

  if (!res.ok) {
    let errJson: ApiErrorResponse | undefined;
    try {
      errJson = await res.json();
    } catch {
      // non-json
    }
    throw new ApiError(
      errJson?.message || `Failed to export bundle: HTTP ${res.status}`,
      res.status,
      errJson,
      false
    );
  }

  // Extract filename from Content-Disposition header if available
  const disposition = res.headers.get("Content-Disposition");
  let filename = `ulpf_evidence_block_${String(blockId).padStart(5, "0")}.tar.gz`;
  if (disposition) {
    const match = disposition.match(/filename=["']?([^"';]+)["']?/i);
    if (match && match[1]) {
      filename = match[1];
    }
  }

  const blob = await res.blob();
  if (typeof window !== "undefined") {
    const url = window.URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    setTimeout(() => {
      window.URL.revokeObjectURL(url);
      a.remove();
    }, 100);
  }

  return { filename };
}

// =============================================================================
// Issue #15: Parser & Integrity Management API Functions
// =============================================================================

/**
 * Fetches registered parsers list: GET /parsers
 * Returns active native extractors and dynamic onboarded parsers.
 */
export async function getParsers(): Promise<{
  data: ParserItem[];
  isLive: boolean;
}> {
  const mode = getApiMode();

  if (mode === "LIVE") {
    let res: Response;
    try {
      res = await fetch(`${API_BASE}/parsers`, {
        method: "GET",
        cache: "no-store",
        headers: { Accept: "application/json" },
        signal: AbortSignal.timeout(3000),
      });
      setBackendOnline(true);
    } catch {
      setBackendOnline(false);
      throw new ApiError(
        "Backend Offline: Unable to reach parser registry service.",
        0,
        undefined,
        true
      );
    }

    if (!res.ok) {
      let errJson: ApiErrorResponse | undefined;
      try {
        errJson = await res.json();
      } catch {
        // non-json
      }
      throw new ApiError(
        errJson?.message || `Failed to fetch parsers: HTTP ${res.status}`,
        res.status,
        errJson,
        false
      );
    }

    const json: ParserItem[] = await res.json();
    return { data: json, isLive: true };
  }

  // MOCK mode: authoritative mock fixture replacement
  return {
    data: mockParsers,
    isLive: false,
  };
}

/**
 * Executes a dry-run test of a raw log line against a parser: POST /parsers/test
 * Never writes to disk or persists.
 */
export async function testParser(
  payload: ParserTestRequest
): Promise<{
  data: ParserTestResponse;
  isLive: boolean;
}> {
  const mode = getApiMode();

  if (mode === "LIVE") {
    let res: Response;
    try {
      res = await fetch(`${API_BASE}/parsers/test`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
        },
        body: JSON.stringify(payload),
        signal: AbortSignal.timeout(4000),
      });
      setBackendOnline(true);
    } catch {
      setBackendOnline(false);
      throw new ApiError(
        "Backend Offline: Unable to reach parser test service.",
        0,
        undefined,
        true
      );
    }

    if (!res.ok) {
      let errJson: ApiErrorResponse | undefined;
      try {
        errJson = await res.json();
      } catch {
        // non-json
      }
      throw new ApiError(
        errJson?.message || `Parser test failed: HTTP ${res.status}`,
        res.status,
        errJson,
        false
      );
    }

    const json: ParserTestResponse = await res.json();
    return { data: json, isLive: true };
  }

  // MOCK mode: returns mockParserTest fixture, reflecting vendor if provided
  const matched = !payload.raw_log.toLowerCase().includes("unmatched");
  return {
    data: {
      ...mockParserTest,
      matched,
      vendor: payload.vendor || mockParserTest.vendor,
      notes: "Parsed through dynamic registry / universal baseline (read-only mock simulation)",
    },
    isLive: false,
  };
}

/**
 * Synthesizes a new regex parser definition: POST /onboard
 * If confirm is false: returns preview synthesis without writing files.
 * If confirm is true: writes .json & .yaml to data/parsers/ and hot-loads into active memory.
 */
export async function onboardParser(
  payload: OnboardRequest
): Promise<{
  data: OnboardResponse;
  isLive: boolean;
}> {
  const mode = getApiMode();

  if (mode === "LIVE") {
    let res: Response;
    try {
      res = await fetch(`${API_BASE}/onboard`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
        },
        body: JSON.stringify(payload),
        signal: AbortSignal.timeout(5000),
      });
      setBackendOnline(true);
    } catch {
      setBackendOnline(false);
      throw new ApiError(
        "Backend Offline: Unable to reach onboarding service.",
        0,
        undefined,
        true
      );
    }

    if (!res.ok) {
      let errJson: ApiErrorResponse | undefined;
      try {
        errJson = await res.json();
      } catch {
        // non-json
      }
      throw new ApiError(
        errJson?.message || `Onboarding failed: HTTP ${res.status}`,
        res.status,
        errJson,
        false
      );
    }

    const json: OnboardResponse = await res.json();
    return { data: json, isLive: true };
  }

  // MOCK mode: returns mock preview or mock hot-loaded fixture
  if (payload.confirm) {
    return {
      data: {
        ...mockOnboardHotLoaded,
        vendor: payload.vendor,
        device_model: payload.device_model || "generic",
        json_path: `data/parsers/${payload.vendor.toLowerCase().replace(/[^a-z0-9_-]/g, "_")}.json`,
        yaml_path: `data/parsers/${payload.vendor.toLowerCase().replace(/[^a-z0-9_-]/g, "_")}.yaml`,
      },
      isLive: false,
    };
  }

  return {
    data: {
      ...mockOnboardPreview,
      vendor: payload.vendor,
      device_model: payload.device_model || "generic",
      parser_definition: {
        ...mockOnboardPreview.parser_definition,
        vendor: payload.vendor,
        device_model: payload.device_model || "generic",
        sample_logs: payload.sample_lines,
      },
      validation_report: {
        ...mockOnboardPreview.validation_report,
        total_samples: payload.sample_lines.length,
        matched_samples: payload.sample_lines.length,
      },
    },
    isLive: false,
  };
}

/**
 * Executes safe adversarial tamper drill: POST /tamper/drill
 * Never modifies real evidence in data/parquet/. Clones to data/scratch/ and tampers ONLY the copy.
 */
export async function tamperDrill(
  payload: TamperDrillRequest
): Promise<{
  data: TamperDrillResponse;
  isLive: boolean;
}> {
  const mode = getApiMode();

  if (mode === "LIVE") {
    let res: Response;
    try {
      res = await fetch(`${API_BASE}/tamper/drill`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
        },
        body: JSON.stringify(payload),
        signal: AbortSignal.timeout(6000),
      });
      setBackendOnline(true);
    } catch {
      setBackendOnline(false);
      throw new ApiError(
        "Backend Offline: Unable to reach tamper drill service.",
        0,
        undefined,
        true
      );
    }

    if (!res.ok) {
      let errJson: ApiErrorResponse | undefined;
      try {
        errJson = await res.json();
      } catch {
        // non-json
      }
      throw new ApiError(
        errJson?.message || `Tamper drill failed: HTTP ${res.status}`,
        res.status,
        errJson,
        false
      );
    }

    const json: TamperDrillResponse = await res.json();
    return { data: json, isLive: true };
  }

  // MOCK mode
  if (payload.confirm) {
    return {
      data: {
        ...mockTamperDrillSuccess,
        target_block_id: payload.block_id,
        target_leaf_index: payload.leaf_index ?? 0,
        spoofed_ip: payload.spoofed_ip || "10.99.99.99",
        source_evidence_path: `data/parquet/block_${String(payload.block_id).padStart(5, "0")}.parquet`,
        scratch_drill_path: `data/scratch/tamper_drill_block_${String(payload.block_id).padStart(5, "0")}.parquet`,
      },
      isLive: false,
    };
  }

  return {
    data: {
      ...mockTamperDrillPreview,
      target_block_id: payload.block_id,
      target_leaf_index: payload.leaf_index ?? 0,
      spoofed_ip: payload.spoofed_ip || "10.99.99.99",
      source_evidence_path: `data/parquet/block_${String(payload.block_id).padStart(5, "0")}.parquet`,
      scratch_drill_path: `data/scratch/tamper_drill_block_${String(payload.block_id).padStart(5, "0")}.parquet`,
    },
    isLive: false,
  };
}

/**
 * Fetches system settings & diagnostic scorecard: GET /system
 * Displays batcher thresholds, queue capacity, and comparative benchmark metrics.
 */
export async function getSystem(): Promise<{
  data: SystemResponse;
  isLive: boolean;
}> {
  const mode = getApiMode();

  if (mode === "LIVE") {
    let res: Response;
    try {
      res = await fetch(`${API_BASE}/system`, {
        method: "GET",
        cache: "no-store",
        headers: { Accept: "application/json" },
        signal: AbortSignal.timeout(3000),
      });
      setBackendOnline(true);
    } catch {
      setBackendOnline(false);
      throw new ApiError(
        "Backend Offline: Unable to reach system diagnostics service.",
        0,
        undefined,
        true
      );
    }

    if (!res.ok) {
      let errJson: ApiErrorResponse | undefined;
      try {
        errJson = await res.json();
      } catch {
        // non-json
      }
      throw new ApiError(
        errJson?.message || `Failed to fetch system scorecard: HTTP ${res.status}`,
        res.status,
        errJson,
        false
      );
    }

    const json: SystemResponse = await res.json();
    return { data: json, isLive: true };
  }

  // MOCK mode: authoritative mock fixture replacement
  return {
    data: mockSystem,
    isLive: false,
  };
}

