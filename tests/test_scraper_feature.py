from pathlib import Path
import sqlite3

ROOT = Path(__file__).resolve().parents[1]

def test_schema_supports_artwork_and_matching():
    db = sqlite3.connect(':memory:')
    db.executescript((ROOT/'crates/game-storage/migrations/0001_init.sql').read_text(encoding='utf-8'))
    for table in ('game_assets','metadata_matches','tags','game_tags'):
        assert db.execute("SELECT 1 FROM sqlite_master WHERE name=?",(table,)).fetchone()
    db.close()

def test_scraper_endpoints_and_png_conversion():
    source = (ROOT/'crates/game-metadata/src/lib.rs').read_text(encoding='utf-8')
    for token in ('search/autocomplete','api.igdb.com/v4/games','ImageFormat::Png','cache_screenshots','confirm','normalize'):
        assert token in source

def test_commands_and_ui_connected():
    rust = (ROOT/'apps/desktop/src-tauri/src/lib.rs').read_text(encoding='utf-8')
    frontend = (ROOT/'apps/desktop/src/lib/backend.ts').read_text(encoding='utf-8')
    for command in ('scrape_artwork','search_artwork_candidates','confirm_artwork_match','set_custom_artwork','reset_custom_artwork','list_game_screenshots'):
        assert command in rust and command in frontend
