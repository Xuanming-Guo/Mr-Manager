CREATE TABLE readme_templates (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  body TEXT NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE portfolio_records (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL UNIQUE REFERENCES registered_projects(id) ON DELETE CASCADE,
  updated_at_ms INTEGER NOT NULL,
  record_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE portfolio_assets (
  id TEXT PRIMARY KEY,
  portfolio_id TEXT NOT NULL REFERENCES portfolio_records(id) ON DELETE CASCADE,
  created_at_ms INTEGER NOT NULL,
  asset_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX readme_templates_updated_idx ON readme_templates (updated_at_ms DESC);
CREATE INDEX portfolio_records_updated_idx ON portfolio_records (updated_at_ms DESC);
CREATE INDEX portfolio_assets_portfolio_idx ON portfolio_assets (portfolio_id, created_at_ms);

INSERT INTO schema_migrations (version) VALUES (5);
