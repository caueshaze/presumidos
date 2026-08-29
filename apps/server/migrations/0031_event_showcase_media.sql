-- Recursos editoriais opcionais: eventos e opções simples continuam válidos sem eles.
ALTER TABLE events ADD COLUMN description TEXT;
ALTER TABLE events ADD COLUMN cover_url TEXT;
ALTER TABLE events ADD COLUMN external_url TEXT;
ALTER TABLE custom_question_options ADD COLUMN image_url TEXT;

CREATE TABLE option_links (
    id TEXT PRIMARY KEY,
    option_id TEXT NOT NULL REFERENCES custom_question_options(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('video', 'audio', 'official', 'other')),
    label TEXT NOT NULL,
    url TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(option_id, sort_order),
    UNIQUE(option_id, kind, url)
);
CREATE INDEX idx_option_links_option_sort ON option_links(option_id, sort_order);

-- Progresso pessoal de mídia: não é palpite e não depende de lock/reveal.
CREATE TABLE option_media_progress (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    option_id TEXT NOT NULL REFERENCES custom_question_options(id) ON DELETE CASCADE,
    seen_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY(user_id, option_id)
);
