CREATE TABLE IF NOT EXISTS platform_playtime (
    source_id TEXT PRIMARY KEY REFERENCES game_sources(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL,
    minutes INTEGER NOT NULL CHECK(minutes >= 0),
    last_played_at TEXT,
    synced_at TEXT NOT NULL
);
