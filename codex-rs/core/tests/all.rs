// Single integration test binary that aggregates all test modules.
// The submodules live in `tests/all/`.

#[ctor::ctor]
fn clear_originator_override_for_tests() {
    // SAFETY: Tests must be hermetic and not depend on host environment overrides.
    unsafe {
        std::env::remove_var(
            codex_core::default_client::CODEX_INTERNAL_ORIGINATOR_OVERRIDE_ENV_VAR,
        );
    }
}

mod suite;
