//! Smart history compaction for Auto Drive sessions.
//!
//! This module provides semantic-aware history compaction that preserves
//! important decisions and errors while reducing context size.

use std::collections::HashSet;

/// Importance level of a history item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemImportance {
    /// Low importance, can be safely removed.
    Low = 0,
    /// Normal importance, remove if needed.
    Normal = 1,
    /// High importance, preserve if possible.
    High = 2,
    /// Critical importance, must be preserved.
    Critical = 3,
}

/// Classification of a history item for compaction.
#[derive(Clone, Debug)]
pub struct ItemClassification {
    /// Index in the original history.
    pub index: usize,
    /// Importance level.
    pub importance: ItemImportance,
    /// Estimated token count.
    pub tokens: u64,
    /// Whether this is the goal message.
    pub is_goal: bool,
    /// Whether this contains an error.
    pub is_error: bool,
    /// Whether this is a key decision.
    pub is_decision: bool,
    /// Brief summary for inclusion in compacted history.
    pub summary: Option<String>,
}

/// Configuration for compaction behavior.
#[derive(Clone, Debug)]
pub struct CompactionConfig {
    /// Target token count after compaction.
    pub target_tokens: u64,
    /// Minimum tokens to preserve (floor).
    pub min_tokens: u64,
    /// Maximum percentage of context to use (0.0-1.0).
    pub max_context_ratio: f32,
    /// Whether to preserve all errors.
    pub preserve_errors: bool,
    /// Whether to preserve all decisions.
    pub preserve_decisions: bool,
    /// Number of recent items to always keep.
    pub keep_recent: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            target_tokens: 50000,
            min_tokens: 10000,
            max_context_ratio: 0.7,
            preserve_errors: true,
            preserve_decisions: true,
            keep_recent: 5,
        }
    }
}

/// Result of a compaction operation.
#[derive(Clone, Debug)]
pub struct CompactionResult {
    /// Indices of items to keep.
    pub keep_indices: Vec<usize>,
    /// Indices of items to remove.
    pub remove_indices: Vec<usize>,
    /// Token count before compaction.
    pub tokens_before: u64,
    /// Estimated token count after compaction.
    pub tokens_after: u64,
    /// Summary of removed content.
    pub removal_summary: String,
    /// Whether the goal was preserved.
    pub goal_preserved: bool,
}

impl CompactionResult {
    /// Returns the number of tokens saved.
    pub fn tokens_saved(&self) -> u64 {
        self.tokens_before.saturating_sub(self.tokens_after)
    }

    /// Returns the savings percentage.
    pub fn savings_percentage(&self) -> f32 {
        if self.tokens_before == 0 {
            return 0.0;
        }
        (self.tokens_saved() as f32 / self.tokens_before as f32) * 100.0
    }
}

/// Engine for performing smart history compaction.
pub struct CompactionEngine {
    config: CompactionConfig,
}

impl CompactionEngine {
    /// Creates a new compaction engine with default config.
    pub fn new() -> Self {
        Self {
            config: CompactionConfig::default(),
        }
    }

