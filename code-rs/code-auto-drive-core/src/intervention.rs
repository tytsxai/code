//! Interactive intervention handling for Auto Drive sessions.
//!
//! This module provides state management and handling for user interventions
//! during Auto Drive execution, including pause/resume, goal modification,
//! and step skipping.

use crate::budget::BudgetAlert;
use crate::diagnostics::DiagnosticAlert;

/// Reason for requiring user intervention.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InterventionReason {
    /// User explicitly requested intervention.
    UserRequested,
    /// User wants to modify the goal.
    GoalModification,
    /// User wants to skip the current step.
    StepSkip,
    /// Budget limit reached, user decision needed.
    BudgetDecision { alert: BudgetAlertKind },
    /// Diagnostic issue detected, user review needed.
    DiagnosticReview { alert: DiagnosticAlertKind },
}

/// Simplified budget alert kind for intervention.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BudgetAlertKind {
    TokenWarning,
    TokenExceeded,
    TurnLimitReached,
    DurationExceeded,
    BackpressureWarning,
    BackpressureExceeded,
}

impl From<&BudgetAlert> for BudgetAlertKind {
    fn from(alert: &BudgetAlert) -> Self {
        match alert {
            BudgetAlert::TokenWarning { .. } => Self::TokenWarning,
            BudgetAlert::TokenExceeded { .. } => Self::TokenExceeded,
            BudgetAlert::TurnLimitReached { .. } => Self::TurnLimitReached,
            BudgetAlert::DurationExceeded { .. } => Self::DurationExceeded,
            BudgetAlert::BackpressureWarning { .. } => Self::BackpressureWarning,
            BudgetAlert::BackpressureExceeded { .. } => Self::BackpressureExceeded,
        }
    }
}

/// Simplified diagnostic alert kind for intervention.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticAlertKind {
    LoopDetected,
    GoalDrift,
    TokenOverrun,
    RepetitiveResponse,
    SessionSlow,
    SessionStuck,
    SessionMigrated,
    LowConcurrency,
}

impl From<&DiagnosticAlert> for DiagnosticAlertKind {
    fn from(alert: &DiagnosticAlert) -> Self {
        match alert {
            DiagnosticAlert::LoopDetected { .. } => Self::LoopDetected,
            DiagnosticAlert::GoalDrift { .. } => Self::GoalDrift,
            DiagnosticAlert::TokenOverrun { .. } => Self::TokenOverrun,
            DiagnosticAlert::RepetitiveResponse { .. } => Self::RepetitiveResponse,
            DiagnosticAlert::SessionSlow { .. } => Self::SessionSlow,
            DiagnosticAlert::SessionStuck { .. } => Self::SessionStuck,
            DiagnosticAlert::SessionMigrated { .. } => Self::SessionMigrated,
            DiagnosticAlert::LowConcurrency { .. } => Self::LowConcurrency,
        }
    }
}

/// State of an intervention request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InterventionState {
    /// No intervention active.
    None,
    /// Intervention requested, awaiting user action.
    Pending { reason: InterventionReason },
    /// User is editing the prompt.
    EditingPrompt { original_prompt: String },
    /// User is modifying the goal.
    ModifyingGoal { original_goal: String },
    /// Intervention resolved, ready to resume.
    Resolved { action: InterventionAction },
}

impl Default for InterventionState {
    fn default() -> Self {
        Self::None
    }
}

/// Action taken to resolve an intervention.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InterventionAction {
    /// Resume with no changes.
    Resume,
    /// Resume with modified prompt.
    ResumeWithPrompt { new_prompt: String },
    /// Resume with modified goal.
    ResumeWithGoal { new_goal: String },
    /// Skip the current step.
    SkipStep,
    /// Stop the session.
    Stop,
    /// Extend budget and continue.
    ExtendBudget {
        additional_tokens: Option<u64>,
        additional_turns: Option<u32>,
    },
}

/// Handler for managing intervention state and transitions.
pub struct InterventionHandler {
    state: InterventionState,
    pending_clarification: Option<String>,
}

impl InterventionHandler {
    /// Creates a new intervention handler.
    pub fn new() -> Self {
        Self {
            state: InterventionState::None,
            pending_clarification: None,
        }
    }

    /// Returns the current intervention state.
    pub fn state(&self) -> &InterventionState {
        &self.state
    }

    /// Returns whether an intervention is active.
    pub fn is_active(&self) -> bool {
        !matches!(self.state, InterventionState::None)
    }

    /// Returns whether the handler is awaiting user input.
    pub fn is_awaiting_input(&self) -> bool {
        matches!(
            self.state,
            InterventionState::Pending { .. }
                | InterventionState::EditingPrompt { .. }
                | InterventionState::ModifyingGoal { .. }
        )
    }

