use crossbeam_channel::{bounded, Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use ulpf_core::parser::classifier::VendorFormat;
use ulpf_core::parser::lru_cache::{LruStats, SignatureLruCache};
use ulpf_core::parser::UniversalParser;
use ulpf_core::schema::ocsf::NetworkActivity;

use crate::drain::{AnomalyAlert, DrainConfig, DrainMiner};
use crate::laya::LayaDecisionEngine;
use crate::onboarder::{DynamicParserRegistry, Onboarder};

/// Task sent to the asynchronous out-of-band Laya System 1 decision worker
#[derive(Debug, Clone)]
pub struct AsyncTriageTask {
    /// Exemplar budget/buffer key: the Tier-1 signature hash (format-level
    /// identity computable on every process() path without the drain lock).
    pub cluster_id: usize,
    /// Drain template when known; empty on the Tier-1 fast path (worker uses sample_log)
    pub template: String,
    pub sample_log: String,
    pub timestamp_ms: i64,
}

/// Tier-3 exemplar budget per cluster: the onboarder needs >= 3 samples, the
/// extra headroom covers bounded-channel drops. The former budget of exactly 1
/// made onboarding mathematically impossible (generate_parser requires 3).
const MAX_EXEMPLARS_PER_CLUSTER: u8 = 8;
/// Minimum sample count before the onboarder synthesizes a parser from a cluster.
const ONBOARD_MIN_SAMPLES: usize = 3;
/// Hot-path bound: distinct Tier-3 exemplar keys retained in
/// `triage_dispatch_counts`. Past this the oldest key is evicted (its budget
/// restarts if the shape ever returns).
const MAX_TRIAGE_KEYS: usize = 10_000;
/// Worker-side bound: distinct per-cluster exemplar buffers retained.
/// Past this the oldest cluster is evicted before its samples onboard.
const MAX_CLUSTER_BUFFERS: usize = 10_000;
/// Compaction trigger: the worker's `buffer_order` queue holds one entry per
/// buffer insertion but entries go stale on onboarding removal, so the queue
/// can outgrow `buffers`. Past 2x the cap it is rebuilt from live keys only.
const BUFFER_ORDER_COMPACT_LEN: usize = 2 * MAX_CLUSTER_BUFFERS;

/// Pop the oldest live cluster buffer (FIFO). Stale entries — keys already
/// onboarded-and-removed, or an older duplicate of a reused cluster id that
/// still has a newer entry later in the queue — are skipped WITHOUT evicting:
/// evicting on a stale duplicate would discard the NEWER buffer's samples.
fn pop_oldest_live_buffer(
    buffers: &mut HashMap<usize, Vec<String>>,
    order: &mut VecDeque<usize>,
) -> Option<usize> {
    while let Some(old) = order.pop_front() {
        if !buffers.contains_key(&old) {
            continue;
        }
        // Stale duplicate: the id was onboarded, removed, then reused for a
        // newer buffer that sits later in the queue. Evicting here would kill
        // the newer buffer, so skip and let the fresh position decide.
        if order.contains(&old) {
            continue;
        }
        if buffers.remove(&old).is_some() {
            return Some(old);
        }
    }
    None
}

/// Rebuild the order queue from live buffers only (deduped, newest position
/// wins). Called when the queue outgrows `BUFFER_ORDER_COMPACT_LEN` so it
/// stays bounded over the process lifetime despite onboarding churn.
fn compact_buffer_order(order: &mut VecDeque<usize>, buffers: &HashMap<usize, Vec<String>>) {
    if order.len() <= BUFFER_ORDER_COMPACT_LEN {
        return;
    }
    let mut seen = std::collections::HashSet::with_capacity(buffers.len());
    let mut kept = Vec::with_capacity(buffers.len());
    // Newest-first pass: the first sighting of a key walking back from the
    // tail is its freshest position, which is the one we keep.
    for &key in order.iter().rev() {
        if buffers.contains_key(&key) && seen.insert(key) {
            kept.push(key);
        }
    }
    kept.reverse();
    *order = VecDeque::from(kept);
}

/// Real-time throughput and triage statistics across all 3 tiers
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PipelineStats {
    pub total_events: u64,
    pub tier1_lru_hits: u64,
    pub tier2_drain_hits: u64,
    pub tier3_laya_dispatches: u64,
    pub tier3_laya_onboarded: u64,
    /// Wired Laya `classify_action` head: high-confidence action decisions seen
    pub laya_action_flags: u64,
    /// Wired Laya `score_threat_risk` head: scores at/above the 0.50 threshold
    pub laya_threat_flags: u64,
    /// Worker-side exemplar buffer evictions (oldest cluster dropped past
    /// `MAX_CLUSTER_BUFFERS` before onboarding)
    pub cluster_evictions: u64,
    /// Hot-path dispatch-table evictions (oldest exemplar key dropped past
    /// `MAX_TRIAGE_KEYS`; its budget restarts on next sighting)
    pub triage_evictions: u64,
    pub lru_hit_ratio: f64,
    pub lru_stats: LruStats,
}

/// 3-Tier Enterprise Log Processing Pipeline:
/// - Tier 1: Lock-Free Signature LRU Cache (~0.06 µs fast path)
/// - Tier 2: DrainDotNet Engine with Anchor Tokens (line-rate clustering on cache miss)
/// - Tier 3: Asynchronous Decoupled Laya System 1 Decision Engine (zero-hallucination out-of-band triage)
pub struct TieredPipeline {
    parser: Arc<UniversalParser>,
    drain: Arc<Mutex<DrainMiner>>,
    dynamic_registry: Arc<Mutex<DynamicParserRegistry>>,
    laya_sender: Sender<AsyncTriageTask>,
    _worker_handle: Option<JoinHandle<()>>,
    total_events: AtomicU64,
    tier2_drain_hits: AtomicU64,
    tier3_laya_dispatches: AtomicU64,
    tier3_laya_onboarded: Arc<AtomicU64>,
    /// Exemplar dispatch budget per format key (Tier-3 needs >= 3 samples)
    triage_dispatch_counts: Mutex<HashMap<usize, u8>>,
    /// Insertion order of dispatch keys: pops the oldest key when the table
    /// hits `MAX_TRIAGE_KEYS`. Only touched on brand-new keys, never on the
    /// per-event fast path for known keys.
    triage_dispatch_order: Mutex<VecDeque<usize>>,
    /// Hot-path dispatch-table evictions (see `PipelineStats::triage_evictions`)
    triage_evictions: AtomicU64,
    /// Worker-side exemplar buffer evictions, shared with the triage thread
    cluster_evictions: Arc<AtomicU64>,
    /// Count of format keys still under their exemplar budget: the Tier-1 fast
    /// path reads this single atomic and pays ~1 ns in the steady state (all
    /// budgets closed) instead of touching the dispatch map at all.
    exemplar_budget_open: AtomicU64,
    /// Tier-1b promotion routes: signature hash -> registry key for shapes parsed
    /// by a dynamic (onboarded) parser. Lets repeat lines skip the Drain mutex and
    /// the full registry scan.
    dynamic_routes: Arc<Mutex<HashMap<u64, String>>>,
    /// Number of installed routes — one atomic gates the Tier-1b lock (0 = skip).
    dynamic_routes_open: Arc<AtomicU64>,
    /// Wired Laya action/threat heads (feeds P8 adjudication metrics)
    laya_action_flags: Arc<AtomicU64>,
    laya_threat_flags: Arc<AtomicU64>,
}

impl TieredPipeline {
    /// Create and start a new 3-tier pipeline with default ring-buffer capacity (10,000 tasks)
    pub fn new() -> Self {
        Self::with_ring_buffer_capacity(10_000)
    }

    /// Create and start a new 3-tier pipeline with custom ring-buffer capacity
    pub fn with_ring_buffer_capacity(capacity: usize) -> Self {
        let parser = Arc::new(UniversalParser::new());
        let drain = Arc::new(Mutex::new(DrainMiner::new(DrainConfig::default())));
        let dynamic_registry = Arc::new(Mutex::new(DynamicParserRegistry::new()));
        let (sender, receiver): (Sender<AsyncTriageTask>, Receiver<AsyncTriageTask>) =
            bounded(capacity);

        let onboarded_counter = Arc::new(AtomicU64::new(0));
        let onboarded_ref = onboarded_counter.clone();
        let registry_ref = dynamic_registry.clone();
        let action_flags = Arc::new(AtomicU64::new(0));
        let action_flags_ref = action_flags.clone();
        let threat_flags = Arc::new(AtomicU64::new(0));
        let threat_flags_ref = threat_flags.clone();
        let dynamic_routes: Arc<Mutex<HashMap<u64, String>>> = Arc::new(Mutex::new(HashMap::new()));
        let routes_ref = dynamic_routes.clone();
        let routes_open_ref = Arc::new(AtomicU64::new(0));
        let routes_open_for_worker = routes_open_ref.clone();
        let cluster_evictions = Arc::new(AtomicU64::new(0));
        let cluster_evictions_for_worker = cluster_evictions.clone();

        // Spawn Asynchronous Tier-3 Control Plane Worker (Out-of-Band)
        let worker_handle = thread::Builder::new()
            .name("laya-triage-worker".into())
            .spawn(move || {
                let laya = LayaDecisionEngine::new();
                // Per-cluster exemplar buffers: onboard only at >= ONBOARD_MIN_SAMPLES
                // so the synthesizer sees real variation instead of a single line
                // (generate_parser rejects slices shorter than 3).
                let mut buffers: HashMap<usize, Vec<String>> = HashMap::new();
                // Insertion order for the buffer bound below. Entries go stale
                // on onboarding removal; the eviction helper skips them and
                // the queue is compacted past 2x the cap so it stays bounded.
                let mut buffer_order: VecDeque<usize> = VecDeque::new();

                while let Ok(task) = receiver.recv() {
                    // 1. Non-autoregressive vendor classification
                    let vendor_choice = laya.classify_vendor(&task.sample_log);
                    // Previously-discarded heads wired into triage outcome flags
                    let action_choice = laya.classify_action(&task.sample_log);
                    let threat_score = laya.score_threat_risk(&task.sample_log);
                    if action_choice.probability >= 0.85 {
                        action_flags_ref.fetch_add(1, Ordering::Relaxed);
                    }
                    if threat_score.score >= 0.50 {
                        threat_flags_ref.fetch_add(1, Ordering::Relaxed);
                    }

                    // Guardrail 3: Calibrated Confidence Gating (P >= 0.85)
                    if vendor_choice.probability < 0.85 {
                        continue;
                    }

                    if !buffers.contains_key(&task.cluster_id) {
                        buffer_order.push_back(task.cluster_id);
                        // Bound the accumulator: a flood of distinct novel
                        // shapes must not grow this map without limit. The
                        // oldest cluster is evicted first (its samples never
                        // onboard — counted so operators can see the loss).
                        while buffers.len() >= MAX_CLUSTER_BUFFERS {
                            if pop_oldest_live_buffer(&mut buffers, &mut buffer_order).is_some() {
                                cluster_evictions_for_worker.fetch_add(1, Ordering::Relaxed);
                            } else {
                                break;
                            }
                        }
                    }
                    let buf = buffers.entry(task.cluster_id).or_default();
                    if buf.len() < MAX_EXEMPLARS_PER_CLUSTER as usize {
                        buf.push(task.sample_log);
                    }
                    if buf.len() < ONBOARD_MIN_SAMPLES {
                        continue;
                    }

                    let samples: Vec<&str> = buf.iter().map(|s| s.as_str()).collect();
                    let outcome = Onboarder::generate_parser(
                        &vendor_choice.label,
                        &format!("cluster-{}", task.cluster_id),
                        &samples,
                    );
                    // Retry with more exemplars until the buffer cap, then give up.
                    let done = match outcome {
                        Ok((parser_def, report)) => {
                            if report.passed && report.match_percentage == 100.0 {
                                let key = {
                                    let mut reg = registry_ref.lock().unwrap();
                                    reg.register(parser_def)
                                };
                                // Install the Tier-1b promotion route for this format
                                // key so repeat lines skip Drain + registry scan.
                                if let Ok(mut routes) = routes_ref.lock() {
                                    if routes.insert(task.cluster_id as u64, key).is_none() {
                                        routes_open_for_worker.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                                onboarded_ref.fetch_add(1, Ordering::Relaxed);
                                true
                            } else {
                                buf.len() >= MAX_EXEMPLARS_PER_CLUSTER as usize
                            }
                        }
                        Err(_) => buf.len() >= MAX_EXEMPLARS_PER_CLUSTER as usize,
                    };
                    if done {
                        buffers.remove(&task.cluster_id);
                        // Onboarding removal leaves a stale queue entry behind.
                        // Compact past 2x the cap so the queue stays bounded
                        // over process lifetime instead of growing per onboard.
                        compact_buffer_order(&mut buffer_order, &buffers);
                    }
                }
            })
            .expect("Failed to spawn laya worker thread");

        Self {
            parser,
            drain,
            dynamic_registry,
            laya_sender: sender,
            _worker_handle: Some(worker_handle),
            total_events: AtomicU64::new(0),
            tier2_drain_hits: AtomicU64::new(0),
            tier3_laya_dispatches: AtomicU64::new(0),
            tier3_laya_onboarded: onboarded_counter,
            triage_dispatch_counts: Mutex::new(HashMap::new()),
            triage_dispatch_order: Mutex::new(VecDeque::new()),
            triage_evictions: AtomicU64::new(0),
            cluster_evictions,
            exemplar_budget_open: AtomicU64::new(0),
            dynamic_routes,
            dynamic_routes_open: routes_open_ref,
            laya_action_flags: action_flags,
            laya_threat_flags: threat_flags,
        }
    }

    /// Process a raw log line through the 3-Tier Pipeline
    /// Guarantees sub-microsecond line rate without blocking for Tier-3 Laya
    pub fn process(&self, raw: &str) -> NetworkActivity {
        // Scoring behavior is pinned here: no Drain accounting on the fast
        // paths, anomaly discarded — `process_live` opts into both.
        self.process_inner(raw, false).0
    }

    /// Live-ingest entry point: identical Tier-1/Tier-2/Tier-3 routing and
    /// identical parse output as [`process`](Self::process), but the Drain
    /// disposition `process` swallows is returned alongside the event.
    ///
    /// Why the extra pass exists: on a Tier-1 LRU (or Tier-1b route) hit
    /// `process` never touches Drain, so once Tier-1 warms the rare-cluster
    /// surge detector would go blind and the live `[SECURITY ALERT]` line
    /// would fall silent. The accounting replay feeds the hit through Drain
    /// purely to advance cluster counts and surface anomalies; the parse
    /// result stays on the zero-copy fast path. Each ingest worker owns its
    /// pipeline by value, so this mutex is per-worker uncontended — it never
    /// reintroduces the shared-pipeline contention this design avoids.
    /// Additive only: drain internals and scoring behavior are untouched.
    pub fn process_live(&self, raw: &str) -> (NetworkActivity, Option<AnomalyAlert>) {
        self.process_inner(raw, true)
    }

    /// Shared core behind [`process`](Self::process) (`tier1_accounting =
    /// false`, anomaly discarded) and [`process_live`](Self::process_live)
    /// (`true`, anomaly returned). The `false` path is the historical
    /// `process` body verbatim — no behavioral change for existing callers.
    fn process_inner(
        &self,
        raw: &str,
        tier1_accounting: bool,
    ) -> (NetworkActivity, Option<AnomalyAlert>) {
        self.total_events.fetch_add(1, Ordering::Relaxed);

        // ---------------------------------------------------------------------
        // TIER 1: Signature LRU Cache Fast-Path Lookup (~0.06 µs)
        // ---------------------------------------------------------------------
        let sig_hash = SignatureLruCache::compute_signature_hash(raw);
        if let Some(format) = self.parser.cache.get(sig_hash) {
            // Fast-path exemplar dispatch: gated by one atomic read so the steady
            // state (every format's exemplar budget closed) costs ~1 ns. Without
            // this, Tier-1 promotion would starve Tier-3 at exactly 2 samples and
            // onboarding could never reach its 3-sample minimum.
            if self.exemplar_budget_open.load(Ordering::Relaxed) > 0 {
                // Template is unavailable without touching the drain lock; the
                // worker consumes sample_log, so an empty template is fine here.
                self.dispatch_triage_exemplar(sig_hash as usize, String::new(), raw);
            }
            // Direct zero-copy parse using cached vendor extractor
            let activity = self.parse_with_format(raw, format);
            if !tier1_accounting {
                return (activity, None);
            }
            // Live ingest only: replay the hit through Drain so cluster
            // counts keep advancing (and the surge detector keeps firing)
            // after Tier-1 warms. Parse result is already in hand.
            let anomaly = self
                .drain
                .lock()
                .ok()
                .and_then(|mut miner| miner.add_log(raw).anomaly);
            return (activity, anomaly);
        }

        // ---------------------------------------------------------------------
        // TIER 1b: Promoted dynamic route (onboarded unknown shapes)
        // ---------------------------------------------------------------------
        // Shapes the registry learned skip both the Drain mutex and the full
        // registry scan. Gated by one atomic read: with no routes installed the
        // Tier-1-miss path pays ~1 ns.
        if self.dynamic_routes_open.load(Ordering::Relaxed) > 0 {
            let route = self
                .dynamic_routes
                .lock()
                .ok()
                .and_then(|routes| routes.get(&sig_hash).cloned());
            if let Some(key) = route {
                // Native owns known formats: a route may never shadow them
                // (signature-hash collisions could otherwise route a native line
                // to a dynamic parser).
                if self.parser.classify(raw) == VendorFormat::Unknown {
                    let parsed = self
                        .dynamic_registry
                        .lock()
                        .ok()
                        .and_then(|mut reg| reg.parse_key(&key, raw));
                    if let Some(activity) = parsed {
                        if !tier1_accounting {
                            return (activity, None);
                        }
                        // Same accounting replay as the Tier-1 path: onboarded
                        // shapes must advance Drain counts too.
                        let anomaly = self
                            .drain
                            .lock()
                            .ok()
                            .and_then(|mut miner| miner.add_log(raw).anomaly);
                        return (activity, anomaly);
                    }
                    // Stale route (parser evicted by the registry bound, or the
                    // shape drifted): drop it and fall through to Tier-2.
                    if let Ok(mut routes) = self.dynamic_routes.lock() {
                        if routes.remove(&sig_hash).is_some() {
                            self.dynamic_routes_open.fetch_sub(1, Ordering::Relaxed);
                        }
                    }
                }
                // Known format: fall through to the normal Tier-2/native path;
                // the route stays (it is valid for the shape it was learned on).
            }
        }

        // ---------------------------------------------------------------------
        // TIER 2: Cache Miss -> DrainDotNet Clustering Engine (~10 µs)
        // ---------------------------------------------------------------------
        let cluster_res = {
            let mut miner = self.drain.lock().unwrap();
            miner.add_log(raw)
        };

        if !cluster_res.is_new {
            // Matched known cluster template!
            self.tier2_drain_hits.fetch_add(1, Ordering::Relaxed);

            // Budgeted exemplars for Tier-3 (up to MAX_EXEMPLARS_PER_CLUSTER per
            // format key) so the worker can accumulate >= 3 samples and onboard.
            self.dispatch_triage_exemplar(sig_hash as usize, cluster_res.template, raw);

            // Pinned parse order: native → registry → lossless (promotes on success)
            let activity = self.parse_pinned(raw, sig_hash);
            let anomaly = if tier1_accounting {
                cluster_res.anomaly
            } else {
                None
            };
            return (activity, anomaly);
        }

        // ---------------------------------------------------------------------
        // TIER 3: Unseen Cluster Template -> Asynchronous Laya Dispatch
        // ---------------------------------------------------------------------
        // Guardrail 1 relaxed: exemplar budget per format key (see dispatch helper).
        // Keyed by sig_hash (not the Drain cluster id) so exemplars accumulate on
        // the same identity the Tier-1 fast path can compute without the drain lock.
        self.dispatch_triage_exemplar(sig_hash as usize, cluster_res.template, raw);

        // Pinned parse order on first sight too: the old path went straight to
        // lossless here, leaving the first line of every new cluster unparsed.
        let activity = self.parse_pinned(raw, sig_hash);
        let anomaly = if tier1_accounting {
            cluster_res.anomaly
        } else {
            None
        };
        (activity, anomaly)
    }

    /// Pinned parse order on every Tier-1 miss:
    /// native extractor → dynamic registry → lossless.
    ///
    /// Native always wins for known formats — a dynamic parser can never hijack
    /// (poison) a shape the native classifier recognizes; the registry serves
    /// unknown shapes only; lossless never drops a log. Successful parses
    /// promote (native → Tier-1 LRU, dynamic → Tier-1b route) so future
    /// same-shape lines skip the Drain mutex entirely.
    fn parse_pinned(&self, raw: &str, sig_hash: u64) -> NetworkActivity {
        // 1. Native extractor — known formats never reach the registry
        let format = self.parser.classify(raw);
        if format != VendorFormat::Unknown {
            return match self.parser.parse_with_format(raw, format) {
                Ok(activity) => {
                    // LRU promotion: future same-shape lines hit Tier-1 directly
                    self.parser.cache.insert(sig_hash, format);
                    activity
                }
                // Native owns known formats even when its extractor errs —
                // fall to lossless, never to a dynamic parser.
                Err(_) => self.parser.parse_lossless(raw),
            };
        }

        // 2. Dynamic registry — unknown shapes only (pinned order, full scan)
        let registry_hit = self
            .dynamic_registry
            .lock()
            .ok()
            .and_then(|mut reg| reg.parse_any_keyed(raw));
        if let Some((key, activity)) = registry_hit {
            // Promotion: future same-shape lines skip Drain + the registry scan
            if let Ok(mut routes) = self.dynamic_routes.lock() {
                if routes.insert(sig_hash, key).is_none() {
                    self.dynamic_routes_open.fetch_add(1, Ordering::Relaxed);
                }
            }
            return activity;
        }

        // 3. Lossless fallback — never drop a log
        self.parser.parse_lossless(raw)
    }

    /// Dispatch a Tier-3 exemplar within the per-format-key budget (bounded, non-blocking).
    /// Guardrail 1 becomes a budget of `MAX_EXEMPLARS_PER_CLUSTER` instead of exactly
    /// 1 — the old single-exemplar rule starved the onboarder, which needs >= 3 samples.
    /// All three process() paths share this key so a cluster never splits its budget.
    fn dispatch_triage_exemplar(&self, exemplar_key: usize, template: String, raw: &str) {
        let (within_budget, opened, closed) = match self.triage_dispatch_counts.lock() {
            Ok(mut counts) => {
                if let Some(entry) = counts.get_mut(&exemplar_key) {
                    if *entry >= MAX_EXEMPLARS_PER_CLUSTER {
                        (false, false, false)
                    } else {
                        *entry += 1;
                        (true, false, *entry >= MAX_EXEMPLARS_PER_CLUSTER)
                    }
                } else {
                    // Brand-new shape: evict the oldest key past the bound so
                    // a cardinality flood can't grow this map without limit.
                    // An evicted key still holding budget gives it back first.
                    if counts.len() >= MAX_TRIAGE_KEYS {
                        if let Ok(mut order) = self.triage_dispatch_order.lock() {
                            while let Some(old) = order.pop_front() {
                                if let Some(removed) = counts.remove(&old) {
                                    if removed < MAX_EXEMPLARS_PER_CLUSTER {
                                        self.exemplar_budget_open.fetch_sub(1, Ordering::Relaxed);
                                    }
                                    self.triage_evictions.fetch_add(1, Ordering::Relaxed);
                                    break;
                                }
                            }
                        }
                    }
                    counts.insert(exemplar_key, 1);
                    if let Ok(mut order) = self.triage_dispatch_order.lock() {
                        order.push_back(exemplar_key);
                    }
                    (true, true, false)
                }
            }
            Err(_) => (false, false, false),
        };
        if opened {
            self.exemplar_budget_open.fetch_add(1, Ordering::Relaxed);
        }
        if closed {
            self.exemplar_budget_open.fetch_sub(1, Ordering::Relaxed);
        }
        if !within_budget {
            return;
        }
        // Guardrail 2: Decoupled bounded ring buffer (non-blocking try_send)
        let _ = self.laya_sender.try_send(AsyncTriageTask {
            cluster_id: exemplar_key,
            template,
            sample_log: raw.to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        });
        self.tier3_laya_dispatches.fetch_add(1, Ordering::Relaxed);
    }

    /// Helper to parse raw log with verified VendorFormat
    #[inline]
    fn parse_with_format(&self, raw: &str, format: VendorFormat) -> NetworkActivity {
        match self.parser.parse_with_format(raw, format) {
            Ok(activity) => activity,
            Err(_) => self.parser.parse_lossless(raw),
        }
    }

    /// Current Tier-2 Drain cluster count (real telemetry for evaluator reports)
    pub fn tier2_cluster_count(&self) -> usize {
        self.drain
            .lock()
            .map(|drain| drain.cluster_count())
            .unwrap_or(0)
    }

    /// Query multi-tier statistics
    pub fn stats(&self) -> PipelineStats {
        let total = self.total_events.load(Ordering::Relaxed);
        let lru_stats = self.parser.cache_stats();
        let drain_hits = self.tier2_drain_hits.load(Ordering::Relaxed);
        let laya_dispatches = self.tier3_laya_dispatches.load(Ordering::Relaxed);
        let laya_onboarded = self.tier3_laya_onboarded.load(Ordering::Relaxed);

        PipelineStats {
            total_events: total,
            tier1_lru_hits: lru_stats.hits,
            tier2_drain_hits: drain_hits,
            tier3_laya_dispatches: laya_dispatches,
            tier3_laya_onboarded: laya_onboarded,
            laya_action_flags: self.laya_action_flags.load(Ordering::Relaxed),
            laya_threat_flags: self.laya_threat_flags.load(Ordering::Relaxed),
            cluster_evictions: self.cluster_evictions.load(Ordering::Relaxed),
            triage_evictions: self.triage_evictions.load(Ordering::Relaxed),
            lru_hit_ratio: lru_stats.hit_ratio,
            lru_stats,
        }
    }

    /// Current dispatch-table size (bounded at `MAX_TRIAGE_KEYS`)
    pub fn triage_table_len(&self) -> usize {
        self.triage_dispatch_counts
            .lock()
            .map(|counts| counts.len())
            .unwrap_or(0)
    }

    /// Access the dynamic parser registry
    pub fn dynamic_registry(&self) -> Arc<Mutex<DynamicParserRegistry>> {
        self.dynamic_registry.clone()
    }
}

impl Default for TieredPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drain::AnomalyType;

    /// Stale order entries must never evict a newer buffer for a reused id,
    /// and the queue must stay bounded under onboard-then-reuse flood churn.
    #[test]
    fn test_buffer_order_stale_entries_never_evict_newer_buffer() {
        // Focused case: id 1 was onboarded (stale head entry), then reused for
        // a newer buffer whose fresh entry sits at the tail. Id 2 is a pure
        // stale entry (onboarded, never reused). Id 3 is the true oldest live
        // buffer with no duplicate.
        let mut buffers: HashMap<usize, Vec<String>> = HashMap::new();
        buffers.insert(1, vec!["new-sample-a".to_string()]);
        buffers.insert(3, vec!["oldest-live".to_string()]);
        let mut order: VecDeque<usize> = VecDeque::from(vec![1, 2, 3, 1]);

        let evicted = pop_oldest_live_buffer(&mut buffers, &mut order);
        assert_eq!(
            evicted,
            Some(3),
            "must evict the oldest live id without a newer duplicate, not the reused id 1"
        );
        assert!(
            buffers.contains_key(&1),
            "stale head entry for reused id 1 must not discard its newer buffer"
        );
        assert!(
            !buffers.contains_key(&3),
            "oldest live buffer 3 must be the one evicted"
        );
    }

    /// Onboard-then-reuse flood: the order queue stays bounded (compacted to
    /// live keys, deduped) instead of growing one stale entry per onboard.
    #[test]
    fn test_buffer_order_stays_bounded_under_onboard_reuse_flood() {
        let mut buffers: HashMap<usize, Vec<String>> = HashMap::new();
        let mut order: VecDeque<usize> = VecDeque::new();

        // Fill to the cap.
        for id in 0..MAX_CLUSTER_BUFFERS {
            buffers.insert(id, vec![format!("sample-{id}")]);
            order.push_back(id);
        }
        // Steady-state churn the way the worker sees it: each cycle onboards
        // (removes) 100 oldest-live ids without touching the queue — leaving
        // stale entries — then inserts 100 fresh ids (plus a few reused ids
        // with stale+fresh duplicates). Buffers stay at the cap; only the
        // queue grows, which is exactly the leak being fixed.
        let mut next_id = MAX_CLUSTER_BUFFERS;
        let mut onboard_cursor = 0;
        while order.len() <= BUFFER_ORDER_COMPACT_LEN {
            // Onboard-remove 100 live ids.
            let mut removed = 0;
            while removed < 100 && onboard_cursor < next_id {
                if buffers.remove(&onboard_cursor).is_some() {
                    removed += 1;
                }
                onboard_cursor += 1;
            }
            // Reuse one just-onboarded id for a newer buffer (duplicate).
            let reused = onboard_cursor.saturating_sub(1);
            buffers.insert(reused, vec![format!("reused-{reused}")]);
            order.push_back(reused);
            // Insert fresh ids to refill to the cap.
            while buffers.len() < MAX_CLUSTER_BUFFERS {
                buffers.insert(next_id, vec![format!("flood-{next_id}")]);
                order.push_back(next_id);
                next_id += 1;
            }
        }
        assert!(
            order.len() > BUFFER_ORDER_COMPACT_LEN,
            "harness must actually exceed the compaction trigger, len={}",
            order.len()
        );
        compact_buffer_order(&mut order, &buffers);

        assert!(
            order.len() <= buffers.len(),
            "compacted queue must hold at most one entry per live buffer, queue={} buffers={}",
            order.len(),
            buffers.len()
        );
        assert!(
            order.len() <= MAX_CLUSTER_BUFFERS,
            "compacted queue must respect the cap, len={}",
            order.len()
        );
        let mut seen = std::collections::HashSet::with_capacity(order.len());
        for &key in order.iter() {
            assert!(
                buffers.contains_key(&key),
                "compacted queue must retain only live keys, found stale {key}"
            );
            assert!(
                seen.insert(key),
                "compacted queue must not hold duplicates, found {key} twice"
            );
        }
    }

    #[test]
    fn test_tiered_pipeline_lifecycle_and_promotion() {
        let pipeline = TieredPipeline::new();

        let log_asa1 = "%ASA-6-302013: Built inbound UDP connection 1001 for outside:1.1.1.1/53 to inside:2.2.2.2/53";
        let log_asa2 = "%ASA-6-302013: Built inbound UDP connection 1002 for outside:1.1.1.2/53 to inside:2.2.2.3/53";
        let log_fgt =
            r#"date=2026-09-21 time=14:00:02 devname="FGT-DC-EDGE" type="traffic" srcip=10.0.0.1"#;

        // 1st log: Misses LRU, enters Tier 2 DrainDotNet (new cluster #1), triggers async Laya task
        let act1 = pipeline.process(log_asa1);
        assert_eq!(act1.metadata.product.vendor_name, "Cisco");

        // 2nd log: Matches cluster #1 in DrainDotNet, promotes to Tier 1 LRU
        let act2 = pipeline.process(log_asa2);
        assert_eq!(act2.metadata.product.vendor_name, "Cisco");

        // 3rd log of same signature: Hits Tier 1 LRU directly!
        let _act3 = pipeline.process(log_asa1);

        // Process Fortigate
        let _act_fgt = pipeline.process(log_fgt);

        let stats = pipeline.stats();
        assert_eq!(stats.total_events, 4);
        assert!(
            stats.tier3_laya_dispatches >= 2,
            "Dispatched new clusters to Laya"
        );
    }

    /// `process_live` must parse byte-identically to `process` while surfacing
    /// the new-template anomaly `process` swallows (live `[SECURITY ALERT]`).
    #[test]
    fn test_process_live_matches_process_parse_and_reports_new_template() {
        let live = TieredPipeline::new();
        let plain = TieredPipeline::new();
        let log = "%ASA-6-302013: Built inbound UDP connection 9001 for outside:9.9.9.9/53 to inside:8.8.8.8/53";

        let (act_live, anomaly_first) = live.process_live(log);
        let act_plain = plain.process(log);
        // Clock/UUIDv7 metadata (time, event_id, ingest_time) is minted per
        // call by construction — everything derived from the line must match.
        let mut norm_live = act_live.clone();
        let mut norm_plain = act_plain.clone();
        for act in [&mut norm_live, &mut norm_plain] {
            act.time = 0;
            act.metadata.ingest_time = 0;
            act.metadata.event_id.clear();
        }
        assert_eq!(
            norm_live, norm_plain,
            "live accessor must not change parse output"
        );
        assert_eq!(act_live.metadata.product.vendor_name, "Cisco");
        let first = anomaly_first.expect("first sighting is a new template");
        assert_eq!(first.anomaly_type, AnomalyType::NewTemplate);

        // Second sighting of the same shape: Tier-1 hit, no new-template alert.
        let (act2, anomaly_second) = live.process_live(log);
        assert_eq!(act2.metadata.product.vendor_name, "Cisco");
        assert!(
            anomaly_second.is_none(),
            "repeat shape must not re-alert, got {anomaly_second:?}"
        );
    }

    /// The reason `process_live` exists: once Tier-1 warms, plain `process`
    /// stops feeding Drain, so the rare-cluster surge detector goes blind.
    /// The accounting replay must keep it firing on the live path.
    #[test]
    fn test_process_live_keeps_surge_detector_firing_after_tier1_warms() {
        // Surge needs a baseline: total > rare_threshold * 3 = 15 first.
        let filler = "%ASA-6-302013: Built inbound UDP connection 1001 for outside:1.1.1.1/53 to inside:2.2.2.2/53";
        // Distinct syslog tag => its own cluster, rare until hammered.
        let rare = "%ASA-4-106023: Deny udp src outside:203.0.113.9/53 dst inside:10.0.0.9/53 by access-group";

        let live = TieredPipeline::new();
        let plain = TieredPipeline::new();

        for _ in 0..16 {
            live.process_live(filler);
            plain.process(filler);
        }
        // Hammer the rare shape: lines 2+ are Tier-1 hits on both paths. Only
        // the live path replays them through Drain, so only it can reach the
        // count > 5 surge tripwire (burst in one ms => rate >> 3.0).
        let mut saw_surge_live = false;
        for _ in 0..8 {
            let (_, anomaly) = live.process_live(rare);
            if matches!(anomaly, Some(ref a) if a.anomaly_type == AnomalyType::RareClusterSurge) {
                saw_surge_live = true;
            }
            plain.process(rare);
        }

        assert!(
            saw_surge_live,
            "live path must fire RareClusterSurge for the hammered rare shape"
        );
        // The plain path fed Drain exactly once (first sighting); Tier-1
        // starved it after that — the blindness the live accessor fixes.
        // Read-only observation: `syslog_tag` is the P6.1 cluster anchor.
        let rare_count = plain
            .drain
            .lock()
            .unwrap()
            .all_clusters_by_frequency()
            .into_iter()
            .find(|c| {
                c.syslog_tag
                    .as_deref()
                    .is_some_and(|t| t.contains("106023"))
            })
            .map(|c| c.count)
            .unwrap_or(0);
        assert_eq!(
            rare_count, 1,
            "plain process path must leave the rare cluster frozen at first sight"
        );
    }
}
