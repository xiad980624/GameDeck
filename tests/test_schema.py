"""依赖 Python 标准库的 SQL schema 冒烟测试。运行：python -m unittest discover -s tests"""
import sqlite3
import unittest
from pathlib import Path

DDL = (Path(__file__).resolve().parents[1] / 'crates/game-storage/migrations/0001_init.sql').read_text(encoding='utf-8')


class GameDeckSchemaTest(unittest.TestCase):
    def setUp(self):
        self.db = sqlite3.connect(':memory:')
        self.db.execute('PRAGMA foreign_keys=ON')
        self.db.executescript(DDL)

    def tearDown(self):
        self.db.close()

    def test_required_tables(self):
        names = {row[0] for row in self.db.execute("SELECT name FROM sqlite_master WHERE type='table'")}
        required = {'games', 'game_sources', 'installations', 'launch_profiles', 'game_assets', 'play_sessions', 'game_tags', 'tags', 'settings', 'metadata_matches', 'scan_runs'}
        self.assertTrue(required.issubset(names))

    def test_game_source_identity_unique(self):
        self.db.execute("INSERT INTO games(id,name,normalized_name,created_at) VALUES('g1','Test','test','2026-11-20')")
        self.db.execute("INSERT INTO game_sources(id,game_id,provider,external_id) VALUES('s1','g1','steam','123')")
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute("INSERT INTO game_sources(id,game_id,provider,external_id) VALUES('s2','g1','steam','123')")

    def test_session_game_foreign_key(self):
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute("INSERT INTO play_sessions(id,game_id,started_at,launch_state) VALUES('p1','unknown','2026-11-20','running')")

    def test_hidden_space_migration_preserves_existing_library(self):
        self.db.execute("INSERT INTO games(id,name,normalized_name,created_at) VALUES('old','Existing','existing','2026-11-20')")
        migration_dir = Path(__file__).resolve().parents[1] / 'crates/game-storage/migrations'
        for path in sorted(migration_dir.glob('*.sql')):
            if path.name != '0001_init.sql':
                self.db.executescript(path.read_text(encoding='utf-8'))
        self.assertEqual(self.db.execute("SELECT name,hidden FROM games WHERE id='old'").fetchone(), ('Existing',0))
        self.db.execute("UPDATE games SET hidden=1 WHERE id='old'")
        self.assertEqual(self.db.execute("SELECT hidden FROM games WHERE id='old'").fetchone()[0],1)
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute("UPDATE games SET hidden=2 WHERE id='old'")


if __name__ == '__main__':
    unittest.main()