    /// Creates a new compaction engine with custom config.
    pub fn with_config(config: CompactionConfig) -> Self {
        Self { config }
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &CompactionConfig {
        &self.config
    }

    /// Updates the configuration.
    pub fn set_config(&mut self, config: CompactionConfig) {
        self.config = config;
    }

    /// Checks if compaction should be triggered.
    pub fn should_compact(&self, current_tokens: u64, context_limit: u64) -> bool {
        let threshold = (context_limit as f32 * self.config.max_context_ratio) as u64;
        current_tokens > threshold
    }

    /// Performs compaction on classified items.
    pub fn compact(&self, items: &[ItemClassification]) -> CompactionResult {
        if items.is_empty() {
            return CompactionResult {
                keep_indices: Vec::new(),
                remove_indices: Vec::new(),
                tokens_before: 0,
                tokens_after: 0,
                removal_summary: String::new(),
                goal_preserved: true,
            };
        }

        let tokens_before: u64 = items.iter().map(|i| i.tokens).sum();

        // Start with all items marked for keeping
        let mut keep_set: HashSet<usize> = items.iter().map(|i| i.index).collect();

        // Always keep goal
        let goal_index = items.iter().find(|i| i.is_goal).map(|i| i.index);

        // Always keep recent items
        let recent_start = items.len().saturating_sub(self.config.keep_recent);
        let recent_indices: HashSet<usize> =
            items[recent_start..].iter().map(|i| i.index).collect();

        // Sort items by importance (ascending) for removal candidates
        let mut removal_candidates: Vec<&ItemClassification> = items
            .iter()
            .filter(|i| {
                // Don't remove goal
                if i.is_goal {
                    return false;
                }
                // Don't remove recent items
                if recent_indices.contains(&i.index) {
                    return false;
                }
                // Don't remove errors if configured
                if self.config.preserve_errors && i.is_error {
                    return false;
                }
                // Don't remove decisions if configured
                if self.config.preserve_decisions && i.is_decision {
                    return false;
                }
                // Don't remove critical items
                if i.importance == ItemImportance::Critical {
                    return false;
                }
                true
            })
            .collect();

        // Sort by importance (low first) then by index (older first)
        removal_candidates
            .sort_by(|a, b| a.importance.cmp(&b.importance).then(a.index.cmp(&b.index)));

        // Remove items until we reach target
        let mut current_tokens = tokens_before;
        let mut removed_summaries: Vec<String> = Vec::new();

        for item in removal_candidates {
            if current_tokens <= self.config.target_tokens {
                break;
            }
            if current_tokens.saturating_sub(item.tokens) < self.config.min_tokens {
                break;
            }

            keep_set.remove(&item.index);
            current_tokens = current_tokens.saturating_sub(item.tokens);

            if let Some(summary) = &item.summary {
                removed_summaries.push(summary.clone());
            }
        }

        let keep_indices: Vec<usize> = {
            let mut indices: Vec<usize> = keep_set.into_iter().collect();
            indices.sort();
            indices
        };

        let remove_indices: Vec<usize> = items
            .iter()
            .map(|i| i.index)
            .filter(|idx| !keep_indices.contains(idx))
            .collect();

        let removal_summary = if removed_summaries.is_empty() {
            "No items removed".to_string()
        } else {
            format!(
                "Removed {} items: {}",
                removed_summaries.len(),
                removed_summaries.join("; ")
            )
        };

        let goal_preserved = goal_index
            .map(|i| !remove_indices.contains(&i))
            .unwrap_or(true);

        CompactionResult {
            keep_indices,
            remove_indices,
            tokens_before,
            tokens_after: current_tokens,
            removal_summary,
            goal_preserved,
        }
    }

    /// Classifies a text item based on content analysis.
    pub fn classify_item(
        &self,
        index: usize,
        content: &str,
        tokens: u64,
        is_first: bool,
    ) -> ItemClassification {
        let is_goal = is_first;
        let is_error = Self::detect_error(content);
        let is_decision = Self::detect_decision(content);

        let importance = if is_goal {
            ItemImportance::Critical
        } else if is_error {
            ItemImportance::High
        } else if is_decision {
            ItemImportance::High
        } else if Self::detect_important_content(content) {
            ItemImportance::Normal
        } else {
            ItemImportance::Low
        };

        let summary = Self::generate_summary(content);

        ItemClassification {
            index,
            importance,
            tokens,
            is_goal,
            is_error,
            is_decision,
            summary,
        }
    }

    fn detect_error(content: &str) -> bool {
        let lower = content.to_lowercase();
        lower.contains("error")
            || lower.contains("failed")
            || lower.contains("exception")
            || lower.contains("panic")
            || lower.contains("traceback")
    }

    fn detect_decision(content: &str) -> bool {
        let lower = content.to_lowercase();
        lower.contains("decision:")
            || lower.contains("decided to")
            || lower.contains("choosing")
            || lower.contains("selected")
            || lower.contains("approach:")
    }

    fn detect_important_content(content: &str) -> bool {
        let lower = content.to_lowercase();
        lower.contains("important")
            || lower.contains("note:")
            || lower.contains("warning")
            || lower.contains("todo")
            || lower.contains("fixme")
    }

    fn generate_summary(content: &str) -> Option<String> {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Take first line or first 100 chars
        let first_line = trimmed.lines().next().unwrap_or(trimmed);
        let summary = if first_line.len() > 100 {
            format!("{}...", &first_line[..97])
        } else {
            first_line.to_string()
        };

        Some(summary)
    }
}

impl Default for CompactionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_compact() {
        let engine = CompactionEngine::new();

        // Default max_context_ratio is 0.7
        // With context_limit of 100000, threshold is 70000
        assert!(!engine.should_compact(50000, 100000));
        assert!(engine.should_compact(80000, 100000));
    }

    #[test]
    fn test_compact_empty() {
        let engine = CompactionEngine::new();
        let result = engine.compact(&[]);

        assert!(result.keep_indices.is_empty());
        assert!(result.remove_indices.is_empty());
        assert!(result.goal_preserved);
    }

    #[test]
    fn test_compact_preserves_goal() {
        let engine = CompactionEngine::with_config(CompactionConfig {
            target_tokens: 100,
            ..Default::default()
        });

        let items = vec![
            ItemClassification {
                index: 0,
                importance: ItemImportance::Critical,
                tokens: 500,
                is_goal: true,
                is_error: false,
                is_decision: false,
                summary: Some("Goal".to_string()),
            },
            ItemClassification {
                index: 1,
                importance: ItemImportance::Low,
                tokens: 500,
                is_goal: false,
                is_error: false,
                is_decision: false,
                summary: Some("Item 1".to_string()),
            },
        ];

        let result = engine.compact(&items);

        assert!(result.goal_preserved);
        assert!(result.keep_indices.contains(&0));
    }

