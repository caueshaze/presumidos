CREATE INDEX idx_events_created_by ON events(created_by);
CREATE INDEX IF NOT EXISTS idx_prediction_items_event_sort ON prediction_items(event_id, sort_order);
