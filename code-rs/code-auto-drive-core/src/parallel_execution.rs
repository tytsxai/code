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
    Executor(u8),  // Executor ID (1, 2)
    /// Tester role - writes and runs tests
    Tester,
    /// Reviewer role - reviews, merges, and cleans up
    Reviewer,
}

impl ParallelRole {
    /// Returns the role-specific prompt prefix
    pub fn prompt_prefix(&self) -> &'static str {
        match self {
            Self::Coordinator => 
                "You are the COORDINATOR (战略规划者). Your job is to:\n\
                 1. Analyze the task complexity and dependencies\n\
                 2. Define clear acceptance criteria (Definition of Done)\n\
                 3. Assign core implementation to EXECUTOR-1, alternative approach to EXECUTOR-2\n\
                 4. Direct TESTER to prepare test cases based on acceptance criteria\n\
                 5. Monitor progress and adjust strategy if needed\n\
                 Be specific about what SUCCESS looks like for this task.\n\
                 Now coordinate:",
            Self::Executor(id) => match id {
                1 => "You are EXECUTOR-1 (核心实现). Your job is to:\n\
                      1. Deliver production-quality implementation following project conventions\n\
                      2. Prioritize correctness, maintainability, and best practices\n\
                      3. Notify TESTER when core functionality is ready\n\
                      4. Document any assumptions or trade-offs made\n\
                      Focus on THE RIGHT solution, not just A solution.\n\
                      Now implement:",
                2 => "You are EXECUTOR-2 (创新方案). Your job is to:\n\
                      1. Explore alternative implementation approaches\n\
                      2. Focus on performance optimization or architectural improvements\n\
                      3. Even if EXECUTOR-1 finishes first, provide your perspective\n\
                      4. Challenge assumptions and propose creative solutions\n\
                      Your diversity of thought improves the final result.\n\
                      Now implement:",
                3 => "You are EXECUTOR-3 (重构优化). Your job is to:\n\
                      1. Refactor and improve code structure\n\
                      2. Reduce duplication and improve maintainability\n\
                      3. Apply design patterns where appropriate\n\
                      4. Ensure code is clean and well-organized\n\
                      Now implement:",
                4 => "You are EXECUTOR-4 (文档完善). Your job is to:\n\
                      1. Add comprehensive documentation and comments\n\
                      2. Update README and API docs as needed\n\
                      3. Create examples and usage guides\n\
                      4. Ensure code is self-documenting\n\
                      Now implement:",
                5 => "You are EXECUTOR-5 (边缘处理). Your job is to:\n\
                      1. Handle edge cases and error conditions\n\
                      2. Add input validation and safety checks\n\
                      3. Improve error messages and diagnostics\n\
                      4. Make the solution robust and resilient\n\
                      Now implement:",
                6 => "You are EXECUTOR-6 (性能调优). Your job is to:\n\
                      1. Profile and optimize performance bottlenecks\n\
                      2. Reduce memory usage and improve efficiency\n\
                      3. Add caching or lazy evaluation where helpful\n\
                      4. Ensure the solution scales well\n\
                      Now implement:",
                _ => "You are an EXECUTOR. Complete your assigned work efficiently:",
            },
            Self::Tester => 
                "You are the TESTER (测试验证). Your job is to:\n\
                 1. Write test cases based on COORDINATOR's acceptance criteria\n\
                 2. Cover edge cases, error handling, and boundary conditions\n\
                 3. Verify EXECUTOR implementations meet requirements\n\
                 4. Report test coverage and any failing scenarios\n\
                 5. Ensure the solution works in realistic conditions\n\
                 Quality assurance is your responsibility.\n\
                 Now test:",
            Self::Reviewer => 
                "You are the REVIEWER (合并管理). Your job is to:\n\
                 1. Evaluate ALL solutions from EXECUTORs objectively\n\
                 2. Check TESTER's results - all tests must pass\n\
                 3. Merge the best parts into a unified, optimal solution\n\
                 4. Ensure code quality, consistency, and documentation\n\
                 5. CLEANUP: Delete obsolete code-* branches after merging\n\
                    Run: git branch | grep 'code-' to list branches\n\
                    Keep only the branch with the accepted solution\n\
                 Your decision is final. Deliver excellence.\n\
                 Now review:",
        }
    }
    
    /// Returns role name for display
    pub fn name(&self) -> String {
        match self {
            Self::Coordinator => "Coordinator".to_string(),
            Self::Executor(id) => format!("Executor-{}", id),
            Self::Tester => "Tester".to_string(),
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
    /// - 5: Coordinator + 2 Executors + Tester + Reviewer
    /// - 6: Coordinator + 3 Executors + Tester + Reviewer
    /// - 7: Coordinator + 4 Executors + Tester + Reviewer
    /// - 8: Coordinator + 5 Executors + Tester + Reviewer
    /// - 9: Coordinator + 5 Executors + 2 Testers + Reviewer
    /// - 10: Coordinator + 6 Executors + 2 Testers + Reviewer (max throughput)
    pub fn roles_for_count(count: u8) -> Vec<Self> {
        match count.min(10) {
            1 => vec![Self::Coordinator],
            2 => vec![Self::Coordinator, Self::Executor(1)],
            3 => vec![Self::Coordinator, Self::Executor(1), Self::Reviewer],
            4 => vec![
                Self::Coordinator,
                Self::Executor(1),
                Self::Executor(2),
                Self::Reviewer,
            ],
            5 => vec![
                Self::Coordinator,
                Self::Executor(1),
                Self::Executor(2),
                Self::Tester,
                Self::Reviewer,
            ],
            6 => vec![
                Self::Coordinator,
                Self::Executor(1),
                Self::Executor(2),
                Self::Executor(3),
                Self::Tester,
                Self::Reviewer,
            ],
            7 => vec![
                Self::Coordinator,
                Self::Executor(1),
                Self::Executor(2),
                Self::Executor(3),
                Self::Executor(4),
                Self::Tester,
                Self::Reviewer,
            ],
            8 => vec![
                Self::Coordinator,
                Self::Executor(1),
                Self::Executor(2),
                Self::Executor(3),
                Self::Executor(4),
                Self::Executor(5),
                Self::Tester,
                Self::Reviewer,
            ],
            9 => vec![
                Self::Coordinator,
                Self::Executor(1),
                Self::Executor(2),
                Self::Executor(3),
                Self::Executor(4),
                Self::Executor(5),
                Self::Tester,
                Self::Tester, // Second tester for parallel test coverage
                Self::Reviewer,
            ],
            10 | _ => vec![
                Self::Coordinator,
                Self::Executor(1),
                Self::Executor(2),
                Self::Executor(3),
                Self::Executor(4),
                Self::Executor(5),
                Self::Executor(6),
                Self::Tester,
                Self::Tester, // Second tester
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
