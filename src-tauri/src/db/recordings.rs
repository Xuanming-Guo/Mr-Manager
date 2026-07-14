use rusqlite::{Row, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::domain::{
    CorrelationFinding, MetricRecordingDetail, RecordingAnnotation, RecordingSample,
    RecordingSessionSummary, RecordingStatus,
};

use super::{Database, DatabaseError};

impl Database {
    pub fn save_metric_recording(
        &self,
        detail: &MetricRecordingDetail,
    ) -> Result<(), DatabaseError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(DatabaseError::Sql)?;
        let stopped_at = detail
            .summary
            .stopped_at_ms
            .map(|value| i64::try_from(value).unwrap_or(i64::MAX));
        transaction
            .execute(
                "INSERT INTO metric_recording_sessions (
                   id, name, started_at_ms, stopped_at_ms, status, sample_count, annotation_count,
                   downsampled, local_only, findings_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET
                   name = excluded.name,
                   stopped_at_ms = excluded.stopped_at_ms,
                   status = excluded.status,
                   sample_count = excluded.sample_count,
                   annotation_count = excluded.annotation_count,
                   downsampled = excluded.downsampled,
                   local_only = excluded.local_only,
                   findings_json = excluded.findings_json,
                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
                params![
                    detail.summary.id,
                    detail.summary.name,
                    i64::try_from(detail.summary.started_at_ms).unwrap_or(i64::MAX),
                    stopped_at,
                    status_text(&detail.summary.status),
                    i64::from(detail.summary.sample_count),
                    i64::from(detail.summary.annotation_count),
                    bool_int(detail.summary.downsampled),
                    bool_int(detail.summary.local_only),
                    encode(&detail.summary.findings)?,
                ],
            )
            .map_err(DatabaseError::Sql)?;
        transaction
            .execute(
                "DELETE FROM metric_recording_samples WHERE session_id = ?1",
                params![detail.summary.id],
            )
            .map_err(DatabaseError::Sql)?;
        for sample in &detail.samples {
            transaction
                .execute(
                    "INSERT INTO metric_recording_samples (
                       session_id, sequence, collected_at_ms, sample_json
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        detail.summary.id,
                        i64::try_from(sample.sequence).unwrap_or(i64::MAX),
                        i64::try_from(sample.collected_at_ms).unwrap_or(i64::MAX),
                        encode(sample)?,
                    ],
                )
                .map_err(DatabaseError::Sql)?;
        }
        transaction
            .execute(
                "DELETE FROM metric_recording_annotations WHERE session_id = ?1",
                params![detail.summary.id],
            )
            .map_err(DatabaseError::Sql)?;
        for annotation in &detail.annotations {
            transaction
                .execute(
                    "INSERT INTO metric_recording_annotations (
                       id, session_id, at_ms, label
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        annotation.id,
                        detail.summary.id,
                        i64::try_from(annotation.at_ms).unwrap_or(i64::MAX),
                        annotation.label,
                    ],
                )
                .map_err(DatabaseError::Sql)?;
        }
        transaction.commit().map_err(DatabaseError::Sql)
    }

    pub fn list_metric_sessions(&self) -> Result<Vec<RecordingSessionSummary>, DatabaseError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, name, status, started_at_ms, stopped_at_ms, sample_count,
                        annotation_count, downsampled, local_only, findings_json
                 FROM metric_recording_sessions
                 ORDER BY started_at_ms DESC",
            )
            .map_err(DatabaseError::Sql)?;
        let rows = statement
            .query_map([], recording_summary_from_row)
            .map_err(DatabaseError::Sql)?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row.map_err(DatabaseError::Sql)?);
        }
        Ok(sessions)
    }

    pub fn get_metric_recording(
        &self,
        session_id: &str,
    ) -> Result<Option<MetricRecordingDetail>, DatabaseError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, name, status, started_at_ms, stopped_at_ms, sample_count,
                        annotation_count, downsampled, local_only, findings_json
                 FROM metric_recording_sessions
                 WHERE id = ?1",
            )
            .map_err(DatabaseError::Sql)?;
        let mut rows = statement
            .query(params![session_id])
            .map_err(DatabaseError::Sql)?;
        let Some(row) = rows.next().map_err(DatabaseError::Sql)? else {
            return Ok(None);
        };
        let summary = recording_summary_from_row(row).map_err(DatabaseError::Sql)?;
        drop(rows);
        drop(statement);

        let samples = load_recording_samples(&connection, session_id)?;
        let annotations = load_recording_annotations(&connection, session_id)?;
        Ok(Some(MetricRecordingDetail {
            summary,
            samples,
            annotations,
        }))
    }

    pub fn delete_metric_recording(&self, session_id: &str) -> Result<bool, DatabaseError> {
        let connection = self.connection()?;
        let changed = connection
            .execute(
                "DELETE FROM metric_recording_sessions WHERE id = ?1",
                params![session_id],
            )
            .map_err(DatabaseError::Sql)?;
        Ok(changed > 0)
    }
}

