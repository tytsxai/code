//! Parallel execution module for same-model concurrent Auto Drive.
//!
//! When `parallel_instances > 1` in `AutoDriveSettings`, this module enables
//! dispatching multiple concurrent API calls to the same model with different
//! role prompts (coordinator, executor, reviewer).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use code_core::ModelClient;
use code_core::Prompt;
use code_core::ResponseEvent;
use futures::Stream;
use futures::StreamExt;
use futures::future::join_all;
use tokio::sync::Semaphore;

use code_protocol::models::ContentItem;
use code_protocol::models::ResponseItem;

/// Role definition for parallel instance execution
#[derive(Debug, Clone)]
pub enum ParallelRole {
    /// Primary coordinator role - orchestrates the overall task
    Coordinator,
    /// Architect role - designs system structure (for complex tasks)
    Architect,
    /// Executor role - implements code changes (can have multiple)
    Executor(u8), // Executor ID (1, 2, 3)
    /// Tester role - writes and runs tests
    Tester,
    /// Debugger role - fixes bugs and issues
    Debugger,
    /// Reviewer role - reviews, merges, and cleans up
    Reviewer,
}

impl ParallelRole {
    /// Returns the role-specific prompt prefix
    pub fn prompt_prefix(&self) -> &'static str {
        match self {
            Self::Coordinator => {
                "You are the COORDINATOR (战略规划). Your job is to:\n\
                 1. Analyze task complexity and dependencies\n\
                 2. Define clear acceptance criteria (Definition of Done)\n\
                 3. If complex, ask ARCHITECT to design first\n\
                 4. Assign implementation to EXECUTORs by specialty\n\
                 5. Direct TESTER to prepare test cases\n\
                 6. Monitor progress and adjust strategy\n\
                 Be specific about what SUCCESS looks like.\n\
                 Now coordinate:"
            }
            Self::Architect => {
                "You are the ARCHITECT (架构设计). Your job is to:\n\
                 1. Design overall structure and component layout\n\
                 2. Define interfaces, data flow, module boundaries\n\
                 3. Identify technical risks and mitigations\n\
                 4. Create a blueprint for EXECUTORs to follow\n\
                 5. Ensure design is scalable and maintainable\n\
                 Think before building. Design shapes the solution.\n\
                 Now design:"
            }
            Self::Executor(id) => match id {
                1 => {
                    "You are EXECUTOR-1 (核心实现). Your job is to:\n\
                      1. Deliver production-quality core implementation\n\
                      2. Follow ARCHITECT's design if provided\n\
                      3. Prioritize correctness and maintainability\n\
                      4. Document assumptions and trade-offs\n\
                      Focus on THE RIGHT solution, not just A solution.\n\
                      Now implement:"
                }
                2 => {
                    "You are EXECUTOR-2 (创新方案). Your job is to:\n\
                      1. Explore alternative approaches\n\
                      2. Focus on performance or architecture improvements\n\
                      3. Challenge assumptions and innovate\n\
                      4. Provide diversity of thought\n\
                      Your different perspective improves the result.\n\
                      Now implement:"
                }
                3 => {
                    "You are EXECUTOR-3 (边缘处理). Your job is to:\n\
                      1. Handle edge cases and error conditions\n\
                      2. Add input validation and safety checks\n\
                      3. Improve error messages and diagnostics\n\
                      4. Make the solution robust and resilient\n\
                      Now implement:"
                }
                _ => "You are an EXECUTOR. Complete your assigned work efficiently:",
            },
            Self::Tester => {
                "You are the TESTER (测试验证). Your job is to:\n\
                 1. Write test cases based on acceptance criteria\n\
                 2. Cover edge cases, error handling, boundaries\n\
                 3. Verify EXECUTOR implementations meet requirements\n\
                 4. Report test coverage and any failures\n\
                 5. Work with DEBUGGER to resolve issues\n\
                 Quality assurance is your responsibility.\n\
                 Now test:"
            }
            Self::Debugger => {
                "You are the DEBUGGER (问题修复). Your job is to:\n\
                 1. Investigate failures reported by TESTER\n\
                 2. Identify root causes of bugs\n\
                 3. Implement minimal, targeted fixes\n\
                 4. Verify fixes don't introduce regressions\n\
                 5. Document the issue and resolution\n\
                 Fix fast, fix right.\n\
                 Now debug:"
            }
            Self::Reviewer => {
                "You are the REVIEWER (合并管理). Your job is to:\n\
                 1. Evaluate ALL solutions objectively\n\
                 2. Confirm TESTER's tests pass\n\
                 3. Merge the best parts into optimal solution\n\
                 4. Ensure quality, consistency, documentation\n\
                 5. CLEANUP: Delete obsolete code-* branches\n\
                    Run: git branch | grep 'code-' to list\n\
                    Keep only the accepted solution branch\n\
                 Your decision is final. Deliver excellence.\n\
                 Now review:"
            }
        }
    }

    /// Returns role name for display
    pub fn name(&self) -> String {
        match self {
            Self::Coordinator => "Coordinator".to_string(),
            Self::Architect => "Architect".to_string(),
            Self::Executor(id) => format!("Executor-{id}"),
            Self::Tester => "Tester".to_string(),
            Self::Debugger => "Debugger".to_string(),
            Self::Reviewer => "Reviewer".to_string(),
        }
    }

    /// Returns roles for a given parallel instance count
    ///
    /// Optimized distribution strategy:
    /// - 1: Coordinator only (serial mode)
    /// - 2: Coordinator + Executor
    /// - 3: Coordinator + Executor + Reviewer
    /// - 4: Coordinator + 2 Executors + Reviewer  
    /// - 5: Coordinator + 2 Executors + Tester + Reviewer
    /// - 6: Coordinator + Architect + 2 Executors + Tester + Reviewer
    /// - 7: Coordinator + Architect + 2 Executors + Tester + Debugger + Reviewer
    /// - 8: Coordinator + Architect + 3 Executors + Tester + Debugger + Reviewer (optimal)
    /// - 9: Coordinator + Architect + 3 Executors + 2 Testers + Debugger + Reviewer
    /// - 10: Coordinator + Architect + 3 Executors + 2 Testers + 2 Debuggers + Reviewer
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
                Self::Architect,
                Self::Executor(1),
                Self::Executor(2),
                Self::Tester,
                Self::Reviewer,
            ],
            7 => vec![
                Self::Coordinator,
                Self::Architect,
                Self::Executor(1),
                Self::Executor(2),
                Self::Tester,
                Self::Debugger,
                Self::Reviewer,
            ],
            8 => vec![
                Self::Coordinator,
                Self::Architect,
                Self::Executor(1),
                Self::Executor(2),
                Self::Executor(3),
                Self::Tester,
                Self::Debugger,
                Self::Reviewer,
            ],
            9 => vec![
                Self::Coordinator,
                Self::Architect,
                Self::Executor(1),
                Self::Executor(2),
                Self::Executor(3),
                Self::Tester,
                Self::Tester,
                Self::Debugger,
                Self::Reviewer,
            ],
            10 | _ => vec![
                Self::Coordinator,
                Self::Architect,
                Self::Executor(1),
                Self::Executor(2),
                Self::Executor(3),
                Self::Tester,
                Self::Tester,
                Self::Debugger,
                Self::Debugger,
                Self::Reviewer,
            ],
        }
    }
}

