use rusqlite::{Connection, OptionalExtension, Result, Row, params};
use serde::{Deserialize, Serialize};

/// Persisted project record loaded from the `projects` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,

    pub created_at: String,
    pub updated_at: String,
}

impl Project {
    pub fn from_row(row: &Row<'_>) -> Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

/// Payload used to insert a new project into persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewProject {
    pub name: String,
    pub description: Option<String>,
}

/// SQLite-backed repository for project persistence operations.
pub struct ProjectRepository<'a> {
    conn: &'a Connection,
}

impl<'a> ProjectRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "
            CREATE TABLE IF NOT EXISTS projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            ",
            [],
        )?;

        Ok(())
    }

    pub fn create(&self, new_project: &NewProject) -> Result<Project> {
        self.conn.execute(
            "
            INSERT INTO projects (name, description)
            VALUES (?1, ?2)
            ",
            params![new_project.name, new_project.description],
        )?;

        let id = self.conn.last_insert_rowid();
        self.find_by_id(id)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn find_by_id(&self, id: i64) -> Result<Option<Project>> {
        self.conn
            .query_row(
                "
                SELECT id, name, description, created_at, updated_at
                FROM projects
                WHERE id = ?1
                ",
                params![id],
                Project::from_row,
            )
            .optional()
    }

    pub fn list(&self) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, name, description, created_at, updated_at
            FROM projects
            ORDER BY created_at ASC
            ",
        )?;

        let rows = stmt.query_map([], Project::from_row)?;

        let mut projects = Vec::new();
        for project in rows {
            projects.push(project?);
        }

        Ok(projects)
    }
}