    /// Requests an intervention for the given reason.
    pub fn request(&mut self, reason: InterventionReason) {
        self.state = InterventionState::Pending { reason };
    }

    /// Requests intervention due to a budget alert.
    pub fn request_for_budget(&mut self, alert: &BudgetAlert) {
        self.request(InterventionReason::BudgetDecision {
            alert: BudgetAlertKind::from(alert),
        });
    }

    /// Requests intervention due to a diagnostic alert.
    pub fn request_for_diagnostic(&mut self, alert: &DiagnosticAlert) {
        self.request(InterventionReason::DiagnosticReview {
            alert: DiagnosticAlertKind::from(alert),
        });
    }

    /// Starts editing the prompt.
    pub fn start_edit_prompt(&mut self, current_prompt: &str) {
        self.state = InterventionState::EditingPrompt {
            original_prompt: current_prompt.to_string(),
        };
    }

    /// Starts modifying the goal.
    pub fn start_modify_goal(&mut self, current_goal: &str) {
        self.state = InterventionState::ModifyingGoal {
            original_goal: current_goal.to_string(),
        };
    }

    /// Resolves the intervention with the given action.
    pub fn resolve(&mut self, action: InterventionAction) {
        self.state = InterventionState::Resolved { action };
    }

    /// Sets a clarification message to inject into the coordinator.
    pub fn set_clarification(&mut self, clarification: String) {
        self.pending_clarification = Some(clarification);
    }

    /// Takes the pending clarification if any.
    pub fn take_clarification(&mut self) -> Option<String> {
        self.pending_clarification.take()
    }

    /// Clears the intervention state.
    pub fn clear(&mut self) {
        self.state = InterventionState::None;
        self.pending_clarification = None;
    }

    /// Takes the resolved action and clears the state.
    pub fn take_action(&mut self) -> Option<InterventionAction> {
        if let InterventionState::Resolved { action } = &self.state {
            let action = action.clone();
            self.clear();
            Some(action)
        } else {
            None
        }
    }

    /// Returns a human-readable description of the current state.
    pub fn status_message(&self) -> Option<String> {
        match &self.state {
            InterventionState::None => None,
            InterventionState::Pending { reason } => Some(match reason {
                InterventionReason::UserRequested => "Paused by user".to_string(),
                InterventionReason::GoalModification => "Awaiting goal modification".to_string(),
                InterventionReason::StepSkip => "Awaiting step skip confirmation".to_string(),
                InterventionReason::BudgetDecision { alert } => match alert {
                    BudgetAlertKind::TokenWarning => "Token budget warning".to_string(),
                    BudgetAlertKind::TokenExceeded => "Token budget exceeded".to_string(),
                    BudgetAlertKind::TurnLimitReached => "Turn limit reached".to_string(),
                    BudgetAlertKind::DurationExceeded => "Duration limit exceeded".to_string(),
                    BudgetAlertKind::BackpressureWarning => {
                        "Session pool backpressure warning".to_string()
                    }
                    BudgetAlertKind::BackpressureExceeded => {
                        "Session pool backpressure exceeded".to_string()
                    }
                },
                InterventionReason::DiagnosticReview { alert } => match alert {
                    DiagnosticAlertKind::LoopDetected => "Loop detected".to_string(),
                    DiagnosticAlertKind::GoalDrift => "Goal drift detected".to_string(),
                    DiagnosticAlertKind::TokenOverrun => "Token usage anomaly".to_string(),
                    DiagnosticAlertKind::RepetitiveResponse => "Repetitive responses".to_string(),
                    DiagnosticAlertKind::SessionSlow => "Session running slowly".to_string(),
                    DiagnosticAlertKind::SessionStuck => "Session stuck".to_string(),
                    DiagnosticAlertKind::SessionMigrated => "Session migrated".to_string(),
                    DiagnosticAlertKind::LowConcurrency => {
                        "Parallel execution concurrency is below target".to_string()
                    }
                },
            }),
            InterventionState::EditingPrompt { .. } => Some("Editing prompt...".to_string()),
            InterventionState::ModifyingGoal { .. } => Some("Modifying goal...".to_string()),
            InterventionState::Resolved { action } => Some(match action {
                InterventionAction::Resume => "Resuming...".to_string(),
                InterventionAction::ResumeWithPrompt { .. } => {
                    "Resuming with new prompt...".to_string()
                }
                InterventionAction::ResumeWithGoal { .. } => {
                    "Resuming with new goal...".to_string()
                }
                InterventionAction::SkipStep => "Skipping step...".to_string(),
                InterventionAction::Stop => "Stopping...".to_string(),
                InterventionAction::ExtendBudget { .. } => "Extending budget...".to_string(),
            }),
        }
    }
}

