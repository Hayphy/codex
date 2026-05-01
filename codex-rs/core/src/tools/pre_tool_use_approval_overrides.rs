use codex_hooks::PreToolUsePermissionDecision;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

/// Turn-local approval overrides emitted by `PreToolUse` hooks.
///
/// Decisions are keyed by the originating tool-use id so later approval
/// boundaries can consult the same hook-authored guidance without threading it
/// through every intermediate runtime structure.
#[derive(Clone, Debug, Default)]
pub(crate) struct PreToolUseApprovalOverrides {
    inner: Arc<Mutex<HashMap<String, PreToolUsePermissionDecision>>>,
}

impl PreToolUseApprovalOverrides {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, PreToolUsePermissionDecision>> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub(crate) fn record(&self, tool_use_id: String, decision: PreToolUsePermissionDecision) {
        self.lock().insert(tool_use_id, decision);
    }

    pub(crate) fn get(&self, tool_use_id: &str) -> Option<PreToolUsePermissionDecision> {
        self.lock().get(tool_use_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn cloned_handles_share_turn_local_overrides() {
        let overrides = PreToolUseApprovalOverrides::default();
        let cloned = overrides.clone();
        let decision = PreToolUsePermissionDecision::Ask {
            reason: Some("confirm this".to_string()),
        };

        overrides.record("call-1".to_string(), decision.clone());

        assert_eq!(cloned.get("call-1"), Some(decision));
    }
}
