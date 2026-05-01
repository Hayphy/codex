use codex_hooks::PreToolUsePermissionDecision;
/// Whether an approval boundary would normally need to ask anyone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ApprovalRequirement {
    Skip,
    NeedsApproval,
}

/// The reviewer that should receive the approval request, if any.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ApprovalRoute {
    Skip,
    PromptUser {
        reason: Option<String>,
        cache_policy: ApprovalCachePolicy,
    },
    RouteToGuardian,
}

/// Whether a user prompt may reuse a prior session approval.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ApprovalCachePolicy {
    UseCachedApprovals,
    BypassCachedApprovals,
}

/// Resolves how a concrete approval boundary should handle hook-authored
/// `allow` / `ask` decisions.
///
/// `ask` is intentionally a human-user request, not a generic reviewer request:
/// it bypasses both normal auto review and strict auto review so a hook can
/// require explicit user confirmation. Strict auto review still outranks
/// `allow`, because that mode is a user-selected promise to review every later
/// command in the turn.
pub(crate) fn resolve_approval_route(
    requirement: ApprovalRequirement,
    pre_tool_use: Option<&PreToolUsePermissionDecision>,
    route_to_guardian: bool,
    strict_auto_review: bool,
) -> ApprovalRoute {
    match pre_tool_use {
        Some(PreToolUsePermissionDecision::Ask { reason }) => {
            return ApprovalRoute::PromptUser {
                reason: reason.clone(),
                cache_policy: ApprovalCachePolicy::BypassCachedApprovals,
            };
        }
        Some(PreToolUsePermissionDecision::Allow { .. }) if !strict_auto_review => {
            return ApprovalRoute::Skip;
        }
        Some(PreToolUsePermissionDecision::Allow { .. }) | None => {}
    }

    if strict_auto_review {
        ApprovalRoute::RouteToGuardian
    } else {
        match requirement {
            ApprovalRequirement::Skip => ApprovalRoute::Skip,
            ApprovalRequirement::NeedsApproval if route_to_guardian => {
                ApprovalRoute::RouteToGuardian
            }
            ApprovalRequirement::NeedsApproval => ApprovalRoute::PromptUser {
                reason: None,
                cache_policy: ApprovalCachePolicy::UseCachedApprovals,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn ask_routes_to_the_user_even_when_auto_review_is_enabled() {
        assert_eq!(
            resolve_approval_route(
                ApprovalRequirement::NeedsApproval,
                Some(&PreToolUsePermissionDecision::Ask {
                    reason: Some("show this to the user".to_string()),
                }),
                /*route_to_guardian*/ true,
                /*strict_auto_review*/ false,
            ),
            ApprovalRoute::PromptUser {
                reason: Some("show this to the user".to_string()),
                cache_policy: ApprovalCachePolicy::BypassCachedApprovals,
            }
        );
    }

    #[test]
    fn ask_routes_to_the_user_even_during_strict_auto_review() {
        assert_eq!(
            resolve_approval_route(
                ApprovalRequirement::Skip,
                Some(&PreToolUsePermissionDecision::Ask {
                    reason: Some("show this to the user".to_string()),
                }),
                /*route_to_guardian*/ true,
                /*strict_auto_review*/ true,
            ),
            ApprovalRoute::PromptUser {
                reason: Some("show this to the user".to_string()),
                cache_policy: ApprovalCachePolicy::BypassCachedApprovals,
            }
        );
    }

    #[test]
    fn allow_skips_normal_auto_review() {
        assert_eq!(
            resolve_approval_route(
                ApprovalRequirement::NeedsApproval,
                Some(&PreToolUsePermissionDecision::Allow { reason: None }),
                /*route_to_guardian*/ true,
                /*strict_auto_review*/ false,
            ),
            ApprovalRoute::Skip,
        );
    }

    #[test]
    fn strict_auto_review_still_reviews_allow() {
        assert_eq!(
            resolve_approval_route(
                ApprovalRequirement::Skip,
                Some(&PreToolUsePermissionDecision::Allow { reason: None }),
                /*route_to_guardian*/ false,
                /*strict_auto_review*/ true,
            ),
            ApprovalRoute::RouteToGuardian,
        );
    }
}
