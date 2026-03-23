CREATE TABLE IF NOT EXISTS applications (
    id TEXT PRIMARY KEY,
    company TEXT NOT NULL,
    position TEXT NOT NULL,
    jd_url TEXT,
    jd_text TEXT,
    location TEXT,
    status TEXT NOT NULL DEFAULT 'bookmarked',
    stage TEXT,
    outcome TEXT,
    fitness_score REAL,
    fitness_notes TEXT,
    resume_typ TEXT,
    resume_pdf BLOB,
    -- enriched fields
    salary TEXT,
    salary_min REAL,
    salary_max REAL,
    salary_currency TEXT,
    job_type TEXT,
    is_remote INTEGER,
    job_level TEXT,
    skills TEXT,
    experience_range TEXT,
    source TEXT,
    company_url TEXT,
    notes TEXT,
    -- tailoring
    tailored_summary TEXT,
    tailored_headline TEXT,
    tailored_skills TEXT,
    -- timestamps
    applied_at DATETIME,
    closed_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS interviews (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    round INTEGER NOT NULL,
    type TEXT NOT NULL DEFAULT 'other',
    interviewer TEXT,
    scheduled_at DATETIME,
    status TEXT NOT NULL DEFAULT 'scheduled',
    outcome TEXT,
    duration_mins INTEGER,
    questions TEXT,
    notes TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS status_history (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    from_status TEXT NOT NULL,
    to_status TEXT NOT NULL,
    note TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

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

CREATE TABLE IF NOT EXISTS stage_events (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    from_stage TEXT,
    to_stage TEXT NOT NULL,
    occurred_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    metadata TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS migrations (
    version INTEGER PRIMARY KEY,
    applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_applications_status ON applications(status);
CREATE INDEX IF NOT EXISTS idx_applications_company ON applications(company);
CREATE INDEX IF NOT EXISTS idx_interviews_application ON interviews(application_id);
CREATE INDEX IF NOT EXISTS idx_status_history_application ON status_history(application_id);
CREATE INDEX IF NOT EXISTS idx_tasks_application ON tasks(application_id);
CREATE INDEX IF NOT EXISTS idx_tasks_due ON tasks(due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_completed ON tasks(is_completed);
CREATE INDEX IF NOT EXISTS idx_stage_events_application ON stage_events(application_id);
