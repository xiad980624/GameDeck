ALTER TABLE installations ADD COLUMN update_status TEXT NOT NULL DEFAULT 'unknown';
ALTER TABLE installations ADD COLUMN installed_version TEXT;
ALTER TABLE installations ADD COLUMN update_detail TEXT;
ALTER TABLE installations ADD COLUMN update_checked_at TEXT;
