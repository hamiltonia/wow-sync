//! Retirement command flow (ops.md §2). The barrier itself is in `authority.rs`.
//! Step-1 stub, handed to R4-core-ops.

use crate::decision::{CycleCx, RetirePlan};
use crate::hooks::{Neutral, RetireHooks};
use crate::kinds::EnrollmentRef;

#[derive(Debug, Clone, Copy, Default)]
pub struct RetireBehaviour;

impl RetireHooks for RetireBehaviour {
    fn plan_retirement(&self, cx: &CycleCx, subject: &EnrollmentRef) -> RetirePlan {
        Neutral.plan_retirement(cx, subject)
    }
}
