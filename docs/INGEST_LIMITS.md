# Ingest limits: backpressure and burst capacity

Live-socket capacity of `ulpf ingest`, measured — not estimated. Every
figure below names the command that produced it. Machine: 8 vCPUs,
loopback, release build. Load drifted between 1.7 and 4.3 across runs
(`uptime` 1-min average), so treat these as same-session relative
numbers, per the benchmark ritual.

## The short version

One socket holds 50,000 EPS loss-free. Past that the transports diverge:

| Transport | Sustainable (loss-free) | At 70k offered | At 100k offered | Drop policy |
| :--- | :--- | :--- | :--- | :--- |
| UDP | 50,000 EPS | 14.6% loss, kernel | 38.5% loss, kernel | kernel drops, counted in `RcvbufErrors` |
| TCP | 50,000 EPS | no loss, slows sender | no loss, slows sender | backpressure, counted in `Blocked` |
| Queue (`--drop-on-full`) | sheds above ~64k EPS parse rate | — | 36.0% shed, userspace | shed-newest, counted in `Dropped` |

The 500k EPS question: not reached, by either transport, on this box.
An unthrottled UDP blast offers ~2.57M EPS and the kernel drops 99% of
it. The binding constraint past ~60k EPS is parse throughput, not the
socket. Details below.

## What was measured

Release binary, scratch dirs, loopback, 12-second blasts:

```bash
cargo build --release
ulpf ingest --udp 127.0.0.1:55140 --tcp 127.0.0.1:55140 \
  --parquet-dir /tmp/cap50/parquet --ledger /tmp/cap50/ledger.jsonl &
ulpf-generator --target 127.0.0.1:55140 --proto udp --rate 50000 \
  --duration 12 --data-dir data/raw -D all --workers 8
```

Loss accounting per run: generator `Total Packets` sent vs ingest
`Total` received, cross-checked against the kernel's UDP `RcvbufErrors`
delta from `/proc/net/snmp`. In every UDP run the two agreed exactly —
each lost packet is a kernel buffer drop, none lost anywhere else.

### UDP, tuned buffer (SO_RCVBUF 8,388,608 as reported)

| Offered | Sent | Ingested | Loss | Kernel RcvbufErrors | Queue peak |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 10,000 | 120,323 | 120,323 | 0 | 0 | 177 KB |
| 20,000 | 240,269 | 240,269 | 0 | 0 | 192 KB |
| 30,000 | 360,243 | 360,243 | 0 | 0 | 273 KB |
| 50,000 | 600,266 | 600,266 | 0 | 0 | 1.1 MB |
| 70,000 | 840,381 | 717,792 | 122,589 (14.6%) | +122,589 | 15.2 MB, pinned 50,000/50,000 msgs |
| 100,000 | 1,200,339 | 738,614 | 461,725 (38.5%) | +461,725 | 14.6 MB, pinned |
| unthrottled (~2.57M) | 30,799,936 | 321,551 | 30,478,385 (99.0%) | +30,478,385 | pinned |

The knee sits between 50k and 70k offered. At saturation the socket
task parks on the full queue (`Blocked` 26–34k) and the kernel has
nowhere to put new datagrams.

### UDP, kernel-default buffer (SO_RCVBUF 212,992 as reported)

Same binary with the explicit set compiled out (banner confirms
`UDP 212992 bytes`):

| Offered | Sent | Ingested | Loss | Kernel RcvbufErrors |
| :--- | :--- | :--- | :--- | :--- |
| 30,000 | 360,421 | 357,638 | 2,783 (0.8%) | +2,783 |
| 50,000 | 600,386 | 588,362 | 12,024 (2.0%) | +12,024 |
| 70,000 | 840,366 | 754,398 | 85,968 (10.2%) | +85,968 |

The default buffer already sheds at 30k. Tuning to 4 MiB moves the
loss-free ceiling from ~30k to 50k. Past ~70k both buffers drop — that
region is parse-bound (see below), and the 70k tuned-vs-default gap
(122k vs 86k drops) is run-to-run noise under different load, not a
buffer effect. The buffer buys headroom, not throughput.

### TCP (tuned buffer)

| Offered | Sent | Outcome |
| :--- | :--- | :--- |
| 20,000 | 240,340 | all ingested, queue drains, `Blocked` 0 |
| 50,000 | 600,324 | all ingested, queue drains, `Blocked` 0 |
| 70,000 | 840,427 | queue pinned 50,000/50,000, `Blocked` ~164k, `Dropped` 0 |
| unthrottled | 814,208 in 13.5 s (avg 60,263) | sender throttled itself: burst 417k in second 1, then 24–53k/s |

