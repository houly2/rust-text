use std::{path::PathBuf, sync::Arc};

use anyhow::Result;

use gpui::{point, size, AppContext, Bounds, Global, Pixels};
use rusqlite::{
    params,
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, Value, ValueRef},
    Connection, ToSql,
};
use uuid::Uuid;

use crate::paths::app_data_path;

pub struct DB {
    connection: Arc<Connection>,
}

#[derive(Debug)]
struct ExistingMigration {
    migration: String,
}

#[derive(Debug, Clone, Copy)]
struct Migration {
    migration: &'static str,
    statement: &'static str,
}

const MIGRATIONS: &'static [Migration] = &[
    Migration {
        migration: "0_create_window_positions",
        statement: "CREATE TABLE window_positions (
            id          INTEGER PRIMARY KEY,
            file_path   TEXT NOT NULL,
            display_id  BLOB NOT NULL,
            origin_x    REAL NOT NULL,
            origin_y    REAL NOT NULL,
            size_width  REAL NOT NULL,
            size_height REAL NOT NULL,
            created_at  TEXT DEFAULT current_timestamp,
            UNIQUE(file_path, display_id)
        )",
    },
    Migration {
        migration: "1_create_file_settings",
        statement: "CREATE TABLE file_settings (
            id          INTEGER PRIMARY KEY,
            file_path   TEXT NOT NULL UNIQUE,
            word_wrap   INTEGER DEFAULT 0,
            created_at  TEXT DEFAULT current_timestamp
        )",
    },
];

#[derive(Debug)]
pub struct WindowPosition {
    pub display_id: MyUuid,
    pub bounds: Bounds<f32>,
}

#[derive(Debug)]
pub struct PathSettings {
    pub word_wrap: bool,
}

impl DB {
    pub fn register_global(cx: &mut AppContext) -> Result<()> {
        let app_data_path = app_data_path()?;
        let db_file = app_data_path.join("db.sqlite");
        let connection = Connection::open(db_file)?;

        Self::migrate(&connection)?;
        Self::cleanup(&connection)?;

        let this = Self {
            connection: Arc::new(connection),
        };

        cx.set_global::<DB>(this);

        Ok(())
    }

    pub fn window_position(&self, file_path: PathBuf) -> Option<Vec<WindowPosition>> {
        let file_path_str = file_path.to_str()?;

        let mut stmt = self
            .connection
            .prepare(
                "SELECT display_id, origin_x, origin_y, size_width, size_height
                FROM window_positions
                WHERE file_path = ?1",
            )
            .ok()?;

        let iter = stmt
            .query_map((file_path_str,), |row| {
                Ok(WindowPosition {
                    display_id: row.get(0)?,
                    bounds: Bounds {
                        origin: point(row.get(1)?, row.get(2)?),
                        size: size(row.get(3)?, row.get(4)?),
                    },
                })
            })
            .ok()?;

        Some(iter.filter_map(|m| m.ok()).collect())
    }

    pub fn update_window_position(
        &self,
        file_path: &PathBuf,
        display_id: Uuid,
        bounds: Bounds<Pixels>,
    ) {
        let Some(file_path_str) = file_path.to_str() else {
            return;
        };

        let display_id = MyUuid { 0: display_id };
        let origin_x = MyPixels { 0: bounds.origin.x };
        let origin_y = MyPixels { 0: bounds.origin.y };
        let size_width = MyPixels {
            0: bounds.size.width,
        };
        let size_height = MyPixels {
            0: bounds.size.height,
        };

        _ = self.connection.execute("
            INSERT INTO window_positions (file_path, display_id, origin_x, origin_y, size_width, size_height)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT DO
            UPDATE SET origin_x = ?3, origin_y = ?4, size_width = ?5, size_height = ?6
            ", params![file_path_str, display_id, origin_x, origin_y, size_width, size_height]);
    }

    pub fn path_settings(&self, file_path: &PathBuf) -> Option<PathSettings> {
        let file_path_str = file_path.to_str()?;

        self.connection
            .query_row(
                "SELECT word_wrap FROM file_settings WHERE file_path = ?1",
                params![file_path_str],
                |row| {
                    Ok(PathSettings {
                        word_wrap: row.get(0)?,
                    })
                },
            )
            .ok()
    }

    pub fn update_path_settings(&self, file_path: &PathBuf, word_wrap: bool) {
        let Some(file_path_str) = file_path.to_str() else {
            return;
        };

        _ = self.connection.execute(
            "
            INSERT INTO file_settings (file_path, word_wrap)
            VALUES (?1, ?2)
            ON CONFLICT DO
            UPDATE SET word_wrap = ?2
            ",
            params![file_path_str, word_wrap],
        );
    }

    fn cleanup(connection: &Connection) -> Result<()> {
        connection.execute_batch(
            "
            DELETE FROM window_positions WHERE created_at < datetime('now', '-6 month');
            DELETE FROM file_settings WHERE created_at < datetime('now', '-6 month');
            ",
        )?;

        Ok(())
    }

    fn migrate(connection: &Connection) -> Result<()> {
        _ = connection.execute(
            "CREATE TABLE IF NOT EXISTS migrations (
                id          INTEGER PRIMARY KEY,
                migration   TEXT
            )",
            (),
        );

        let mut check_migration_stmt = connection.prepare("SELECT migration FROM migrations")?;
        let migration_iter = check_migration_stmt
            .query_map([], |row| {
                Ok(ExistingMigration {
                    migration: row.get(0)?,
                })
            })?
            .filter_map(|m| m.ok());

        let mut migrations_to_do = MIGRATIONS.to_vec();
        migrations_to_do.sort_by_key(|m| m.migration);

        for migration in migration_iter {
            if let Ok(r) = migrations_to_do
                .binary_search_by_key(&migration.migration.as_str(), |&m| m.migration)
            {
                migrations_to_do.remove(r);
            }
        }

        for migration in migrations_to_do {
            _ = connection.execute(migration.statement, ())?;
            _ = connection.execute(
                "INSERT INTO migrations (migration) VALUES (?1)",
                (&migration.migration,),
            )?;
        }

        Ok(())
    }
}

impl Global for DB {}

pub trait DbConnection {
    fn db_connection(&self) -> &DB;
}

impl DbConnection for AppContext {
    fn db_connection(&self) -> &DB {
        self.global::<DB>()
    }
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct MyPixels(Pixels);

impl FromSql for MyPixels {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let value = value.as_f64()?;
        Ok(MyPixels {
            0: Pixels { 0: value as f32 },
        })
    }
}

impl ToSql for MyPixels {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Real(self.0.to_f64())))
    }
}

#[derive(Debug, PartialEq)]
pub struct MyUuid(Uuid);

impl FromSql for MyUuid {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let value = value.as_blob()?;
        let Ok(arr) = value.try_into() else {
            return Err(FromSqlError::InvalidType);
        };

        Ok(MyUuid {
            0: Uuid::from_bytes(arr),
        })
    }
}

impl ToSql for MyUuid {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Blob(self.0.as_bytes().into())))
    }
}

impl PartialEq<Uuid> for MyUuid {
    fn eq(&self, other: &Uuid) -> bool {
        self.0 == *other
    }
}
