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

## Commands

| Command | Description |
|---------|-------------|
| `tenki init` | Initialize the database |
| `tenki app add\|list\|show\|update\|delete` | Manage applications |
| `tenki interview add\|update\|note\|list` | Track interviews |
| `tenki task add\|update\|done\|delete\|list` | Manage tasks and reminders |
| `tenki stage set\|list` | Track stage transitions |
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
