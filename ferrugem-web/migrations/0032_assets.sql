CREATE TABLE assets (
    id TEXT PRIMARY KEY,
    storage_key TEXT NOT NULL UNIQUE,
    sha256 TEXT NOT NULL UNIQUE,
    media_type TEXT NOT NULL CHECK(media_type = 'image/webp'),
    width INTEGER NOT NULL CHECK(width > 0),
    height INTEGER NOT NULL CHECK(height > 0),
    byte_size INTEGER NOT NULL CHECK(byte_size > 0),
    uploaded_by TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE asset_variants (
    asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    variant TEXT NOT NULL CHECK(variant IN ('master', 'thumb', 'card', 'cover')),
    storage_key TEXT NOT NULL UNIQUE,
    width INTEGER NOT NULL CHECK(width > 0),
    height INTEGER NOT NULL CHECK(height > 0),
    byte_size INTEGER NOT NULL CHECK(byte_size > 0),
    PRIMARY KEY(asset_id, variant)
);

ALTER TABLE events ADD COLUMN cover_asset_id TEXT REFERENCES assets(id) ON DELETE SET NULL;
ALTER TABLE custom_question_options ADD COLUMN image_asset_id TEXT REFERENCES assets(id) ON DELETE SET NULL;

CREATE INDEX idx_assets_sha256 ON assets(sha256);
CREATE INDEX idx_asset_variants_storage_key ON asset_variants(storage_key);
CREATE INDEX idx_events_cover_asset ON events(cover_asset_id);
CREATE INDEX idx_options_image_asset ON custom_question_options(image_asset_id);
