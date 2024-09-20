use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
pub struct HealthchecksCheckInfo {
    pub name: String,
    pub slug: String,
    pub tags: String,
    pub desc: String,
    pub grace: u64,
    pub n_pings: u64,
    pub status: String,
    pub started: bool,
    pub last_ping: Option<String>,
    pub next_ping: Option<String>,
    pub manual_resume: bool,
    pub methods: String,
    pub subject: String,
    pub subject_fail: String,
    pub start_kw: String,
    pub success_kw: String,
    pub failure_kw: String,
    pub filter_subject: bool,
    pub filter_body: bool,
    pub ping_url: String,
    pub update_url: String,
    pub pause_url: String,
    pub resume_url: String,
    pub channels: String,
    pub timeout: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct HealthchecksListChecksResponse {
    pub checks: Vec<HealthchecksCheckInfo>,
}

// curl http://localhost:8000/api/v3/checks/ \
//     --header "X-Api-Key: your-api-key" \
//     --data '{"name": "Backups", "tags": "prod www", "timeout": 3600, "grace": 60}
#[derive(Debug, Deserialize, Serialize)]
pub struct HealthchecksCreateCheckRequest {
    pub name: String,
    pub tags: String,
    pub schedule: String,
    pub grace: u64,
    pub slug: String,
    pub unique: Vec<String>,
    pub timeout: u64,
}
