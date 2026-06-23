use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Application state shared across HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    pub conn: Arc<Mutex<Connection>>,
}

impl AppState {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }
}
