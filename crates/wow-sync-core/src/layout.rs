//! Provider and local-state path layout (publication.md §1). All paths are '/'-separated
//! and relative to `<shared>/WoWSync` or `<variant>/.wowsync`.

use crate::ids::{ComputerId, EnrollmentId, GroupId, OpId, Sha256};
use crate::paths::FolderKey;

pub fn group_dir(g: &GroupId) -> String {
    format!("groups/{g}")
}

pub fn group_record(g: &GroupId) -> String {
    format!("groups/{g}/group.json")
}

pub fn members_dir(g: &GroupId) -> String {
    format!("groups/{g}/members")
}

pub fn member_dir(g: &GroupId, c: &ComputerId) -> String {
    format!("groups/{g}/members/{c}")
}

pub fn member_record(g: &GroupId, c: &ComputerId) -> String {
    format!("groups/{g}/members/{c}/member.json")
}

pub fn state_dir(g: &GroupId, c: &ComputerId) -> String {
    format!("groups/{g}/members/{c}/state")
}

pub fn state_file_name(e: &EnrollmentId, seq: u64) -> String {
    format!("{e}-{seq:010}.json")
}

pub fn state_record(g: &GroupId, c: &ComputerId, e: &EnrollmentId, seq: u64) -> String {
    format!("{}/{}", state_dir(g, c), state_file_name(e, seq))
}

/// `<enrollment>-<seq:010>.json` only; temp and conflicted copies do not parse.
pub fn parse_state_name(file_name: &str) -> Option<(EnrollmentId, u64)> {
    let stem = file_name.strip_suffix(".json")?;
    let (e, seq) = stem.split_once('-')?;
    if seq.len() != 10 || !seq.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((EnrollmentId::parse(e).ok()?, seq.parse().ok()?))
}

pub fn tree_record(g: &GroupId, h: &Sha256) -> String {
    format!("groups/{g}/trees/{}/{h}.json", h.prefix())
}

pub fn blob(g: &GroupId, h: &Sha256) -> String {
    format!("groups/{g}/blobs/{}/{h}", h.prefix())
}

pub fn staging_dir(op: &OpId) -> String {
    format!("staging/{op}")
}

pub fn recovery_dir(pkg: &FolderKey, op: &OpId) -> String {
    format!("recovery/{pkg}/{op}")
}

pub fn journal(op: &OpId) -> String {
    format!("journals/{op}.json")
}

/// Authority records of one member: own records and verbatim mirrors (contracts.md §1.2).
pub fn authority_dir(g: &GroupId, c: &ComputerId) -> String {
    format!("groups/{g}/members/{c}/authority")
}

pub fn authority_record(g: &GroupId, c: &ComputerId, h: &Sha256) -> String {
    format!("{}/{h}.json", authority_dir(g, c))
}

pub fn authority_index_dir(g: &GroupId, c: &ComputerId) -> String {
    format!("groups/{g}/members/{c}/authority-index")
}

pub fn authority_index_chunk(g: &GroupId, c: &ComputerId, h: &Sha256) -> String {
    format!("{}/{h}.json", authority_index_dir(g, c))
}

/// Settings state stream of one member (settings.md §3.4).
pub fn settings_state_dir(g: &GroupId, c: &ComputerId) -> String {
    format!("groups/{g}/members/{c}/settings")
}

/// Local ledger: append-only segment files under the app data directory (contracts.md §1.7).
pub fn ledger_segment(index: u32) -> String {
    format!("authority/ledger/{index:08}.json")
}

pub fn ledger_dir() -> &'static str {
    "authority/ledger"
}

/// Verbatim record bytes of the local ledger.
pub fn ledger_object(h: &Sha256) -> String {
    format!("authority/objects/{h}.json")
}

/// Local "Continue without it" choices; never published.
pub fn ledger_dismissed() -> &'static str {
    "authority/dismissed.json"
}

/// Historical membership set (contracts.md §1.10), local and grow-only.
pub fn membership_file() -> &'static str {
    "authority/membership.json"
}

/// Relative to `<variant>/.wowsync` (settings.md §3.1, §3.5).
pub fn settings_local() -> &'static str {
    "settings/local.json"
}

pub fn settings_blob(h: &Sha256) -> String {
    format!("settings/blobs/{h}")
}

pub fn settings_manifest(h: &Sha256) -> String {
    format!("settings/manifests/{h}.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit;

    #[test]
    fn state_names_round_trip_and_reject_others() {
        let (g, c, e) = (
            testkit::group_id(),
            testkit::computer(1),
            testkit::enrollment(1),
        );
        let rel = state_record(&g, &c, &e, 12);
        let name = rel.rsplit('/').next().unwrap();
        assert_eq!(parse_state_name(name), Some((e.clone(), 12)));
        assert!(rel.starts_with(&format!("groups/{g}/members/{c}/state/")));
        for bad in [
            format!("{e}-12.json"),
            format!("{e}-0000000012.json.tmp"),
            format!("{e}-0000000012 (conflicted copy).json"),
            "x-0000000012.json".to_string(),
        ] {
            assert_eq!(parse_state_name(&bad), None, "{bad}");
        }
    }
}
