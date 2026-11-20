//! App update notice, pure part (ops.md §4, A-28; D-24 P4-1): response parsing, version comparison,
//! the schedule rule and the persisted state. No I/O; the transport is in the adapters.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// The tag becomes part of this link, so it is validated first (§4.3).
pub const RELEASE_PAGE_PREFIX: &str = "https://github.com/hamiltonia/wow-sync/releases/tag/";
/// Response bound (technical cap, contracts.md §8).
pub const MAX_RESPONSE_BYTES: usize = 256 * 1024;
/// Q-87: at most one automatic check per 24 hours, failures included.
pub const CHECK_INTERVAL_MS: i64 = 24 * 60 * 60 * 1000;
/// The app must have been ready this long before an automatic check, so it never competes
/// with the ten-second sync start.
pub const READY_DELAY_MS: i64 = 60_000;
/// A manual check repeats at most once a minute.
pub const MANUAL_DEBOUNCE_MS: i64 = 60_000;
/// `update_check.json` limit (technical cap).
pub const MAX_STATE_BYTES: usize = 4096;

pub const COULD_NOT_CHECK: &str = "Could not check for updates.";
pub const CHECKING: &str = "Checking for updates…";
pub const NEWEST: &str = "You have the newest version.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SemVer {
    /// Strict `N.N.N` (1 to 9 digits each), as in `CARGO_PKG_VERSION` for a release build.
    pub fn parse(s: &str) -> Option<SemVer> {
        let mut parts = s.split('.');
        let mut next = || {
            let p = parts.next()?;
            if p.is_empty() || p.len() > 9 || !p.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            p.parse::<u32>().ok()
        };
        let v = SemVer {
            major: next()?,
            minor: next()?,
            patch: next()?,
        };
        parts.next().is_none().then_some(v)
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A tag that matches `^v?[0-9]{1,9}\.[0-9]{1,9}\.[0-9]{1,9}$`, kept exactly as written so the
/// release page link points at the release's own tag.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedTag(String);

impl ValidatedTag {
    pub fn parse(s: &str) -> Option<ValidatedTag> {
        let digits = s.strip_prefix('v').unwrap_or(s);
        SemVer::parse(digits)?;
        Some(ValidatedTag(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn version(&self) -> SemVer {
        SemVer::parse(self.0.strip_prefix('v').unwrap_or(&self.0)).expect("validated at parse")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub tag: ValidatedTag,
    pub version: SemVer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateParseError {
    TooLarge,
    NotJson,
    MissingTag,
    Draft,
    Prerelease,
    BadTag,
}

#[derive(Deserialize)]
struct RawRelease {
    tag_name: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// Reads only `tag_name`, `draft` and `prerelease`; every other field (name, body, html_url, …)
/// is ignored.
pub fn parse_latest(bytes: &[u8]) -> Result<ReleaseInfo, UpdateParseError> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(UpdateParseError::TooLarge);
    }
    let raw: RawRelease = serde_json::from_slice(bytes).map_err(|_| UpdateParseError::NotJson)?;
    if raw.draft {
        return Err(UpdateParseError::Draft);
    }
    if raw.prerelease {
        return Err(UpdateParseError::Prerelease);
    }
    let tag = raw.tag_name.ok_or(UpdateParseError::MissingTag)?;
    let tag = ValidatedTag::parse(&tag).ok_or(UpdateParseError::BadTag)?;
    let version = tag.version();
    Ok(ReleaseInfo { tag, version })
}

/// Numeric comparison of the running version with the latest release.
pub fn compare(current: SemVer, latest: SemVer) -> Ordering {
    current.cmp(&latest)
}

pub fn release_page_url(tag: &ValidatedTag) -> String {
    format!("{RELEASE_PAGE_PREFIX}{}", tag.as_str())
}

/// Whether an automatic check should run now (§4.2). `ready_for_ms` is how long the app has
/// been ready. A last attempt more than one interval in the future means the clock moved.
pub fn due(
    enabled: bool,
    ready_for_ms: i64,
    last_attempt_unix_ms: Option<i64>,
    now_unix_ms: i64,
    interval_ms: i64,
) -> bool {
    if !enabled || ready_for_ms < READY_DELAY_MS {
        return false;
    }
    match last_attempt_unix_ms {
        None => true,
        Some(t) => t.saturating_sub(now_unix_ms) > interval_ms || now_unix_ms - t >= interval_ms,
    }
}

/// Why a check failed. The kind only is ever recorded or shown (§4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateFailKind {
    Offline,
    Timeout,
    Tls,
    Redirect,
    Http,
    TooLarge,
    BadResponse,
    Other,
}

impl UpdateFailKind {
    pub fn label(self) -> &'static str {
        match self {
            UpdateFailKind::Offline => "offline",
            UpdateFailKind::Timeout => "timed out",
            UpdateFailKind::Tls => "secure connection failed",
            UpdateFailKind::Redirect => "unexpected redirect",
            UpdateFailKind::Http => "server refused",
            UpdateFailKind::TooLarge => "response too large",
            UpdateFailKind::BadResponse => "unreadable response",
            UpdateFailKind::Other => "unknown error",
        }
    }

    /// The one Activity line a failure produces.
    pub fn activity_line(self) -> String {
        format!("Update check failed: {}", self.label())
    }
}

impl From<UpdateParseError> for UpdateFailKind {
    fn from(e: UpdateParseError) -> Self {
        match e {
            UpdateParseError::TooLarge => UpdateFailKind::TooLarge,
            _ => UpdateFailKind::BadResponse,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateResult {
    Newest { checked: i64 },
    Available { tag: String, checked: i64 },
    Failed { at: i64, kind: UpdateFailKind },
}

impl UpdateResult {
    /// The About text for this result (§4.3, screens.md "About and updates").
    pub fn text(&self) -> String {
        match self {
            UpdateResult::Newest { .. } => NEWEST.to_string(),
            UpdateResult::Available { tag, .. } => {
                format!(
                    "Version {} is available",
                    tag.strip_prefix('v').unwrap_or(tag)
                )
            }
            UpdateResult::Failed { .. } => COULD_NOT_CHECK.to_string(),
        }
    }
}

/// `update_check.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCheckState {
    pub last_attempt_unix_ms: Option<i64>,
    pub last_result: Option<UpdateResult>,
    pub notified_tag: Option<String>,
}

impl UpdateCheckState {
    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// `None` for anything unreadable, oversize, or holding a tag that is not a valid tag; callers
    /// start from the default (an attempt is then simply due).
    pub fn decode(bytes: &[u8]) -> Option<UpdateCheckState> {
        if bytes.len() > MAX_STATE_BYTES {
            return None;
        }
        let s: UpdateCheckState = serde_json::from_slice(bytes).ok()?;
        let tag_ok = |t: &str| ValidatedTag::parse(t).is_some();
        if let Some(UpdateResult::Available { tag, .. }) = &s.last_result {
            if !tag_ok(tag) {
                return None;
            }
        }
        if s.notified_tag.as_deref().is_some_and(|t| !tag_ok(t)) {
            return None;
        }
        Some(s)
    }

    /// Outcome of a fetch: records the result and, for a newer release, `Available`.
    pub fn with_result(
        &self,
        fetched: Result<&[u8], UpdateFailKind>,
        current: SemVer,
        now_unix_ms: i64,
    ) -> UpdateCheckState {
        let result = match fetched {
            Err(kind) => UpdateResult::Failed {
                at: now_unix_ms,
                kind,
            },
            Ok(bytes) => match parse_latest(bytes) {
                Err(e) => UpdateResult::Failed {
                    at: now_unix_ms,
                    kind: e.into(),
                },
                Ok(info) if compare(current, info.version) == Ordering::Less => {
                    UpdateResult::Available {
                        tag: info.tag.as_str().to_string(),
                        checked: now_unix_ms,
                    }
                }
                Ok(_) => UpdateResult::Newest {
                    checked: now_unix_ms,
                },
            },
        };
        UpdateCheckState {
            last_result: Some(result),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(tag: &str, draft: bool, pre: bool) -> Vec<u8> {
        format!(
            r#"{{"tag_name":"{tag}","draft":{draft},"prerelease":{pre},"name":"x","body":"hi","html_url":"https://github.com/hamiltonia/wow-sync/releases/tag/v9.9.9"}}"#
        )
        .into_bytes()
    }

    #[test]
    fn parse_latest_accepts_both_tag_forms_and_rejects_everything_else() {
        let v = parse_latest(&json("v0.2.0", false, false)).unwrap();
        assert_eq!(v.version, SemVer::parse("0.2.0").unwrap());
        assert_eq!(v.tag.as_str(), "v0.2.0");
        assert_eq!(
            parse_latest(&json("0.2.0", false, false))
                .unwrap()
                .tag
                .as_str(),
            "0.2.0"
        );
        assert_eq!(
            parse_latest(&json("v0.2.0", true, false)),
            Err(UpdateParseError::Draft)
        );
        assert_eq!(
            parse_latest(&json("v0.2.0", false, true)),
            Err(UpdateParseError::Prerelease)
        );
        for bad in [
            "0.2",
            "v0.2.0-rc1",
            "V0.2.0",
            "1234567890.0.0",
            "v0.2.0.1",
            "v0..1",
            "",
            " v0.2.0",
            "v0.2.0\\n",
            "v0.2.0/../x",
            "v-1.2.3",
        ] {
            assert_eq!(
                parse_latest(&json(bad, false, false)),
                Err(UpdateParseError::BadTag),
                "{bad:?}"
            );
        }
        assert_eq!(parse_latest(b"{}"), Err(UpdateParseError::MissingTag));
        assert_eq!(parse_latest(b"not json"), Err(UpdateParseError::NotJson));
        assert_eq!(
            parse_latest(br#"{"tag_name":5}"#),
            Err(UpdateParseError::NotJson)
        );
        let mut big = json("v0.2.0", false, false);
        big.extend(std::iter::repeat(b' ').take(MAX_RESPONSE_BYTES));
        assert_eq!(parse_latest(&big), Err(UpdateParseError::TooLarge));
    }

    #[test]
    fn the_release_link_is_built_from_the_exact_tag_only() {
        let with_v = parse_latest(&json("v0.2.0", false, false)).unwrap();
        assert_eq!(
            release_page_url(&with_v.tag),
            "https://github.com/hamiltonia/wow-sync/releases/tag/v0.2.0"
        );
        let plain = parse_latest(&json("0.2.0", false, false)).unwrap();
        assert_eq!(
            release_page_url(&plain.tag),
            "https://github.com/hamiltonia/wow-sync/releases/tag/0.2.0"
        );
        assert!(!release_page_url(&plain.tag).contains("evil"));
    }

    #[test]
    fn comparison_is_numeric_per_part() {
        let v = |s| SemVer::parse(s).unwrap();
        assert_eq!(compare(v("0.2.0"), v("0.10.0")), Ordering::Less);
        assert_eq!(compare(v("1.0.0"), v("0.99.99")), Ordering::Greater);
        assert_eq!(compare(v("0.2.0"), v("0.2.0")), Ordering::Equal);
        assert!(SemVer::parse("0.2.0-dev").is_none());
    }

    #[test]
    fn due_follows_setting_interval_ready_delay_and_clock_changes() {
        let day = CHECK_INTERVAL_MS;
        let now = 10 * day;
        assert!(
            !due(false, 10 * READY_DELAY_MS, None, now, day),
            "off: never"
        );
        assert!(due(true, READY_DELAY_MS, None, now, day), "no attempt yet");
        assert!(
            !due(true, READY_DELAY_MS - 1, None, now, day),
            "not ready for 60 s"
        );
        assert!(!due(true, READY_DELAY_MS, Some(now - day + 1), now, day));
        assert!(due(true, READY_DELAY_MS, Some(now - day), now, day));
        assert!(
            !due(true, READY_DELAY_MS, Some(now + day), now, day),
            "a little in the future waits"
        );
        assert!(
            due(true, READY_DELAY_MS, Some(now + day + 1), now, day),
            "far in the future means the clock changed"
        );
    }

    #[test]
    fn state_round_trips_and_rejects_bad_content() {
        let s = UpdateCheckState {
            last_attempt_unix_ms: Some(5),
            last_result: Some(UpdateResult::Available {
                tag: "v0.2.0".into(),
                checked: 5,
            }),
            notified_tag: Some("v0.2.0".into()),
        };
        assert_eq!(UpdateCheckState::decode(&s.encode()), Some(s));
        let bad = br#"{"last_attempt_unix_ms":1,"last_result":{"Available":{"tag":"x/../y","checked":1}},"notified_tag":null}"#;
        assert_eq!(UpdateCheckState::decode(bad), None);
        assert_eq!(
            UpdateCheckState::decode(&vec![b' '; MAX_STATE_BYTES + 1]),
            None
        );
        assert_eq!(UpdateCheckState::decode(b"nope"), None);
    }

    #[test]
    fn results_map_to_state_and_text() {
        let cur = SemVer::parse("0.1.0").unwrap();
        let base = UpdateCheckState::default();
        let newer = base.with_result(Ok(&json("v0.2.0", false, false)), cur, 9);
        assert_eq!(
            newer.last_result,
            Some(UpdateResult::Available {
                tag: "v0.2.0".into(),
                checked: 9
            })
        );
        assert_eq!(
            newer.last_result.as_ref().unwrap().text(),
            "Version 0.2.0 is available"
        );
        let same = base.with_result(Ok(&json("0.1.0", false, false)), cur, 9);
        assert_eq!(same.last_result, Some(UpdateResult::Newest { checked: 9 }));
        let older = base.with_result(Ok(&json("0.0.9", false, false)), cur, 9);
        assert_eq!(older.last_result, Some(UpdateResult::Newest { checked: 9 }));
        let failed = base.with_result(Err(UpdateFailKind::Offline), cur, 9);
        assert_eq!(failed.last_result.as_ref().unwrap().text(), COULD_NOT_CHECK);
        assert_eq!(
            UpdateFailKind::Offline.activity_line(),
            "Update check failed: offline"
        );
        let garbage = base.with_result(Ok(b"<html>"), cur, 9);
        assert_eq!(
            garbage.last_result,
            Some(UpdateResult::Failed {
                at: 9,
                kind: UpdateFailKind::BadResponse
            })
        );
    }
}
