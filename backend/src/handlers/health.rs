use axum::{Router, routing::get, Json};
use serde_json::json;
use crate::db::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "vuln-samurai-api",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
