use bson::oid::ObjectId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScanJob {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub owner_id: ObjectId,
    pub target: String,
    pub engines: Vec<ScanEngine>,
    pub status: ScanStatus,
    pub results: Vec<ScanResult>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScanEngine {
    Whatweb,
    Nikto,
    Gobuster,
    Wapiti,
    Sqlmap,
    Nuclei,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScanStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScanResult {
    pub engine: ScanEngine,
    pub output: String,
    pub exit_code: i32,
    pub ran_at: DateTime<Utc>,
}

// ── Request DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct CreateScanRequest {
    #[validate(url)]
    pub target: String,
    pub engines: Vec<ScanEngine>,
}

#[derive(Debug, Deserialize)]
pub struct AppendResultRequest {
    pub engine: ScanEngine,
    pub output: String,
    pub exit_code: i32,
}

// ── Response DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ScanJobResponse {
    pub id: String,
    pub owner_id: String,
    pub target: String,
    pub engines: Vec<ScanEngine>,
    pub status: ScanStatus,
    pub results: Vec<ScanResult>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl From<ScanJob> for ScanJobResponse {
    fn from(j: ScanJob) -> Self {
        Self {
            id: j.id.map(|id| id.to_hex()).unwrap_or_default(),
            owner_id: j.owner_id.to_hex(),
            target: j.target,
            engines: j.engines,
            status: j.status,
            results: j.results,
            created_at: j.created_at,
            updated_at: j.updated_at,
            finished_at: j.finished_at,
        }
    }
}