fn load_recording_samples(
    connection: &rusqlite::Connection,
    session_id: &str,
) -> Result<Vec<RecordingSample>, DatabaseError> {
    let mut statement = connection
        .prepare(
            "SELECT sample_json
             FROM metric_recording_samples
             WHERE session_id = ?1
             ORDER BY sequence ASC",
        )
        .map_err(DatabaseError::Sql)?;
    let rows = statement
        .query_map(params![session_id], |row| row.get::<_, String>(0))
        .map_err(DatabaseError::Sql)?;
    let mut samples = Vec::new();
    for row in rows {
        samples.push(decode(&row.map_err(DatabaseError::Sql)?)?);
    }
    Ok(samples)
}

fn load_recording_annotations(
    connection: &rusqlite::Connection,
    session_id: &str,
) -> Result<Vec<RecordingAnnotation>, DatabaseError> {
    let mut statement = connection
        .prepare(
            "SELECT id, at_ms, label
             FROM metric_recording_annotations
             WHERE session_id = ?1
             ORDER BY at_ms ASC",
        )
        .map_err(DatabaseError::Sql)?;
    let rows = statement
        .query_map(params![session_id], |row| {
            Ok(RecordingAnnotation {
                id: row.get(0)?,
                at_ms: row.get::<_, i64>(1)?.max(0) as u64,
                label: row.get(2)?,
            })
        })
        .map_err(DatabaseError::Sql)?;
    let mut annotations = Vec::new();
    for row in rows {
        annotations.push(row.map_err(DatabaseError::Sql)?);
    }
    Ok(annotations)
}

fn recording_summary_from_row(row: &Row<'_>) -> rusqlite::Result<RecordingSessionSummary> {
    let status_text: String = row.get(2)?;
    let findings_json: String = row.get(9)?;
    let findings = decode::<Vec<CorrelationFinding>>(&findings_json)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    Ok(RecordingSessionSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        status: if status_text == "active" {
            RecordingStatus::Active
        } else {
            RecordingStatus::Completed
        },
        started_at_ms: row.get::<_, i64>(3)?.max(0) as u64,
        stopped_at_ms: row
            .get::<_, Option<i64>>(4)?
            .map(|value| value.max(0) as u64),
        sample_count: row.get::<_, i64>(5)?.max(0) as u32,
        annotation_count: row.get::<_, i64>(6)?.max(0) as u32,
        downsampled: row.get::<_, i64>(7)? != 0,
        local_only: row.get::<_, i64>(8)? != 0,
        findings,
    })
}

fn status_text(status: &RecordingStatus) -> &'static str {
    match status {
        RecordingStatus::Active => "active",
        RecordingStatus::Completed => "completed",
    }
}

