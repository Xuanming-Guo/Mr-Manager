use rusqlite::{OptionalExtension, params};

use crate::domain::{CleanupPlan, CleanupPlanState, QuarantineManifest, QuarantineManifestState};

use super::{Database, DatabaseError};

impl Database {
    pub fn save_cleanup_plan(&self, plan: &CleanupPlan) -> Result<(), DatabaseError> {
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO cleanup_plans (
                   id, scan_id, created_at_ms, state, manifest_id, plan_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    plan.id,
                    plan.scan_id,
                    to_i64(plan.created_at_ms),
                    plan_state(&plan.state),
                    plan.manifest_id,
                    encode(plan)?,
                ],
            )
            .map_err(DatabaseError::Sql)?;
        Ok(())
    }

    pub fn get_cleanup_plan(&self, plan_id: &str) -> Result<Option<CleanupPlan>, DatabaseError> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT plan_json FROM cleanup_plans WHERE id = ?1",
                params![plan_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(DatabaseError::Sql)?
            .map(|json| decode(&json))
            .transpose()
    }

    pub fn start_quarantine_manifest(
        &self,
        plan: &mut CleanupPlan,
        manifest: &QuarantineManifest,
    ) -> Result<bool, DatabaseError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(DatabaseError::Sql)?;
        plan.state = CleanupPlanState::Executed;
        plan.manifest_id = Some(manifest.id.clone());
        let changed = transaction
            .execute(
                "UPDATE cleanup_plans
                 SET state = 'executed', manifest_id = ?2, plan_json = ?3,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?1 AND state = 'reviewed' AND manifest_id IS NULL",
                params![plan.id, manifest.id, encode(plan)?],
            )
            .map_err(DatabaseError::Sql)?;
        if changed != 1 {
            return Ok(false);
        }
        transaction
            .execute(
                "INSERT INTO quarantine_manifests (
                   id, plan_id, created_at_ms, updated_at_ms, state, manifest_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    manifest.id,
                    manifest.plan_id,
                    to_i64(manifest.created_at_ms),
                    to_i64(manifest.updated_at_ms),
                    manifest_state(&manifest.state),
                    encode(manifest)?,
                ],
            )
            .map_err(DatabaseError::Sql)?;
        transaction.commit().map_err(DatabaseError::Sql)?;
        Ok(true)
    }

    pub fn update_quarantine_manifest(
        &self,
        manifest: &QuarantineManifest,
    ) -> Result<(), DatabaseError> {
        let connection = self.connection()?;
        let changed = connection
            .execute(
                "UPDATE quarantine_manifests
                 SET updated_at_ms = ?2, state = ?3, manifest_json = ?4,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?1",
                params![
                    manifest.id,
                    to_i64(manifest.updated_at_ms),
                    manifest_state(&manifest.state),
                    encode(manifest)?,
                ],
            )
            .map_err(DatabaseError::Sql)?;
        if changed == 1 {
            Ok(())
        } else {
            Err(DatabaseError::Data(
                "the quarantine manifest no longer exists".to_owned(),
            ))
        }
    }

    pub fn get_quarantine_manifest(
        &self,
        manifest_id: &str,
    ) -> Result<Option<QuarantineManifest>, DatabaseError> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT manifest_json FROM quarantine_manifests WHERE id = ?1",
                params![manifest_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(DatabaseError::Sql)?
            .map(|json| decode(&json))
            .transpose()
    }

    pub fn list_quarantine_manifests(&self) -> Result<Vec<QuarantineManifest>, DatabaseError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT manifest_json
                 FROM quarantine_manifests
                 ORDER BY updated_at_ms DESC",
            )
            .map_err(DatabaseError::Sql)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(DatabaseError::Sql)?;
        let mut manifests = Vec::new();
        for row in rows {
            manifests.push(decode(&row.map_err(DatabaseError::Sql)?)?);
        }
        Ok(manifests)
    }
}

fn plan_state(state: &CleanupPlanState) -> &'static str {
    match state {
        CleanupPlanState::Reviewed => "reviewed",
        CleanupPlanState::Executed => "executed",
    }
}

fn manifest_state(state: &QuarantineManifestState) -> &'static str {
    match state {
        QuarantineManifestState::InProgress => "in_progress",
        QuarantineManifestState::Complete => "complete",
        QuarantineManifestState::Partial => "partial",
        QuarantineManifestState::Restored => "restored",
        QuarantineManifestState::Purged => "purged",
    }
}

fn to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn encode<T: serde::Serialize>(value: &T) -> Result<String, DatabaseError> {
    serde_json::to_string(value).map_err(|error| DatabaseError::Data(error.to_string()))
}

fn decode<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, DatabaseError> {
    serde_json::from_str(value).map_err(|error| DatabaseError::Data(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleaner::begin_manifest;
    use crate::domain::{
        CleanupCandidate, CleanupCategory, CleanupConfidence, CleanupLockState, CleanupRisk,
    };

    fn fixture_plan() -> CleanupPlan {
        CleanupPlan {
            id: "plan-1".to_owned(),
            scan_id: "scan-1".to_owned(),
            created_at_ms: 10,
            roots: vec!["C:\\fixture".to_owned()],
            items: vec![CleanupCandidate {
                id: "candidate-1".to_owned(),
                root_path: "C:\\fixture".to_owned(),
                canonical_path: "C:\\fixture\\node_modules".to_owned(),
                display_name: "node_modules".to_owned(),
                category: CleanupCategory::DependencyCache,
                reason: "fixture".to_owned(),
                confidence: CleanupConfidence::Certain,
                risk: CleanupRisk::Low,
                estimated_size_bytes: 7,
                file_count: 1,
                lock_state: CleanupLockState::Unknown,
                selected: false,
                regeneration_instructions: "install".to_owned(),
                identity_fingerprint: "abc".to_owned(),
                is_directory: true,
            }],
            total_size_bytes: 7,
            total_file_count: 1,
            state: CleanupPlanState::Reviewed,
            confirmation_phrase: "QUARANTINE 1 ITEMS".to_owned(),
            manifest_id: None,
        }
    }

    #[test]
    fn cleanup_plan_and_manifest_are_persisted_transactionally() {
        let database = Database::open_in_memory().expect("database should initialize");
        let mut plan = fixture_plan();
        database
            .save_cleanup_plan(&plan)
            .expect("plan should persist");
        let manifest = begin_manifest(&plan, std::path::Path::new("C:\\quarantine"));
        assert!(
            database
                .start_quarantine_manifest(&mut plan, &manifest)
                .expect("manifest should begin")
        );
        assert!(
            !database
                .start_quarantine_manifest(&mut plan, &manifest)
                .expect("second execution is safely rejected")
        );
        let loaded = database
            .get_quarantine_manifest(&manifest.id)
            .expect("manifest should load")
            .expect("manifest should exist");
        assert_eq!(loaded.plan_id, "plan-1");
        assert_eq!(
            database
                .list_quarantine_manifests()
                .expect("list works")
                .len(),
            1
        );
    }
}
