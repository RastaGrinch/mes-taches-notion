use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub notion_id: Option<String>,
    pub title: String,
    pub done: bool,
    pub status: String,
    pub priority: String,
    pub due: Option<String>,
    pub completed_at: Option<String>,
    pub assignee: String,
    pub source: String,
    pub task_type: String,
    pub description: String,
    pub comment: String,
    pub notion_url: Option<String>,
    pub dirty: bool,
    pub sync_error: Option<String>,
    pub updated_at: String,
}

impl Task {
    pub fn new(title: String, settings: &Settings) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            notion_id: None,
            title,
            done: false,
            status: settings.default_status.clone(),
            priority: settings.default_priority.clone(),
            due: None,
            completed_at: None,
            assignee: settings.default_assignee.clone(),
            source: settings.default_source.clone(),
            task_type: String::new(),
            description: String::new(),
            comment: String::new(),
            notion_url: None,
            dirty: true,
            sync_error: None,
            updated_at: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub data_source_id: String,
    pub default_status: String,
    pub default_priority: String,
    pub default_assignee: String,
    pub default_source: String,
    pub sync_interval_minutes: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            data_source_id: String::new(),
            default_status: "A faire".into(),
            default_priority: "P3 - A faire".into(),
            default_assignee: String::new(),
            default_source: String::new(),
            sync_interval_minutes: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub configured: bool,
    pub token_stored: bool,
    pub settings: Settings,
    pub pending_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectInput {
    pub token: String,
    pub settings: Settings,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub pushed: usize,
    pub pulled: usize,
    pub failed: usize,
    pub pending: usize,
    pub message: String,
}
