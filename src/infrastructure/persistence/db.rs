use crate::infrastructure::persistence::{
    project::ProjectRepository,
    spec::SpecRepository,
};
use rusqlite::{Connection, Result};

/// Opens a SQLite connection, enables foreign keys, and initializes schema.
pub fn init_connection(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;

    conn.execute("PRAGMA foreign_keys = ON", [])?;

    ProjectRepository::new(&conn).init_schema()?;
    SpecRepository::new(&conn).init_schema()?;

    Ok(conn)
}
