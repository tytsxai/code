//! Checkpoint management for Auto Drive session persistence and recovery.
//!
//! This module provides functionality to save and restore Auto Drive sessions,
//! enabling recovery from interruptions without losing progress.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use code_protocol::models::ResponseItem;

use crate::AutoRunPhase;

/// Token usage statistics for a checkpoint.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

/// A checkpoint representing the state of an Auto Drive session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoDriveCheckpoint {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Unique identifier for this session.
    pub session_id: String,
    /// The original goal/task description.
    pub goal: String,
    /// Conversation history snapshot.
    pub history: Vec<ResponseItem>,
    /// Number of turns completed.
    pub turns_completed: usize,
    /// Cumulative token usage.
    pub token_usage: TokenUsage,
    /// Current phase when checkpoint was created.
    pub phase: CheckpointPhase,
    /// When the checkpoint was first created.
    pub created_at: DateTime<Utc>,
    /// When the checkpoint was last updated.
    pub updated_at: DateTime<Utc>,
    /// SHA-256 checksum for integrity validation.
    pub checksum: String,
}

/// Simplified phase representation for serialization.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckpointPhase {
    Idle,
    Active,
    Paused,
    Completed,
}

impl From<&AutoRunPhase> for CheckpointPhase {
    fn from(phase: &AutoRunPhase) -> Self {
        match phase {
            AutoRunPhase::Idle => CheckpointPhase::Idle,
            AutoRunPhase::Active => CheckpointPhase::Active,
            AutoRunPhase::PausedManual { .. } => CheckpointPhase::Paused,
            _ => CheckpointPhase::Active,
        }
    }
}

/// Summary information for listing recoverable sessions.
#[derive(Clone, Debug)]
pub struct CheckpointSummary {
    pub session_id: String,
    pub goal_preview: String,
    pub turns_completed: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Manages checkpoint persistence and recovery.
pub struct CheckpointManager {
    checkpoint_dir: PathBuf,
    current_session_id: Option<String>,
    _auto_save_interval: Duration,
}

impl CheckpointManager {
    /// Creates a new CheckpointManager with the specified directory.
    pub fn new(checkpoint_dir: PathBuf) -> Self {
        Self {
            checkpoint_dir,
            current_session_id: None,
            _auto_save_interval: Duration::from_secs(60),
        }
    }

    /// Creates a new CheckpointManager for testing.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self::new(std::env::temp_dir().join("code-checkpoints-test"))
    }

    /// Creates a new checkpoint for a session.
    pub fn create(&mut self, goal: &str, session_id: &str) -> Result<AutoDriveCheckpoint> {
        self.current_session_id = Some(session_id.to_string());
        let now = Utc::now();

        let mut checkpoint = AutoDriveCheckpoint {
            version: 1,
            session_id: session_id.to_string(),
            goal: goal.to_string(),
            history: Vec::new(),
            turns_completed: 0,
            token_usage: TokenUsage::default(),
            phase: CheckpointPhase::Idle,
            created_at: now,
            updated_at: now,
            checksum: String::new(),
        };

        checkpoint.checksum = Self::calculate_checksum(&checkpoint);
        Ok(checkpoint)
    }

    /// Saves a checkpoint to disk.
    pub fn save(&self, checkpoint: &AutoDriveCheckpoint) -> Result<()> {
        std::fs::create_dir_all(&self.checkpoint_dir)?;

        let path = self.checkpoint_path(&checkpoint.session_id);
        let temp_path = path.with_extension("tmp");

        // Serialize and write to temp file first
        let json = serde_json::to_string_pretty(checkpoint)?;
        std::fs::write(&temp_path, &json)?;

        // Atomic rename
        std::fs::rename(&temp_path, &path)?;

        tracing::debug!(
            session_id = %checkpoint.session_id,
            turns = checkpoint.turns_completed,
            "Checkpoint saved"
        );

        Ok(())
    }

    /// Restores a checkpoint from disk.
    pub fn restore(&self, session_id: &str) -> Result<Option<AutoDriveCheckpoint>> {
        let path = self.checkpoint_path(session_id);

        if !path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(&path)?;
        let checkpoint: AutoDriveCheckpoint = serde_json::from_str(&json)?;

        // Validate checksum
        if !self.validate(&checkpoint)? {
            anyhow::bail!("Checkpoint integrity validation failed");
        }

        tracing::info!(
            session_id = %checkpoint.session_id,
            turns = checkpoint.turns_completed,
            "Checkpoint restored"
        );

        Ok(Some(checkpoint))
    }

    /// Validates checkpoint integrity.
    pub fn validate(&self, checkpoint: &AutoDriveCheckpoint) -> Result<bool> {
        let expected = Self::calculate_checksum(checkpoint);
        Ok(checkpoint.checksum == expected)
    }

