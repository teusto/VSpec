mod app;
mod infrastructure;
mod observability;
mod presentation;

use tracing::{error, info};
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
    let app = presentation::http::routes::router(state.clone());

    if std::env::var("ENABLE_MCP_STDIO").ok().as_deref() == Some("1") {
        let mcp_state = state.clone();
        tokio::spawn(async move {
            if let Err(err) = presentation::mcp::server::McpServer::new(mcp_state)
                .run_stdio()
                .await
            {
                error!(error = ?err, "MCP stdio server stopped with error");
            }
        });
        info!("MCP stdio server enabled (ENABLE_MCP_STDIO=1)");
    }

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    info!("Server running on http://0.0.0.0:3000");

    axum::serve(listener, app).await.unwrap();
}
