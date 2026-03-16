use bson::oid::ObjectId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VulnEntry {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub scan_id: ObjectId,
    pub owner_id: ObjectId,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub cve_id: Option<String>,
    pub cvss_score: Option<f64>,
    pub affected_url: String,
    pub evidence: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

// ── Request DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateVulnRequest {
    pub scan_id: String,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub cve_id: Option<String>,
    pub cvss_score: Option<f64>,
    pub affected_url: String,
    pub evidence: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct VulnQueryParams {
    pub severity: Option<String>,
    pub scan_id: Option<String>,
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

// ── Response DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct VulnResponse {
    pub id: String,
    pub scan_id: String,
    pub owner_id: String,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub cve_id: Option<String>,
    pub cvss_score: Option<f64>,
    pub affected_url: String,
    pub evidence: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl From<VulnEntry> for VulnResponse {
    fn from(v: VulnEntry) -> Self {
        Self {
            id: v.id.map(|id| id.to_hex()).unwrap_or_default(),
            scan_id: v.scan_id.to_hex(),
            owner_id: v.owner_id.to_hex(),
            title: v.title,
            description: v.description,
            severity: v.severity,
            cve_id: v.cve_id,
            cvss_score: v.cvss_score,
            affected_url: v.affected_url,
            evidence: v.evidence,
            tags: v.tags,
            created_at: v.created_at,
        }
    }
}
