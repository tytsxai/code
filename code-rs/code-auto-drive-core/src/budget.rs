//! Budget controller for resource limit management.
//!
//! This module provides tracking and enforcement of resource limits including
//! token budgets, turn counts, and duration limits.

use std::time::{Duration, Instant};

/// Configuration for budget limits.
#[derive(Clone, Debug, Default)]
pub struct BudgetConfig {
    /// Maximum tokens allowed for the session.
    pub token_budget: Option<u64>,
    /// Maximum number of turns allowed.
    pub turn_limit: Option<u32>,
    /// Maximum duration allowed.
    pub duration_limit: Option<Duration>,
}

/// Current resource usage statistics.
#[derive(Clone, Debug, Default)]
pub struct ResourceUsage {
    /// Total tokens consumed.
    pub total_tokens: u64,
    /// Number of turns completed.
    pub turns_completed: u32,
    /// Time elapsed since start.
    pub elapsed: Duration,
}

/// Alerts emitted when budget thresholds are reached.
#[derive(Clone, Debug)]
pub enum BudgetAlert {
    /// Token usage has reached warning threshold (80%).
    TokenWarning {
        used: u64,
        limit: u64,
        percentage: f32,
    },
    /// Token usage has exceeded the budget.
    TokenExceeded { used: u64, limit: u64 },
    /// Turn limit has been reached.
    TurnLimitReached { count: u32, limit: u32 },
    /// Duration limit has been exceeded.
    DurationExceeded { elapsed: Duration, limit: Duration },
}

/// Controller for managing resource budgets.
pub struct BudgetController {
    config: BudgetConfig,
    current_usage: ResourceUsage,
    started_at: Option<Instant>,
}

impl BudgetController {
    /// Creates a new BudgetController with no limits.
    pub fn new() -> Self {
        Self {
            config: BudgetConfig::default(),
            current_usage: ResourceUsage::default(),
            started_at: None,
        }
    }

    /// Configures budget limits.
    pub fn configure(&mut self, config: BudgetConfig) {
        self.config = config;
    }

    /// Starts tracking time.
    pub fn start(&mut self) {
        self.started_at = Some(Instant::now());
    }

    /// Records resource usage.
    pub fn record_usage(&mut self, tokens: u64, turn_completed: bool) {
        self.current_usage.total_tokens += tokens;
        if turn_completed {
            self.current_usage.turns_completed += 1;
        }
        if let Some(started) = self.started_at {
            self.current_usage.elapsed = started.elapsed();
        }
    }

    /// Checks budget status and returns any alerts.
    pub fn check_budget(&self) -> Option<BudgetAlert> {
        // Check token budget
        if let Some(limit) = self.config.token_budget {
            let used = self.current_usage.total_tokens;
            let percentage = used as f32 / limit as f32 * 100.0;

            if used >= limit {
                return Some(BudgetAlert::TokenExceeded { used, limit });
            } else if percentage >= 80.0 {
                return Some(BudgetAlert::TokenWarning {
                    used,
                    limit,
                    percentage,
                });
            }
        }

        // Check turn limit
        if let Some(limit) = self.config.turn_limit {
            if self.current_usage.turns_completed >= limit {
                return Some(BudgetAlert::TurnLimitReached {
                    count: self.current_usage.turns_completed,
                    limit,
                });
            }
        }

        // Check duration limit
        if let Some(limit) = self.config.duration_limit {
            let elapsed = self.started_at.map(|s| s.elapsed()).unwrap_or_default();
            if elapsed >= limit {
                return Some(BudgetAlert::DurationExceeded { elapsed, limit });
            }
        }

        None
    }

    /// Returns remaining budget.
    pub fn remaining(&self) -> ResourceUsage {
        let remaining_tokens = self
            .config
            .token_budget
            .map(|limit| limit.saturating_sub(self.current_usage.total_tokens))
            .unwrap_or(u64::MAX);

        let remaining_turns = self
            .config
            .turn_limit
            .map(|limit| limit.saturating_sub(self.current_usage.turns_completed))
            .unwrap_or(u32::MAX);

        let remaining_duration = self
            .config
            .duration_limit
            .map(|limit| {
                let elapsed = self.started_at.map(|s| s.elapsed()).unwrap_or_default();
                limit.saturating_sub(elapsed)
            })
            .unwrap_or(Duration::MAX);

        ResourceUsage {
            total_tokens: remaining_tokens,
            turns_completed: remaining_turns,
            elapsed: remaining_duration,
        }
    }

    /// Returns whether the session should pause due to budget constraints.
    pub fn should_pause(&self) -> bool {
        matches!(
            self.check_budget(),
            Some(BudgetAlert::TokenExceeded { .. })
                | Some(BudgetAlert::TurnLimitReached { .. })
                | Some(BudgetAlert::DurationExceeded { .. })
        )
    }

    /// Returns current usage statistics.
    pub fn current_usage(&self) -> &ResourceUsage {
        &self.current_usage
    }

    /// Resets the controller state.
    pub fn reset(&mut self) {
        self.current_usage = ResourceUsage::default();
        self.started_at = None;
    }
}

impl Default for BudgetController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_limits() {
        let controller = BudgetController::new();
        assert!(controller.check_budget().is_none());
        assert!(!controller.should_pause());
    }

    #[test]
    fn test_token_warning_at_80_percent() {
        let mut controller = BudgetController::new();
        controller.configure(BudgetConfig {
            token_budget: Some(1000),
            ..Default::default()
        });

        controller.record_usage(800, false);

        let alert = controller.check_budget();
        assert!(matches!(alert, Some(BudgetAlert::TokenWarning { .. })));
        assert!(!controller.should_pause());
    }

    #[test]
    fn test_token_exceeded() {
        let mut controller = BudgetController::new();
        controller.configure(BudgetConfig {
            token_budget: Some(1000),
            ..Default::default()
        });

        controller.record_usage(1000, false);

        let alert = controller.check_budget();
        assert!(matches!(alert, Some(BudgetAlert::TokenExceeded { .. })));
        assert!(controller.should_pause());
    }

    #[test]
    fn test_turn_limit() {
        let mut controller = BudgetController::new();
        controller.configure(BudgetConfig {
            turn_limit: Some(5),
            ..Default::default()
        });

        for _ in 0..5 {
            controller.record_usage(100, true);
        }

        let alert = controller.check_budget();
        assert!(matches!(alert, Some(BudgetAlert::TurnLimitReached { .. })));
        assert!(controller.should_pause());
    }

    #[test]
    fn test_remaining_budget() {
        let mut controller = BudgetController::new();
        controller.configure(BudgetConfig {
            token_budget: Some(1000),
            turn_limit: Some(10),
            ..Default::default()
        });

        controller.record_usage(300, true);
        controller.record_usage(200, true);

        let remaining = controller.remaining();
        assert_eq!(remaining.total_tokens, 500);
        assert_eq!(remaining.turns_completed, 8);
    }

    #[test]
    fn test_reset() {
        let mut controller = BudgetController::new();
        controller.configure(BudgetConfig {
            token_budget: Some(1000),
            ..Default::default()
        });

        controller.record_usage(500, true);
        assert_eq!(controller.current_usage().total_tokens, 500);

        controller.reset();
        assert_eq!(controller.current_usage().total_tokens, 0);
        assert_eq!(controller.current_usage().turns_completed, 0);
    }
}
