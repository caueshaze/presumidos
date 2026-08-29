CREATE TABLE notification_preferences (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 0,
    lead_time_minutes INTEGER NOT NULL DEFAULT 20,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE push_subscriptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL UNIQUE,
    p256dh TEXT NOT NULL,
    auth TEXT NOT NULL,
    expiration_time_ms INTEGER,
    user_agent TEXT,
    device_label TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_error TEXT,
    last_sent_at TEXT
);

CREATE INDEX idx_push_subscriptions_user_active
    ON push_subscriptions (user_id, active);

CREATE TABLE push_reminder_deliveries (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    match_id TEXT NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    sent_at TEXT NOT NULL DEFAULT (datetime('now')),
    payload_json TEXT NOT NULL,
    UNIQUE(user_id, match_id)
);

CREATE INDEX idx_push_reminder_deliveries_user_match
    ON push_reminder_deliveries (user_id, match_id);
