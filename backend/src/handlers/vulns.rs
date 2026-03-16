use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json,
};
use bson::{doc, oid::ObjectId};
use chrono::Utc;
use futures::TryStreamExt;

use crate::{
    db::AppState,
    middleware::auth::AuthUser,
    models::vuln::{CreateVulnRequest, VulnEntry, VulnQueryParams, VulnResponse},
};
use super::ApiError;

const VULNS_COLL: &str = "vulns";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/vulns",     post(create_vuln))
        .route("/api/vulns",     get(list_vulns))
        .route("/api/vulns/:id", get(get_vuln))
        .route("/api/vulns/:id", delete(delete_vuln))
}

// POST /api/vulns
async fn create_vuln(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateVulnRequest>,
) -> Result<(StatusCode, Json<VulnResponse>), ApiError> {
    let owner_id = ObjectId::parse_str(&claims.sub)
        .map_err(|_| ApiError::BadRequest("Invalid user id".into()))?;

    let scan_id = ObjectId::parse_str(&body.scan_id)
        .map_err(|_| ApiError::BadRequest("Invalid scan_id".into()))?;

    let entry = VulnEntry {
        id: None,
        scan_id,
        owner_id,
        title: body.title,
        description: body.description,
        severity: body.severity,
        cve_id: body.cve_id,
        cvss_score: body.cvss_score,
        affected_url: body.affected_url,
        evidence: body.evidence,
        tags: body.tags.unwrap_or_default(),
        created_at: Utc::now(),
    };

    let coll = state.db.collection::<VulnEntry>(VULNS_COLL);
    let insert = coll.insert_one(&entry, None).await?;
    let oid = insert.inserted_id.as_object_id().unwrap();

    let mut returned = entry;
    returned.id = Some(oid);

    Ok((StatusCode::CREATED, Json(VulnResponse::from(returned))))
}

// GET /api/vulns?severity=high&scan_id=...&page=1&limit=20
async fn list_vulns(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(params): Query<VulnQueryParams>,
) -> Result<Json<Vec<VulnResponse>>, ApiError> {
    use crate::models::user::UserRole;

    let mut filter = doc! {};

    // Non-admins only see their own
    if claims.role != UserRole::Admin {
        let oid = ObjectId::parse_str(&claims.sub)
            .map_err(|_| ApiError::BadRequest("Invalid user id".into()))?;
        filter.insert("owner_id", oid);
    }

    if let Some(sev) = &params.severity {
        filter.insert("severity", sev.to_lowercase());
    }

    if let Some(scan_id_str) = &params.scan_id {
        let sid = ObjectId::parse_str(scan_id_str)
            .map_err(|_| ApiError::BadRequest("Invalid scan_id".into()))?;
        filter.insert("scan_id", sid);
    }

    let page  = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).min(100);
    let skip  = (page - 1) * limit;

    let options = mongodb::options::FindOptions::builder()
        .skip(skip)
        .limit(limit as i64)
        .sort(doc! { "created_at": -1 })
        .build();

    let coll = state.db.collection::<VulnEntry>(VULNS_COLL);
    let cursor = coll.find(filter, options).await?;
    let vulns: Vec<VulnEntry> = cursor.try_collect().await?;

    Ok(Json(vulns.into_iter().map(VulnResponse::from).collect()))
}

// GET /api/vulns/:id
async fn get_vuln(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
) -> Result<Json<VulnResponse>, ApiError> {
    let oid = ObjectId::parse_str(&id)
        .map_err(|_| ApiError::BadRequest("Invalid vuln id".into()))?;

    let coll = state.db.collection::<VulnEntry>(VULNS_COLL);
    let vuln = coll
        .find_one(doc! { "_id": oid }, None)
        .await?
        .ok_or_else(|| ApiError::NotFound("Vuln not found".into()))?;

    use crate::models::user::UserRole;
    if claims.role != UserRole::Admin && vuln.owner_id.to_hex() != claims.sub {
        return Err(ApiError::NotFound("Vuln not found".into()));
    }

    Ok(Json(VulnResponse::from(vuln)))
}

// DELETE /api/vulns/:id
async fn delete_vuln(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    use crate::models::user::UserRole;

    let oid = ObjectId::parse_str(&id)
        .map_err(|_| ApiError::BadRequest("Invalid vuln id".into()))?;

    let coll = state.db.collection::<VulnEntry>(VULNS_COLL);
    let vuln = coll
        .find_one(doc! { "_id": oid }, None)
        .await?
        .ok_or_else(|| ApiError::NotFound("Vuln not found".into()))?;

    if claims.role != UserRole::Admin && vuln.owner_id.to_hex() != claims.sub {
        return Err(ApiError::NotFound("Vuln not found".into()));
    }

    coll.delete_one(doc! { "_id": oid }, None).await?;
    Ok(StatusCode::NO_CONTENT)
}
