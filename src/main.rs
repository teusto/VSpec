mod app;
mod infrastructure;
mod presentation;

#[tokio::main]
async fn main() {
    let app = presentation::http::routes::router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Server running on http://0.0.0.0:3000");

    axum::serve(listener, app).await.unwrap();
}
