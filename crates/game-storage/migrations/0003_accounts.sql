ALTER TABLE platform_playtime RENAME TO platform_playtime_old;
CREATE TABLE platform_playtime (
    source_id TEXT NOT NULL REFERENCES game_sources(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL,
    minutes INTEGER NOT NULL CHECK(minutes >= 0),
    last_played_at TEXT,
    synced_at TEXT NOT NULL,
    PRIMARY KEY(source_id,account_id)
);
INSERT INTO platform_playtime SELECT * FROM platform_playtime_old;
DROP TABLE platform_playtime_old;
INSERT OR IGNORE INTO settings(key,value_json) SELECT 'steam_active_account',account_id FROM platform_playtime LIMIT 1;
CREATE TABLE account_games (
    provider TEXT NOT NULL,
    account_id TEXT NOT NULL,
    source_id TEXT NOT NULL REFERENCES game_sources(id) ON DELETE CASCADE,
    scope TEXT NOT NULL DEFAULT 'cache',
    PRIMARY KEY(provider,account_id,source_id)
);
