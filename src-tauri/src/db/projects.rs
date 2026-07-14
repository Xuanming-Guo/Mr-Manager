use rusqlite::{Row, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::domain::{
    ChecklistItem, EnvironmentFileSummary, GitSummary, PackageManagerSummary, Project,
    ProjectMetadataUpdate, ProjectScanHealth, ProjectStack,
};

use super::{Database, DatabaseError};

impl Database {
    pub fn list_projects(&self) -> Result<Vec<Project>, DatabaseError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, display_name, root_path, canonical_root_path, tags_json, notes, \
                 checklist_json, pinned, archived, detected_stacks_json, manifests_json, \
                 package_manager_json, scripts_json, git_summary_json, compose_files_json, \
                 environment_files_json, local_database_hints_json, last_scanned_at_ms, \
                 scan_health_json \
                 FROM registered_projects \
                 ORDER BY archived ASC, pinned DESC, display_name COLLATE NOCASE ASC",
            )
            .map_err(DatabaseError::Sql)?;
        let mut rows = statement.query([]).map_err(DatabaseError::Sql)?;
        let mut projects = Vec::new();

        while let Some(row) = rows.next().map_err(DatabaseError::Sql)? {
            projects.push(project_from_row(row)?);
        }

        Ok(projects)
    }

    pub fn get_project(&self, project_id: &str) -> Result<Option<Project>, DatabaseError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, display_name, root_path, canonical_root_path, tags_json, notes, \
                 checklist_json, pinned, archived, detected_stacks_json, manifests_json, \
                 package_manager_json, scripts_json, git_summary_json, compose_files_json, \
                 environment_files_json, local_database_hints_json, last_scanned_at_ms, \
                 scan_health_json \
                 FROM registered_projects \
                 WHERE id = ?1",
            )
            .map_err(DatabaseError::Sql)?;
        let mut rows = statement
            .query(params![project_id])
            .map_err(DatabaseError::Sql)?;

        rows.next()
            .map_err(DatabaseError::Sql)?
            .map(project_from_row)
            .transpose()
    }

    pub fn get_project_by_canonical_path(
        &self,
        canonical_root_path: &str,
    ) -> Result<Option<Project>, DatabaseError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, display_name, root_path, canonical_root_path, tags_json, notes, \
                 checklist_json, pinned, archived, detected_stacks_json, manifests_json, \
                 package_manager_json, scripts_json, git_summary_json, compose_files_json, \
                 environment_files_json, local_database_hints_json, last_scanned_at_ms, \
                 scan_health_json \
                 FROM registered_projects \
                 WHERE canonical_root_path = ?1",
            )
            .map_err(DatabaseError::Sql)?;
        let mut rows = statement
            .query(params![canonical_root_path])
            .map_err(DatabaseError::Sql)?;

        rows.next()
            .map_err(DatabaseError::Sql)?
            .map(project_from_row)
            .transpose()
    }

    pub fn save_project(&self, project: &Project) -> Result<Project, DatabaseError> {
        let last_scanned_at = project
            .last_scanned_at
            .map(|value| i64::try_from(value).unwrap_or(i64::MAX));

        {
            let mut connection = self.connection()?;
            let transaction = connection.transaction().map_err(DatabaseError::Sql)?;
            transaction
                .execute(
                    "INSERT INTO registered_projects (
                   id, display_name, root_path, canonical_root_path, tags_json, notes,
                   checklist_json, pinned, archived, detected_stacks_json, manifests_json,
                   package_manager_json, scripts_json, git_summary_json, compose_files_json,
                   environment_files_json, local_database_hints_json, last_scanned_at_ms,
                   scan_health_json
                 ) VALUES (
                   ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                   ?18, ?19
                 )
                 ON CONFLICT(canonical_root_path) DO UPDATE SET
                   id = excluded.id,
                   display_name = excluded.display_name,
                   root_path = excluded.root_path,
                   tags_json = excluded.tags_json,
                   notes = excluded.notes,
                   checklist_json = excluded.checklist_json,
                   pinned = excluded.pinned,
                   archived = excluded.archived,
                   detected_stacks_json = excluded.detected_stacks_json,
                   manifests_json = excluded.manifests_json,
                   package_manager_json = excluded.package_manager_json,
                   scripts_json = excluded.scripts_json,
                   git_summary_json = excluded.git_summary_json,
                   compose_files_json = excluded.compose_files_json,
                   environment_files_json = excluded.environment_files_json,
                   local_database_hints_json = excluded.local_database_hints_json,
                   last_scanned_at_ms = excluded.last_scanned_at_ms,
                   scan_health_json = excluded.scan_health_json,
                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
                    params![
                        project.id,
                        project.name,
                        project.root_path,
                        project.canonical_root_path,
                        encode(&project.tags)?,
                        project.notes,
                        encode(&project.checklist)?,
                        bool_int(project.pinned),
                        bool_int(project.archived),
                        encode(&project.detected_stacks)?,
                        encode(&project.manifests)?,
                        encode(&project.package_manager)?,
                        encode(&project.scripts)?,
                        encode(&project.git_summary)?,
                        encode(&project.compose_files)?,
                        encode(&project.environment_files)?,
                        encode(&project.local_database_hints)?,
                        last_scanned_at,
                        encode(&project.scan_health)?,
                    ],
                )
                .map_err(DatabaseError::Sql)?;
            transaction.commit().map_err(DatabaseError::Sql)?;
        }

        self.get_project_by_canonical_path(&project.canonical_root_path)?
            .ok_or_else(|| DatabaseError::Data("saved project could not be reloaded".to_owned()))
    }

    pub fn remove_project(&self, project_id: &str) -> Result<(), DatabaseError> {
        let connection = self.connection()?;
        connection
            .execute(
                "DELETE FROM registered_projects WHERE id = ?1",
                params![project_id],
            )
            .map_err(DatabaseError::Sql)?;
        Ok(())
    }

    pub fn update_project_metadata(
        &self,
        project_id: &str,
        metadata: &ProjectMetadataUpdate,
    ) -> Result<Option<Project>, DatabaseError> {
        let changed = {
            let connection = self.connection()?;
            connection
                .execute(
                    "UPDATE registered_projects \
                 SET tags_json = ?2, notes = ?3, checklist_json = ?4, pinned = ?5, archived = ?6, \
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
                 WHERE id = ?1",
                    params![
                        project_id,
                        encode(&metadata.tags)?,
                        metadata.notes,
                        encode(&metadata.checklist)?,
                        bool_int(metadata.pinned),
                        bool_int(metadata.archived),
                    ],
                )
                .map_err(DatabaseError::Sql)?
        };

        if changed == 0 {
            Ok(None)
        } else {
            self.get_project(project_id)
        }
    }

    pub fn save_project_root(
        &self,
        operation_id: &str,
        root_path: &str,
        canonical_root_path: &str,
        maximum_depth: u8,
        last_scanned_at_ms: u64,
    ) -> Result<(), DatabaseError> {
        let last_scanned_at_ms = i64::try_from(last_scanned_at_ms).unwrap_or(i64::MAX);
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO project_roots (
                   id, root_path, canonical_root_path, maximum_depth, last_scanned_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(canonical_root_path) DO UPDATE SET
                   root_path = excluded.root_path,
                   maximum_depth = excluded.maximum_depth,
                   last_scanned_at_ms = excluded.last_scanned_at_ms,
                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
                params![
                    operation_id,
                    root_path,
                    canonical_root_path,
                    i64::from(maximum_depth),
                    last_scanned_at_ms,
                ],
            )
            .map_err(DatabaseError::Sql)?;
        Ok(())
    }
}

