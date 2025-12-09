//! Parallel execution module for same-model concurrent Auto Drive.
//!
//! When `parallel_instances > 1` in `AutoDriveSettings`, this module enables
//! dispatching multiple concurrent API calls to the same model with different
//! role prompts (coordinator, executor, reviewer).

use std::sync::Arc;

use anyhow::Result;
use code_core::ModelClient;
use futures::future::join_all;

/// Role definition for parallel instance execution
#[derive(Debug, Clone)]
pub enum ParallelRole {
    /// Primary coordinator role - orchestrates the overall task
    Coordinator,
    /// Executor role - implements code changes (can have multiple)
    Executor(u8),  // Executor ID (1, 2, 3...)
    /// Reviewer role - reviews and validates changes
    Reviewer,
}

impl ParallelRole {
    /// Returns the role-specific prompt prefix
    pub fn prompt_prefix(&self) -> &'static str {
        match self {
            Self::Coordinator => 
                "As the COORDINATOR, you MUST distribute work to keep ALL executors busy. \
                 Break down the task into parallel sub-tasks, each assigned to a different executor. \
                 If the task can be done faster by multiple executors working on different parts, split it. \
                 If the same task benefits from parallel attempts, have all executors work on it simultaneously:",
            Self::Executor(id) => match id {
                1 => "As EXECUTOR-1, focus on the primary implementation. Work efficiently:",
                2 => "As EXECUTOR-2, handle secondary components or provide an alternative solution:",
                3 => "As EXECUTOR-3, work on supporting code, tests, or a third approach:",
                _ => "As an EXECUTOR, implement the assigned code changes:",
            },
            Self::Reviewer => 
                "As the REVIEWER, carefully check ALL executor outputs for bugs, edge cases, \
                 inconsistencies, and potential issues. Merge the best parts if multiple approaches exist:",
        }
    }
    
    /// Returns role name for display
    pub fn name(&self) -> String {
        match self {
            Self::Coordinator => "Coordinator".to_string(),
            Self::Executor(id) => format!("Executor-{}", id),
            Self::Reviewer => "Reviewer".to_string(),
        }
    }

    /// Returns roles for a given parallel instance count
    /// 
    /// Distribution strategy:
    /// - 1: Coordinator only (serial mode)
    /// - 2: Coordinator + Executor
    /// - 3: Coordinator + Executor + Reviewer
    /// - 4: Coordinator + 2 Executors + Reviewer  
    /// - 5: Coordinator + 3 Executors + Reviewer (recommended for speed)
    pub fn roles_for_count(count: u8) -> Vec<Self> {
        match count.min(5) {
            1 => vec![Self::Coordinator],
            2 => vec![Self::Coordinator, Self::Executor(1)],
            3 => vec![Self::Coordinator, Self::Executor(1), Self::Reviewer],
            4 => vec![
                Self::Coordinator,
                Self::Executor(1),
                Self::Executor(2),
                Self::Reviewer,
            ],
            5 | _ => vec![
                Self::Coordinator,
                Self::Executor(1),
                Self::Executor(2),
                Self::Executor(3),
                Self::Reviewer,
            ],
        }
    }
}

/// Result from a parallel execution instance
#[derive(Debug)]
pub struct ParallelResult {
    pub role: ParallelRole,
    pub response: String,
    pub success: bool,
}

/// Configuration for parallel execution
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Number of parallel instances (1-5)
    pub instance_count: u8,
    /// Base prompt to send to all instances
    pub base_prompt: String,
    /// Model to use for all instances
    pub model: String,
}

impl ParallelConfig {
    /// Create config from AutoDriveSettings.parallel_instances
    pub fn from_instances(count: u8, base_prompt: String, model: String) -> Self {
        Self {
            instance_count: count.clamp(1, 5),
            base_prompt,
            model,
        }
    }

    /// Returns true if parallel execution is enabled (count > 1)
    pub fn is_parallel(&self) -> bool {
        self.instance_count > 1
    }

    /// Get roles for this configuration
    pub fn roles(&self) -> Vec<ParallelRole> {
        ParallelRole::roles_for_count(self.instance_count)
    }
}

/// Execute parallel instances using the same model with different roles.
/// 
/// This function spawns multiple concurrent API calls, each with a role-specific
/// prompt prefix, and collects results from all instances.
pub async fn execute_parallel(
    _client: Arc<ModelClient>,
    config: &ParallelConfig,
) -> Result<Vec<ParallelResult>> {
    if !config.is_parallel() {
        // Single instance mode - no parallelization needed
        return Ok(vec![ParallelResult {
            role: ParallelRole::Coordinator,
            response: String::new(),
            success: true,
        }]);
    }

    let roles = config.roles();
    let futures: Vec<_> = roles
        .into_iter()
        .map(|role| {
            let _prompt = format!("{} {}", role.prompt_prefix(), config.base_prompt);
            async move {
                // TODO: Implement actual API call with role-specific prompt
                // For now, return placeholder result
                ParallelResult {
                    role,
                    response: String::new(),
                    success: true,
                }
            }
        })
        .collect();

    let results = join_all(futures).await;
    Ok(results)
}

/// Merge results from parallel execution into a unified response.
/// 
/// Strategy: Coordinator provides the plan, Executors provide implementations,
/// Reviewer validates and merges the best parts.
pub fn merge_parallel_results(results: Vec<ParallelResult>) -> String {
    let coordinator_result = results
        .iter()
        .find(|r| matches!(r.role, ParallelRole::Coordinator));
    let reviewer_result = results
        .iter()
        .find(|r| matches!(r.role, ParallelRole::Reviewer));

    let mut merged = String::new();

    // 1. Start with coordinator's plan
    if let Some(coord) = coordinator_result {
        if !coord.response.is_empty() {
            merged.push_str(&format!("[Coordinator Plan]\n{}\n", coord.response));
        }
    }

    // 2. Add all executor outputs
    for result in results.iter() {
        if matches!(result.role, ParallelRole::Executor(_)) && result.success {
            if !result.response.is_empty() {
                merged.push_str(&format!(
                    "\n[{}]\n{}\n",
                    result.role.name(), result.response
                ));
            }
        }
    }

    // 3. End with reviewer's analysis
    if let Some(review) = reviewer_result {
        if !review.response.is_empty() {
            merged.push_str(&format!("\n[Reviewer Analysis]\n{}\n", review.response));
        }
    }

    merged.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roles_for_count() {
        assert_eq!(ParallelRole::roles_for_count(1).len(), 1);
        assert_eq!(ParallelRole::roles_for_count(3).len(), 3);
        assert_eq!(ParallelRole::roles_for_count(5).len(), 5);
        // Clamped to max 5
        assert_eq!(ParallelRole::roles_for_count(10).len(), 5);
    }

    #[test]
    fn test_parallel_config() {
        let config = ParallelConfig::from_instances(3, "test".into(), "gpt-5.1".into());
        assert!(config.is_parallel());
        assert_eq!(config.roles().len(), 3);

        let single = ParallelConfig::from_instances(1, "test".into(), "gpt-5.1".into());
        assert!(!single.is_parallel());
    }
}
