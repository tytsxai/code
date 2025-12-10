//! Automatic cleanup of stale code-* branches created by agents.
//!
//! This module provides background cleanup functionality that removes
//! old agent-created branches that are no longer needed.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Minimum interval between cleanup runs to avoid excessive git operations.
const MIN_CLEANUP_INTERVAL: Duration = Duration::from_secs(300); // 5 minutes

/// Maximum age for branches to be considered for cleanup (in days).
const DEFAULT_MAX_BRANCH_AGE_DAYS: u64 = 1;

static LAST_CLEANUP: Mutex<Option<Instant>> = Mutex::const_new(None);

/// Cleanup stale code-* branches in the background.
///
/// This function is designed to be called after task completion.
/// It will skip if called too frequently to avoid performance impact.
pub async fn cleanup_stale_code_branches(workdir: PathBuf) {
    {
        let mut last = LAST_CLEANUP.lock().await;
        if let Some(last_time) = *last {
            if last_time.elapsed() < MIN_CLEANUP_INTERVAL {
                debug!("Skipping branch cleanup, ran recently");
                return;
            }
        }
        *last = Some(Instant::now());
    }

    let cwd = workdir;
    tokio::task::spawn_blocking(move || {
        if let Err(e) = cleanup_branches_sync(&cwd) {
            warn!("Branch cleanup failed: {}", e);
        }
    })
    .await
    .ok();
}

/// Synchronous branch cleanup implementation.
fn cleanup_branches_sync(workdir: &Path) -> Result<(), String> {
    if !workdir.exists() {
        return Ok(());
    }

    // Get current branch to avoid deleting it
    let current_branch = Command::new("git")
        .current_dir(workdir)
        .args(["branch", "--show-current"])
        .output()
        .map_err(|e| format!("Failed to get current branch in {:?}: {}", workdir, e))?;

    let current = String::from_utf8_lossy(&current_branch.stdout)
        .trim()
        .to_string();

    // List all code-* branches
    let output = Command::new("git")
        .current_dir(workdir)
        .args(["branch", "--list", "code-*"])
        .output()
        .map_err(|e| format!("Failed to list branches in {:?}: {}", workdir, e))?;

    if !output.status.success() {
        debug!("Skipping branch cleanup: {:?} is not a git repository", workdir);
        return Ok(()); // Not in a git repo, skip
    }

    let branches: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim().trim_start_matches("* ").to_string())
        .filter(|s| !s.is_empty() && s.starts_with("code-"))
        .collect();

    if branches.is_empty() {
        return Ok(());
    }

    let mut deleted = 0;
    let max_age_seconds = DEFAULT_MAX_BRANCH_AGE_DAYS * 86400;

    for branch in branches {
        // Skip current branch
        if branch == current {
            continue;
        }

        // Check branch age via last commit time
        let age_check = Command::new("git")
            .current_dir(workdir)
            .args(["log", "-1", "--format=%ct", &branch])
            .output();

        let should_delete = match age_check {
            Ok(out) if out.status.success() => {
                let timestamp_str = String::from_utf8_lossy(&out.stdout);
                if let Ok(timestamp) = timestamp_str.trim().parse::<u64>() {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    now.saturating_sub(timestamp) > max_age_seconds
                } else {
                    false
                }
            }
            _ => false,
        };

        if should_delete {
            let result = Command::new("git")
                .current_dir(workdir)
                .args(["branch", "-D", &branch])
                .output();

            match result {
                Ok(out) if out.status.success() => {
                    deleted += 1;
                    debug!("Deleted stale branch: {}", branch);
                }
                Ok(out) => {
                    debug!(
                        "Failed to delete branch {}: {}",
                        branch,
                        String::from_utf8_lossy(&out.stderr)
                    );
                }
                Err(e) => {
                    debug!("Failed to delete branch {}: {}", branch, e);
                }
            }
        }
    }

    if deleted > 0 {
        debug!("Cleaned up {} stale code-* branches", deleted);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_interval_constant() {
        assert_eq!(MIN_CLEANUP_INTERVAL.as_secs(), 300);
    }
}
