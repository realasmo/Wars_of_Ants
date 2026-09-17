//! Wars of Ants — authoritative game server.
//!
//! Phase 0: skeleton only. Simulation and WebSocket arrive in Phase 4.

use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/healthz", get(|| async { "ok" }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("woa-server listening on 0.0.0.0:8080");
    axum::serve(listener, app).await.unwrap();
}
