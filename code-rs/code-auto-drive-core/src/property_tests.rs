//! Property-based tests for Auto Drive enhancement components.
//!
//! These tests use proptest to verify correctness properties across
//! a wide range of inputs.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::AutoTurnAgentsTiming;
    use crate::budget::BudgetAlert;
    use crate::budget::BudgetConfig;
    use crate::budget::BudgetController;
    use crate::checkpoint::CheckpointManager;
    use crate::checkpoint::TokenUsage;
    use crate::compaction::CompactionConfig;
    use crate::compaction::CompactionEngine;
    use crate::compaction::ItemClassification;
    use crate::compaction::ItemImportance;
    use crate::diagnostics::DiagnosticAlert;
    use crate::diagnostics::DiagnosticsEngine;
    use crate::diagnostics::ToolCallRecord;
    use crate::diagnostics::ToolOutcome;
    use crate::progress::AutoDrivePhase;
    use crate::progress::ProgressCollector;
    use crate::progress::ProgressViewModel;
    use crate::retry_enhanced::AutoDriveError;
    use crate::retry_enhanced::FailureCounter;
    use crate::retry_enhanced::RetryStrategy;
    use crate::scheduler::AgentId;
    use crate::scheduler::AgentScheduler;
    use crate::scheduler::AgentTask;
    use crate::telemetry::SessionOutcome;
    use crate::telemetry::TelemetryCollector;
    use crate::telemetry::TurnOutcome;

    // =========================================================================
    // Property 1: Checkpoint Round-Trip Consistency
    // For any valid Auto Drive session state, saving to checkpoint and then
    // restoring should produce an equivalent state.
    // Validates: Requirements 1.1, 1.2, 1.3
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn checkpoint_round_trip(
            goal in "[a-zA-Z0-9 ]{1,100}",
            turns in 0..50usize,
            tokens in 0..100000u64,
        ) {
            let temp_dir = tempfile::TempDir::new().unwrap();
            let mut manager = CheckpointManager::new(temp_dir.path().to_path_buf());

            let session_id = format!("session-{turns}-{tokens}");
            let mut checkpoint = manager.create(&goal, &session_id).unwrap();

            // Update checkpoint with state using the proper update method
            let token_usage = TokenUsage {
                input_tokens: tokens / 2,
                output_tokens: tokens / 2,
                total_tokens: tokens,
            };
            manager.update(
                &mut checkpoint,
                vec![],
                turns,
                token_usage,
                &crate::AutoRunPhase::Active,
            ).unwrap();
            let restored = manager.restore(&session_id).unwrap().unwrap();

            // Verify round-trip consistency
            prop_assert_eq!(&checkpoint.goal, &restored.goal);
            prop_assert_eq!(checkpoint.turns_completed, restored.turns_completed);
            prop_assert_eq!(checkpoint.token_usage.total_tokens, restored.token_usage.total_tokens);
            prop_assert_eq!(&checkpoint.session_id, &restored.session_id);
        }
    }

    // =========================================================================
    // Property 2: Checkpoint Integrity Validation
    // For any corrupted or tampered checkpoint, validation should detect it.
    // Validates: Requirements 1.5
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(30))]

        #[test]
        fn checkpoint_integrity_validation(
            goal in "[a-zA-Z0-9 ]{1,50}",
            tampered_goal in "[a-zA-Z0-9 ]{1,50}",
        ) {
            prop_assume!(goal != tampered_goal);

            let temp_dir = tempfile::TempDir::new().unwrap();
            let mut manager = CheckpointManager::new(temp_dir.path().to_path_buf());

            let checkpoint = manager.create(&goal, "test-session").unwrap();

            // Original should validate
            prop_assert!(manager.validate(&checkpoint).unwrap());

            // Tampered checkpoint should fail validation
            let mut tampered = checkpoint;
            tampered.goal = tampered_goal;
            prop_assert!(!manager.validate(&tampered).unwrap());
        }
    }

    // =========================================================================
    // Property 5: Loop Detection Accuracy
    // For any sequence of tool calls, if the same tool with identical arguments
    // appears three or more times consecutively, the loop detector should flag it.
    // Validates: Requirements 3.1
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn loop_detection_accuracy(
            tool_name in "[a-z_]{1,20}",
            args_hash in any::<u64>(),
            repeat_count in 3..10usize,
        ) {
            let mut engine = DiagnosticsEngine::new();

            // Add identical tool calls
            for _ in 0..repeat_count {
                engine.record_tool_call(ToolCallRecord {
                    tool_name: tool_name.clone(),
                    arguments_hash: args_hash,
                    timestamp: std::time::Instant::now(),
                    outcome: ToolOutcome::Success,
                });
            }

            let alert = engine.check_loop();
            prop_assert!(alert.is_some(), "Loop should be detected for {} consecutive calls", repeat_count);

            if let Some(DiagnosticAlert::LoopDetected { count, .. }) = alert {
                prop_assert!(count >= 3, "Loop count should be at least 3, got {count}");
            }
        }

        #[test]
        fn no_loop_for_varied_calls(
            tools in prop::collection::vec("[a-z_]{1,10}", 3..10),
        ) {
            let mut engine = DiagnosticsEngine::new();

            // Add varied tool calls with different hashes
            for (i, tool) in tools.iter().enumerate() {
                engine.record_tool_call(ToolCallRecord {
                    tool_name: tool.clone(),
                    arguments_hash: i as u64,
                    timestamp: std::time::Instant::now(),
                    outcome: ToolOutcome::Success,
                });
            }

            // Should not detect loop if calls are varied
            let has_consecutive = tools.windows(3).any(|w| w[0] == w[1] && w[1] == w[2]);
            if !has_consecutive {
                prop_assert!(engine.check_loop().is_none());
            }
        }
    }

    // =========================================================================
    // Property 6: Token Anomaly Detection
    // For any token usage that exceeds the projected estimate by more than 50%,
    // the diagnostics engine should emit a warning alert.
    // Validates: Requirements 3.3
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn token_anomaly_detection(
            projected in 1000..100000u64,
            overrun_percent in 51..200u32,
        ) {
            let mut engine = DiagnosticsEngine::new();
            engine.set_projection(projected, 10);

            let actual = projected + (projected * overrun_percent as u64 / 100);
            engine.update_token_usage(actual);

            let alert = engine.check_token_anomaly();
            prop_assert!(alert.is_some(), "Should detect anomaly at {}% overrun", overrun_percent);

            if let Some(DiagnosticAlert::TokenOverrun { ratio, .. }) = alert {
                prop_assert!(ratio > 1.5, "Ratio should be > 1.5, got {ratio}");
            }
        }

        #[test]
        fn no_anomaly_within_threshold(
            projected in 1000..100000u64,
            usage_percent in 0..150u32,
        ) {
            let mut engine = DiagnosticsEngine::new();
            engine.set_projection(projected, 10);

            let actual = projected * usage_percent as u64 / 100;
            engine.update_token_usage(actual);

            let alert = engine.check_token_anomaly();

            if usage_percent <= 150 {
                // Within 50% overrun threshold
                prop_assert!(alert.is_none(), "Should not alert at {}% usage", usage_percent);
            }
        }
    }

    // =========================================================================
    // Property 9: Budget Threshold Enforcement
    // For any configured token budget, warnings should trigger at 80% usage
    // and pause should trigger at 100% usage.
    // Validates: Requirements 8.1, 8.2, 8.3
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn budget_threshold_enforcement(
            budget in 1000..1000000u64,
            usage_percent in 0..120u32,
        ) {
            let mut controller = BudgetController::new();
            controller.configure(BudgetConfig {
                token_budget: Some(budget),
                ..Default::default()
            });

            let usage = (budget as f64 * usage_percent as f64 / 100.0) as u64;
            controller.record_usage(usage, false);

            let alert = controller.check_budget();
            let actual_percent = usage as f32 / budget as f32 * 100.0;

            if actual_percent >= 100.0 {
                prop_assert!(
                    matches!(alert, Some(BudgetAlert::TokenExceeded { .. })),
                    "Should exceed at {actual_percent:.2}%"
                );
                prop_assert!(controller.should_pause());
            } else if actual_percent >= 80.0 {
                prop_assert!(
                    matches!(alert, Some(BudgetAlert::TokenWarning { .. })),
                    "Should warn at {actual_percent:.2}%"
                );
                prop_assert!(!controller.should_pause());
            } else {
                prop_assert!(alert.is_none(), "Should not alert at {actual_percent:.2}%");
            }
        }
    }

    // =========================================================================
    // Property 10: Agent Concurrency Limit
    // For any parallel agent dispatch, the number of simultaneously running
    // agents should never exceed the configured concurrency limit.
    // Validates: Requirements 9.1
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn agent_concurrency_limit(
            max_concurrent in 1..10usize,
            task_count in 1..20usize,
        ) {
            let mut scheduler = AgentScheduler::new(max_concurrent);

            let tasks: Vec<_> = (0..task_count)
                .map(|i| AgentTask {
                    id: AgentId(i as u64),
                    prompt: format!("Task {i}"),
                    context: None,
                    write_access: false,
                    models: None,
                    dispatch_order: i,
                })
                .collect();

            scheduler.schedule(tasks, AutoTurnAgentsTiming::Parallel);

            // Get all runnable tasks
            let mut running = 0;
            while scheduler.next_runnable().is_some() {
                running += 1;
                prop_assert!(
                    running <= max_concurrent,
                    "Running {} exceeds limit {}", running, max_concurrent
                );
            }

            prop_assert!(
                scheduler.active_count() <= max_concurrent,
                "Active count {} exceeds limit {}", scheduler.active_count(), max_concurrent
            );
        }
    }

    // =========================================================================
    // Property 11: Agent Result Ordering
    // For any set of completed agent results, merging into history should
    // preserve the original dispatch order for blocking agents.
    // Validates: Requirements 9.5
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(30))]

        #[test]
        fn agent_result_ordering_blocking(
            task_count in 2..10usize,
        ) {
            let mut scheduler = AgentScheduler::new(4);

            let tasks: Vec<_> = (0..task_count)
                .map(|i| AgentTask {
                    id: AgentId(i as u64),
                    prompt: format!("Task {i}"),
                    context: None,
                    write_access: false,
                    models: None,
                    dispatch_order: i,
                })
                .collect();

            scheduler.schedule(tasks, AutoTurnAgentsTiming::Blocking);

            // Execute tasks in order (blocking mode)
            for i in 0..task_count {
                if let Some(task) = scheduler.next_runnable() {
                    // Use dispatch_order from the task
                    scheduler.report_completion_with_order(
                        task.id,
                        format!("Result {i}"),
                        Some(task.dispatch_order),
                    );
                }
            }

            let results = scheduler.collect_results();

            // For blocking, results should be in dispatch order
            for (i, result) in results.iter().enumerate() {
                prop_assert_eq!(
                    result.dispatch_order, i,
                    "Result {} has wrong dispatch order", i
                );
            }
        }
    }

    // =========================================================================
    // Property 13: Telemetry Span Coverage
    // For any completed Auto Drive session, there should exist a root span
    // covering the entire session duration with child spans for each turn.
    // Validates: Requirements 7.1, 7.2
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(30))]

        #[test]
        fn telemetry_span_coverage(
            turn_count in 1..20u32,
            tokens_per_turn in 10..1000u64,
        ) {
            let mut collector = TelemetryCollector::new();

            collector.start_session("Test goal", "test-session");

            // Execute turns
            for i in 1..=turn_count {
                let turn = collector.start_turn(i);
                collector.end_turn(turn, TurnOutcome::Success { tokens_used: tokens_per_turn });
            }

            collector.end_session(SessionOutcome::Completed {
                turns: turn_count,
                success: true,
            });

            // Verify span coverage
            prop_assert!(collector.session_span().is_some(), "Session span should exist");
            prop_assert_eq!(
                collector.turn_spans().len(),
                turn_count as usize,
                "Should have {} turn spans", turn_count
            );

            // Each turn span should have session as parent
            let session_id = collector.session_span().unwrap().span_id.clone();
            for turn_span in collector.turn_spans() {
                prop_assert_eq!(
                    turn_span.parent_id.as_ref(),
                    Some(&session_id),
                    "Turn span should have session as parent"
                );
            }

            // Verify metrics
            let metrics = collector.export_metrics();
            prop_assert_eq!(metrics.total_turns, turn_count);
            prop_assert_eq!(metrics.total_tokens, tokens_per_turn * turn_count as u64);
        }
    }

    // =========================================================================
    // Property 8: Compaction Goal Preservation
    // For any history compaction operation, the goal message (first user message)
    // should be preserved in the resulting history.
    // Validates: Requirements 6.2
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn compaction_goal_preservation(
            item_count in 2..20usize,
            target_tokens in 100..1000u64,
        ) {
            let engine = CompactionEngine::with_config(CompactionConfig {
                target_tokens,
                min_tokens: 50,
                keep_recent: 1,
                preserve_errors: false,
                preserve_decisions: false,
                ..Default::default()
            });

            // Create items with goal as first item
            let items: Vec<ItemClassification> = (0..item_count)
                .map(|i| ItemClassification {
                    index: i,
                    importance: if i == 0 { ItemImportance::Critical } else { ItemImportance::Low },
                    tokens: 100,
                    is_goal: i == 0,
                    is_error: false,
                    is_decision: false,
                    summary: Some(format!("Item {i}")),
                })
                .collect();

            let result = engine.compact(&items);

            // Goal must always be preserved
            prop_assert!(result.goal_preserved, "Goal should be preserved");
            prop_assert!(
                result.keep_indices.contains(&0),
                "Goal index (0) should be in keep_indices"
            );
            prop_assert!(
                !result.remove_indices.contains(&0),
                "Goal index (0) should not be in remove_indices"
            );
        }

        #[test]
        fn compaction_respects_min_tokens(
            item_count in 5..20usize,
            min_tokens in 200..500u64,
        ) {
            let engine = CompactionEngine::with_config(CompactionConfig {
                target_tokens: 100, // Very low target
                min_tokens,
                keep_recent: 0,
                preserve_errors: false,
                preserve_decisions: false,
                ..Default::default()
            });

            let items: Vec<ItemClassification> = (0..item_count)
                .map(|i| ItemClassification {
                    index: i,
                    importance: if i == 0 { ItemImportance::Critical } else { ItemImportance::Low },
                    tokens: 100,
                    is_goal: i == 0,
                    is_error: false,
                    is_decision: false,
                    summary: None,
                })
                .collect();

            let result = engine.compact(&items);

            // Should not go below min_tokens
            prop_assert!(
                result.tokens_after >= min_tokens || result.keep_indices.len() <= 2,
                "tokens_after {} should be >= min_tokens {} (or very few items kept)",
                result.tokens_after, min_tokens
            );
        }
    }

    // =========================================================================
    // Property 7: Progress Display Completeness
    // For any running Auto Drive session, the view model should contain
    // non-null values for phase, turns completed, and elapsed time.
    // Validates: Requirements 4.1, 4.2, 4.3, 4.4
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn progress_display_completeness(
            turns in 0..100usize,
            input_tokens in 0..50000u64,
            output_tokens in 0..50000u64,
        ) {
            let mut collector = ProgressCollector::new();

            // Start session
            collector.start();
            collector.set_goal("Test goal");
            collector.set_phase(AutoDrivePhase::Running);

            // Record turns
            for _ in 0..turns {
                collector.record_turn();
            }

            // Update tokens
            collector.update_tokens(input_tokens, output_tokens);

            let model = collector.build_view_model();

            // Verify completeness - all required fields should be populated
            prop_assert!(model.is_complete(), "View model should be complete");
            prop_assert_eq!(model.turns_completed, turns);
            prop_assert!(model.elapsed.as_nanos() > 0 || turns == 0, "Elapsed should be tracked");
            prop_assert_eq!(model.token_metrics.input_tokens, input_tokens);
            prop_assert_eq!(model.token_metrics.output_tokens, output_tokens);
            prop_assert_eq!(model.token_metrics.total_tokens, input_tokens + output_tokens);
            prop_assert!(model.goal.is_some(), "Goal should be set");
            prop_assert!(model.is_active, "Session should be active");
        }

        #[test]
        fn progress_phase_transitions(
            phase_index in 0..10usize,
        ) {
            let phases = [
                AutoDrivePhase::AwaitingGoal,
                AutoDrivePhase::Initializing,
                AutoDrivePhase::Running,
                AutoDrivePhase::AwaitingConfirmation,
                AutoDrivePhase::PausedBudget,
                AutoDrivePhase::PausedDiagnostic,
                AutoDrivePhase::AwaitingIntervention,
                AutoDrivePhase::Checkpointing,
                AutoDrivePhase::Completed,
                AutoDrivePhase::Stopped,
            ];

            let phase = phases[phase_index % phases.len()];
            let mut model = ProgressViewModel::new();
            model.set_phase(phase);

            // Verify phase is set correctly
            prop_assert_eq!(model.phase, phase);

            // Verify is_active is correct based on phase
            let expected_active = matches!(
                model.phase,
                AutoDrivePhase::Running
                    | AutoDrivePhase::Initializing
                    | AutoDrivePhase::AwaitingConfirmation
                    | AutoDrivePhase::Checkpointing
                    | AutoDrivePhase::Recovering
            );
            prop_assert_eq!(model.is_active, expected_active);

            // Status string should never be empty
            prop_assert!(!model.status_string().is_empty());
        }
    }

    // =========================================================================
    // Property 3: Retry Delay Classification
    // For any error type, the retry delay should follow the classification rules:
    // rate limit errors use longer base delay than network errors.
    // Validates: Requirements 2.1, 2.2
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn retry_delay_classification(
            retry_after_secs in 1..300u64,
        ) {
            let rate_limit = AutoDriveError::rate_limit(Some(std::time::Duration::from_secs(retry_after_secs)));
            let network = AutoDriveError::network("connection error");

            let rate_strategy = RetryStrategy::for_error(&rate_limit);
            let network_strategy = RetryStrategy::for_error(&network);

            // Rate limit should use the provided retry_after as base delay
            prop_assert_eq!(
                rate_strategy.base_delay,
                std::time::Duration::from_secs(retry_after_secs),
                "Rate limit should use retry_after as base delay"
            );

            // Rate limit base delay should be >= network base delay when retry_after >= 5
            if retry_after_secs >= 5 {
                prop_assert!(
                    rate_strategy.base_delay >= network_strategy.base_delay,
                    "Rate limit delay {} should be >= network delay {}",
                    rate_strategy.base_delay.as_secs(),
                    network_strategy.base_delay.as_secs()
                );
            }
        }
    }

    // =========================================================================
    // Property 4: Failure Counter Reset
    // For any sequence of failures followed by a successful recovery,
    // the failure counter should reset to zero after the successful operation.
    // Validates: Requirements 2.5
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn failure_counter_reset(
            failure_count in 1..20u32,
        ) {
            let mut counter = FailureCounter::new();

            // Record failures
            for _ in 0..failure_count {
                counter.record_failure(AutoDriveError::network("test error"));
            }

            prop_assert_eq!(counter.count(), failure_count);

            // Record success
            counter.record_success();

            // Counter should be reset
            prop_assert_eq!(counter.count(), 0, "Counter should reset to 0 after success");
            prop_assert!(counter.last_error().is_none(), "Last error should be cleared");
        }
    }
}