impl Default for InterventionHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let handler = InterventionHandler::new();
        assert!(!handler.is_active());
        assert!(!handler.is_awaiting_input());
        assert!(handler.status_message().is_none());
    }

    #[test]
    fn test_request_intervention() {
        let mut handler = InterventionHandler::new();

        handler.request(InterventionReason::UserRequested);

        assert!(handler.is_active());
        assert!(handler.is_awaiting_input());
        assert!(matches!(
            handler.state(),
            InterventionState::Pending {
                reason: InterventionReason::UserRequested
            }
        ));
    }

    #[test]
    fn test_budget_intervention() {
        let mut handler = InterventionHandler::new();
        let alert = BudgetAlert::TokenExceeded {
            used: 1000,
            limit: 1000,
        };

        handler.request_for_budget(&alert);

        assert!(handler.is_active());
        assert!(matches!(
            handler.state(),
            InterventionState::Pending {
                reason: InterventionReason::BudgetDecision {
                    alert: BudgetAlertKind::TokenExceeded
                }
            }
        ));
    }

    #[test]
    fn test_diagnostic_intervention() {
        let mut handler = InterventionHandler::new();
        let alert = DiagnosticAlert::LoopDetected {
            tool_name: "test".to_string(),
            count: 3,
        };

        handler.request_for_diagnostic(&alert);

        assert!(handler.is_active());
        assert!(matches!(
            handler.state(),
            InterventionState::Pending {
                reason: InterventionReason::DiagnosticReview {
                    alert: DiagnosticAlertKind::LoopDetected
                }
            }
        ));
    }

    #[test]
    fn test_edit_prompt_flow() {
        let mut handler = InterventionHandler::new();

        handler.start_edit_prompt("original prompt");

        assert!(handler.is_active());
        assert!(handler.is_awaiting_input());
        assert!(matches!(
            handler.state(),
            InterventionState::EditingPrompt { original_prompt } if original_prompt == "original prompt"
        ));

        handler.resolve(InterventionAction::ResumeWithPrompt {
            new_prompt: "new prompt".to_string(),
        });

        let action = handler.take_action();
        assert!(matches!(
            action,
            Some(InterventionAction::ResumeWithPrompt { new_prompt }) if new_prompt == "new prompt"
        ));
        assert!(!handler.is_active());
    }

    #[test]
    fn test_modify_goal_flow() {
        let mut handler = InterventionHandler::new();

        handler.start_modify_goal("original goal");

        assert!(handler.is_active());
        assert!(matches!(
            handler.state(),
            InterventionState::ModifyingGoal { original_goal } if original_goal == "original goal"
        ));

        handler.resolve(InterventionAction::ResumeWithGoal {
            new_goal: "new goal".to_string(),
        });

        let action = handler.take_action();
        assert!(matches!(
            action,
            Some(InterventionAction::ResumeWithGoal { new_goal }) if new_goal == "new goal"
        ));
    }

    #[test]
    fn test_skip_step() {
        let mut handler = InterventionHandler::new();

        handler.request(InterventionReason::StepSkip);
        handler.resolve(InterventionAction::SkipStep);

        let action = handler.take_action();
        assert!(matches!(action, Some(InterventionAction::SkipStep)));
    }

    #[test]
    fn test_extend_budget() {
        let mut handler = InterventionHandler::new();
        let alert = BudgetAlert::TokenExceeded {
            used: 1000,
            limit: 1000,
        };

        handler.request_for_budget(&alert);
        handler.resolve(InterventionAction::ExtendBudget {
            additional_tokens: Some(5000),
            additional_turns: None,
        });

        let action = handler.take_action();
        assert!(matches!(
            action,
            Some(InterventionAction::ExtendBudget {
                additional_tokens: Some(5000),
                ..
            })
        ));
    }

    #[test]
    fn test_clarification() {
        let mut handler = InterventionHandler::new();

        handler.set_clarification("Please focus on the main task".to_string());

        let clarification = handler.take_clarification();
        assert_eq!(
            clarification,
            Some("Please focus on the main task".to_string())
        );
        assert!(handler.take_clarification().is_none());
    }

    #[test]
    fn test_clear() {
        let mut handler = InterventionHandler::new();

        handler.request(InterventionReason::UserRequested);
        handler.set_clarification("test".to_string());

        handler.clear();

        assert!(!handler.is_active());
        assert!(handler.take_clarification().is_none());
    }

    #[test]
    fn test_status_messages() {
        let mut handler = InterventionHandler::new();

        // Test various states produce status messages
        handler.request(InterventionReason::UserRequested);
        assert!(handler.status_message().is_some());

        handler.start_edit_prompt("test");
        assert!(handler.status_message().is_some());

        handler.resolve(InterventionAction::Resume);
        assert!(handler.status_message().is_some());

        handler.clear();
        assert!(handler.status_message().is_none());
    }
}