    /// Lists all recoverable sessions.
    pub fn list_recoverable(&self) -> Result<Vec<CheckpointSummary>> {
        let mut summaries = Vec::new();

        if !self.checkpoint_dir.exists() {
            return Ok(summaries);
        }

        for entry in std::fs::read_dir(&self.checkpoint_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(checkpoint) = serde_json::from_str::<AutoDriveCheckpoint>(&json) {
                        summaries.push(CheckpointSummary {
                            session_id: checkpoint.session_id,
                            goal_preview: checkpoint.goal.chars().take(100).collect(),
                            turns_completed: checkpoint.turns_completed,
                            created_at: checkpoint.created_at,
                            updated_at: checkpoint.updated_at,
                        });
                    }
                }
            }
        }

        // Sort by updated_at descending
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(summaries)
    }

    /// Cleans up checkpoints older than the specified age.
    pub fn cleanup(&self, max_age: Duration) -> Result<usize> {
        let mut removed = 0;
        let cutoff = Utc::now() - chrono::Duration::from_std(max_age)?;

        if !self.checkpoint_dir.exists() {
            return Ok(0);
        }

        for entry in std::fs::read_dir(&self.checkpoint_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(checkpoint) = serde_json::from_str::<AutoDriveCheckpoint>(&json) {
                        if checkpoint.updated_at < cutoff {
                            std::fs::remove_file(&path)?;
                            removed += 1;
                        }
                    }
                }
            }
        }

        Ok(removed)
    }

    /// Updates an existing checkpoint with new state.
    pub fn update(
        &self,
        checkpoint: &mut AutoDriveCheckpoint,
        history: Vec<ResponseItem>,
        turns_completed: usize,
        token_usage: TokenUsage,
        phase: &AutoRunPhase,
    ) -> Result<()> {
        checkpoint.history = history;
        checkpoint.turns_completed = turns_completed;
        checkpoint.token_usage = token_usage;
        checkpoint.phase = CheckpointPhase::from(phase);
        checkpoint.updated_at = Utc::now();
        checkpoint.checksum = Self::calculate_checksum(checkpoint);

        self.save(checkpoint)
    }

    fn checkpoint_path(&self, session_id: &str) -> PathBuf {
        self.checkpoint_dir.join(format!("{session_id}.json"))
    }

    fn calculate_checksum(checkpoint: &AutoDriveCheckpoint) -> String {
        // Create a copy without checksum for hashing
        let mut hasher = Sha256::new();
        hasher.update(checkpoint.version.to_le_bytes());
        hasher.update(checkpoint.session_id.as_bytes());
        hasher.update(checkpoint.goal.as_bytes());
        hasher.update(checkpoint.turns_completed.to_le_bytes());
        hasher.update(checkpoint.token_usage.total_tokens.to_le_bytes());
        hasher.update(checkpoint.created_at.timestamp().to_le_bytes());

        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (CheckpointManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = CheckpointManager::new(temp_dir.path().to_path_buf());
        (manager, temp_dir)
    }

    #[test]
    fn test_create_checkpoint() {
        let (mut manager, _temp) = create_test_manager();
        let checkpoint = manager.create("Test goal", "session-123").unwrap();

        assert_eq!(checkpoint.goal, "Test goal");
        assert_eq!(checkpoint.session_id, "session-123");
        assert_eq!(checkpoint.turns_completed, 0);
        assert!(!checkpoint.checksum.is_empty());
    }

    #[test]
    fn test_save_and_restore() {
        let (mut manager, _temp) = create_test_manager();
        let checkpoint = manager.create("Test goal", "session-456").unwrap();

        manager.save(&checkpoint).unwrap();
        let restored = manager.restore("session-456").unwrap().unwrap();

        assert_eq!(restored.goal, checkpoint.goal);
        assert_eq!(restored.session_id, checkpoint.session_id);
        assert_eq!(restored.checksum, checkpoint.checksum);
    }

    #[test]
    fn test_validate_checkpoint() {
        let (mut manager, _temp) = create_test_manager();
        let checkpoint = manager.create("Test goal", "session-789").unwrap();

        assert!(manager.validate(&checkpoint).unwrap());

        // Tamper with checkpoint
        let mut tampered = checkpoint.clone();
        tampered.goal = "Modified goal".to_string();

        assert!(!manager.validate(&tampered).unwrap());
    }

    #[test]
    fn test_restore_nonexistent() {
        let (manager, _temp) = create_test_manager();
        let result = manager.restore("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_recoverable() {
        let (mut manager, _temp) = create_test_manager();

        let cp1 = manager.create("Goal 1", "session-1").unwrap();
        manager.save(&cp1).unwrap();

        let cp2 = manager.create("Goal 2", "session-2").unwrap();
        manager.save(&cp2).unwrap();

        let summaries = manager.list_recoverable().unwrap();
        assert_eq!(summaries.len(), 2);
    }
}
