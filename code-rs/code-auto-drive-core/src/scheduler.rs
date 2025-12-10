//! Agent scheduler for managing parallel and sequential agent execution.
//!
//! This module provides scheduling and coordination of agent tasks with
//! configurable concurrency limits and result aggregation.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crate::AutoTurnAgentsTiming;

/// Unique identifier for an agent task.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AgentId(pub u64);

/// A task to be executed by an agent.
#[derive(Clone, Debug)]
pub struct AgentTask {
    /// Unique identifier for this task.
    pub id: AgentId,
    /// The prompt/instruction for the agent.
    pub prompt: String,
    /// Optional context to provide to the agent.
    pub context: Option<String>,
    /// Whether the agent has write access.
    pub write_access: bool,
    /// Optional list of models to use.
    pub models: Option<Vec<String>>,
    /// Order in which this task was dispatched.
    pub dispatch_order: usize,
}

/// Current state of an agent task.
#[derive(Clone, Debug)]
pub enum AgentState {
    /// Task is waiting to be executed.
    Pending,
    /// Task is currently running.
    Running { started_at: Instant },
    /// Task completed successfully.
    Completed { result: AgentResult },
    /// Task failed with an error.
    Failed { error: String },
}

/// Result from an agent execution.
#[derive(Clone, Debug)]
pub struct AgentResult {
    /// The agent that produced this result.
    pub agent_id: AgentId,
    /// The output from the agent.
    pub output: String,
    /// Duration of execution.
    pub duration: std::time::Duration,
    /// Order for result merging.
    pub completion_order: usize,
    /// Original dispatch order.
    pub dispatch_order: usize,
}

/// Scheduler for managing agent task execution.
pub struct AgentScheduler {
    max_concurrent: usize,
    active_agents: HashMap<AgentId, AgentState>,
    pending_queue: VecDeque<AgentTask>,
    results: Vec<AgentResult>,
    next_completion_order: usize,
    timing_mode: Option<AutoTurnAgentsTiming>,
}

impl AgentScheduler {
    /// Creates a new AgentScheduler with the specified concurrency limit.
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent: max_concurrent.max(1),
            active_agents: HashMap::new(),
            pending_queue: VecDeque::new(),
            results: Vec::new(),
            next_completion_order: 0,
            timing_mode: None,
        }
    }

    /// Schedules agent tasks for execution.
    pub fn schedule(&mut self, tasks: Vec<AgentTask>, timing: AutoTurnAgentsTiming) {
        self.timing_mode = Some(timing);

        for task in tasks {
            self.active_agents.insert(task.id, AgentState::Pending);
            self.pending_queue.push_back(task);
        }
    }

    /// Gets the next task that can be executed.
    pub fn next_runnable(&mut self) -> Option<AgentTask> {
        let timing = self.timing_mode?;

        match timing {
            AutoTurnAgentsTiming::Parallel => {
                // Check if we're at concurrency limit
                let running_count = self
                    .active_agents
                    .values()
                    .filter(|s| matches!(s, AgentState::Running { .. }))
                    .count();

                if running_count >= self.max_concurrent {
                    return None;
                }

                // Get next pending task
                if let Some(task) = self.pending_queue.pop_front() {
                    self.active_agents
                        .insert(task.id, AgentState::Running { started_at: Instant::now() });
                    Some(task)
                } else {
                    None
                }
            }
            AutoTurnAgentsTiming::Blocking => {
                // For blocking, only run one at a time and wait for completion
                let any_running = self
                    .active_agents
                    .values()
                    .any(|s| matches!(s, AgentState::Running { .. }));

                if any_running {
                    return None;
                }

                if let Some(task) = self.pending_queue.pop_front() {
                    self.active_agents
                        .insert(task.id, AgentState::Running { started_at: Instant::now() });
                    Some(task)
                } else {
                    None
                }
            }
        }
    }

    /// Reports that a task has completed successfully.
    pub fn report_completion(&mut self, id: AgentId, output: String) {
        self.report_completion_with_order(id, output, None);
    }

    /// Reports that a task has completed successfully with explicit dispatch order.
    pub fn report_completion_with_order(
        &mut self,
        id: AgentId,
        output: String,
        dispatch_order: Option<usize>,
    ) {
        if let Some(state) = self.active_agents.get(&id) {
            let duration = match state {
                AgentState::Running { started_at } => started_at.elapsed(),
                _ => std::time::Duration::ZERO,
            };

            let order = dispatch_order.unwrap_or(id.0 as usize);

            let result = AgentResult {
                agent_id: id,
                output,
                duration,
                completion_order: self.next_completion_order,
                dispatch_order: order,
            };

            self.next_completion_order += 1;
            self.results.push(result.clone());
            self.active_agents.insert(id, AgentState::Completed { result });
        }
    }

    /// Reports that a task has failed.
    pub fn report_failure(&mut self, id: AgentId, error: String) {
        self.active_agents.insert(id, AgentState::Failed { error });
    }

    /// Collects all completed results, ordered appropriately.
    pub fn collect_results(&mut self) -> Vec<AgentResult> {
        let mut results = std::mem::take(&mut self.results);

        // Order based on timing mode
        match self.timing_mode {
            Some(AutoTurnAgentsTiming::Blocking) => {
                // For blocking, preserve dispatch order
                results.sort_by_key(|r| r.dispatch_order);
            }
            Some(AutoTurnAgentsTiming::Parallel) => {
                // For parallel, use completion order
                results.sort_by_key(|r| r.completion_order);
            }
            None => {}
        }

        results
    }

    /// Returns the number of currently active (running) agents.
    pub fn active_count(&self) -> usize {
        self.active_agents
            .values()
            .filter(|s| matches!(s, AgentState::Running { .. }))
            .count()
    }

    /// Returns the number of pending tasks.
    pub fn pending_count(&self) -> usize {
        self.pending_queue.len()
    }

    /// Returns whether all tasks are complete.
    pub fn is_complete(&self) -> bool {
        self.pending_queue.is_empty()
            && self
                .active_agents
                .values()
                .all(|s| matches!(s, AgentState::Completed { .. } | AgentState::Failed { .. }))
    }

    /// Returns the state of a specific agent.
    pub fn get_state(&self, id: AgentId) -> Option<&AgentState> {
        self.active_agents.get(&id)
    }

    /// Resets the scheduler state.
    pub fn reset(&mut self) {
        self.active_agents.clear();
        self.pending_queue.clear();
        self.results.clear();
        self.next_completion_order = 0;
        self.timing_mode = None;
    }
}

