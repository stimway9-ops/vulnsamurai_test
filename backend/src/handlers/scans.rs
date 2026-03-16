use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
    Json,
};
use bson::{doc, oid::ObjectId};
use chrono::Utc;
use futures::TryStreamExt;
use validator::Validate;

use crate::{
    db::AppState,
    middleware::auth::AuthUser,
    models::scan::{
        AppendResultRequest, CreateScanRequest, ScanJob, ScanJobResponse, ScanResult, ScanStatus,
    },
};
use super::ApiError;

const SCANS_COLL: &str = "scans";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/scans",                         post(create_scan))
        .route("/api/scans",                         get(list_scans))
        .route("/api/scans/:id",                     get(get_scan))
        .route("/api/scans/:id/status",              put(update_status))
        .route("/api/scans/:id/results",             post(append_result))
}

// POST /api/scans
async fn create_scan(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateScanRequest>,
) -> Result<(StatusCode, Json<ScanJobResponse>), ApiError> {
    body.validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    if body.engines.is_empty() {
        return Err(ApiError::BadRequest("At least one engine required".into()));
    }

    let owner_id = ObjectId::parse_str(&claims.sub)
        .map_err(|_| ApiError::BadRequest("Invalid user id".into()))?;

    let now = Utc::now();
    let job = ScanJob {
        id: None,
        owner_id,
        target: body.target,
        engines: body.engines,
        status: ScanStatus::Queued,
        results: vec![],
        created_at: now,
        updated_at: now,
        finished_at: None,
    };

    let coll = state.db.collection::<ScanJob>(SCANS_COLL);
    let insert = coll.insert_one(&job, None).await?;
    let oid = insert.inserted_id.as_object_id().unwrap();

    let mut returned = job;
    returned.id = Some(oid);

    Ok((StatusCode::CREATED, Json(ScanJobResponse::from(returned))))
}

// GET /api/scans  (only own scans, unless admin)
async fn list_scans(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Vec<ScanJobResponse>>, ApiError> {
    use crate::models::user::UserRole;

    let coll = state.db.collection::<ScanJob>(SCANS_COLL);

    let filter = match claims.role {
        UserRole::Admin => doc! {},
        UserRole::Analyst => {
            let oid = ObjectId::parse_str(&claims.sub)
                .map_err(|_| ApiError::BadRequest("Invalid user id".into()))?;
            doc! { "owner_id": oid }
        }
    };

    let cursor = coll.find(filter, None).await?;
    let jobs: Vec<ScanJob> = cursor.try_collect().await?;

    Ok(Json(jobs.into_iter().map(ScanJobResponse::from).collect()))
}

// GET /api/scans/:id
async fn get_scan(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ScanJobResponse>, ApiError> {
    let oid = ObjectId::parse_str(&id)
        .map_err(|_| ApiError::BadRequest("Invalid scan id".into()))?;

    let coll = state.db.collection::<ScanJob>(SCANS_COLL);
    let job = coll
        .find_one(doc! { "_id": oid }, None)
        .await?
        .ok_or_else(|| ApiError::NotFound("Scan not found".into()))?;

    // Analysts can only see their own scans
    use crate::models::user::UserRole;
    if claims.role != UserRole::Admin && job.owner_id.to_hex() != claims.sub {
        return Err(ApiError::NotFound("Scan not found".into()));
    }

    Ok(Json(ScanJobResponse::from(job)))
}

// PUT /api/scans/:id/status  { "status": "running" | "completed" | "failed" }
async fn update_status(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ScanJobResponse>, ApiError> {
    let oid = ObjectId::parse_str(&id)
        .map_err(|_| ApiError::BadRequest("Invalid scan id".into()))?;

    let status_str = body.get("status")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("Missing status field".into()))?;

    let status: ScanStatus = serde_json::from_value(serde_json::json!(status_str))
        .map_err(|_| ApiError::BadRequest("Invalid status value".into()))?;

    let now = Utc::now();
    let mut update_doc = doc! {
        "$set": {
            "status": bson::to_bson(&status).unwrap(),
            "updated_at": bson::DateTime::from_chrono(now),
        }
    };

    if status == ScanStatus::Completed || status == ScanStatus::Failed {
        update_doc.get_document_mut("$set").unwrap()
            .insert("finished_at", bson::DateTime::from_chrono(now));
    }

    let coll = state.db.collection::<ScanJob>(SCANS_COLL);
    coll.update_one(doc! { "_id": oid }, update_doc, None).await?;

    let job = coll
        .find_one(doc! { "_id": oid }, None)
        .await?
        .ok_or_else(|| ApiError::NotFound("Scan not found".into()))?;

    Ok(Json(ScanJobResponse::from(job)))
}

// POST /api/scans/:id/results
async fn append_result(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(id): Path<String>,
    Json(body): Json<AppendResultRequest>,
) -> Result<Json<ScanJobResponse>, ApiError> {
    let oid = ObjectId::parse_str(&id)
        .map_err(|_| ApiError::BadRequest("Invalid scan id".into()))?;

    let result = ScanResult {
        engine: body.engine,
        output: body.output,
        exit_code: body.exit_code,
        ran_at: Utc::now(),
    };

    let result_bson = bson::to_bson(&result)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let now = Utc::now();
    let coll = state.db.collection::<ScanJob>(SCANS_COLL);
    coll.update_one(
        doc! { "_id": oid },
        doc! {
            "$push": { "results": result_bson },
            "$set":  { "updated_at": bson::DateTime::from_chrono(now) }
        },
        None,
    ).await?;

    let job = coll
        .find_one(doc! { "_id": oid }, None)
        .await?
        .ok_or_else(|| ApiError::NotFound("Scan not found".into()))?;

    Ok(Json(ScanJobResponse::from(job)))
}
