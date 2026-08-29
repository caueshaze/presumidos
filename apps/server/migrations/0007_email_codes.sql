-- Codigos de verificacao por email (confirmacao de cadastro e reset de senha).

CREATE TABLE pending_registrations (
    email TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    username_lookup TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    code_hash TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE password_reset_codes (
    email TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
