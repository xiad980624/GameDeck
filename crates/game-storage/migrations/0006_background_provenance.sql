-- Screenshots belong to the screenshot gallery, not automatic background art.
DELETE FROM game_assets WHERE asset_type='hero' AND priority<100 AND source='steam-original';