impl Default for AgentScheduler {
    fn default() -> Self {
        Self::new(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_task(id: u64, order: usize) -> AgentTask {
        AgentTask {
            id: AgentId(id),
            prompt: format!("Task {id}"),
            context: None,
            write_access: false,
            models: None,
            dispatch_order: order,
        }
    }

    #[test]
    fn test_parallel_scheduling() {
        let mut scheduler = AgentScheduler::new(2);

        let tasks = vec![create_task(1, 0), create_task(2, 1), create_task(3, 2)];

        scheduler.schedule(tasks, AutoTurnAgentsTiming::Parallel);

        // Should get first two tasks (concurrency limit = 2)
        assert!(scheduler.next_runnable().is_some());
        assert!(scheduler.next_runnable().is_some());
        assert!(scheduler.next_runnable().is_none()); // At limit

        assert_eq!(scheduler.active_count(), 2);
    }

    #[test]
    fn test_blocking_scheduling() {
        let mut scheduler = AgentScheduler::new(4);

        let tasks = vec![create_task(1, 0), create_task(2, 1)];

        scheduler.schedule(tasks, AutoTurnAgentsTiming::Blocking);

        // Should only get one task at a time
        let task1 = scheduler.next_runnable();
        assert!(task1.is_some());
        assert!(scheduler.next_runnable().is_none()); // Must wait for completion

        // Complete first task
        scheduler.report_completion(AgentId(1), "Result 1".to_string());

        // Now can get next task
        assert!(scheduler.next_runnable().is_some());
    }

    #[test]
    fn test_result_collection() {
        let mut scheduler = AgentScheduler::new(4);

        let tasks = vec![create_task(1, 0), create_task(2, 1)];

        scheduler.schedule(tasks, AutoTurnAgentsTiming::Parallel);

        scheduler.next_runnable();
        scheduler.next_runnable();

        // Complete in reverse order
        scheduler.report_completion(AgentId(2), "Result 2".to_string());
        scheduler.report_completion(AgentId(1), "Result 1".to_string());

        let results = scheduler.collect_results();
        assert_eq!(results.len(), 2);

        // For parallel, should be in completion order
        assert_eq!(results[0].agent_id, AgentId(2));
        assert_eq!(results[1].agent_id, AgentId(1));
    }

    #[test]
    fn test_failure_handling() {
        let mut scheduler = AgentScheduler::new(4);

        let tasks = vec![create_task(1, 0), create_task(2, 1)];

        scheduler.schedule(tasks, AutoTurnAgentsTiming::Parallel);

        scheduler.next_runnable();
        scheduler.next_runnable();

        scheduler.report_failure(AgentId(1), "Error occurred".to_string());
        scheduler.report_completion(AgentId(2), "Success".to_string());

        assert!(scheduler.is_complete());

        let state = scheduler.get_state(AgentId(1));
        assert!(matches!(state, Some(AgentState::Failed { .. })));
    }

    #[test]
    fn test_concurrency_limit_respected() {
        let mut scheduler = AgentScheduler::new(3);

        let tasks: Vec<_> = (0..10).map(|i| create_task(i, i as usize)).collect();

        scheduler.schedule(tasks, AutoTurnAgentsTiming::Parallel);

        // Get all available tasks
        let mut running = 0;
        while scheduler.next_runnable().is_some() {
            running += 1;
        }

        assert_eq!(running, 3); // Should respect limit
        assert_eq!(scheduler.active_count(), 3);
        assert_eq!(scheduler.pending_count(), 7);
    }
}
