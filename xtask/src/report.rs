use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Pass,
    Fail,
    Blocked,
    Skipped,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Pass => "PASS",
            Status::Fail => "FAIL",
            Status::Blocked => "BLOCKED",
            Status::Skipped => "SKIPPED",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub tier: String,
    pub name: String,
    pub status: Status,
    pub detail: String,
}

impl Check {
    pub fn new(tier: &str, name: &str, status: Status, detail: impl Into<String>) -> Self {
        Check {
            tier: tier.to_string(),
            name: name.to_string(),
            status,
            detail: detail.into(),
        }
    }
}

/// Overall status: any FAIL wins, then BLOCKED, then SKIPPED; no checks is not a pass.
pub fn overall(checks: &[Check]) -> Status {
    if checks.is_empty() {
        return Status::Fail;
    }
    let has = |s: Status| checks.iter().any(|c| c.status == s);
    if has(Status::Fail) {
        Status::Fail
    } else if has(Status::Blocked) {
        Status::Blocked
    } else if has(Status::Skipped) {
        Status::Skipped
    } else {
        Status::Pass
    }
}

pub fn exit_ok(checks: &[Check], allow_blocked: bool) -> bool {
    match overall(checks) {
        Status::Pass => true,
        Status::Blocked | Status::Skipped => allow_blocked,
        Status::Fail => false,
    }
}

#[derive(Serialize)]
pub struct Report<'a> {
    pub tiers: Vec<String>,
    pub allow_blocked: bool,
    pub overall: Status,
    pub exit_ok: bool,
    pub checks: &'a [Check],
}

pub fn build<'a>(tiers: &[String], allow_blocked: bool, checks: &'a [Check]) -> Report<'a> {
    Report {
        tiers: tiers.to_vec(),
        allow_blocked,
        overall: overall(checks),
        exit_ok: exit_ok(checks, allow_blocked),
        checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: Status) -> Check {
        Check::new("offline", "x", s, "")
    }

    #[test]
    fn all_pass_is_pass_and_ok() {
        let checks = [c(Status::Pass), c(Status::Pass)];
        assert_eq!(overall(&checks), Status::Pass);
        assert!(exit_ok(&checks, false));
    }

    #[test]
    fn fail_dominates_and_allow_blocked_does_not_rescue_it() {
        let checks = [c(Status::Pass), c(Status::Blocked), c(Status::Fail)];
        assert_eq!(overall(&checks), Status::Fail);
        assert!(!exit_ok(&checks, true));
    }

    #[test]
    fn blocked_or_skipped_fails_unless_allowed() {
        for s in [Status::Blocked, Status::Skipped] {
            let checks = [c(Status::Pass), c(s)];
            assert_eq!(overall(&checks), s);
            assert!(!exit_ok(&checks, false));
            assert!(exit_ok(&checks, true));
        }
    }

    #[test]
    fn empty_check_list_is_not_a_pass() {
        assert_eq!(overall(&[]), Status::Fail);
        assert!(!exit_ok(&[], true));
    }
}
