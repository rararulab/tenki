#!/usr/bin/env bash
# pipeline-demo.sh — End-to-end verification of tenki's automation pipeline.
#
# Runs entirely offline using keyword-fallback scoring (no LLM agent needed).
# Requires: tenki binary on PATH (cargo install --path . or cargo build --release).
#
# Usage:
#   ./examples/pipeline-demo.sh

set -euo pipefail

# ── Temp workspace ──────────────────────────────────────────────────
TMPDIR=$(mktemp -d)
export TENKI_DATA_DIR="$TMPDIR"
trap 'rm -rf "$TMPDIR"' EXIT

echo "=== tenki pipeline demo ==="
echo "Data dir: $TENKI_DATA_DIR"
echo

# ── 1. Initialize ──────────────────────────────────────────────────
echo "--- Step 1: Initialize database ---"
tenki init
echo

# ── 2. Add test applications ───────────────────────────────────────
echo "--- Step 2: Add test applications ---"

ID1=$(tenki app add \
  --company "ByteDance" \
  --position "Senior Rust Engineer" \
  --location "Shanghai" \
  --source boss \
  --is-remote \
  --json | python3 -c "import sys,json; print(json.load(sys.stdin)['id'][:8])")

tenki app update "$ID1" \
  --skills "Rust, Tokio, gRPC, distributed systems, performance optimization" \
  --jd-text "We are looking for a Senior Rust Engineer to build high-performance distributed systems. Requirements: 5+ years Rust, Tokio, gRPC, performance optimization, distributed systems experience."
echo "Added ByteDance: $ID1"

ID2=$(tenki app add \
  --company "Stripe" \
  --position "Backend Engineer" \
  --location "San Francisco" \
  --source linkedin \
  --json | python3 -c "import sys,json; print(json.load(sys.stdin)['id'][:8])")

tenki app update "$ID2" \
  --skills "Ruby, Python, API design" \
  --jd-text "Backend Engineer to work on payment processing APIs. Requirements: Ruby or Python, API design, SQL, microservices."
echo "Added Stripe: $ID2"

ID3=$(tenki app add \
  --company "Cloudflare" \
  --position "Systems Engineer" \
  --location "Remote" \
  --source linkedin \
  --is-remote \
  --json | python3 -c "import sys,json; print(json.load(sys.stdin)['id'][:8])")

tenki app update "$ID3" \
  --skills "Go, networking, Linux kernel" \
  --jd-text "Systems Engineer for edge network. Requirements: Go or C++, Linux networking, kernel development, CDN experience."
echo "Added Cloudflare: $ID3"
echo

# ── 3. Batch score (keyword fallback) ─────────────────────────────
echo "--- Step 3: Batch score (keyword fallback) ---"
tenki analyze --unscored
echo

echo "Scores after analysis:"
tenki app list --json | python3 -c "
import sys, json
apps = json.load(sys.stdin)
for a in apps:
    score = a.get('fitness_score', 'N/A')
    print(f\"  {a['company']:15s} {a['position']:25s} score={score}\")
"
echo

# ── 4. Batch tailor (keyword fallback) ────────────────────────────
echo "--- Step 4: Batch tailor (keyword fallback) ---"
tenki tailor --untailored
echo

# ── 5. Pipeline run (expects opencli-not-found) ──────────────────
echo "--- Step 5: Pipeline run (discover step — expects error without opencli) ---"
if tenki pipeline run --query "test" --location "anywhere" --skip-export --json 2>&1; then
  echo "(pipeline succeeded — opencli must be installed)"
else
  echo "(expected: opencli not found — discovery requires opencli)"
fi
echo

# ── 6. Summary ────────────────────────────────────────────────────
echo "--- Step 6: Summary ---"
tenki stats
echo
echo "Total applications:"
tenki app list --json | python3 -c "import sys,json; print(f'  {len(json.load(sys.stdin))} apps')"
echo
echo "=== Demo complete ==="
