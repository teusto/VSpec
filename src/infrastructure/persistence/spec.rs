use rusqlite::{Connection, Error as SqlError, ErrorCode, Result, Row, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// JSON template stored in `Spec.content` and serialized to SQLite TEXT.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpecContentTemplate {
    pub summary: String,
    pub goals: Vec<String>,
    pub requirements: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SpecTag {
    Architecture,
    Business,
    Qa,
    Security,
}

impl SpecTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Architecture => "architecture",
            Self::Business => "business",
            Self::Qa => "qa",
            Self::Security => "security",
        }
    }
}

impl FromStr for SpecTag {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "architecture" => Ok(Self::Architecture),
            "business" => Ok(Self::Business),
            "qa" => Ok(Self::Qa),
            "security" => Ok(Self::Security),
            _ => Err(format!("invalid spec tag: {value}")),
        }
    }
}

/// Persisted specification record for a project and tag pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub id: i64,
    pub project_id: i64,
    pub tag: SpecTag,
    pub content: SpecContentTemplate,
    pub created_at: String,
    pub updated_at: String,
}

impl Spec {
    fn from_row(row: &Row<'_>) -> std::result::Result<Self, RepoError> {
        let tag: String = row.get("tag")?;
        let raw_content: String = row.get("content")?;

        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            tag: SpecTag::from_str(&tag).map_err(RepoError::InvalidTag)?,
            content: serde_json::from_str(&raw_content)
                .map_err(|error| RepoError::JsonDecode(error.to_string()))?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

/// Payload used to create a new spec row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSpec {
    pub project_id: i64,
    pub tag: SpecTag,
    pub content: SpecContentTemplate,
}

#[derive(Debug)]
pub enum RepoError {
    Sqlite(SqlError),
    UniqueConstraint,
    InvalidTag(String),
    JsonEncode(String),
    JsonDecode(String),
}

impl From<SqlError> for RepoError {
    fn from(value: SqlError) -> Self {
        Self::Sqlite(value)
    }
}

/// SQLite-backed repository for CRUD-like operations on specs.
pub struct SpecRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SpecRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn init_schema(&self) -> Result<()> {
        if !self.table_exists("specs")? {
            self.create_specs_table_with_fk()?;
            return Ok(());
        }

        if self.specs_has_project_fk()? {
            return Ok(());
        }

        self.migrate_specs_table_add_project_fk()?;

        Ok(())
    }

    pub fn create(&self, new_spec: &NewSpec) -> std::result::Result<Spec, RepoError> {
        let content_json = serde_json::to_string(&new_spec.content)
            .map_err(|error| RepoError::JsonEncode(error.to_string()))?;

        let result = self.conn.execute(
            "
            INSERT INTO specs (project_id, tag, content)
            VALUES (?1, ?2, ?3)
            ",
            params![new_spec.project_id, new_spec.tag.as_str(), content_json],
        );

        if let Err(error) = result {
            if is_unique_violation(&error) {
                return Err(RepoError::UniqueConstraint);
            }

            return Err(RepoError::Sqlite(error));
        }

        let id = self.conn.last_insert_rowid();
        self.find_by_id(id)?
            .ok_or_else(|| RepoError::Sqlite(SqlError::QueryReturnedNoRows))
    }

    pub fn find_by_project_and_tag(
        &self,
        project_id: i64,
        tag: SpecTag,
    ) -> std::result::Result<Option<Spec>, RepoError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, project_id, tag, content, created_at, updated_at
            FROM specs
            WHERE project_id = ?1 AND tag = ?2
            ",
        )?;

        let mut rows = stmt.query(params![project_id, tag.as_str()])?;

        if let Some(row) = rows.next()? {
            return Ok(Some(Spec::from_row(row)?));
        }

        Ok(None)
    }

    pub fn list_by_project(&self, project_id: i64) -> std::result::Result<Vec<Spec>, RepoError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, project_id, tag, content, created_at, updated_at
            FROM specs
            WHERE project_id = ?1
            ORDER BY created_at ASC
            ",
        )?;

        let mut specs = Vec::new();
        let mut rows = stmt.query(params![project_id])?;

        while let Some(row) = rows.next()? {
            specs.push(Spec::from_row(row)?);
        }

        Ok(specs)
    }

    fn find_by_id(&self, id: i64) -> std::result::Result<Option<Spec>, RepoError> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, project_id, tag, content, created_at, updated_at
            FROM specs
            WHERE id = ?1
            ",
        )?;

        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            return Ok(Some(Spec::from_row(row)?));
        }

        Ok(None)
    }

    fn table_exists(&self, table_name: &str) -> Result<bool> {
        let exists: i64 = self.conn.query_row(
            "
            SELECT EXISTS(
                SELECT 1
                FROM sqlite_master
                WHERE type = 'table' AND name = ?1
            )
            ",
            params![table_name],
            |row| row.get(0),
        )?;

        Ok(exists == 1)
    }

    fn specs_has_project_fk(&self) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare("PRAGMA foreign_key_list(specs)")?;

        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let referenced_table: String = row.get("table")?;
            let from_col: String = row.get("from")?;
            let to_col: String = row.get("to")?;

            if referenced_table == "projects" && from_col == "project_id" && to_col == "id" {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn create_specs_table_with_fk(&self) -> Result<()> {
        self.conn.execute(
            "
            CREATE TABLE IF NOT EXISTS specs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL,
                tag TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                UNIQUE(project_id, tag)
            )
            ",
            [],
        )?;

        Ok(())
    }

    fn migrate_specs_table_add_project_fk(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            BEGIN IMMEDIATE;

            CREATE TABLE specs_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL,
                tag TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                UNIQUE(project_id, tag)
            );

            INSERT INTO specs_new (id, project_id, tag, content, created_at, updated_at)
            SELECT id, project_id, tag, content, created_at, updated_at
            FROM specs;

            DROP TABLE specs;
            ALTER TABLE specs_new RENAME TO specs;

            COMMIT;
            ",
        )?;

        Ok(())
    }
}

fn is_unique_violation(error: &SqlError) -> bool {
    matches!(
        error,
        SqlError::SqliteFailure(sqlite_error, _)
            if sqlite_error.code == ErrorCode::ConstraintViolation
    )
}
