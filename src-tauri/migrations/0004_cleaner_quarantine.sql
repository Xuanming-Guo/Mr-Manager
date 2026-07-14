CREATE TABLE cleanup_plans (
  id TEXT PRIMARY KEY,
  scan_id TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('reviewed', 'executed')),
  manifest_id TEXT,
  plan_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE quarantine_manifests (
  id TEXT PRIMARY KEY,
  plan_id TEXT NOT NULL UNIQUE REFERENCES cleanup_plans(id),
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('in_progress', 'complete', 'partial', 'restored', 'purged')),
  manifest_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX cleanup_plans_created_idx ON cleanup_plans (created_at_ms DESC);
CREATE INDEX quarantine_manifests_updated_idx ON quarantine_manifests (updated_at_ms DESC);

INSERT INTO schema_migrations (version) VALUES (4);
