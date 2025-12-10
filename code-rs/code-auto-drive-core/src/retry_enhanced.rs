//! Enhanced retry strategy for Auto Drive error recovery.
//!
//! This module provides intelligent retry logic with exponential backoff,
//! jitter, and error classification for different failure types.

use std::time::Duration;

use rand::Rng;
use thiserror::Error;

use crate::budget::BudgetAlert;
use crate::diagnostics::DiagnosticAlert;

/// Default base delay for rate limit errors (30 seconds).
const RATE_LIMIT_BASE_DELAY_SECS: u64 = 30;

/// Default base delay for network errors (5 seconds).
const NETWORK_BASE_DELAY_SECS: u64 = 5;

/// Default base delay for malformed response errors (1 second).
const MALFORMED_BASE_DELAY_SECS: u64 = 1;

/// Maximum consecutive failures before pausing.
const DEFAULT_FAILURE_THRESHOLD: u32 = 5;

/// Errors that can occur during Auto Drive execution.
#[derive(Debug, Error, Clone)]
pub enum AutoDriveError {
    /// Rate limit exceeded, should wait before retrying.
    #[error("rate limit exceeded, retry after {retry_after:?}")]
    RateLimit { retry_after: Option<Duration> },

    /// Network connectivity error.
    #[error("network error: {message}")]
    Network { message: String },

    /// Model returned malformed or invalid response.
    #[error("malformed model response: {message}")]
    MalformedResponse { message: String },

    /// Budget limit exceeded, requires user decision.
    #[error("budget exceeded")]
    BudgetExceeded { alert: BudgetAlert },

    /// Diagnostic alert triggered, may require intervention.
    #[error("diagnostic alert")]
    DiagnosticAlert { alert: DiagnosticAlert },

    /// Checkpoint file is corrupted.
    #[error("checkpoint corruption: {message}")]
    CheckpointCorruption { message: String },

    /// API quota exceeded (not retryable).
    #[error("quota exceeded")]
    QuotaExceeded,

    /// Authentication failed (not retryable).
    #[error("authentication failed")]
    AuthenticationFailed,

    /// Generic internal error.
    #[error("internal error: {message}")]
    Internal { message: String },
}

impl AutoDriveError {
    /// Returns true if this error is potentially recoverable through retry.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimit { .. } | Self::Network { .. } | Self::MalformedResponse { .. }
        )
    }

    /// Returns true if this error requires user intervention.
    pub fn requires_intervention(&self) -> bool {
        matches!(
            self,
            Self::BudgetExceeded { .. } | Self::DiagnosticAlert { .. }
        )
    }

    /// Returns true if this error is fatal and cannot be recovered.
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::CheckpointCorruption { .. } | Self::QuotaExceeded | Self::AuthenticationFailed
        )
    }

    /// Creates a rate limit error from an optional retry-after duration.
    pub fn rate_limit(retry_after: Option<Duration>) -> Self {
        Self::RateLimit { retry_after }
    }

    /// Creates a network error from a message.
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    /// Creates a malformed response error.
    pub fn malformed_response(message: impl Into<String>) -> Self {
        Self::MalformedResponse {
            message: message.into(),
        }
    }
}

/// Configuration for retry behavior.
#[derive(Clone, Debug)]
pub struct RetryStrategy {
    /// Base delay before first retry.
    pub base_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Maximum number of retry attempts.
    pub max_attempts: u32,
    /// Jitter factor (0.0 to 1.0) to randomize delays.
    pub jitter_factor: f32,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(60),
            max_attempts: 3,
            jitter_factor: 0.1,
        }
    }
}

