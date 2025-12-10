//! Audit logger for tracking Auto Drive operations.
//!
//! This module provides comprehensive logging of all operations performed
//! during Auto Drive sessions for security and debugging purposes.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::Serialize;

/// An entry in the audit log.
#[derive(Clone, Debug, Serialize)]
pub struct AuditEntry {
    /// When the operation occurred.
    pub timestamp: DateTime<Utc>,
    /// The operation that was performed.
    pub operation: AuditOperation,
    /// The outcome of the operation.
    pub outcome: AuditOutcome,
    /// Optional additional context.
    pub context: Option<String>,
}

/// Types of operations that can be audited.
#[derive(Clone, Debug, Serialize)]
pub enum AuditOperation {
    /// A tool was executed.
    ToolExecution { tool: String, args_hash: u64 },
    /// A file was modified.
    FileModification { path: PathBuf, action: FileAction },
    /// Network access was attempted.
    NetworkAccess { url: String, method: String },
    /// An agent was dispatched.
    AgentDispatch { agent_id: String, write_access: bool },
    /// A checkpoint was saved.
    CheckpointSave { checkpoint_id: String },
    /// A budget warning was triggered.
    BudgetWarning { alert: String },
    /// Session started.
    SessionStart { goal: String },
    /// Session ended.
    SessionEnd { turns: usize, success: bool },
}

/// Actions that can be performed on files.
#[derive(Clone, Debug, Serialize)]
pub enum FileAction {
    Create,
    Modify,
    Delete,
    Read,
}

/// Outcome of an audited operation.
#[derive(Clone, Debug, Serialize)]
pub enum AuditOutcome {
    Success,
    Failure(String),
    Denied(String),
    Skipped(String),
}

/// Filter for querying audit entries.
#[derive(Clone, Debug, Default)]
pub struct AuditFilter {
    /// Filter by operation type.
    pub operation_type: Option<String>,
    /// Filter by outcome.
    pub outcome_success: Option<bool>,
    /// Filter by time range start.
    pub after: Option<DateTime<Utc>>,
    /// Filter by time range end.
    pub before: Option<DateTime<Utc>>,
}

/// Summary of audit log contents.
#[derive(Clone, Debug, Default, Serialize)]
pub struct AuditSummary {
    pub total_operations: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub denied_operations: usize,
    pub tool_executions: usize,
    pub file_modifications: usize,
    pub network_accesses: usize,
    pub agent_dispatches: usize,
}

/// Export format for audit logs.
#[derive(Clone, Copy, Debug)]
pub enum ExportFormat {
    Json,
    Csv,
}

/// Logger for auditing Auto Drive operations.
pub struct AuditLogger {
    session_id: String,
    entries: Vec<AuditEntry>,
    log_path: Option<PathBuf>,
    workspace_root: Option<PathBuf>,
    network_allowlist: Vec<String>,
}

