mod auto_coordinator;
mod auto_drive_history;
mod auto_compact;
mod session_metrics;
mod coordinator_router;
mod coordinator_user_schema;
mod controller;
mod retry;
pub mod parallel_execution;

// Enhanced Auto Drive feature modules
pub mod audit;
pub mod budget;
pub mod checkpoint;
pub mod compaction;
pub mod diagnostics;
pub mod enhanced;
pub mod intervention;
pub mod progress;
pub mod retry_enhanced;
pub mod scheduler;
pub mod telemetry;

#[cfg(feature = "dev-faults")]
mod faults;

#[cfg(test)]
mod property_tests;

pub use auto_coordinator::{
    start_auto_coordinator,
    AutoCoordinatorCommand,
    AutoCoordinatorEvent,
    AutoCoordinatorEventSender,
    AutoCoordinatorHandle,
    AutoCoordinatorStatus,
    AutoTurnAgentsAction,
    AutoTurnAgentsTiming,
    AutoTurnCliAction,
    TurnComplexity,
    TurnConfig,
    TurnDescriptor,
    TurnMode,
    MODEL_SLUG,
};

pub use controller::{
    AutoContinueMode,
    AutoControllerEffect,
    AutoDriveController,
    AutoRunPhase,
    AutoResolvePhase,
    AutoResolveState,
    AutoRestartState,
    AutoRunSummary,
    AutoTurnReviewState,
    PhaseTransition,
    TransitionEffects,
    AUTO_RESTART_BASE_DELAY,
    AUTO_RESTART_MAX_ATTEMPTS,
    AUTO_RESTART_MAX_DELAY,
    AUTO_RESOLVE_MAX_REVIEW_ATTEMPTS,
    AUTO_RESOLVE_REVIEW_FOLLOWUP,
};

pub use auto_drive_history::AutoDriveHistory;
pub use session_metrics::SessionMetrics;
pub use coordinator_router::{
    route_user_message,
    CoordinatorContext,
    CoordinatorRouterResponse,
};
pub use coordinator_user_schema::{
    parse_user_turn_reply,
    user_turn_schema,
};