/// Result from a parallel execution instance
#[derive(Debug, Clone)]
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
            instance_count: count.clamp(1, 10),
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

fn build_prompt_text(role: &ParallelRole, base_prompt: &str) -> String {
    format!("{}\n\n{}", role.prompt_prefix(), base_prompt)
}

/// Stream type returned by parallel role executors.
pub type ParallelResponseStream = Pin<Box<dyn Stream<Item = anyhow::Result<ResponseEvent>> + Send>>;

/// Minimal interface required for parallel role execution.
pub trait ParallelModel: Send + Sync {
    fn stream_prompt(
        &self,
        prompt: Prompt,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<ParallelResponseStream>> + Send + '_>>;
}

impl ParallelModel for ModelClient {
    fn stream_prompt(
        &self,
        prompt: Prompt,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<ParallelResponseStream>> + Send + '_>> {
        Box::pin(async move {
            let stream = ModelClient::stream(self, &prompt)
                .await
                .map_err(|err| anyhow::anyhow!(err))?;
            let mapped = stream.map(|event| event.map_err(|err| anyhow::anyhow!(err)));
            Ok(Box::pin(mapped) as ParallelResponseStream)
        })
    }
}

/// Execute parallel instances using the same model with different roles.
///
/// This function spawns multiple concurrent API calls, each with a role-specific
/// prompt prefix, and collects results from all instances.
pub async fn execute_parallel(
    client: Arc<dyn ParallelModel>,
    config: &ParallelConfig,
    max_concurrent_agents: i32,
) -> Result<Vec<ParallelResult>> {
    if max_concurrent_agents < 8 {
        tracing::warn!(
            max_concurrent_agents,
            "Parallel execution concurrency below target"
        );
    }

    if !config.is_parallel() {
        // Single instance mode - no parallelization needed
        return Ok(vec![ParallelResult {
            role: ParallelRole::Coordinator,
            response: config.base_prompt.clone(),
            success: true,
        }]);
    }

    execute_parallel_roles(
        client,
        config.roles(),
        &config.base_prompt,
        &config.model,
        max_concurrent_agents,
    )
    .await
}

/// Execute a specific set of roles using the provided model client.
pub async fn execute_parallel_roles(
    client: Arc<dyn ParallelModel>,
    roles: Vec<ParallelRole>,
    base_prompt: &str,
    model: &str,
    max_concurrent_agents: i32,
) -> Result<Vec<ParallelResult>> {
    if roles.is_empty() {
        return Ok(Vec::new());
    }

    let semaphore = Arc::new(Semaphore::new(max_concurrent_agents.max(1) as usize));

    let tasks = roles.into_iter().map(|role| {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let base_prompt = base_prompt.to_string();
        let model = model.to_string();

        async move {
            let permit = match semaphore.acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => {
                    return ParallelResult {
                        role,
                        response: "semaphore unavailable".to_string(),
                        success: false,
                    };
                }
            };

            let mut prompt = Prompt::default();
            prompt.user_instructions = Some(build_prompt_text(&role, &base_prompt));
            prompt.model_override = Some(model);

            let mut response = String::new();
            let result = match client.stream_prompt(prompt).await {
                Ok(mut stream) => {
                    while let Some(event) = stream.next().await {
                        match event {
                            Ok(ResponseEvent::OutputTextDelta { delta, .. }) => {
                                response.push_str(&delta);
                            }
                            Ok(ResponseEvent::OutputItemDone { item, .. }) => {
                                if let ResponseItem::Message { content, .. } = item {
                                    for item in content {
                                        if let ContentItem::OutputText { text } = item {
                                            response.push_str(&text);
                                        }
                                    }
                                }
                            }
                            Ok(ResponseEvent::Completed { .. }) => break,
                            _ => {}
                        }
                    }
                    ParallelResult {
                        role,
                        response,
                        success: true,
                    }
                }
                Err(err) => ParallelResult {
                    role,
                    response: format!("error: {err}"),
                    success: false,
                },
            };
            drop(permit);
            result
        }
    });