    #[test]
    fn test_compact_preserves_errors() {
        let engine = CompactionEngine::with_config(CompactionConfig {
            target_tokens: 100,
            min_tokens: 50,
            preserve_errors: true,
            keep_recent: 0,
            ..Default::default()
        });

        let items = vec![
            ItemClassification {
                index: 0,
                importance: ItemImportance::Critical,
                tokens: 100,
                is_goal: true,
                is_error: false,
                is_decision: false,
                summary: None,
            },
            ItemClassification {
                index: 1,
                importance: ItemImportance::High,
                tokens: 200,
                is_goal: false,
                is_error: true,
                is_decision: false,
                summary: Some("Error".to_string()),
            },
            ItemClassification {
                index: 2,
                importance: ItemImportance::Low,
                tokens: 200,
                is_goal: false,
                is_error: false,
                is_decision: false,
                summary: Some("Normal".to_string()),
            },
        ];

        let result = engine.compact(&items);

        // Error should be preserved (preserve_errors = true)
        assert!(
            result.keep_indices.contains(&1),
            "Error item should be preserved"
        );
        // Goal should be preserved
        assert!(result.keep_indices.contains(&0), "Goal should be preserved");
    }

    #[test]
    fn test_compact_keeps_recent() {
        let engine = CompactionEngine::with_config(CompactionConfig {
            target_tokens: 100,
            keep_recent: 2,
            ..Default::default()
        });

        let items = vec![
            ItemClassification {
                index: 0,
                importance: ItemImportance::Critical,
                tokens: 100,
                is_goal: true,
                is_error: false,
                is_decision: false,
                summary: None,
            },
            ItemClassification {
                index: 1,
                importance: ItemImportance::Low,
                tokens: 100,
                is_goal: false,
                is_error: false,
                is_decision: false,
                summary: None,
            },
            ItemClassification {
                index: 2,
                importance: ItemImportance::Low,
                tokens: 100,
                is_goal: false,
                is_error: false,
                is_decision: false,
                summary: None,
            },
            ItemClassification {
                index: 3,
                importance: ItemImportance::Low,
                tokens: 100,
                is_goal: false,
                is_error: false,
                is_decision: false,
                summary: None,
            },
        ];

        let result = engine.compact(&items);

        // Last 2 items should be kept
        assert!(result.keep_indices.contains(&2));
        assert!(result.keep_indices.contains(&3));
    }

    #[test]
    fn test_classify_item_goal() {
        let engine = CompactionEngine::new();
        let item = engine.classify_item(0, "Build a web app", 100, true);

        assert!(item.is_goal);
        assert_eq!(item.importance, ItemImportance::Critical);
    }

    #[test]
    fn test_classify_item_error() {
        let engine = CompactionEngine::new();
        let item = engine.classify_item(1, "Error: compilation failed", 100, false);

        assert!(item.is_error);
        assert_eq!(item.importance, ItemImportance::High);
    }

    #[test]
    fn test_classify_item_decision() {
        let engine = CompactionEngine::new();
        let item = engine.classify_item(1, "Decision: use React for the frontend", 100, false);

        assert!(item.is_decision);
        assert_eq!(item.importance, ItemImportance::High);
    }

    #[test]
    fn test_compaction_result_metrics() {
        let result = CompactionResult {
            keep_indices: vec![0, 2],
            remove_indices: vec![1],
            tokens_before: 1000,
            tokens_after: 600,
            removal_summary: "Removed 1 item".to_string(),
            goal_preserved: true,
        };

        assert_eq!(result.tokens_saved(), 400);
        assert!((result.savings_percentage() - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_removal_by_importance() {
        let engine = CompactionEngine::with_config(CompactionConfig {
            target_tokens: 200,
            min_tokens: 100,
            keep_recent: 0,
            preserve_errors: false,
            preserve_decisions: false,
            ..Default::default()
        });

        let items = vec![
            ItemClassification {
                index: 0,
                importance: ItemImportance::Critical,
                tokens: 100,
                is_goal: true,
                is_error: false,
                is_decision: false,
                summary: None,
            },
            ItemClassification {
                index: 1,
                importance: ItemImportance::Low,
                tokens: 100,
                is_goal: false,
                is_error: false,
                is_decision: false,
                summary: Some("Low".to_string()),
            },
            ItemClassification {
                index: 2,
                importance: ItemImportance::Normal,
                tokens: 100,
                is_goal: false,
                is_error: false,
                is_decision: false,
                summary: Some("Normal".to_string()),
            },
            ItemClassification {
                index: 3,
                importance: ItemImportance::High,
                tokens: 100,
                is_goal: false,
                is_error: false,
                is_decision: false,
                summary: Some("High".to_string()),
            },
        ];

        let result = engine.compact(&items);

        // Low importance should be removed first
        assert!(!result.keep_indices.contains(&1));
        // Goal should always be kept
        assert!(result.keep_indices.contains(&0));
    }
}
