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
    pub git_sha: Option<String>,
    pub git_dirty: Option<bool>,
    pub generated_at: String,
    pub tiers: Vec<String>,
    pub allow_blocked: bool,
    pub overall: Status,
    pub exit_ok: bool,
    pub checks: &'a [Check],
}

pub struct Meta {
    pub git_sha: Option<String>,
    pub git_dirty: Option<bool>,
    pub unix_secs: u64,
}

/// Formats Unix seconds as `YYYY-MM-DDTHH:MM:SSZ` (civil-from-days, proleptic Gregorian).
pub fn rfc3339_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

pub fn build<'a>(
    meta: Meta,
    tiers: &[String],
    allow_blocked: bool,
    checks: &'a [Check],
) -> Report<'a> {
    Report {
        git_sha: meta.git_sha,
        git_dirty: meta.git_dirty,
        generated_at: rfc3339_utc(meta.unix_secs),
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

    #[test]
    fn report_carries_sha_dirty_flag_and_timestamp() {
        let meta = Meta {
            git_sha: Some("abc123".into()),
            git_dirty: Some(true),
            unix_secs: 1_700_000_000,
        };
        let rep = build(meta, &["offline".to_string()], false, &[]);
        let json = serde_json::to_value(&rep).unwrap();
        assert_eq!(json["git_sha"], "abc123");
        assert_eq!(json["git_dirty"], true);
        assert_eq!(json["generated_at"], "2023-11-14T22:13:20Z");
    }

    #[test]
    fn rfc3339_formats_epoch_and_leap_day() {
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(1_709_164_800), "2024-02-29T00:00:00Z");
    }

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