fn project_from_row(row: &Row<'_>) -> Result<Project, DatabaseError> {
    let tags_json: String = row.get(4).map_err(DatabaseError::Sql)?;
    let checklist_json: String = row.get(6).map_err(DatabaseError::Sql)?;
    let stacks_json: String = row.get(9).map_err(DatabaseError::Sql)?;
    let manifests_json: String = row.get(10).map_err(DatabaseError::Sql)?;
    let package_manager_json: String = row.get(11).map_err(DatabaseError::Sql)?;
    let scripts_json: String = row.get(12).map_err(DatabaseError::Sql)?;
    let git_summary_json: String = row.get(13).map_err(DatabaseError::Sql)?;
    let compose_files_json: String = row.get(14).map_err(DatabaseError::Sql)?;
    let environment_files_json: String = row.get(15).map_err(DatabaseError::Sql)?;
    let local_database_hints_json: String = row.get(16).map_err(DatabaseError::Sql)?;
    let last_scanned_at_ms: Option<i64> = row.get(17).map_err(DatabaseError::Sql)?;
    let scan_health_json: String = row.get(18).map_err(DatabaseError::Sql)?;

    let last_scanned_at = match last_scanned_at_ms {
        Some(value) if value >= 0 => Some(value as u64),
        Some(_) => {
            return Err(DatabaseError::Data(
                "project last_scanned_at_ms was negative".to_owned(),
            ));
        }
        None => None,
    };

    Ok(Project {
        id: row.get(0).map_err(DatabaseError::Sql)?,
        name: row.get(1).map_err(DatabaseError::Sql)?,
        root_path: row.get(2).map_err(DatabaseError::Sql)?,
        canonical_root_path: row.get(3).map_err(DatabaseError::Sql)?,
        tags: decode("tags_json", &tags_json)?,
        notes: row.get(5).map_err(DatabaseError::Sql)?,
        checklist: decode::<Vec<ChecklistItem>>("checklist_json", &checklist_json)?,
        pinned: int_bool(row.get(7).map_err(DatabaseError::Sql)?),
        archived: int_bool(row.get(8).map_err(DatabaseError::Sql)?),
        detected_stacks: decode::<Vec<ProjectStack>>("detected_stacks_json", &stacks_json)?,
        manifests: decode("manifests_json", &manifests_json)?,
        package_manager: decode::<Option<PackageManagerSummary>>(
            "package_manager_json",
            &package_manager_json,
        )?,
        scripts: decode("scripts_json", &scripts_json)?,
        git_summary: decode::<Option<GitSummary>>("git_summary_json", &git_summary_json)?,
        compose_files: decode("compose_files_json", &compose_files_json)?,
        environment_files: decode::<Vec<EnvironmentFileSummary>>(
            "environment_files_json",
            &environment_files_json,
        )?,
        local_database_hints: decode("local_database_hints_json", &local_database_hints_json)?,
        last_scanned_at,
        scan_health: decode::<ProjectScanHealth>("scan_health_json", &scan_health_json)?,
    })
}