    Ok(join_all(tasks).await)
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
    if let Some(coord) = coordinator_result
        && !coord.response.is_empty()
    {
        merged.push_str(&format!("[Coordinator Plan]\n{}\n", coord.response));
    }

    // 2. Add all executor outputs
    for result in results.iter() {
        if matches!(result.role, ParallelRole::Executor(_))
            && result.success
            && !result.response.is_empty()
        {
            merged.push_str(&format!(
                "\n[{}]\n{}\n",
                result.role.name(),
                result.response
            ));
        }
    }

    // 3. End with reviewer's analysis
    if let Some(review) = reviewer_result
        && !review.response.is_empty()
    {
        merged.push_str(&format!("\n[Reviewer Analysis]\n{}\n", review.response));
    }

    merged.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use std::future::Future;
    use std::sync::Arc;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::Ordering;
    use tokio::sync::Mutex;
    use tokio::time::Duration;
    use tokio::time::sleep;

    #[test]
    fn test_roles_for_count() {
        assert_eq!(ParallelRole::roles_for_count(1).len(), 1);
        assert_eq!(ParallelRole::roles_for_count(3).len(), 3);
        assert_eq!(ParallelRole::roles_for_count(5).len(), 5);
        assert_eq!(ParallelRole::roles_for_count(10).len(), 10);
        // Clamped to max 10
        assert_eq!(ParallelRole::roles_for_count(15).len(), 10);
    }

    #[test]
    fn test_parallel_config() {
        let config = ParallelConfig::from_instances(3, "test".into(), "gpt-5.1".into());
        assert!(config.is_parallel());
        assert_eq!(config.roles().len(), 3);

        let single = ParallelConfig::from_instances(1, "test".into(), "gpt-5.1".into());
        assert!(!single.is_parallel());
    }

