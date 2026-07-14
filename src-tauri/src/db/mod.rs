use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

#[cfg(test)]
use rusqlite::OptionalExtension;
use rusqlite::{Connection, params};
use thiserror::Error;

use crate::domain::{AppSettings, RefreshMode};

const INITIAL_MIGRATION: &str = include_str!("../../migrations/0001_initial.sql");
const PROJECT_MIGRATION: &str = include_str!("../../migrations/0002_project_scans.sql");
const RECORDINGS_MIGRATION: &str = include_str!("../../migrations/0003_metric_recordings.sql");
const CLEANER_MIGRATION: &str = include_str!("../../migrations/0004_cleaner_quarantine.sql");
const CONTENT_MIGRATION: &str = include_str!("../../migrations/0005_content_portfolio.sql");
const REFRESH_MODE_MIGRATION: &str = include_str!("../../migrations/0006_refresh_modes.sql");
const LATEST_SCHEMA_VERSION: i64 = 6;

mod cleaner;
mod projects;
mod recordings;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("the local database could not be opened")]
    Open(#[source] rusqlite::Error),
    #[error("the local database directory could not be created")]
    Directory(#[source] std::io::Error),
    #[error("the local database operation failed")]
    Sql(#[source] rusqlite::Error),
    #[error("the local database contains invalid structured data: {0}")]
    Data(String),
    #[error("the local database lock is unavailable")]
    LockUnavailable,
    #[error("the local database schema version {found} is newer than this application supports")]
    SchemaTooNew { found: i64 },
    #[error("the required setting {key} is missing from the local database")]
    MissingSetting { key: &'static str },
    #[error("the stored value for setting {key} is invalid")]
    InvalidSetting { key: &'static str },
}

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, DatabaseError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(DatabaseError::Directory)?;
        }
        let connection = Connection::open(path).map_err(DatabaseError::Open)?;
        Self::initialize(connection)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, DatabaseError> {
        let connection = Connection::open_in_memory().map_err(DatabaseError::Open)?;
        Self::initialize(connection)
    }

    fn initialize(connection: Connection) -> Result<Self, DatabaseError> {
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(DatabaseError::Sql)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(DatabaseError::Sql)?;

        let database = Self {
            connection: Mutex::new(connection),
        };
        database.apply_migrations()?;
        Ok(database)
    }

    fn connection(&self) -> Result<MutexGuard<'_, Connection>, DatabaseError> {
        self.connection
            .lock()
            .map_err(|_| DatabaseError::LockUnavailable)
    }

    pub fn apply_migrations(&self) -> Result<(), DatabaseError> {
        let mut connection = self.connection()?;
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS schema_migrations (\
                 version INTEGER PRIMARY KEY, \
                 applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP\
                 );",
            )
            .map_err(DatabaseError::Sql)?;

        let current_version = connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get::<_, Option<i64>>(0)
            })
            .map_err(DatabaseError::Sql)?
            .unwrap_or(0);

        if current_version > LATEST_SCHEMA_VERSION {
            return Err(DatabaseError::SchemaTooNew {
                found: current_version,
            });
        }

        if current_version < 1 {
            let transaction = connection.transaction().map_err(DatabaseError::Sql)?;
            transaction
                .execute_batch(INITIAL_MIGRATION)
                .map_err(DatabaseError::Sql)?;
            transaction.commit().map_err(DatabaseError::Sql)?;
        }
        if current_version < 2 {
            let transaction = connection.transaction().map_err(DatabaseError::Sql)?;
            transaction
                .execute_batch(PROJECT_MIGRATION)
                .map_err(DatabaseError::Sql)?;
            transaction.commit().map_err(DatabaseError::Sql)?;
        }
        if current_version < 3 {
            let transaction = connection.transaction().map_err(DatabaseError::Sql)?;
            transaction
                .execute_batch(RECORDINGS_MIGRATION)
                .map_err(DatabaseError::Sql)?;
            transaction.commit().map_err(DatabaseError::Sql)?;
        }
        if current_version < 4 {
            let transaction = connection.transaction().map_err(DatabaseError::Sql)?;
            transaction
                .execute_batch(CLEANER_MIGRATION)
                .map_err(DatabaseError::Sql)?;
            transaction.commit().map_err(DatabaseError::Sql)?;
        }
        if current_version < 5 {
            let transaction = connection.transaction().map_err(DatabaseError::Sql)?;
            transaction
                .execute_batch(CONTENT_MIGRATION)
                .map_err(DatabaseError::Sql)?;
            transaction.commit().map_err(DatabaseError::Sql)?;
        }
        if current_version < 6 {
            let transaction = connection.transaction().map_err(DatabaseError::Sql)?;
            transaction
                .execute_batch(REFRESH_MODE_MIGRATION)
                .map_err(DatabaseError::Sql)?;
            transaction.commit().map_err(DatabaseError::Sql)?;
        }

        Ok(())
    }

    pub fn get_settings(&self) -> Result<AppSettings, DatabaseError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare("SELECT key, value FROM settings")
            .map_err(DatabaseError::Sql)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(DatabaseError::Sql)?;
        let mut values = HashMap::new();
        for row in rows {
            let (key, value) = row.map_err(DatabaseError::Sql)?;
            values.insert(key, value);
        }

        let refresh_mode = required(&values, "refresh_mode")?
            .parse::<RefreshMode>()
            .map_err(|()| DatabaseError::InvalidSetting {
                key: "refresh_mode",
            })?;

        Ok(AppSettings {
            refresh_mode,
            external_network_checks: parse_bool(
                "external_network_checks",
                required(&values, "external_network_checks")?,
            )?,
            metric_history_enabled: parse_bool(
                "metric_history_enabled",
                required(&values, "metric_history_enabled")?,
            )?,
            reduced_motion: parse_bool("reduced_motion", required(&values, "reduced_motion")?)?,
        })
    }

    pub fn update_settings(&self, settings: &AppSettings) -> Result<AppSettings, DatabaseError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(DatabaseError::Sql)?;

        update_setting(&transaction, "refresh_mode", settings.refresh_mode.as_str())?;
        update_setting(
            &transaction,
            "external_network_checks",
            bool_text(settings.external_network_checks),
        )?;
        update_setting(
            &transaction,
            "metric_history_enabled",
            bool_text(settings.metric_history_enabled),
        )?;
        update_setting(
            &transaction,
            "reduced_motion",
            bool_text(settings.reduced_motion),
        )?;

        transaction.commit().map_err(DatabaseError::Sql)?;
        Ok(settings.clone())
    }

    #[cfg(test)]
    fn schema_version(&self) -> Result<Option<i64>, DatabaseError> {
        self.connection()?
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(DatabaseError::Sql)
    }
}

