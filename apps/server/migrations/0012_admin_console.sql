-- Estrutura de suporte ao console admin completo.

ALTER TABLE users ADD COLUMN blocked_at TEXT;
ALTER TABLE users ADD COLUMN blocked_reason TEXT;
ALTER TABLE users ADD COLUMN blocked_by TEXT;

ALTER TABLE pools ADD COLUMN description TEXT NOT NULL DEFAULT '';
ALTER TABLE pools ADD COLUMN visible_rules TEXT NOT NULL DEFAULT '';
ALTER TABLE pools ADD COLUMN join_closed_at TEXT;

CREATE TABLE prediction_admin_overrides (
    id TEXT PRIMARY KEY,
    match_id TEXT NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason TEXT NOT NULL DEFAULT '',
    reopened_by TEXT NOT NULL REFERENCES users(id),
    expires_at TEXT NOT NULL,
    used_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    revoked_at TEXT,
    UNIQUE(match_id, user_id, revoked_at)
);

CREATE INDEX idx_prediction_admin_overrides_match_user
    ON prediction_admin_overrides (match_id, user_id);

CREATE INDEX idx_prediction_admin_overrides_active
    ON prediction_admin_overrides (match_id, user_id, expires_at, revoked_at, used_at);

CREATE TABLE scoring_jobs (
    id TEXT PRIMARY KEY,
    scope_type TEXT NOT NULL,
    scope_id TEXT,
    triggered_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT,
    summary_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_scoring_jobs_scope ON scoring_jobs (scope_type, scope_id, started_at);

CREATE TABLE prediction_score_breakdowns (
    id TEXT PRIMARY KEY,
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    match_id TEXT NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    exact_score_points INTEGER NOT NULL DEFAULT 0,
    outcome_points INTEGER NOT NULL DEFAULT 0,
    goal_bonus_points INTEGER NOT NULL DEFAULT 0,
    qualifier_points INTEGER NOT NULL DEFAULT 0,
    penalties_points INTEGER NOT NULL DEFAULT 0,
    total_points INTEGER NOT NULL DEFAULT 0,
    eligible INTEGER NOT NULL DEFAULT 0,
    eligibility_reason TEXT NOT NULL DEFAULT '',
    official_source TEXT,
    computed_at TEXT NOT NULL DEFAULT (datetime('now')),
    job_id TEXT REFERENCES scoring_jobs(id) ON DELETE SET NULL,
    UNIQUE(pool_id, user_id, match_id)
);

CREATE INDEX idx_prediction_score_breakdowns_pool
    ON prediction_score_breakdowns (pool_id, total_points);

CREATE INDEX idx_prediction_score_breakdowns_user
    ON prediction_score_breakdowns (user_id, pool_id);

CREATE TABLE sync_runs (
    id TEXT PRIMARY KEY,
    triggered_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    trigger_source TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT,
    summary_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_sync_runs_started_at ON sync_runs (started_at DESC);

INSERT INTO app_settings (key, value) VALUES ('auto_sync_enabled', '1')
ON CONFLICT(key) DO NOTHING;

INSERT INTO app_settings (key, value) VALUES ('sync_interval_minutes', '10')
ON CONFLICT(key) DO NOTHING;

INSERT INTO app_settings (key, value) VALUES ('prediction_lock_minutes', '0')
ON CONFLICT(key) DO NOTHING;

INSERT INTO app_settings (key, value) VALUES ('global_banner_enabled', '0')
ON CONFLICT(key) DO NOTHING;

INSERT INTO app_settings (key, value) VALUES ('global_banner_text', '')
ON CONFLICT(key) DO NOTHING;
