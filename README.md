# tenki

Job application tracker CLI with agent-native JSON output.

Track applications, interviews, tasks, and stage transitions in a local SQLite database. Every command supports `--json` for scripting and AI agent integration.

## Install

```bash
cargo install tenki
```

Or build from source:

```bash
git clone https://github.com/rararulab/tenki.git
cd tenki
cargo build --release
```

## Quick Start

```bash
# Initialize the database
tenki init

# Add an application
tenki app add --company "Acme Corp" --position "Senior Engineer" \
  --location "Remote" --source linkedin --is-remote

# List all applications
tenki app list

# Move to interview stage
tenki stage set <app-id> technical --note "LC-style round scheduled"

# Schedule an interview
tenki interview add --app-id <app-id> --round 1 --type technical \
  --scheduled-at "2025-04-10T14:00:00"

# Add a follow-up task
tenki task add --app-id <app-id> --type follow-up "Send thank-you email" \
  --due-date 2025-04-11
```

IDs are 8-character prefixes of the full UUID (shown on creation).

## Automation Pipeline

tenki can automatically discover jobs, score fitness, tailor resumes, and generate PDFs — powered by [OpenCLI](https://github.com/jackwener/opencli) and an LLM agent backend.

### Prerequisites

```bash
# Install opencli for job discovery
cargo install opencli

# Install Typst for resume PDF rendering
# macOS: brew install typst
# other platforms: https://typst.app/docs/reference/cli/

# Configure agent backend (for scoring/tailoring)
tenki config set agent.backend claude
```

### Discover Jobs

```bash
# Discover from all sources (Boss直聘 + LinkedIn)
tenki discover --query "rust developer" --location "shanghai"

# Single source
tenki discover --source boss --query "后端开发" --limit 20

# JSON output for scripting
tenki discover --source linkedin --query "backend engineer" --json
```

Discovered jobs are imported as applications with status `discovered`. Duplicates (same `jd_url`) are skipped automatically.

### Batch Score & Tailor

```bash
# Score all unscored applications via LLM (falls back to keyword matching)
tenki analyze --unscored

# Score top 10 only
tenki analyze --unscored --top-n 10

# Tailor resumes for all scored but untailored applications
tenki tailor --untailored

# Single-item mode still works
tenki analyze <app-id>
tenki tailor <app-id>
```

### Full Pipeline

Run the entire flow in one command:

```bash
tenki pipeline run \
  --query "rust developer" \
  --location "shanghai" \
  --min-score 60 \
  --top-n 5 \
  --json
```

Pipeline steps:
1. **Discover** — crawl jobs via OpenCLI, import new ones
2. **Score** — analyze all unscored applications
3. **Filter** — keep top N above minimum score
4. **Tailor** — generate tailored resume content (skip with `--skip-tailor`)
5. **Export** — build PDF resumes via agent (skip with `--skip-export`)

### End-to-End Example (Stop Before Apply)

Goal: a synthetic 3-year Python candidate targeting Tokyo LLM/AI roles.
This walkthrough stops after tailoring (pre-application review).
The resume repo template lives at `examples/fake_resume_repo/` (Typst + Makefile).

Runnable Rust example (recommended):

```bash
cargo run --example pipeline_demo
# optional overrides
QUERY="python llm ai" LOCATION="Tokyo" cargo run --example pipeline_demo
```

Equivalent step-by-step commands:

```bash
# 1) Prepare fake resume repo and render a real PDF once
cp -R examples/fake_resume_repo ~/code/fake-resume-repo
(cd ~/code/fake-resume-repo && make pdf)

# 2) Configure resume repo (kept for later export/apply phase)
tenki config set resume.repo_path ~/code/fake-resume-repo
tenki config set resume.build_command "make pdf"
tenki config set resume.output_path build/resume.pdf

# 3) Set job preferences
tenki config set preferences.query "python llm ai"
tenki config set preferences.location "Tokyo"
tenki config set preferences.sources "linkedin"

# 4) Discover jobs from LinkedIn
tenki discover --source linkedin --query "python llm ai" --location "Tokyo"

# 5) Inject synthetic candidate profile (3-year Python resume) into discovered apps
for id in $(tenki app list --status discovered --json | jq -r '.[].id'); do
  short=${id:0:8}
  tenki app update "$short" \
    --skills "Python,FastAPI,LLM,RAG,Prompt Engineering,Vector Database,Docker" \
    --notes "Synthetic profile: 3 years Python engineer targeting Tokyo LLM/AI roles"
done

# 6) Score + tailor (stop here, before export/apply)
tenki analyze --unscored --top-n 10
tenki tailor --untailored --top-n 10

# 7) Pre-application review
tenki app list --json | jq '[.[] | {id: .id[0:8], company, position, score: .fitness_score, tailored_summary}]'
```

Notes:
- This flow intentionally does not run `export` or actual application submission.
- If you use `pipeline run` without `--sources`, it falls back to `preferences.sources`, then `linkedin`.

## Commands

| Command | Description |
|---------|-------------|
| `tenki init` | Initialize the database |
| `tenki discover` | Discover jobs from external sources via OpenCLI |
| `tenki app add\|list\|show\|update\|delete` | Manage applications |
| `tenki interview add\|update\|note\|list` | Track interviews |
| `tenki task add\|update\|done\|delete\|list` | Manage tasks and reminders |
| `tenki stage set\|list` | Track stage transitions |
| `tenki analyze <id>\|--unscored` | Score job fit (single or batch) |
| `tenki tailor <id>\|--untailored` | Tailor resume (single or batch) |
| `tenki pipeline run` | Run full automation pipeline |
| `tenki stats` | Aggregate statistics |
| `tenki timeline <id>` | Status change history |
| `tenki export <id> --typ\|--pdf` | Export resume |
| `tenki import <id> --typ <file>` | Import resume (Typst format) |
| `tenki config set\|get\|list` | Manage configuration |

Run `tenki --help` or `tenki <command> --help` for full usage details.

## JSON Output

Add `--json` to any command for machine-readable output:

```bash
# Structured JSON for scripting
tenki app list --json
tenki stats --json

# Errors also return JSON when --json is present
tenki app show bad-id --json
# {"ok":false,"error":"..."}
```

## Filtering

```bash
# Filter applications by status, company, outcome, stage, or source
tenki app list --status applied
tenki app list --company "Acme" --stage technical
tenki app list --outcome rejected

# List tasks for a specific app, or all pending tasks
tenki task list <app-id>
tenki task list
```

## Configuration

Data is stored at `~/.tenki/tenki.db` by default. Override with:

```bash
export TENKI_DATA_DIR=/path/to/custom/dir
```

Manage config values:

```bash
tenki config set example.setting value
tenki config get example.setting
tenki config list
```

## Development

```bash
cargo fmt                                          # Format
cargo clippy --all-targets --all-features -- -D warnings  # Lint
cargo test                                         # Test
cargo build                                        # Build
```

## License

MIT