fn int_bool(value: i64) -> bool {
    value != 0
}

fn bool_int(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn encode<T>(value: &T) -> Result<String, DatabaseError>
where
    T: Serialize,
{
    serde_json::to_string(value).map_err(|error| DatabaseError::Data(error.to_string()))
}

fn decode<T>(column: &'static str, value: &str) -> Result<T, DatabaseError>
where
    T: DeserializeOwned,
{
    serde_json::from_str(value).map_err(|error| DatabaseError::Data(format!("{column}: {error}")))
}

#[cfg(test)]
mod tests {
    use crate::domain::{
        ProjectIssue, ProjectIssueSeverity, ProjectManifest, ProjectManifestKind, ProjectScanState,
    };

    use super::*;

    fn sample_project() -> Project {
        Project {
            id: "project-1".to_owned(),
            name: "fixture".to_owned(),
            root_path: "C:\\fixtures\\fixture".to_owned(),
            canonical_root_path: "C:\\fixtures\\fixture".to_owned(),
            tags: vec!["demo".to_owned()],
            notes: "read-only fixture".to_owned(),
            checklist: vec![ChecklistItem {
                id: "item-1".to_owned(),
                text: "Verify launch".to_owned(),
                completed: false,
            }],
            pinned: true,
            archived: false,
            detected_stacks: vec![ProjectStack::Node],
            manifests: vec![ProjectManifest {
                kind: ProjectManifestKind::NodePackage,
                relative_path: "package.json".to_owned(),
            }],
            package_manager: Some(PackageManagerSummary {
                name: "npm".to_owned(),
                evidence: vec!["package-lock.json".to_owned()],
                conflicting_lockfiles: Vec::new(),
            }),
            scripts: Vec::new(),
            git_summary: Some(GitSummary::not_repository()),
            compose_files: Vec::new(),
            environment_files: Vec::new(),
            local_database_hints: Vec::new(),
            last_scanned_at: Some(100),
            scan_health: ProjectScanHealth {
                state: ProjectScanState::Warning,
                issues: vec![ProjectIssue::new(
                    "FIXTURE_ONLY",
                    ProjectIssueSeverity::Information,
                    "Fixture issue.",
                )],
            },
        }
    }

    #[test]
    fn projects_round_trip_with_structured_json() {
        let database = Database::open_in_memory().expect("database should initialize");
        let saved = database
            .save_project(&sample_project())
            .expect("project should save");

        assert_eq!(saved.name, "fixture");
        assert_eq!(saved.tags, vec!["demo"]);
        assert_eq!(
            database.list_projects().expect("projects should list"),
            vec![saved]
        );
    }

    #[test]
    fn metadata_update_preserves_scan_evidence() {
        let database = Database::open_in_memory().expect("database should initialize");
        database
            .save_project(&sample_project())
            .expect("project should save");

        let updated = database
            .update_project_metadata(
                "project-1",
                &ProjectMetadataUpdate {
                    tags: vec!["work".to_owned()],
                    notes: "ship it".to_owned(),
                    checklist: Vec::new(),
                    pinned: false,
                    archived: true,
                },
            )
            .expect("metadata update should work")
            .expect("project should exist");

        assert_eq!(updated.tags, vec!["work"]);
        assert_eq!(updated.notes, "ship it");
        assert_eq!(updated.detected_stacks, vec![ProjectStack::Node]);
        assert!(updated.archived);
    }
}
