//! Decision routing and the kept-record queue (ops.md §1, T-R4D31-held-self, Q-60).
//!
//! Pure routing: core `decide` yields an effect, this module decides whether it is written now,
//! kept unwritten, or refused. A running game never holds a group record (T-26-7): only
//! game-file changes wait for it, so the routing takes no activity input. The service wires this
//! in step 3.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::Path;
use wow_sync_adapters::authority_store;
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::provider::ProviderRoot;
use wow_sync_adapters::{FsError, FsErrorKind};
use wow_sync_core::authority::AuthorityRecord;
use wow_sync_core::decide::encode_authority;
use wow_sync_core::decision::{DecideError, DecisionEffect, LocalResolution};
use wow_sync_core::hash::sha256;
use wow_sync_core::ids::{ComputerId, GroupId, Sha256};
use wow_sync_core::layout;
use wow_sync_core::retire::OwnStanding;

const AUTHORITY_LIMIT: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum KeepWhy {
    /// This computer's own retirement is unresolved: nothing is published yet.
    Held,
    Paused,
    FolderNotWritable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Routed {
    Refused(DecideError),
    WriteNow(Box<AuthorityRecord>),
    Local(LocalResolution),
    /// Valid, not lost: reserved and written when publishing resumes.
    Keep(Box<AuthorityRecord>, KeepWhy),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RouteCx {
    pub standing: OwnStanding,
    pub paused: bool,
    pub folder_writable: bool,
}

pub(crate) fn route(effect: DecisionEffect, cx: &RouteCx) -> Routed {
    let rec = match effect {
        DecisionEffect::Local(l) => return Routed::Local(l),
        DecisionEffect::Write(r) => r,
    };
    match cx.standing {
        OwnStanding::Retired => Routed::Refused(DecideError::NotAllowed),
        OwnStanding::Held => Routed::Keep(rec, KeepWhy::Held),
        OwnStanding::Active if cx.paused => Routed::Keep(rec, KeepWhy::Paused),
        OwnStanding::Active if !cx.folder_writable => Routed::Keep(rec, KeepWhy::FolderNotWritable),
        OwnStanding::Active => Routed::WriteNow(rec),
    }
}

/// A record reserved while publishing is blocked: the exact bytes and sequence it will use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeptRecord {
    pub hash: Sha256,
    pub bytes: Vec<u8>,
    pub reserved_seq: u64,
    pub why: KeepWhy,
}

impl KeptRecord {
    pub fn reserve(rec: &AuthorityRecord, reserved_seq: u64, why: KeepWhy) -> Self {
        let bytes = encode_authority(rec);
        Self {
            hash: sha256(&bytes),
            bytes,
            reserved_seq,
            why,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Settled {
    pub to_write: Vec<KeptRecord>,
    /// Dropped because this computer was retired; the caller records an Activity entry.
    pub discarded: Vec<KeptRecord>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct KeptQueue {
    items: Vec<KeptRecord>,
}

impl KeptQueue {
    pub fn push(&mut self, r: KeptRecord) {
        if !self.items.iter().any(|x| x.hash == r.hash) {
            self.items.push(r);
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn items(&self) -> &[KeptRecord] {
        &self.items
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The owner cancels a kept decision before it is written.
    pub fn cancel(&mut self, hash: &Sha256) -> bool {
        let n = self.items.len();
        self.items.retain(|x| x.hash != *hash);
        self.items.len() != n
    }

    /// Held keeps everything; retired discards everything; an active computer writes what is no
    /// longer blocked by a pause or an unwritable folder.
    pub fn settle(&mut self, standing: OwnStanding, paused: bool, writable: bool) -> Settled {
        match standing {
            OwnStanding::Held => Settled::default(),
            OwnStanding::Retired => Settled {
                to_write: vec![],
                discarded: std::mem::take(&mut self.items),
            },
            OwnStanding::Active if paused || !writable => Settled::default(),
            OwnStanding::Active => Settled {
                to_write: std::mem::take(&mut self.items),
                discarded: vec![],
            },
        }
    }
}

const KEPT_LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct KeptWire {
    hash: Sha256,
    /// The record's exact canonical JSON text.
    record: String,
    reserved_seq: u64,
    why: KeepWhy,
}

#[derive(Default, Serialize, Deserialize)]
struct KeptFile {
    items: Vec<KeptWire>,
}

fn kept_name(group: &GroupId) -> String {
    format!("kept-records-{group}.json")
}

impl KeptQueue {
    /// A missing or damaged file is an empty queue. An item whose bytes do not hash to its name
    /// is dropped, so a damaged queue can never write a record under a name it does not match.
    pub fn load(state: &LocalState, group: &GroupId) -> Self {
        let file: KeptFile = state
            .read(&kept_name(group), KEPT_LIMIT)
            .ok()
            .flatten()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let mut q = Self::default();
        for w in file.items {
            let bytes = w.record.into_bytes();
            if sha256(&bytes) == w.hash {
                q.push(KeptRecord {
                    hash: w.hash,
                    bytes,
                    reserved_seq: w.reserved_seq,
                    why: w.why,
                });
            }
        }
        q
    }

    pub fn save(&self, state: &LocalState, group: &GroupId) -> Result<(), FsError> {
        let file = KeptFile {
            items: self
                .items
                .iter()
                .filter_map(|k| {
                    Some(KeptWire {
                        hash: k.hash,
                        record: String::from_utf8(k.bytes.clone()).ok()?,
                        reserved_seq: k.reserved_seq,
                        why: k.why,
                    })
                })
                .collect(),
        };
        let bytes = serde_json::to_vec(&file).expect("plain data serializes");
        state.write_atomic(&kept_name(group), &bytes)
    }
}

/// Writes a record under this computer's authority directory and verifies it by reading it back.
/// The bytes must hash to the name: a record is never written under a name it does not match.
pub(crate) fn write_record(
    provider: &ProviderRoot,
    group: &GroupId,
    me: &ComputerId,
    r: &KeptRecord,
) -> Result<bool, FsError> {
    let rel = layout::authority_record(group, me, &r.hash);
    if sha256(&r.bytes) != r.hash {
        return Err(FsError::new(
            FsErrorKind::HashMismatch,
            Path::new(&rel),
            "the record does not match its name",
        ));
    }
    authority_store::put_record(provider, &rel, &r.bytes, AUTHORITY_LIMIT, me.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_sync_adapters::testkit::TempDir;
    use wow_sync_core::ids::EnrollmentId;
    use wow_sync_core::kinds::{EnrollmentRef, Issuer, Retirement};

    fn hex(c: char) -> String {
        c.to_string().repeat(32)
    }

    fn rec(cutoff: u64) -> AuthorityRecord {
        let c = ComputerId::parse(&hex('1')).unwrap();
        let e = EnrollmentId::parse(&hex('1')).unwrap();
        AuthorityRecord::Retirement(Retirement {
            group_id: GroupId::parse(&hex('9')).unwrap(),
            issuer: Issuer {
                computer_id: c.clone(),
                enrollment_id: e.clone(),
                issued_at_seq: 9,
            },
            retired: EnrollmentRef {
                computer_id: ComputerId::parse(&hex('2')).unwrap(),
                enrollment_id: EnrollmentId::parse(&hex('2')).unwrap(),
            },
            cutoff_seq: cutoff,
            was_settings_master: false,
            retirements_known: vec![],
            written_unix_ms: 1,
        })
    }

    fn cx(standing: OwnStanding) -> RouteCx {
        RouteCx {
            standing,
            paused: false,
            folder_writable: true,
        }
    }

    #[test]
    fn a_retired_computer_refuses_group_writes_but_not_local_choices() {
        // T-45-7, Q-60 (routing form)
        let w = DecisionEffect::Write(Box::new(rec(3)));
        assert_eq!(
            route(w, &cx(OwnStanding::Retired)),
            Routed::Refused(DecideError::NotAllowed)
        );
        let l = DecisionEffect::Local(LocalResolution::Dismiss(sha256(b"x")));
        assert!(matches!(
            route(l, &cx(OwnStanding::Retired)),
            Routed::Local(_)
        ));
    }

    #[test]
    fn held_paused_or_unwritable_keeps_the_record_and_an_active_one_writes_it() {
        // T-26-6, T-26-7: nothing about a running game appears in the routing.
        let w = || DecisionEffect::Write(Box::new(rec(3)));
        assert!(matches!(
            route(w(), &cx(OwnStanding::Held)),
            Routed::Keep(_, KeepWhy::Held)
        ));
        let paused = RouteCx {
            paused: true,
            ..cx(OwnStanding::Active)
        };
        assert!(matches!(
            route(w(), &paused),
            Routed::Keep(_, KeepWhy::Paused)
        ));
        let ro = RouteCx {
            folder_writable: false,
            ..cx(OwnStanding::Active)
        };
        assert!(matches!(
            route(w(), &ro),
            Routed::Keep(_, KeepWhy::FolderNotWritable)
        ));
        assert!(matches!(
            route(w(), &cx(OwnStanding::Active)),
            Routed::WriteNow(_)
        ));
    }

    #[test]
    fn held_record_is_written_with_reserved_bytes_when_the_retirement_turns_void() {
        // T-R4D31-held-self (a)
        let t = TempDir::new();
        let root = ProviderRoot::open_or_create(t.path()).unwrap();
        let (g, me) = (
            GroupId::parse(&hex('9')).unwrap(),
            ComputerId::parse(&hex('1')).unwrap(),
        );
        let kept = KeptRecord::reserve(&rec(3), 12, KeepWhy::Held);
        let mut q = KeptQueue::default();
        q.push(kept.clone());
        q.push(kept.clone());
        assert_eq!(q.len(), 1, "the same record is kept once");
        assert_eq!(q.settle(OwnStanding::Held, false, true), Settled::default());
        assert_eq!(q.len(), 1, "held keeps it unwritten");
        let s = q.settle(OwnStanding::Active, false, true);
        assert_eq!(s.to_write, vec![kept.clone()]);
        assert!(s.discarded.is_empty());
        assert!(write_record(&root, &g, &me, &s.to_write[0]).unwrap());
        let rel = layout::authority_record(&g, &me, &kept.hash);
        assert_eq!(root.read(&rel, AUTHORITY_LIMIT).unwrap(), kept.bytes);
        assert_eq!(kept.reserved_seq, 12);
    }

    #[test]
    fn held_record_is_discarded_and_never_written_when_the_retirement_turns_effective() {
        // T-R4D31-held-self (b)
        let t = TempDir::new();
        let root = ProviderRoot::open_or_create(t.path()).unwrap();
        let g = GroupId::parse(&hex('9')).unwrap();
        let kept = KeptRecord::reserve(&rec(4), 13, KeepWhy::Held);
        let mut q = KeptQueue::default();
        q.push(kept.clone());
        let s = q.settle(OwnStanding::Retired, false, true);
        assert!(s.to_write.is_empty());
        assert_eq!(s.discarded, vec![kept.clone()]);
        assert_eq!(q.len(), 0);
        let me = ComputerId::parse(&hex('1')).unwrap();
        let rel = layout::authority_record(&g, &me, &kept.hash);
        assert!(
            root.read(&rel, AUTHORITY_LIMIT).is_err(),
            "nothing was written"
        );
    }

    #[test]
    fn a_pause_keeps_the_queue_and_a_cancel_removes_one_record() {
        let a = KeptRecord::reserve(&rec(5), 1, KeepWhy::Paused);
        let b = KeptRecord::reserve(&rec(6), 2, KeepWhy::Paused);
        let mut q = KeptQueue::default();
        q.push(a.clone());
        q.push(b.clone());
        assert_eq!(
            q.settle(OwnStanding::Active, true, true),
            Settled::default()
        );
        assert!(q.cancel(&a.hash));
        assert!(!q.cancel(&a.hash));
        assert_eq!(q.settle(OwnStanding::Active, false, true).to_write, vec![b]);
    }

    #[test]
    fn a_kept_queue_survives_a_restart_and_a_tampered_item_is_dropped() {
        let t = TempDir::new();
        let state = LocalState::open_at(t.path()).unwrap();
        let g = GroupId::parse(&hex('9')).unwrap();
        let a = KeptRecord::reserve(&rec(5), 7, KeepWhy::Held);
        let b = KeptRecord::reserve(&rec(6), 8, KeepWhy::Paused);
        let mut q = KeptQueue::default();
        q.push(a.clone());
        q.push(b.clone());
        q.save(&state, &g).unwrap();
        let back = KeptQueue::load(&state, &g);
        assert_eq!(back.items(), &[a.clone(), b.clone()]);
        let other = GroupId::parse(&hex('8')).unwrap();
        assert!(KeptQueue::load(&state, &other).is_empty());
        let name = kept_name(&g);
        let text = String::from_utf8(state.read(&name, KEPT_LIMIT).unwrap().unwrap()).unwrap();
        let tampered = text.replacen("cutoff_seq\\\": 5,", "cutoff_seq\\\": 50,", 1);
        assert_ne!(tampered, text, "the fixture must actually change a byte");
        state.write_atomic(&name, tampered.as_bytes()).unwrap();
        assert_eq!(KeptQueue::load(&state, &g).items(), &[b]);
        state.write_atomic(&name, b"{ not json").unwrap();
        assert!(KeptQueue::load(&state, &g).is_empty());
    }

    #[test]
    fn a_record_that_does_not_match_its_name_is_never_written() {
        let t = TempDir::new();
        let root = ProviderRoot::open_or_create(t.path()).unwrap();
        let (g, me) = (
            GroupId::parse(&hex('9')).unwrap(),
            ComputerId::parse(&hex('1')).unwrap(),
        );
        let mut kept = KeptRecord::reserve(&rec(3), 1, KeepWhy::Held);
        kept.bytes.push(b' ');
        let err = write_record(&root, &g, &me, &kept).unwrap_err();
        assert_eq!(err.kind, FsErrorKind::HashMismatch);
        assert!(root
            .read(
                &layout::authority_record(&g, &me, &kept.hash),
                AUTHORITY_LIMIT
            )
            .is_err());
    }
}