impl RetryStrategy {
    /// Creates a retry strategy appropriate for the given error type.
    pub fn for_error(error: &AutoDriveError) -> Self {
        match error {
            AutoDriveError::RateLimit { retry_after } => Self {
                base_delay: retry_after.unwrap_or(Duration::from_secs(RATE_LIMIT_BASE_DELAY_SECS)),
                max_delay: Duration::from_secs(300),
                max_attempts: 10,
                jitter_factor: 0.1,
            },
            AutoDriveError::Network { .. } => Self {
                base_delay: Duration::from_secs(NETWORK_BASE_DELAY_SECS),
                max_delay: Duration::from_secs(60),
                max_attempts: 5,
                jitter_factor: 0.2,
            },
            AutoDriveError::MalformedResponse { .. } => Self {
                base_delay: Duration::from_secs(MALFORMED_BASE_DELAY_SECS),
                max_delay: Duration::from_secs(10),
                max_attempts: 3,
                jitter_factor: 0.0,
            },
            _ => Self::default(),
        }
    }

    /// Calculates the delay for a given attempt number (0-indexed).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base = self.base_delay.as_secs_f64() * 2.0_f64.powi(attempt as i32);
        let capped = base.min(self.max_delay.as_secs_f64());

        let jitter: f64 = if self.jitter_factor > 0.0 {
            let mut rng = rand::rng();
            rng.random_range(0.0..self.jitter_factor as f64)
        } else {
            0.0
        };

        Duration::from_secs_f64(capped * (1.0 + jitter))
    }

    /// Returns true if more retries are allowed.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

/// Tracks consecutive failures and manages recovery state.
pub struct FailureCounter {
    consecutive_failures: u32,
    threshold: u32,
    last_error: Option<AutoDriveError>,
}

impl FailureCounter {
    /// Creates a new failure counter with default threshold.
    pub fn new() -> Self {
        Self {
            consecutive_failures: 0,
            threshold: DEFAULT_FAILURE_THRESHOLD,
            last_error: None,
        }
    }

    /// Creates a new failure counter with custom threshold.
    pub fn with_threshold(threshold: u32) -> Self {
        Self {
            consecutive_failures: 0,
            threshold,
            last_error: None,
        }
    }

    /// Records a failure.
    pub fn record_failure(&mut self, error: AutoDriveError) {
        self.consecutive_failures += 1;
        self.last_error = Some(error);
    }

    /// Records a successful operation, resetting the counter.
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.last_error = None;
    }

    /// Returns the current failure count.
    pub fn count(&self) -> u32 {
        self.consecutive_failures
    }

    /// Returns true if the failure threshold has been reached.
    pub fn threshold_reached(&self) -> bool {
        self.consecutive_failures >= self.threshold
    }

    /// Returns the last recorded error.
    pub fn last_error(&self) -> Option<&AutoDriveError> {
        self.last_error.as_ref()
    }

    /// Resets the counter.
    pub fn reset(&mut self) {
        self.consecutive_failures = 0;
        self.last_error = None;
    }
}

impl Default for FailureCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Guidance for recovering from malformed responses.
#[derive(Clone, Debug)]
pub struct RecoveryGuidance {
    /// Message to include in retry request.
    pub guidance_message: String,
    /// Whether to request schema validation.
    pub request_schema_validation: bool,
}

impl RecoveryGuidance {
    /// Creates guidance for schema violation recovery.
    pub fn for_schema_violation(violation: &str) -> Self {
        Self {
            guidance_message: format!(
                "Your previous response did not conform to the expected schema. \
                 Error: {violation}. Please ensure your response follows the required format."
            ),
            request_schema_validation: true,
        }
    }

    /// Creates guidance for missing field recovery.
    pub fn for_missing_field(field: &str) -> Self {
        Self {
            guidance_message: format!(
                "Your previous response was missing the required field '{field}'. \
                 Please include this field in your response."
            ),
            request_schema_validation: true,
        }
    }

