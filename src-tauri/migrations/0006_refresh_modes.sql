UPDATE settings
SET value = 'normal', updated_at = CURRENT_TIMESTAMP
WHERE key = 'refresh_mode' AND value IN ('eco', 'balanced');

UPDATE settings
SET value = 'fast', updated_at = CURRENT_TIMESTAMP
WHERE key = 'refresh_mode' AND value = 'realtime';

INSERT INTO schema_migrations (version) VALUES (6);
