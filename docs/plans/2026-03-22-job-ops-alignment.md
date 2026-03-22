# Job-Ops Feature Alignment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Align tenki's data model and CLI with job-ops features: enriched application fields, fine-grained stage system, outcome tracking, tasks/reminders, interview outcomes, and richer stats.

**Architecture:** Schema migration via `init` re-run (SQLite `IF NOT EXISTS` + `ALTER TABLE` migration script). New tables for tasks and stage_events. Existing tables get new columns. CLI gets new subcommands. All changes backward-compatible with existing data.

**Tech Stack:** Same as current — sqlx, clap, comfy-table, serde, snafu, uuid, chrono.

**Working directory:** `/Users/ryan/code/personal/tenki`

---

### Task 1: Schema Migration — Enrich Applications Table

**Files:**
- Create: `src/migrations/v2.sql`
- Modify: `src/schema.sql` — add new columns to CREATE TABLE
- Modify: `src/db.rs` — update `Database::init()` to run migrations

**Step 1: Create migration file `src/migrations/v2.sql`**

This handles existing databases. New databases get the columns from schema.sql directly.

```sql
-- v2: Enrich applications with job-ops fields
ALTER TABLE applications ADD COLUMN salary TEXT;
ALTER TABLE applications ADD COLUMN salary_min REAL;
ALTER TABLE applications ADD COLUMN salary_max REAL;
ALTER TABLE applications ADD COLUMN salary_currency TEXT;
ALTER TABLE applications ADD COLUMN job_type TEXT;        -- full-time, part-time, contract, internship
ALTER TABLE applications ADD COLUMN is_remote INTEGER;    -- 0/1
ALTER TABLE applications ADD COLUMN job_level TEXT;        -- junior, mid, senior, lead, staff, principal
ALTER TABLE applications ADD COLUMN skills TEXT;           -- JSON array
ALTER TABLE applications ADD COLUMN experience_range TEXT; -- e.g. "3-5 years"
ALTER TABLE applications ADD COLUMN outcome TEXT;          -- offer_accepted, offer_declined, rejected, withdrawn, no_response, ghosted
ALTER TABLE applications ADD COLUMN source TEXT;           -- linkedin, indeed, referral, company_site, etc.
ALTER TABLE applications ADD COLUMN applied_at DATETIME;
ALTER TABLE applications ADD COLUMN closed_at DATETIME;
ALTER TABLE applications ADD COLUMN tailored_summary TEXT;
ALTER TABLE applications ADD COLUMN tailored_headline TEXT;
ALTER TABLE applications ADD COLUMN tailored_skills TEXT;  -- JSON array
ALTER TABLE applications ADD COLUMN company_url TEXT;
ALTER TABLE applications ADD COLUMN notes TEXT;
```

**Step 2: Update `src/schema.sql` to include new columns in CREATE TABLE**

Add all new columns to the `applications` CREATE TABLE statement so new databases get them directly.

**Step 3: Update `Database::init()` in `src/db.rs`**

After running schema.sql, run the migration. Use a `migrations` table to track which migrations have been applied:

```sql
CREATE TABLE IF NOT EXISTS migrations (
    version INTEGER PRIMARY KEY,
    applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

For each migration file, check if version exists, if not, run it and record. Wrap each ALTER TABLE in a try (SQLite errors on duplicate column, catch and ignore).

**Step 4: Verify**

Run: `cargo check`

**Step 5: Commit**

```bash
git commit -m "feat: enrich applications schema with job-ops fields"
```

---

### Task 2: Schema — Tasks Table

**Files:**
- Modify: `src/schema.sql` — add tasks table
- Modify: `src/migrations/v2.sql` — add tasks table creation

**Step 1: Add to schema.sql and v2.sql**

```sql
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    type TEXT NOT NULL DEFAULT 'todo',  -- prep, todo, follow_up, check_status
    title TEXT NOT NULL,
    due_date DATETIME,
    is_completed INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_tasks_application ON tasks(application_id);
