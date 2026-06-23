mod app;
mod infrastructure;
mod presentation;

#[tokio::main]
async fn main() {
    let conn = infrastructure::persistence::db::init_connection("specs.db")
        .expect("failed to initialize sqlite connection");
    let state = app::AppState::new(conn);
    let app = presentation::http::routes::router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Server running on http://0.0.0.0:3000");

    axum::serve(listener, app).await.unwrap();
}
