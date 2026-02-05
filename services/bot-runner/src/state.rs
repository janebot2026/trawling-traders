//! State Management - Write "chatty" state files for observability

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, error};

/// Manages state files for observability
pub struct StateManager {
    state_dir: PathBuf,
    journal_dir: PathBuf,
}

/// Current status (state/now.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NowState {
    pub timestamp: String,
    pub mode: String,
    pub status: String,
    pub current_focus: String,
    pub current_action: String,
    pub last_plan_id: Option<String>,
    pub last_plan_time: Option<String>,
    pub session_stats: SessionStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub trades_today: u32,
    pub pnl_today: String,
}

impl StateManager {
    pub fn new(workspace_dir: &str) -> Self {
        let base = PathBuf::from(workspace_dir);
        Self {
            state_dir: base.join("state"),
            journal_dir: base.join("journal").join("decisions"),
        }
    }
    
    pub async fn init(&self) -> anyhow::Result<()> {
        fs::create_dir_all(&self.state_dir).await?;
        fs::create_dir_all(&self.journal_dir).await?;
        Ok(())
    }
    
    pub async fn write_now(&self,
        state: &NowState,
    ) -> anyhow::Result<()> {
        let path = self.state_dir.join("now.json");
        let json = serde_json::to_string_pretty(state)?;
        fs::write(&path, json).await?;
        debug!("Wrote state/now.json");
        Ok(())
    }
}
