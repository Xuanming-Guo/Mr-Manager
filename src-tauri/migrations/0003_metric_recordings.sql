CREATE TABLE metric_recording_sessions (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  started_at_ms INTEGER NOT NULL,
  stopped_at_ms INTEGER,
  status TEXT NOT NULL CHECK (status IN ('active', 'completed')),
  sample_count INTEGER NOT NULL DEFAULT 0,
  annotation_count INTEGER NOT NULL DEFAULT 0,
  downsampled INTEGER NOT NULL DEFAULT 0 CHECK (downsampled IN (0, 1)),
  local_only INTEGER NOT NULL DEFAULT 1 CHECK (local_only IN (0, 1)),
  findings_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE metric_recording_samples (
  session_id TEXT NOT NULL REFERENCES metric_recording_sessions(id) ON DELETE CASCADE,
  sequence INTEGER NOT NULL,
  collected_at_ms INTEGER NOT NULL,
  sample_json TEXT NOT NULL,
  PRIMARY KEY (session_id, sequence)
);

CREATE TABLE metric_recording_annotations (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES metric_recording_sessions(id) ON DELETE CASCADE,
  at_ms INTEGER NOT NULL,
  label TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX metric_recording_sessions_started_idx
  ON metric_recording_sessions (started_at_ms DESC);

CREATE INDEX metric_recording_samples_time_idx
  ON metric_recording_samples (session_id, collected_at_ms);

INSERT INTO schema_migrations (version) VALUES (3);
