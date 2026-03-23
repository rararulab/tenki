#!/usr/bin/env bash
# pipeline-demo.sh — Real pre-application flow using tenki + real opencli.
#
# Scenario:
# - Synthetic candidate profile: 3-year Python engineer
# - Target roles: Tokyo LLM/AI jobs from LinkedIn
# - Stop at tailor step (before export/apply)
#
# Requirements:
# - tenki on PATH
# - opencli on PATH and LinkedIn source usable in your environment
# - python3, git
#
# Usage:
#   ./examples/pipeline-demo.sh
#   QUERY="python llm ai" LOCATION="Tokyo" ./examples/pipeline-demo.sh
#   KEEP_TMP=1 ./examples/pipeline-demo.sh   # keep temp dirs for inspection

set -euo pipefail

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

require_cmd tenki
require_cmd opencli
require_cmd python3
require_cmd git

QUERY=${QUERY:-"python llm ai"}
LOCATION=${LOCATION:-"Tokyo"}
SOURCE=${SOURCE:-"linkedin"}
TOP_N=${TOP_N:-10}

TMP_DATA=$(mktemp -d)
TMP_RESUME=$(mktemp -d)

cleanup() {
  if [ "${KEEP_TMP:-0}" = "1" ]; then
    echo "KEEP_TMP=1, keeping temp dirs:"
    echo "  TENKI_DATA_DIR=$TMP_DATA"
    echo "  RESUME_REPO=$TMP_RESUME"
    return
  fi
  rm -rf "$TMP_DATA" "$TMP_RESUME"
}
trap cleanup EXIT

export TENKI_DATA_DIR="$TMP_DATA"

echo "=== tenki pre-application example ==="
echo "TENKI_DATA_DIR=$TENKI_DATA_DIR"
echo "QUERY=$QUERY | LOCATION=$LOCATION | SOURCE=$SOURCE"
echo

echo "--- Step 1: Create fake resume repo (3-year Python profile) ---"
cat > "$TMP_RESUME/resume.typ" <<'TYP'
= Alex Chen
Python Engineer (3 years)

- 3 years backend development in Python/FastAPI
- Built internal GenAI assistant with RAG and embeddings
- Productionized LLM APIs with monitoring and eval
TYP
(
  cd "$TMP_RESUME"
  git init -q
  git add resume.typ
  git commit -q -m "init synthetic resume profile"
)
echo "Resume repo: $TMP_RESUME"
echo

echo "--- Step 2: Initialize tenki + configure preferences ---"
tenki init
tenki config set resume.repo_path "$TMP_RESUME"
tenki config set resume.build_command "make pdf"
tenki config set resume.output_path "build/resume.pdf"
tenki config set preferences.query "$QUERY"
tenki config set preferences.location "$LOCATION"
tenki config set preferences.sources "$SOURCE"
# Force keyword fallback for analyze/tailor so this example does not depend on an agent CLI.
tenki config set agent.backend not-a-real-backend
echo

echo "--- Step 3: Discover real jobs via opencli ($SOURCE) ---"
tenki discover --source "$SOURCE" --query "$QUERY" --location "$LOCATION" --json
echo

echo "--- Step 4: Inject synthetic candidate profile into discovered jobs ---"
IDS=$(
  tenki app list --status discovered --json | python3 -c '
import sys, json
apps = json.load(sys.stdin)
for a in apps:
    print(a["id"][:8])
'
)

if [ -z "$IDS" ]; then
  echo "No discovered jobs found. Try a different QUERY/LOCATION and rerun." >&2
  exit 1
fi

while IFS= read -r id; do
  [ -z "$id" ] && continue
  tenki app update "$id" \
    --skills "Python,FastAPI,LLM,RAG,Prompt Engineering,Vector Database,Docker" \
    --notes "Synthetic profile: 3 years Python engineer targeting Tokyo LLM/AI roles" \
    --status bookmarked \
    --json >/dev/null
  echo "Profile injected: $id"
done <<< "$IDS"
echo

echo "--- Step 5: Score + tailor (stop before export/apply) ---"
tenki analyze --unscored --top-n "$TOP_N" --json
tenki tailor --untailored --top-n "$TOP_N" --json
echo

echo "--- Step 6: Pre-application review snapshot ---"
tenki app list --json | python3 -c '
import sys, json
apps = json.load(sys.stdin)
for a in apps:
    print(
        f"{a['id'][:8]} | {a['company']} | {a['position']} | "
        f"score={a.get('fitness_score')} | tailored={a.get('tailored_summary') is not None}"
    )
'
echo
echo "Done: flow stops here (pre-application). No export/apply executed."