fn required<'a>(
    values: &'a HashMap<String, String>,
    key: &'static str,
) -> Result<&'a str, DatabaseError> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or(DatabaseError::MissingSetting { key })
}

fn parse_bool(key: &'static str, value: &str) -> Result<bool, DatabaseError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(DatabaseError::InvalidSetting { key }),
    }
}

const fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn update_setting(
    transaction: &rusqlite::Transaction<'_>,
    key: &'static str,
    value: &str,
) -> Result<(), DatabaseError> {
    let changed = transaction
        .execute(
            "UPDATE settings \
             SET value = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
             WHERE key = ?1",
            params![key, value],
        )
        .map_err(DatabaseError::Sql)?;
    if changed == 1 {
        Ok(())
    } else {
        Err(DatabaseError::MissingSetting { key })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_idempotent_and_records_version() {
        let database = Database::open_in_memory().expect("database should initialize");
        database
            .apply_migrations()
            .expect("reapplying migrations should be safe");

        assert_eq!(
            database
                .schema_version()
                .expect("version query should work"),
            Some(6)
        );
        assert_eq!(
            database.get_settings().expect("defaults should load"),
            AppSettings::default()
        );
    }

    #[test]
    fn settings_update_is_atomic_and_round_trips() {
        let database = Database::open_in_memory().expect("database should initialize");
        let expected = AppSettings {
            refresh_mode: RefreshMode::Fast,
            external_network_checks: true,
            metric_history_enabled: true,
            reduced_motion: true,
        };

        assert_eq!(
            database
                .update_settings(&expected)
                .expect("update should work"),
            expected
        );
        assert_eq!(
            database.get_settings().expect("settings should load"),
            expected
        );
    }

    #[test]
    fn malformed_stored_setting_is_rejected() {
        let database = Database::open_in_memory().expect("database should initialize");
        database
            .connection()
            .expect("connection lock should work")
            .execute(
                "UPDATE settings SET value = 'sometimes' WHERE key = 'reduced_motion'",
                [],
            )
            .expect("fixture mutation should work");

        assert!(matches!(
            database.get_settings(),
            Err(DatabaseError::InvalidSetting {
                key: "reduced_motion"
            })
        ));
    }

    #[test]
    fn every_connection_enables_foreign_keys() {
        let database = Database::open_in_memory().expect("database should initialize");
        let enabled: i64 = database
            .connection()
            .expect("connection lock should work")
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("pragma query should work");

        assert_eq!(enabled, 1);
    }

    #[test]
    fn file_database_reopens_with_persisted_settings() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join("desktop-manager.sqlite3");
        let expected = AppSettings {
            refresh_mode: RefreshMode::Normal,
            external_network_checks: false,
            metric_history_enabled: true,
            reduced_motion: true,
        };

        {
            let database = Database::open(&path).expect("file database should initialize");
            database
                .update_settings(&expected)
                .expect("settings update should work");
        }
        let reopened = Database::open(&path).expect("database should reopen");

        assert_eq!(
            reopened.get_settings().expect("settings should persist"),
            expected
        );
    }

    #[test]
    fn refresh_mode_migration_maps_every_legacy_value() {
        for (legacy, expected) in [
            ("eco", RefreshMode::Normal),
            ("balanced", RefreshMode::Normal),
            ("realtime", RefreshMode::Fast),
        ] {
            let connection = Connection::open_in_memory().expect("fixture database should open");
            connection
                .execute_batch(INITIAL_MIGRATION)
                .expect("initial migration should apply");
            connection
                .execute_batch(PROJECT_MIGRATION)
                .expect("project migration should apply");
            connection
                .execute_batch(RECORDINGS_MIGRATION)
                .expect("recordings migration should apply");
            connection
                .execute_batch(CLEANER_MIGRATION)
                .expect("cleaner migration should apply");
            connection
                .execute_batch(CONTENT_MIGRATION)
                .expect("content compatibility migration should apply");
            connection
                .execute(
                    "UPDATE settings SET value = ?1 WHERE key = 'refresh_mode'",
                    [legacy],
                )
                .expect("legacy setting should be written");

            let database = Database::initialize(connection).expect("migration 6 should apply");
            assert_eq!(
                database
                    .get_settings()
                    .expect("migrated settings should load")
                    .refresh_mode,
                expected
            );
        }
    }
}