    #[test]
    fn prompt_prefix_includes_role_and_base_prompt() {
        let role = ParallelRole::Coordinator;
        let text = build_prompt_text(&role, "Do work");
        assert!(text.contains("COORDINATOR"));
        assert!(text.contains("Do work"));
    }

    async fn execute_parallel_with<F, Fut>(
        roles: Vec<ParallelRole>,
        max_concurrent_agents: i32,
        responder: F,
    ) -> Vec<ParallelResult>
    where
        F: FnMut(ParallelRole) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ParallelResult> + Send,
    {
        let semaphore = Arc::new(Semaphore::new(max_concurrent_agents.max(1) as usize));
        let responder = Arc::new(Mutex::new(responder));

        let tasks = roles.into_iter().map(|role| {
            let semaphore = semaphore.clone();
            let responder = responder.clone();
            async move {
                let _permit = semaphore.acquire_owned().await.unwrap();
                let fut = {
                    let mut responder = responder.lock().await;
                    responder(role)
                };
                fut.await
            }
        });

        join_all(tasks).await
    }

    #[tokio::test]
    async fn concurrency_limit_enforced() {
        let roles = ParallelRole::roles_for_count(5);
        let max_concurrent_agents = 2;
        let current = Arc::new(AtomicI32::new(0));
        let peak = Arc::new(AtomicI32::new(0));

        let results = execute_parallel_with(roles, max_concurrent_agents, {
            let current = current.clone();
            let peak = peak.clone();
            move |role| {
                let current = current.clone();
                let peak = peak.clone();
                async move {
                    let now = current.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    sleep(Duration::from_millis(10)).await;
                    current.fetch_sub(1, Ordering::SeqCst);
                    ParallelResult {
                        role,
                        response: "ok".to_string(),
                        success: true,
                    }
                }
            }
        })
        .await;

        assert_eq!(results.len(), 5);
        assert!(peak.load(Ordering::SeqCst) <= max_concurrent_agents);
    }

    #[test]
    fn merge_parallel_results_includes_sections() {
        let results = vec![
            ParallelResult {
                role: ParallelRole::Coordinator,
                response: "Plan".to_string(),
                success: true,
            },
            ParallelResult {
                role: ParallelRole::Executor(1),
                response: "Impl".to_string(),
                success: true,
            },
            ParallelResult {
                role: ParallelRole::Reviewer,
                response: "Review".to_string(),
                success: true,
            },
        ];

        let merged = merge_parallel_results(results);
        assert!(merged.contains("[Coordinator Plan]"));
        assert!(merged.contains("Impl"));
        assert!(merged.contains("[Reviewer Analysis]"));
    }

    struct StubModel {
        fail_on: String,
    }

    impl ParallelModel for StubModel {
        fn stream_prompt(
            &self,
            prompt: Prompt,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<ParallelResponseStream>> + Send + '_>>
        {
            let fail = if self.fail_on.is_empty() { false } else { {
                prompt
                    .user_instructions
                    .as_ref()
                    .map(|p| p.contains(&self.fail_on))
                    .unwrap_or(false)
            } };

            Box::pin(async move {
                if fail {
                    anyhow::bail!("forced failure");
                }
                let events = vec![
                    Ok(ResponseEvent::OutputTextDelta {
                        delta: "ok".to_string(),
                        item_id: None,
                        sequence_number: None,
                        output_index: None,
                    }),
                    Ok(ResponseEvent::Completed {
                        response_id: "r".to_string(),
                        token_usage: None,
                    }),
                ];
                Ok(Box::pin(stream::iter(events)) as ParallelResponseStream)
            })
        }
    }

    #[tokio::test]
    async fn failure_is_isolated_to_role() {
        let config = ParallelConfig::from_instances(2, "Work".to_string(), "gpt-5.1".to_string());
        let model = Arc::new(StubModel {
            fail_on: "EXECUTOR-1".to_string(),
        });
        let results = execute_parallel(model, &config, 2).await.unwrap();
        assert_eq!(results.len(), 2);
        let coordinator = results
            .iter()
            .find(|r| matches!(r.role, ParallelRole::Coordinator))
            .unwrap();
        assert!(coordinator.success);
        let executor = results
            .iter()
            .find(|r| matches!(r.role, ParallelRole::Executor(1)))
            .unwrap();
        assert!(!executor.success);
        assert!(executor.response.contains("error"));
    }
}
