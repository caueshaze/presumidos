CREATE TABLE pool_reports (
    id TEXT PRIMARY KEY,
    pool_id TEXT NOT NULL,
    pool_name TEXT NOT NULL,
    invite_code TEXT NOT NULL,
    reporter_user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    category TEXT NOT NULL CHECK (category IN ('inappropriate_content', 'spam_or_fraud', 'harassment', 'other')),
    details TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'reviewing', 'resolved', 'dismissed')),
    reviewed_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX idx_pool_reports_open_reporter_pool
    ON pool_reports(pool_id, reporter_user_id)
    WHERE status IN ('open', 'reviewing') AND reporter_user_id IS NOT NULL;

CREATE INDEX idx_pool_reports_status_created
    ON pool_reports(status, datetime(created_at) DESC);