    /// Creates guidance for invalid value recovery.
    pub fn for_invalid_value(field: &str, expected: &str) -> Self {
        Self {
            guidance_message: format!(
                "The value for '{field}' was invalid. Expected: {expected}. \
                 Please provide a valid value."
            ),
            request_schema_validation: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        let rate_limit = AutoDriveError::rate_limit(Some(Duration::from_secs(30)));
        assert!(rate_limit.is_retryable());
        assert!(!rate_limit.requires_intervention());
        assert!(!rate_limit.is_fatal());

        let network = AutoDriveError::network("connection refused");
        assert!(network.is_retryable());

        let budget = AutoDriveError::BudgetExceeded {
            alert: BudgetAlert::TokenExceeded {
                used: 1000,
                limit: 1000,
            },
        };
        assert!(!budget.is_retryable());
        assert!(budget.requires_intervention());

        let quota = AutoDriveError::QuotaExceeded;
        assert!(!quota.is_retryable());
        assert!(quota.is_fatal());
    }

    #[test]
    fn test_retry_strategy_for_error() {
        let rate_limit = AutoDriveError::rate_limit(Some(Duration::from_secs(60)));
        let strategy = RetryStrategy::for_error(&rate_limit);
        assert_eq!(strategy.base_delay, Duration::from_secs(60));
        assert_eq!(strategy.max_attempts, 10);

        let network = AutoDriveError::network("timeout");
        let strategy = RetryStrategy::for_error(&network);
        assert_eq!(strategy.base_delay, Duration::from_secs(NETWORK_BASE_DELAY_SECS));
        assert_eq!(strategy.max_attempts, 5);
    }

    #[test]
    fn test_delay_calculation() {
        let strategy = RetryStrategy {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            max_attempts: 5,
            jitter_factor: 0.0, // No jitter for deterministic test
        };

        assert_eq!(strategy.delay_for_attempt(0), Duration::from_secs(1));
        assert_eq!(strategy.delay_for_attempt(1), Duration::from_secs(2));
        assert_eq!(strategy.delay_for_attempt(2), Duration::from_secs(4));
        assert_eq!(strategy.delay_for_attempt(3), Duration::from_secs(8));

        // Should cap at max_delay
        assert_eq!(strategy.delay_for_attempt(10), Duration::from_secs(60));
    }

    #[test]
    fn test_rate_limit_longer_than_network() {
        let rate_limit = AutoDriveError::rate_limit(None);
        let network = AutoDriveError::network("error");

        let rate_strategy = RetryStrategy::for_error(&rate_limit);
        let network_strategy = RetryStrategy::for_error(&network);

        assert!(rate_strategy.base_delay > network_strategy.base_delay);
    }

    #[test]
    fn test_failure_counter() {
        let mut counter = FailureCounter::with_threshold(3);

        assert_eq!(counter.count(), 0);
        assert!(!counter.threshold_reached());

        counter.record_failure(AutoDriveError::network("error 1"));
        assert_eq!(counter.count(), 1);

        counter.record_failure(AutoDriveError::network("error 2"));
        assert_eq!(counter.count(), 2);

        counter.record_failure(AutoDriveError::network("error 3"));
        assert_eq!(counter.count(), 3);
        assert!(counter.threshold_reached());
    }

    #[test]
    fn test_failure_counter_reset_on_success() {
        let mut counter = FailureCounter::new();

        counter.record_failure(AutoDriveError::network("error 1"));
        counter.record_failure(AutoDriveError::network("error 2"));
        assert_eq!(counter.count(), 2);

        counter.record_success();
        assert_eq!(counter.count(), 0);
        assert!(counter.last_error().is_none());
    }

    #[test]
    fn test_recovery_guidance() {
        let guidance = RecoveryGuidance::for_schema_violation("missing required field");
        assert!(guidance.guidance_message.contains("schema"));
        assert!(guidance.request_schema_validation);

        let guidance = RecoveryGuidance::for_missing_field("prompt_sent_to_cli");
        assert!(guidance.guidance_message.contains("prompt_sent_to_cli"));
    }

    #[test]
    fn test_should_retry() {
        let strategy = RetryStrategy {
            max_attempts: 3,
            ..Default::default()
        };

        assert!(strategy.should_retry(0));
        assert!(strategy.should_retry(1));
        assert!(strategy.should_retry(2));
        assert!(!strategy.should_retry(3));
    }
}
