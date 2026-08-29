CREATE TABLE prediction_reactions (
    id TEXT PRIMARY KEY,
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    match_id TEXT NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    target_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reactor_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(pool_id, match_id, target_user_id, reactor_user_id)
);

CREATE INDEX idx_prediction_reactions_target
    ON prediction_reactions (pool_id, target_user_id, match_id);

CREATE TABLE prediction_reaction_views (
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    seen_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (pool_id, user_id)
);

ALTER TABLE notification_preferences
    ADD COLUMN reaction_enabled INTEGER NOT NULL DEFAULT 1;