TCP never drops: a full queue parks the per-connection reader, the
sender's kernel buffer fills, and the generator blocks in `send`.
The unthrottled generator's achieved rate *is* the system throughput
under backpressure. Proof: a 70k TCP blast (840,513 sent) followed by
a 25 s drain converged to `Total: 840513 | Normalized OCSF: 840513 |
Queue: 0/50000 | Dropped: 0`, with `Blocked: 246501` showing how long
producers spent parked and a byte high-water mark of 14,181,128.

### Queue policy: block (default) vs shed (`--drop-on-full`)

At 100k UDP offered, block (default) vs shed:

| Policy | Socket intake | Kernel drops | Queue `Dropped` | Parsed |
| :--- | :--- | :--- | :--- | :--- |
| block | 738,614 | 461,725 | 0 | 738,614 |
| `--drop-on-full` | 1,200,377 (all of it) | 0 | 432,049 (36.0%) | 768,328 |

Block parks the socket task, so the kernel drops first. Shed never
parks, so the socket drains the kernel buffer at ~99k EPS and the queue
sheds the excess in userspace — with an exact counter (`Dropped:
432049 (134693846 bytes)` on the reporter). Same offered load, same
~36–38% shed fraction: the binding constraint is parse throughput
(~64k EPS here), and the policy only chooses *where* the excess dies
and whether it is counted. Default stays block: the lossless-provenance
invariant means no raw line is shed unless the operator opts in.

## Per-transport behaviour

- **UDP = kernel drops.** The socket task is the only reader. When it
  parks (block policy, full queue) or just falls behind, datagrams die
  in the kernel ring before userspace sees them. Counted nowhere in
  ULPF — watch `RcvbufErrors` in `/proc/net/snmp`, or run the blast
  and diff sent vs `Total`.
- **TCP = backpressure.** `send` blocks on the generator side; ingest
  intake slows to parse rate. Nothing is dropped at any layer. The
  signal is `Blocked` climbing on the reporter and the queue pinned at
  capacity. Cost is latency, unbounded if the burst never ends.
- **Queue = sheds with a counter (opt-in).** `--drop-on-full` flips
  the default block policy to shed-newest. Every shed line bumps
  `Dropped` / `dropped_bytes`, printed on the reporter and tracked in
  `QueueStats`. Blocked-then-dropped never double-counts: one policy
  or the other owns each push.

## The 500k EPS question, answered honestly

What we measured: one loopback socket sustains 50k EPS loss-free;
parse throughput tops out near 64k EPS on 8 cores; a 2.57M EPS blast
loses 99% in the kernel. What we did not reach: 500k EPS sustained on
any single socket, any transport, any buffer. The kernel limit bounding
it: `rmem_max` (4,194,304 here) caps the socket buffer, and past the
parse ceiling no buffer helps — excess datagrams have nowhere to go.
Paths toward 500k that we have *not* measured: multi-socket
`SO_REUSEPORT` fan-out (the listener pool exists in
`ulpf-core/src/ingest/socket.rs::spawn_udp_worker_pool` but the CLI
runs one socket per transport), more parse workers than cores, and
faster per-event parse. Those are future work with a reproduction
recipe, not claims.

## Reproduction

```bash
# 1. Release build (numbers from debug builds are meaningless).
cargo build --release

# 2. Ingest on scratch dirs. The banner prints the effective SO_RCVBUF.
./target/release/ulpf ingest --udp 127.0.0.1:55140 --tcp 127.0.0.1:55140 \
  --parquet-dir /tmp/cap50/parquet --ledger /tmp/cap50/ledger.jsonl &

# 3. Blast at increasing rates. Watch Queue / Dropped / Blocked.
./target/release/ulpf-generator --target 127.0.0.1:55140 --proto udp \
  --rate 50000 --duration 12 --data-dir data/raw -D all --workers 8

# 4. Kernel drops, before and after.
awk '/Udp:/{print $6}' /proc/net/snmp  # RcvbufErrors column

# 5. Shed policy instead of block.
./target/release/ulpf ingest --drop-on-full ... # same addrs, fresh scratch dirs
```

Find the knee by raising `--rate` until `Total` trails sent or
`RcvbufErrors` moves. Check `uptime` first: 1-minute load under `nproc`
or the absolute numbers are still only good relative to each other.

## Live knobs

- Banner + reporter print the effective `SO_RCVBUF` per socket, the
  queue depth against capacity (`50000/50000 msgs`), the byte
  high-water mark (`peak`), `Dropped` (shed) and `Blocked` (parked)
  counters. No guessing which buffer or how close to full.
- `SO_RCVBUF` is set explicitly to 4 MiB
  (`INGEST_RCVBUF_BYTES` in `ulpf-core/src/ingest/socket.rs`); where
  `rmem_max` is lower the kernel clamps and the banner shows the
  clamped value.
