mod db;
mod handlers;
mod middleware;
mod models;

use axum::{
    Router,
    http::{HeaderValue, Method},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::db::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env
    dotenvy::dotenv().ok();

    // Tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "vuln_samurai_api=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // DB
    let state = AppState::init().await?;
    tracing::info!("Connected to MongoDB");

    // CORS
    let frontend_origin = std::env::var("FRONTEND_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:3000".into());

    let cors = CorsLayer::new()
        .allow_origin(frontend_origin.parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        .merge(handlers::auth::router())
        .merge(handlers::scans::router())
        .merge(handlers::vulns::router())
        .merge(handlers::health::router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}
