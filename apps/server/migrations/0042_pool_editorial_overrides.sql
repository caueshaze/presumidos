-- Links editoriais pertencem à EventVersion por padrão. Um Pool pode substituir
-- a lista de uma opção sem alterar o conteúdo editorial compartilhado.
CREATE TABLE pool_option_link_overrides (
    pool_id TEXT NOT NULL REFERENCES pools(id) ON DELETE CASCADE,
    option_id TEXT NOT NULL REFERENCES custom_question_options(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT(datetime('now')),
    updated_at TEXT NOT NULL DEFAULT(datetime('now')),
    PRIMARY KEY (pool_id, option_id)
);

CREATE TABLE pool_option_editorial_links (
    id TEXT PRIMARY KEY,
    pool_id TEXT NOT NULL,
    option_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('video', 'audio', 'official', 'other')),
    label TEXT NOT NULL,
    url TEXT NOT NULL,
    sort_order INTEGER NOT NULL CHECK(sort_order >= 0),
    created_at TEXT NOT NULL DEFAULT(datetime('now')),
    updated_at TEXT NOT NULL DEFAULT(datetime('now')),
    FOREIGN KEY (pool_id, option_id)
        REFERENCES pool_option_link_overrides(pool_id, option_id) ON DELETE CASCADE,
    UNIQUE(pool_id, option_id, sort_order)
);

CREATE TRIGGER pool_option_link_override_matches_pool_version
BEFORE INSERT ON pool_option_link_overrides
WHEN NOT EXISTS (
    SELECT 1
    FROM pools p
    JOIN custom_question_options o ON o.id=NEW.option_id
    JOIN prediction_items pi ON pi.id=o.item_id
    WHERE p.id=NEW.pool_id AND pi.event_version_id=p.event_version_id
)
BEGIN
    SELECT RAISE(ABORT, 'opção editorial incompatível com o bolão');
END;
