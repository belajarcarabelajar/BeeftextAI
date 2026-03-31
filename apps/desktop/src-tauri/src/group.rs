use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

/// Group model — mirrors Beeftext's Group class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub uuid: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub created_at: String,
    pub modified_at: String,
}

impl Group {
    pub fn new(name: String, description: String) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            uuid: Uuid::new_v4().to_string(),
            name,
            description,
            enabled: true,
            created_at: now.clone(),
            modified_at: now,
        }
    }
}
