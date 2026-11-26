PRAGMA foreign_keys=ON;
CREATE TABLE IF NOT EXISTS games (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    genre TEXT NOT NULL DEFAULT '游戏',
    favorite INTEGER NOT NULL DEFAULT 0,
    total_minutes INTEGER NOT NULL DEFAULT 0,
    last_played_at TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS game_sources (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    external_id TEXT NOT NULL,
    source_name TEXT,
    UNIQUE(provider,external_id)
);
CREATE TABLE IF NOT EXISTS launch_profiles (
    id TEXT PRIMARY KEY,
    launch_type TEXT NOT NULL,
    target TEXT NOT NULL,
    arguments_json TEXT NOT NULL DEFAULT '[]',
    working_directory TEXT,
    monitor_rules_json TEXT NOT NULL DEFAULT '{}'
);
CREATE TABLE IF NOT EXISTS installations (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES game_sources(id) ON DELETE CASCADE,
    install_path TEXT NOT NULL,
    launch_profile_id TEXT REFERENCES launch_profiles(id),
    installed INTEGER NOT NULL DEFAULT 1,
    discovered_at TEXT NOT NULL,
    UNIQUE(source_id,install_path)
);
CREATE TABLE IF NOT EXISTS game_assets (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    asset_type TEXT NOT NULL,
    path TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'manual',
    priority INTEGER NOT NULL DEFAULT 0,
    width INTEGER,
    height INTEGER,
    UNIQUE(game_id,asset_type,priority)
);
CREATE TABLE IF NOT EXISTS play_sessions (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    installation_id TEXT REFERENCES installations(id),
    started_at TEXT NOT NULL,
    ended_at TEXT,
    duration_seconds INTEGER,
    launch_state TEXT NOT NULL,
    exit_reason TEXT
);
CREATE TABLE IF NOT EXISTS tags(id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE);
CREATE TABLE IF NOT EXISTS game_tags (
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY(game_id,tag_id)
);
CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY,value_json TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS metadata_matches (
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    external_id TEXT NOT NULL,
    confirmed INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(game_id,provider)
);
CREATE TABLE IF NOT EXISTS scan_runs (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    found INTEGER NOT NULL DEFAULT 0,
    error TEXT
);
CREATE INDEX IF NOT EXISTS idx_sources_game ON game_sources(game_id);
CREATE INDEX IF NOT EXISTS idx_installations_source ON installations(source_id);
CREATE INDEX IF NOT EXISTS idx_play_sessions_game_time ON play_sessions(game_id,started_at DESC);
CREATE INDEX IF NOT EXISTS idx_games_recent ON games(last_played_at DESC);