CREATE INDEX IF NOT EXISTS idx_tasks_due ON tasks(due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_completed ON tasks(is_completed);
```

**Step 2: Verify**

Run: `cargo check`

**Step 3: Commit**

```bash
git commit -m "feat: add tasks table for reminders and todos"
```

---

### Task 3: Schema — Stage Events Table

**Files:**
- Modify: `src/schema.sql` — add stage_events table
- Modify: `src/migrations/v2.sql` — add stage_events table creation

**Step 1: Add to schema.sql and v2.sql**

```sql
CREATE TABLE IF NOT EXISTS stage_events (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    from_stage TEXT,
    to_stage TEXT NOT NULL,   -- applied, recruiter_screen, assessment, hiring_manager, technical, onsite, offer, closed
    occurred_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    metadata TEXT,            -- JSON: { note, actor, event_type, external_url }
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_stage_events_application ON stage_events(application_id);
```

Also add `stage` column to applications:

```sql
ALTER TABLE applications ADD COLUMN stage TEXT DEFAULT 'applied';
```

And to schema.sql CREATE TABLE.

**Step 2: Add interview outcome column**

Add to interviews table:

```sql
ALTER TABLE interviews ADD COLUMN outcome TEXT; -- pass, fail, pending, cancelled
ALTER TABLE interviews ADD COLUMN duration_mins INTEGER;
```

And update schema.sql CREATE TABLE to include these.

**Step 3: Verify**

Run: `cargo check`

**Step 4: Commit**

```bash
git commit -m "feat: add stage_events table and interview outcome"
```

---

### Task 4: Update Enums and Domain Structs

**Files:**
- Modify: `src/db.rs` — update/add enums and structs

**Step 1: Update AppStatus enum**

Keep as-is (this is the high-level status). It's fine.

**Step 2: Add new enums**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    OfferAccepted,
    OfferDeclined,
    Rejected,
    Withdrawn,
    NoResponse,
    Ghosted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Applied,
    RecruiterScreen,
    Assessment,
    HiringManager,
    Technical,
    Onsite,
    Offer,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    FullTime,
    PartTime,
    Contract,
    Internship,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobLevel {
    Junior,
    Mid,
    Senior,
    Lead,
    Staff,
    Principal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Prep,
    Todo,
    FollowUp,
    CheckStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InterviewOutcome {
    Pass,
    Fail,
    Pending,
    Cancelled,
}
```

Each enum needs `as_str()`, `Display`, `FromStr` impls following the existing pattern.

**Step 3: Update Application struct**

Add all new fields:

```rust
pub struct Application {
    // existing fields...
    pub salary: Option<String>,
    pub salary_min: Option<f64>,
    pub salary_max: Option<f64>,
    pub salary_currency: Option<String>,
    pub job_type: Option<String>,
    pub is_remote: Option<bool>,
    pub job_level: Option<String>,
    pub skills: Option<String>,         // JSON array
    pub experience_range: Option<String>,
    pub outcome: Option<String>,
    pub stage: Option<String>,
    pub source: Option<String>,
    pub applied_at: Option<String>,
    pub closed_at: Option<String>,
    pub tailored_summary: Option<String>,
    pub tailored_headline: Option<String>,
    pub tailored_skills: Option<String>, // JSON array
    pub company_url: Option<String>,
    pub notes: Option<String>,
}
```

**Step 4: Add new domain structs**

```rust
#[derive(Debug, Clone, Serialize)]
pub struct TaskRow {
    pub id: String,
    pub application_id: String,
    pub r#type: String,
    pub title: String,
    pub due_date: Option<String>,
    pub is_completed: bool,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StageEvent {
    pub id: String,
    pub application_id: String,
    pub from_stage: Option<String>,
    pub to_stage: String,
    pub occurred_at: String,
    pub metadata: Option<String>, // JSON
    pub created_at: String,
}
```

**Step 5: Update InterviewRow**

Add `outcome: Option<String>` and `duration_mins: Option<i64>`.

**Step 6: Update Stats struct**

```rust
pub struct Stats {
    pub total: usize,
    pub by_status: Vec<(String, usize)>,
    pub by_outcome: Vec<(String, usize)>,
    pub by_stage: Vec<(String, usize)>,
    pub by_source: Vec<(String, usize)>,
    pub pending_tasks: usize,
    pub upcoming_interviews: usize,
}
```

**Step 7: Verify**

Run: `cargo check`
Expected: Errors from CLI handlers not matching new struct fields — that's fine, fixed in later tasks.

**Step 8: Commit**

```bash
git commit -m "feat: add enums and structs for stages, outcomes, tasks"
```

---

### Task 5: Update Database CRUD — Applications

**Files:**
- Modify: `src/db.rs` — update application queries

**Step 1: Update all application SQL queries**

Every query that touches `applications` needs the new columns:
- `add_application()` — add optional params for new fields
- `get_application()` — SELECT all new columns
- `list_applications()` — SELECT all new columns, add filters for `outcome`, `stage`, `source`
- `update_application_fields()` — handle all new optional fields
- Add `update_application_outcome()` — set outcome + closed_at + update status to appropriate terminal state
- Add `update_application_stage()` — set stage + record stage_event
- `stats()` — add by_outcome, by_stage, by_source GROUP BY queries, pending_tasks count, upcoming_interviews count

**Step 2: Verify**

Run: `cargo check`

**Step 3: Commit**

```bash
git commit -m "feat: update application CRUD for enriched fields"
```

---

### Task 6: Database CRUD — Tasks

**Files:**
- Modify: `src/db.rs` — add task operations

**Step 1: Add task methods to Database**

```rust
// Task CRUD
pub async fn add_task(application_id, task_type, title, due_date?, notes?) -> Result<String>
pub async fn update_task(id, title?, due_date?, notes?, is_completed?) -> Result<()>
pub async fn complete_task(id) -> Result<()>
pub async fn delete_task(id) -> Result<()>
pub async fn list_tasks(application_id) -> Result<Vec<TaskRow>>
pub async fn list_all_pending_tasks() -> Result<Vec<TaskRow>>  // across all apps, ordered by due_date
pub async fn resolve_task_id(prefix) -> Result<String>
```

**Step 2: Verify**

Run: `cargo check`

**Step 3: Commit**

```bash
git commit -m "feat: task CRUD operations"
```

---

### Task 7: Database CRUD — Stage Events

**Files:**
- Modify: `src/db.rs` — add stage event operations

**Step 1: Add stage event methods to Database**

```rust
pub async fn record_stage_event(application_id, from_stage?, to_stage, metadata_json?) -> Result<String>
pub async fn transition_stage(application_id, to_stage) -> Result<()>
    // Gets current stage, records event, updates application.stage
pub async fn list_stage_events(application_id) -> Result<Vec<StageEvent>>
```

Also update `update_interview_status()` to accept optional outcome:

```rust
pub async fn update_interview(id, status?, outcome?, interviewer?, scheduled_at?, duration_mins?) -> Result<()>
```

**Step 2: Verify**

Run: `cargo check`

**Step 3: Commit**

```bash
git commit -m "feat: stage event tracking and interview outcome"
```

---

### Task 8: CLI — Update App Commands

**Files:**
- Modify: `src/cli/mod.rs` — update AppCommand args
- Modify: `src/cli/app.rs` — update handlers

**Step 1: Update AppCommand::Add**

Add optional args: `--salary`, `--job-type`, `--job-level`, `--is-remote`, `--source`, `--company-url`, `--notes`

**Step 2: Update AppCommand::Update**

Add optional args: `--outcome`, `--stage`, `--salary`, `--job-type`, `--job-level`, `--is-remote`, `--source`, `--notes`

**Step 3: Update AppCommand::List**

Add filters: `--outcome`, `--stage`, `--source`

**Step 4: Update app::show()**

Display all new fields.

**Step 5: Update app::list() table**

Add Stage and Outcome columns. Keep table readable by showing only: ID, Company, Position, Status, Stage, Outcome, Fitness, Updated.

**Step 6: Verify**

Run: `cargo check`

**Step 7: Commit**

```bash
git commit -m "feat: CLI app commands with enriched fields"
```

---

### Task 9: CLI — Task Commands

**Files:**
- Create: `src/cli/task.rs`
- Modify: `src/cli/mod.rs` — add TaskCommand
- Modify: `src/main.rs` — wire up Task dispatch

**Step 1: Add TaskCommand to cli/mod.rs**

```rust
#[derive(Subcommand)]
pub enum TaskCommand {
    Add {
        #[arg(long)] app_id: String,
        #[arg(long, value_enum, default_value_t = TaskType::Todo)] r#type: TaskType,
        title: String,
        #[arg(long)] due_date: Option<String>,
        #[arg(long)] notes: Option<String>,
        #[arg(long)] json: bool,
    },
    Update {
        id: String,
        #[arg(long)] title: Option<String>,
        #[arg(long)] due_date: Option<String>,
        #[arg(long)] notes: Option<String>,
        #[arg(long)] json: bool,
    },
    Done {
        id: String,
        #[arg(long)] json: bool,
    },
    Delete {
        id: String,
        #[arg(long)] json: bool,
    },
    List {
        /// List tasks for a specific app, or all pending if omitted
        app_id: Option<String>,
        #[arg(long)] json: bool,
    },
}
```

**Step 2: Create cli/task.rs**

Implement handlers: add, update, done, delete, list. Table columns: ID(8), App(8), Type, Title, Due, Status.

**Step 3: Wire up in main.rs**

Add `Command::Task(TaskCommand)` and dispatch.

**Step 4: Verify**

Run: `cargo check`

**Step 5: Commit**

```bash
git commit -m "feat: CLI task commands for reminders and todos"
```

---

### Task 10: CLI — Stage Commands

**Files:**
- Create: `src/cli/stage.rs`
- Modify: `src/cli/mod.rs` — add StageCommand
- Modify: `src/main.rs` — wire up

**Step 1: Add to cli/mod.rs**

```rust
#[derive(Subcommand)]
pub enum StageCommand {
    /// Transition application to a new stage
    Set {
        app_id: String,
        #[arg(value_enum)] stage: Stage,
        #[arg(long)] note: Option<String>,
        #[arg(long)] json: bool,
    },
    /// List stage events for an application
    List {
        app_id: String,
        #[arg(long)] json: bool,
    },
}
```

**Step 2: Create cli/stage.rs**

Implement handlers. Table columns: From, To, Note, Time.

**Step 3: Wire up in main.rs**

**Step 4: Verify**

Run: `cargo check`

**Step 5: Commit**

```bash
git commit -m "feat: CLI stage commands for pipeline tracking"
```

---

### Task 11: Update Interview Commands

**Files:**
- Modify: `src/cli/mod.rs` — update InterviewCommand
- Modify: `src/cli/interview.rs` — update handlers

**Step 1: Update InterviewCommand::Add**

Add `--duration-mins` arg.

**Step 2: Update InterviewCommand::Update**

Add `--outcome` arg (value_enum `InterviewOutcome`).

**Step 3: Update interview::list() table**

Add Outcome column.

**Step 4: Verify and commit**

```bash
git commit -m "feat: interview outcome tracking"
```

---

### Task 12: Update Stats

**Files:**
- Modify: `src/cli/stats.rs` — richer stats display

**Step 1: Update stats handler**

Show sections: By Status, By Stage, By Outcome, By Source, Pending Tasks, Upcoming Interviews.

**Step 2: Verify and commit**

```bash
git commit -m "feat: enriched stats with stages, outcomes, sources"
```

---

### Task 13: Update Main.rs Dispatch & Wiring

**Files:**
- Modify: `src/main.rs` — ensure all new commands are wired

**Step 1: Wire up all new commands**

Ensure `Command::Task`, `Command::Stage` are dispatched. Update existing dispatches for new args.

**Step 2: Full build and smoke test**

```bash
cargo build
cargo run -- init
cargo run -- app add --company Google --position SRE --location Singapore --source linkedin --job-type full-time --job-level senior
cargo run -- app list
cargo run -- task add --app-id <id> --type prep "Review system design fundamentals"
cargo run -- task list
cargo run -- stage set <id> recruiter-screen
cargo run -- stage list <id>
cargo run -- interview add --app-id <id> --round 1 --type technical --duration-mins 60
cargo run -- interview update <id> --outcome pass
cargo run -- app update <id> --outcome offer-accepted
cargo run -- stats
```

**Step 3: Fix any issues**

**Step 4: Commit**

```bash
git commit -m "feat: wire up all new commands and smoke test"
```

---

## Summary

| Task | Description | Impact |
|------|-------------|--------|
| 1 | Enrich applications schema | New columns + migration system |
| 2 | Tasks table | New table |
| 3 | Stage events table + interview outcome | New table + column |
| 4 | Enums and domain structs | Rust types for new fields |
| 5 | Application CRUD updates | DB layer |
| 6 | Task CRUD | DB layer |
| 7 | Stage event CRUD | DB layer |
| 8 | CLI app command updates | CLI layer |
| 9 | CLI task commands | New CLI subcommand |
| 10 | CLI stage commands | New CLI subcommand |
| 11 | Interview command updates | CLI layer |
| 12 | Stats updates | CLI layer |
| 13 | Wiring + smoke test | Integration |
