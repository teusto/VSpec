mod app;
mod infrastructure;
mod presentation;

use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tower_http=info")),
        )
        .init();

    let conn = infrastructure::persistence::db::init_connection("specs.db")
        .expect("failed to initialize sqlite connection");
    let state = app::AppState::new(conn);
    let app = presentation::http::routes::router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    info!("Server running on http://0.0.0.0:3000");

    axum::serve(listener, app).await.unwrap();
}
