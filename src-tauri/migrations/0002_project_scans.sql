CREATE TABLE project_roots (
  id TEXT PRIMARY KEY,
  root_path TEXT NOT NULL,
  canonical_root_path TEXT NOT NULL UNIQUE,
  maximum_depth INTEGER NOT NULL CHECK (maximum_depth BETWEEN 0 AND 8),
  last_scanned_at_ms INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE registered_projects ADD COLUMN detected_stacks_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE registered_projects ADD COLUMN manifests_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE registered_projects ADD COLUMN package_manager_json TEXT NOT NULL DEFAULT 'null';
ALTER TABLE registered_projects ADD COLUMN scripts_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE registered_projects ADD COLUMN git_summary_json TEXT NOT NULL DEFAULT 'null';
ALTER TABLE registered_projects ADD COLUMN compose_files_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE registered_projects ADD COLUMN environment_files_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE registered_projects ADD COLUMN local_database_hints_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE registered_projects ADD COLUMN last_scanned_at_ms INTEGER;
ALTER TABLE registered_projects ADD COLUMN scan_health_json TEXT NOT NULL DEFAULT '{"state":"unavailable","issues":[]}';

CREATE INDEX registered_projects_updated_at_idx
  ON registered_projects (updated_at);

INSERT INTO schema_migrations (version) VALUES (2);