const fn bool_int(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn encode<T: Serialize>(value: &T) -> Result<String, DatabaseError> {
    serde_json::to_string(value).map_err(|error| DatabaseError::Data(error.to_string()))
}

fn decode<T: DeserializeOwned>(value: &str) -> Result<T, DatabaseError> {
    serde_json::from_str(value).map_err(|error| DatabaseError::Data(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FeatureAvailability, GpuSnapshot};

    #[test]
    fn recordings_round_trip_and_delete() {
        let database = Database::open_in_memory().expect("database should initialize");
        let detail = MetricRecordingDetail {
            summary: RecordingSessionSummary {
                id: "session-1".to_owned(),
                name: "Fixture build".to_owned(),
                status: RecordingStatus::Completed,
                started_at_ms: 10,
                stopped_at_ms: Some(20),
                sample_count: 1,
                annotation_count: 1,
                downsampled: false,
                local_only: true,
                findings: Vec::new(),
            },
            samples: vec![RecordingSample {
                sequence: 0,
                collected_at_ms: 10,
                local_only: true,
                included_internet_diagnostics: false,
                system: crate::domain::RecordingSystemSample {
                    cpu_total_percent: 1.0,
                    memory: crate::domain::MemorySnapshot {
                        total_bytes: 1,
                        used_bytes: 1,
                        available_bytes: 0,
                        swap_total_bytes: 0,
                        swap_used_bytes: 0,
                    },
                    battery: crate::domain::BatterySnapshot {
                        availability: FeatureAvailability::unsupported("test"),
                        percentage: None,
                        ac_online: None,
                        remaining_seconds: None,
                    },
                    network: crate::domain::AdapterThroughput {
                        received_bytes_per_second: 0,
                        transmitted_bytes_per_second: 0,
                        session_received_bytes: 0,
                        session_transmitted_bytes: 0,
                        total_received_bytes: 0,
                        total_transmitted_bytes: 0,
                        peak_received_bytes_per_second: 0,
                        peak_transmitted_bytes_per_second: 0,
                        timeline: Vec::new(),
                    },
                    disk_read_bytes: 0,
                    disk_write_bytes: 0,
                },
                gpu: GpuSnapshot {
                    availability: FeatureAvailability::unsupported("test"),
                    provider: "test".to_owned(),
                    adapters: Vec::new(),
                    collected_at_ms: 10,
                },
                top_processes: Vec::new(),
                network: crate::domain::NetworkDashboardSnapshot {
                    collected_at_ms: 10,
                    external_diagnostics_enabled: false,
                    combined: crate::domain::AdapterThroughput {
                        received_bytes_per_second: 0,
                        transmitted_bytes_per_second: 0,
                        session_received_bytes: 0,
                        session_transmitted_bytes: 0,
                        total_received_bytes: 0,
                        total_transmitted_bytes: 0,
                        peak_received_bytes_per_second: 0,
                        peak_transmitted_bytes_per_second: 0,
                        timeline: Vec::new(),
                    },
                    adapters: Vec::new(),
                    gateway_reachability: crate::domain::GatewayStatus {
                        state: crate::domain::NetworkDiagnosticState::Unavailable,
                        gateway: None,
                        latency_ms: None,
                        local_only: true,
                        evidence: Vec::new(),
                    },
                    dns_status: crate::domain::DnsStatus {
                        state: crate::domain::NetworkDiagnosticState::Unavailable,
                        local_only: true,
                        configured_server_count: 0,
                        evidence: Vec::new(),
                    },
                    vpn_state: crate::domain::VpnState {
                        likely_active: false,
                        confidence: "none".to_owned(),
                        label: "none".to_owned(),
                        evidence: Vec::new(),
                    },
                    lan_ip_candidates: Vec::new(),
                    local_dev_server_warnings: Vec::new(),
                    per_process_usage: crate::domain::PerProcessNetworkUsage {
                        availability: FeatureAvailability::unsupported("test"),
                        entries: Vec::new(),
                    },
                    privacy_note: "redacted".to_owned(),
                },
                docker: crate::domain::DockerActivitySnapshot {
                    availability: FeatureAvailability::unsupported("test"),
                    docker_process_count: 0,
                    docker_process_names: Vec::new(),
                    evidence: Vec::new(),
                },
                local_dev_servers: Vec::new(),
            }],
            annotations: vec![RecordingAnnotation {
                id: "a1".to_owned(),
                at_ms: 11,
                label: "Started build".to_owned(),
            }],
        };

        database
            .save_metric_recording(&detail)
            .expect("recording should save");
        let sessions = database
            .list_metric_sessions()
            .expect("recordings should list");
        assert_eq!(sessions.len(), 1);
        let loaded = database
            .get_metric_recording("session-1")
            .expect("recording should load")
            .expect("recording should exist");
        assert_eq!(loaded.samples.len(), 1);
        assert_eq!(loaded.annotations[0].label, "Started build");
        assert!(
            database
                .delete_metric_recording("session-1")
                .expect("delete should work")
        );
        assert!(
            database
                .get_metric_recording("session-1")
                .expect("load should work")
                .is_none()
        );
    }
}
