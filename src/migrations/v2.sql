-- v2: Enrich applications with job-ops fields + new tables

-- New application columns
ALTER TABLE applications ADD COLUMN salary TEXT;
ALTER TABLE applications ADD COLUMN salary_min REAL;
ALTER TABLE applications ADD COLUMN salary_max REAL;
ALTER TABLE applications ADD COLUMN salary_currency TEXT;
ALTER TABLE applications ADD COLUMN job_type TEXT;
ALTER TABLE applications ADD COLUMN is_remote INTEGER;
ALTER TABLE applications ADD COLUMN job_level TEXT;
ALTER TABLE applications ADD COLUMN skills TEXT;
ALTER TABLE applications ADD COLUMN experience_range TEXT;
ALTER TABLE applications ADD COLUMN outcome TEXT;
ALTER TABLE applications ADD COLUMN stage TEXT DEFAULT 'applied';
ALTER TABLE applications ADD COLUMN source TEXT;
ALTER TABLE applications ADD COLUMN applied_at DATETIME;
ALTER TABLE applications ADD COLUMN closed_at DATETIME;
ALTER TABLE applications ADD COLUMN tailored_summary TEXT;
ALTER TABLE applications ADD COLUMN tailored_headline TEXT;
ALTER TABLE applications ADD COLUMN tailored_skills TEXT;
ALTER TABLE applications ADD COLUMN company_url TEXT;
ALTER TABLE applications ADD COLUMN notes TEXT;

-- New interview columns
ALTER TABLE interviews ADD COLUMN outcome TEXT;
ALTER TABLE interviews ADD COLUMN duration_mins INTEGER;

-- Tasks table
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    type TEXT NOT NULL DEFAULT 'todo',
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

-- Stage events table
CREATE TABLE IF NOT EXISTS stage_events (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    from_stage TEXT,
    to_stage TEXT NOT NULL,
    occurred_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    metadata TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_stage_events_application ON stage_events(application_id);

-- Migrations tracking table
CREATE TABLE IF NOT EXISTS migrations (
    version INTEGER PRIMARY KEY,
    applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