impl AuditLogger {
    /// Creates a new AuditLogger for a session.
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            entries: Vec::new(),
            log_path: None,
            workspace_root: None,
            network_allowlist: Vec::new(),
        }
    }

    /// Sets the log file path.
    pub fn with_log_path(mut self, path: PathBuf) -> Self {
        self.log_path = Some(path);
        self
    }

    /// Sets the workspace root for path validation.
    pub fn with_workspace_root(mut self, root: PathBuf) -> Self {
        self.workspace_root = Some(root);
        self
    }

    /// Sets the network allowlist.
    pub fn with_network_allowlist(mut self, allowlist: Vec<String>) -> Self {
        self.network_allowlist = allowlist;
        self
    }

    /// Logs an operation.
    pub fn log(&mut self, operation: AuditOperation, outcome: AuditOutcome) {
        self.log_with_context(operation, outcome, None);
    }

    /// Logs an operation with additional context.
    pub fn log_with_context(
        &mut self,
        operation: AuditOperation,
        outcome: AuditOutcome,
        context: Option<String>,
    ) {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            operation,
            outcome,
            context,
        };

        tracing::debug!(
            session_id = %self.session_id,
            operation = ?entry.operation,
            outcome = ?entry.outcome,
            "Audit log entry"
        );

        self.entries.push(entry);
    }

    /// Validates that a file path is within the workspace.
    pub fn validate_file_path(&self, path: &PathBuf) -> Result<(), String> {
        let Some(workspace) = &self.workspace_root else {
            return Ok(()); // No workspace configured, allow all
        };

        let canonical_workspace = workspace
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize workspace: {e}"))?;

        let canonical_path = path
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize path: {e}"))?;

        if canonical_path.starts_with(&canonical_workspace) {
            Ok(())
        } else {
            Err(format!(
                "Path {} is outside workspace {}",
                path.display(),
                workspace.display()
            ))
        }
    }

    /// Validates that a URL is in the allowlist.
    pub fn validate_network_access(&self, url: &str) -> Result<(), String> {
        if self.network_allowlist.is_empty() {
            return Ok(()); // No allowlist configured, allow all
        }

        for allowed in &self.network_allowlist {
            if url.starts_with(allowed) || url.contains(allowed) {
                return Ok(());
            }
        }

        Err(format!("URL {url} is not in the network allowlist"))
    }

    /// Generates a summary of the audit log.
    pub fn generate_summary(&self) -> AuditSummary {
        let mut summary = AuditSummary {
            total_operations: self.entries.len(),
            ..Default::default()
        };

        for entry in &self.entries {
            match &entry.outcome {
                AuditOutcome::Success => summary.successful_operations += 1,
                AuditOutcome::Failure(_) => summary.failed_operations += 1,
                AuditOutcome::Denied(_) => summary.denied_operations += 1,
                AuditOutcome::Skipped(_) => {}
            }

            match &entry.operation {
                AuditOperation::ToolExecution { .. } => summary.tool_executions += 1,
                AuditOperation::FileModification { .. } => summary.file_modifications += 1,
                AuditOperation::NetworkAccess { .. } => summary.network_accesses += 1,
                AuditOperation::AgentDispatch { .. } => summary.agent_dispatches += 1,
                _ => {}
            }
        }

        summary
    }

    /// Exports the audit log in the specified format.
    pub fn export(&self, format: ExportFormat) -> anyhow::Result<String> {
        match format {
            ExportFormat::Json => {
                Ok(serde_json::to_string_pretty(&self.entries)?)
            }
            ExportFormat::Csv => {
                let mut csv = String::from("timestamp,operation_type,outcome,context\n");
                for entry in &self.entries {
                    let op_type = match &entry.operation {
                        AuditOperation::ToolExecution { tool, .. } => format!("tool:{tool}"),
                        AuditOperation::FileModification { action, .. } => {
                            format!("file:{action:?}")
                        }
                        AuditOperation::NetworkAccess { method, .. } => format!("network:{method}"),
                        AuditOperation::AgentDispatch { .. } => "agent".to_string(),
                        AuditOperation::CheckpointSave { .. } => "checkpoint".to_string(),
                        AuditOperation::BudgetWarning { .. } => "budget".to_string(),
                        AuditOperation::SessionStart { .. } => "session_start".to_string(),
                        AuditOperation::SessionEnd { .. } => "session_end".to_string(),
                    };
                    let outcome = match &entry.outcome {
                        AuditOutcome::Success => "success".to_string(),
                        AuditOutcome::Failure(e) => format!("failure:{e}"),
                        AuditOutcome::Denied(r) => format!("denied:{r}"),
                        AuditOutcome::Skipped(r) => format!("skipped:{r}"),
                    };
                    let context = entry.context.as_deref().unwrap_or("");
                    csv.push_str(&format!(
                        "{},{},{},{}\n",
                        entry.timestamp, op_type, outcome, context
                    ));
                }
                Ok(csv)
            }
        }
    }

    /// Queries entries matching the filter.
    pub fn query(&self, filter: &AuditFilter) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                // Filter by time range
                if let Some(after) = filter.after {
                    if entry.timestamp < after {
                        return false;
                    }
                }
                if let Some(before) = filter.before {
                    if entry.timestamp > before {
                        return false;
                    }
                }

                // Filter by outcome
                if let Some(success) = filter.outcome_success {
                    let is_success = matches!(entry.outcome, AuditOutcome::Success);
                    if is_success != success {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Returns all entries.
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Returns the session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Clears all entries from the audit log.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_operation() {
        let mut logger = AuditLogger::new("test-session");

        logger.log(
            AuditOperation::ToolExecution {
                tool: "read_file".to_string(),
                args_hash: 12345,
            },
            AuditOutcome::Success,
        );

        assert_eq!(logger.entries().len(), 1);
    }

    #[test]
    fn test_generate_summary() {
        let mut logger = AuditLogger::new("test-session");

        logger.log(
            AuditOperation::ToolExecution {
                tool: "read_file".to_string(),
                args_hash: 111,
            },
            AuditOutcome::Success,
        );
        logger.log(
            AuditOperation::ToolExecution {
                tool: "write_file".to_string(),
                args_hash: 222,
            },
            AuditOutcome::Failure("Permission denied".to_string()),
        );
        logger.log(
            AuditOperation::FileModification {
                path: PathBuf::from("test.txt"),
                action: FileAction::Create,
            },
            AuditOutcome::Success,
        );

        let summary = logger.generate_summary();
        assert_eq!(summary.total_operations, 3);
        assert_eq!(summary.successful_operations, 2);
        assert_eq!(summary.failed_operations, 1);
        assert_eq!(summary.tool_executions, 2);
        assert_eq!(summary.file_modifications, 1);
    }

    #[test]
    fn test_validate_file_path() {
        let temp_dir = std::env::temp_dir();
        let logger = AuditLogger::new("test").with_workspace_root(temp_dir.clone());

        // Path within workspace should be valid
        let valid_path = temp_dir.join("test.txt");
        std::fs::write(&valid_path, "test").unwrap();
        assert!(logger.validate_file_path(&valid_path).is_ok());
        std::fs::remove_file(&valid_path).unwrap();

        // Path outside workspace should be invalid
        let invalid_path = PathBuf::from("/etc/passwd");
        if invalid_path.exists() {
            assert!(logger.validate_file_path(&invalid_path).is_err());
        }
    }

    #[test]
    fn test_validate_network_access() {
        let logger = AuditLogger::new("test")
            .with_network_allowlist(vec!["api.openai.com".to_string(), "github.com".to_string()]);

        assert!(logger
            .validate_network_access("https://api.openai.com/v1/chat")
            .is_ok());
        assert!(logger
            .validate_network_access("https://github.com/repo")
            .is_ok());
        assert!(logger
            .validate_network_access("https://malicious.com")
            .is_err());
    }

    #[test]
    fn test_export_json() {
        let mut logger = AuditLogger::new("test-session");

        logger.log(
            AuditOperation::SessionStart {
                goal: "Test goal".to_string(),
            },
            AuditOutcome::Success,
        );

        let json = logger.export(ExportFormat::Json).unwrap();
        assert!(json.contains("SessionStart"));
        assert!(json.contains("Test goal"));
    }

    #[test]
    fn test_query_filter() {
        let mut logger = AuditLogger::new("test-session");

        logger.log(
            AuditOperation::ToolExecution {
                tool: "tool1".to_string(),
                args_hash: 1,
            },
            AuditOutcome::Success,
        );
        logger.log(
            AuditOperation::ToolExecution {
                tool: "tool2".to_string(),
                args_hash: 2,
            },
            AuditOutcome::Failure("error".to_string()),
        );

        let filter = AuditFilter {
            outcome_success: Some(true),
            ..Default::default()
        };

        let results = logger.query(&filter);
        assert_eq!(results.len(), 1);
    }
}
