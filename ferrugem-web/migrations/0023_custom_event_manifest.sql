ALTER TABLE prediction_items ADD COLUMN external_key TEXT;
ALTER TABLE custom_question_options ADD COLUMN external_key TEXT;
CREATE UNIQUE INDEX idx_prediction_items_event_external_key
    ON prediction_items(event_id, external_key) WHERE external_key IS NOT NULL;
CREATE UNIQUE INDEX idx_custom_question_options_item_external_key
    ON custom_question_options(item_id, external_key) WHERE external_key IS NOT NULL;
