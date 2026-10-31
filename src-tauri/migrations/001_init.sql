-- Early prototype storage: one canonical JSON snapshot per app install.
-- A single UPSERT is atomic, including the active flight and all its pigeons.
CREATE TABLE IF NOT EXISTS app_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    state_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
