#!/usr/bin/env bash
# bench.sh — enforce the benchmark ritual (AGENTS.md Gotchas; futurescope §4).
#
# Runs the three corpora (core, adversarial, holdout) N times (default 3) via
# the release binary (built if missing) and prints, per corpus, the median +
# spread of the same-run baseline-vs-tiered throughput ratio. Absolute µs are
# never quoted without their baseline. Warns (never fails) when the machine
# is loaded. Bash + coreutils/awk only, no new deps.
#
# Usage: scripts/bench.sh [-n RUNS] [-t THREADS] [--duration S] [--samples N] [--dry-run]
set -u

RUNS=3; THREADS=""; DURATION=3; SAMPLES=10000; DRY_RUN=0
BIN="./target/release/ulpf"

usage() {
  sed -n '2,10p' "$0"
  echo "Options: -n RUNS  -t THREADS  --duration S  --samples N  --dry-run  -h|--help"
  exit "${1:-0}"
}

while [ $# -gt 0 ]; do
  case "$1" in
    -n) RUNS="$2"; shift 2 ;;
    -t|--threads) THREADS="$2"; shift 2 ;;
    --duration) DURATION="$2"; shift 2 ;;
    --samples) SAMPLES="$2"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage 0 ;;
    *) echo "ERROR: unknown arg $1" >&2; usage 1 ;;
  esac
done

NCPU=$(nproc 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null || echo 16)
[ -z "$THREADS" ] && THREADS="$NCPU"

# Idle check — warn only, never fail.
if [ -r /proc/loadavg ]; then LOAD1=$(awk '{print $1}' /proc/loadavg)
else LOAD1=$(uptime | sed 's/.*average: \([0-9.]*\).*/\1/'); fi
if awk -v l="$LOAD1" -v n="$NCPU" 'BEGIN{exit !(l+0 >= n+0)}'; then
  echo "WARN: machine loaded (1-min loadavg $LOAD1 >= nproc $NCPU) — numbers not comparable across runs" >&2
else
  echo "idle check OK (1-min loadavg $LOAD1 < nproc $NCPU)"
fi

if [ "$DRY_RUN" = "1" ]; then
  echo "dry-run: would run corpora core/adversarial/holdout x$RUNS via $BIN"
  echo "dry-run: evaluate --engine all --corpus <c> --duration $DURATION --threads $THREADS --samples $SAMPLES"
  exit 0
fi

if [ ! -x "$BIN" ]; then
  echo "release binary missing — building..."
  cargo build --release || exit 1
fi

# Thread-count flag passthrough check.
if ! "$BIN" evaluate --help 2>&1 | grep -q -- '--threads'; then
  echo "ERROR: release binary lacks --threads passthrough" >&2; exit 1
fi
echo "threads: $THREADS (--threads passthrough OK)"

TMPD=$(mktemp -d); trap 'rm -rf "$TMPD"' EXIT INT TERM

median_spread() { printf '%s\n' "$@" | sort -n | awk '{a[NR]=$1} END{if(NR%2){m=a[(NR+1)/2]}else{m=(a[NR/2]+a[NR/2+1])/2} printf "%s (spread %s)", m, a[NR]-a[1]}'; }

for CORPUS in core adversarial holdout; do
  RATIOS=""; i=1
  while [ "$i" -le "$RUNS" ]; do
    OUT="$TMPD/${CORPUS}_$i.md"
    "$BIN" evaluate --engine all --corpus "$CORPUS" --duration "$DURATION" \
      --threads "$THREADS" --samples "$SAMPLES" --out "$OUT" >/dev/null 2>&1 \
      || { echo "ERROR: evaluate failed ($CORPUS run $i)" >&2; exit 1; }
    LINE=$(grep -m1 'Throughput (EPS)' "$OUT")
    B=$(printf '%s' "$LINE" | awk -F'|' '{print $3}' | tr -cd '0-9')
    T=$(printf '%s' "$LINE" | awk -F'|' '{print $4}' | tr -cd '0-9')
    R=$(awk -v t="$T" -v b="$B" 'BEGIN{printf "%.4f", (b>0 ? t/b : 0)}')
    RATIOS="$RATIOS $R"
    echo "  $CORPUS run $i: baseline ${B} EPS, tiered ${T} EPS, ratio ${R}x"
    i=$((i + 1))
  done
  # shellcheck disable=SC2086
  echo "$CORPUS: same-run tiered/baseline ratio median $(median_spread $RATIOS) over $RUNS runs (threads=$THREADS)"
done
