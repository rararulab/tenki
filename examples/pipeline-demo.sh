#!/usr/bin/env bash
# pipeline-demo.sh — End-to-end automation pipeline demo
#
# This script demonstrates the full tenki pipeline without requiring opencli
# or an LLM backend. It manually inserts test data, then exercises every
# batch and pipeline command to verify the flow works.
#
# Usage:
#   ./examples/pipeline-demo.sh
#
# Prerequisites:
#   cargo build (tenki binary must be available)

set -euo pipefail

TENKI="${TENKI:-cargo run --quiet --}"
TMPDB="$(mktemp -d)/tenki-demo.db"
export TENKI_DATA_DIR="$(dirname "$TMPDB")"

cleanup() { rm -rf "$(dirname "$TMPDB")"; }
trap cleanup EXIT

echo "=== tenki pipeline demo ==="
echo "Using temp DB: $TMPDB"
echo

# ── Step 0: Initialize ──────────────────────────────────────────────
echo ">>> tenki init"
$TENKI init
echo

# ── Step 1: Simulate discovered jobs (what 'tenki discover' would do) ─
echo ">>> Adding test applications (simulating discovery)..."

SKILLS="Rust, Python, Docker, Kubernetes, async programming"

ID1=$($TENKI app add \
  --company "ByteDance" --position "Rust Developer" \
  --location "Shanghai" --source boss \
  --jd-text "We need a Rust developer with experience in distributed systems, async programming, and Linux. Knowledge of Python, Docker, Kubernetes is a plus." \
  --json | python3 -c "import sys,json; print(json.load(sys.stdin)['id'][:8])")
$TENKI app update "$ID1" --skills "$SKILLS" --json > /dev/null

ID2=$($TENKI app add \
  --company "Stripe" --position "Backend Engineer" \
  --location "Remote" --source linkedin \
  --jd-text "Build payment APIs using Go and PostgreSQL. Experience with microservices, gRPC, and cloud infrastructure required. Rust knowledge is a bonus." \
  --json | python3 -c "import sys,json; print(json.load(sys.stdin)['id'][:8])")
$TENKI app update "$ID2" --skills "$SKILLS" --json > /dev/null

ID3=$($TENKI app add \
  --company "Cloudflare" --position "Systems Engineer" \
  --location "Austin, TX" --source linkedin \
  --jd-text "Work on our edge network in Rust. Strong systems programming skills, networking protocols, and performance optimization experience required." \
  --json | python3 -c "import sys,json; print(json.load(sys.stdin)['id'][:8])")
$TENKI app update "$ID3" --skills "$SKILLS" --json > /dev/null

echo "  Created: $ID1 (ByteDance), $ID2 (Stripe), $ID3 (Cloudflare)"
echo

# ── Step 2: Batch analyze (keyword fallback — no LLM needed) ────────
echo ">>> tenki analyze --unscored (batch mode, keyword fallback)"
$TENKI analyze --unscored
echo

# ── Step 3: Verify scores ──────────────────────────────────────────
echo ">>> Checking scores..."
for ID in "$ID1" "$ID2" "$ID3"; do
  SCORE=$($TENKI app show "$ID" --json | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('fitness_score','none'))")
  COMPANY=$($TENKI app show "$ID" --json | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('company','?'))")
  echo "  $COMPANY: score=$SCORE"
done
echo

# ── Step 4: Batch tailor (keyword fallback — no LLM needed) ────────
echo ">>> tenki tailor --untailored (batch mode, keyword fallback)"
$TENKI tailor --untailored
echo

# ── Step 5: Verify tailoring ───────────────────────────────────────
echo ">>> Checking tailored content..."
for ID in "$ID1" "$ID2" "$ID3"; do
  HEADLINE=$($TENKI app show "$ID" --json | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('tailored_headline','none'))")
  COMPANY=$($TENKI app show "$ID" --json | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('company','?'))")
  echo "  $COMPANY: headline=\"$HEADLINE\""
done
echo

# ── Step 6: Pipeline run (skip discover + export, exercise orchestration) ─
echo ">>> tenki pipeline run --query test --skip-export --json"
echo "(This will fail at the discover step since opencli is not installed,"
echo " which is expected in a demo environment.)"
echo
$TENKI pipeline run --query "test" --skip-export --json 2>/dev/null || echo '  (expected: opencli not found error)'
echo

# ── Step 7: Stats ──────────────────────────────────────────────────
echo ">>> tenki stats --json"
$TENKI stats --json | python3 -m json.tool
echo

# ── Step 8: Full app list ──────────────────────────────────────────
echo ">>> tenki app list --json (summary)"
COUNT=$($TENKI app list --json | python3 -c "import sys,json; data=json.load(sys.stdin); print(len(data) if isinstance(data, list) else len(data.get('applications', [])))")
echo "  Total applications: $COUNT"
echo

echo "=== Demo complete ==="
echo "All batch commands executed successfully."
echo "The pipeline 'discover' step requires opencli to be installed."
