//! Decision routing and the kept-record queue (ops.md §1, T-R4D31-held-self, Q-60).
//!
//! Pure routing: core `decide` yields an effect, this module decides whether it is written now,
//! kept unwritten, or refused. A running game never holds a group record (T-26-7): only
//! game-file changes wait for it, so the routing takes no activity input. The service wires this
//! in step 3.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use wow_sync_adapters::authority_store;
use wow_sync_adapters::localstate::LocalState;
use wow_sync_adapters::provider::ProviderRoot;
use wow_sync_adapters::{FsError, FsErrorKind};
use wow_sync_core::authority::decode_authority;
use wow_sync_core::authority::AuthorityRecord;
use wow_sync_core::decide::encode_authority;
use wow_sync_core::decision::{DecideError, DecisionEffect, LocalResolution};
use wow_sync_core::hash::sha256;
use wow_sync_core::ids::{ComputerId, EnrollmentId, GroupId, Sha256};
use wow_sync_core::kinds::{AuthorityIndexChunk, AuthorityRef, EnrollmentRef, MAX_INDEX_BYTES};
use wow_sync_core::layout;
use wow_sync_core::ledger::{
    append_index_refs, decode_segment, derive, encode_segments, read_index_chain, Body, Entry,
    IndexHead, Ledger, Membership, Status,
};
use wow_sync_core::records::{decode, AuthorityIndexHead};
use wow_sync_core::retire::OwnStanding;

const AUTHORITY_LIMIT: u64 = 16 * 1024 * 1024;

/// The two constraint behaviours as one `ConstraintHooks`: keeps and exclusions from the code
/// choices, level-2 rejection and recovered targets from the rejections.
pub(crate) struct Constraints {
    choice: wow_sync_core::choice::ChoiceBehaviour,
    reject: wow_sync_core::reject::RejectBehaviour,
}

impl Constraints {
    pub(crate) fn for_derived(d: &wow_sync_core::ledger::Derived) -> Self {
        Self {
            choice: wow_sync_core::choice::ChoiceBehaviour::for_derived(d),
            reject: wow_sync_core::reject::RejectBehaviour::new(d),
        }
    }
}

impl wow_sync_core::hooks::ConstraintHooks for Constraints {
    fn live_constraints(
        &self,
        l: &Ledger,
        f: &wow_sync_core::paths::FolderKey,
    ) -> Vec<wow_sync_core::decision::Constraint> {
        let mut out = self.choice.live_constraints(l, f);
        for c in self.reject.live_constraints(l, f) {
            if !out.contains(&c) {
                out.push(c);
            }
        }
        out
    }

    fn excluded(
        &self,
        l: &Ledger,
        p: &wow_sync_core::package::Package,
        s: &wow_sync_core::target::Source,
    ) -> wow_sync_core::target::ScopeMatch {
        self.choice.excluded(l, p, s)
    }

    fn rejected(
        &self,
        l: &Ledger,
        p: &wow_sync_core::package::Package,
        s: &wow_sync_core::target::Source,
    ) -> wow_sync_core::target::ScopeMatch {
        self.reject.rejected(l, p, s)
    }
    fn choices_conflict(&self, l: &Ledger, p: &wow_sync_core::package::Package) -> bool {
        self.choice.choices_conflict(l, p)
    }
}

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

// ---------------------------------------------------------------- the authority plane

const MEMBERSHIP_FILE: &str = "authority-membership.json";
const DISMISSED_FILE: &str = "authority-dismissed.json";
const LOCAL_LIMIT: u64 = 16 * 1024 * 1024;

fn segment_name(i: u32) -> String {
    format!("authority-ledger-{i:08}.json")
}

fn object_name(h: &Sha256) -> String {
    format!("authority-object-{h}.json")
}

#[derive(Serialize, Deserialize)]
struct DismissedWire {
    dismissed: Vec<Sha256>,
}

/// The learned records and the historical membership of one group (contracts.md §1.7, §1.10).
/// Both are grow-only. The local files are flat names in the local state directory because
/// that directory takes no sub-paths; the content is the contract's segment, object and
/// membership data.
#[derive(Debug, Default)]
pub(crate) struct Plane {
    pub ledger: Ledger,
    pub members: Membership,
    saved: BTreeSet<Sha256>,
    fresh: Vec<(Sha256, Vec<u8>)>,
    dirty: bool,
}

impl Plane {
    /// Whatever cannot be read is simply not loaded: the shared folder teaches it again, and
    /// nothing local is ever treated as proof that something is absent.
    pub(crate) fn load(state: &LocalState) -> Plane {
        let mut p = Plane::default();
        if let Some(m) = state
            .read(MEMBERSHIP_FILE, LOCAL_LIMIT)
            .ok()
            .flatten()
            .and_then(|b| serde_json::from_slice::<Membership>(&b).ok())
        {
            p.members = m;
        }
        let mut entries = Vec::new();
        for i in 0.. {
            let Ok(Some(b)) = state.read(&segment_name(i), LOCAL_LIMIT) else {
                break;
            };
            let Ok(seg) = decode_segment(&b) else { break };
            entries.extend(seg.entries);
        }
        for e in entries {
            let bytes = state
                .read(&object_name(&e.hash), LOCAL_LIMIT)
                .ok()
                .flatten();
            let mut from = e.learned_from.iter();
            p.ledger.learn(
                e.hash,
                e.kind,
                e.folders.clone(),
                from.next(),
                bytes.as_deref(),
            );
            for c in from {
                p.ledger.learn(e.hash, e.kind, Vec::new(), Some(c), None);
            }
            if let Some(seq) = e.mirrored_seq {
                p.ledger.mark_mirrored(&e.hash, seq);
            }
            if bytes.is_some() {
                p.saved.insert(e.hash);
            }
        }
        if let Some(d) = state
            .read(DISMISSED_FILE, LOCAL_LIMIT)
            .ok()
            .flatten()
            .and_then(|b| serde_json::from_slice::<DismissedWire>(&b).ok())
        {
            for h in d.dismissed {
                p.ledger.dismiss(h);
            }
        }
        p
    }

    /// Record bytes first, then the entry listing, so a listed entry never lacks its bytes
    /// unless the write failed (and then it simply loads as unreadable).
    pub(crate) fn save(&mut self, state: &LocalState) -> Result<(), FsError> {
        if !self.dirty && self.fresh.is_empty() {
            return Ok(());
        }
        for (h, b) in std::mem::take(&mut self.fresh) {
            if let Err(e) = state.write_atomic(&object_name(&h), &b) {
                self.fresh.push((h, b));
                return Err(e);
            }
            self.saved.insert(h);
        }
        let entries: Vec<_> = self.ledger.entries().cloned().collect();
        for (i, seg) in encode_segments(&entries).iter().enumerate() {
            state.write_atomic(&segment_name(i as u32), seg)?;
        }
        state.write_atomic(
            MEMBERSHIP_FILE,
            &serde_json::to_vec(&self.members).expect("plain data serializes"),
        )?;
        let dismissed = DismissedWire {
            dismissed: self.ledger.dismissed().iter().copied().collect(),
        };
        state.write_atomic(
            DISMISSED_FILE,
            &serde_json::to_vec(&dismissed).expect("plain data serializes"),
        )?;
        self.dirty = false;
        Ok(())
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// The verbatim bytes of a learned record, whether still unsaved or already stored locally.
    pub(crate) fn bytes(&self, state: &LocalState, h: &Sha256) -> Option<Vec<u8>> {
        let b = self
            .fresh
            .iter()
            .find(|(f, _)| f == h)
            .map(|(_, b)| b.clone())
            .or_else(|| state.read(&object_name(h), LOCAL_LIMIT).ok().flatten())?;
        (&sha256(&b) == h).then_some(b)
    }

    /// Marks records as listed in the own index from `seq` on, once that state is published.
    pub(crate) fn mark_listed(&mut self, hashes: &[Sha256], seq: u64) {
        for h in hashes {
            self.ledger.mark_mirrored(h, seq);
        }
        if !hashes.is_empty() {
            self.dirty = true;
        }
    }

    pub(crate) fn add_member(&mut self, c: &ComputerId, e: &EnrollmentId) {
        if !self.members.contains(c, e) {
            self.members.add(c, e);
            self.dirty = true;
        }
    }

    /// Learns a record's verbatim bytes. A copy that does not hash to its name, does not decode,
    /// or has an unknown schema teaches nothing here.
    pub(crate) fn learn_bytes(&mut self, h: Sha256, from: Option<&ComputerId>, bytes: &[u8]) {
        if let Some(Body::Record(_)) = self.ledger.body(&h) {
            if let Some(e) = self.ledger.entry(&h).map(|e| e.kind) {
                self.ledger.learn(h, e, Vec::new(), from, None);
            }
            return;
        }
        let Ok(r) = decode_authority(bytes, &h) else {
            return;
        };
        self.ledger
            .learn(h, r.kind(), Vec::new(), from, Some(bytes));
        self.fresh.push((h, bytes.to_vec()));
        self.dirty = true;
    }
}

/// Writes one of this computer's own records (write-if-absent, read back) and learns it, so it is
/// in the ledger before anything is computed from it (contracts.md §1.12 step 3).
pub(crate) fn write_own_record(
    plane: &mut Plane,
    provider: &ProviderRoot,
    group: &GroupId,
    me: &ComputerId,
    r: &AuthorityRecord,
) -> Result<Sha256, FsError> {
    write_own_bytes(plane, provider, group, me, &encode_authority(r))
}

/// Writes already-encoded record or part bytes under the own `authority\` and learns them.
pub(crate) fn write_own_bytes(
    plane: &mut Plane,
    provider: &ProviderRoot,
    group: &GroupId,
    me: &ComputerId,
    bytes: &[u8],
) -> Result<Sha256, FsError> {
    let h = sha256(bytes);
    let rel = layout::authority_record(group, me, &h);
    authority_store::put_record(provider, &rel, bytes, AUTHORITY_LIMIT, me.as_str())?;
    plane.learn_bytes(h, Some(me), bytes);
    Ok(h)
}

/// What the next own state carries for the authority index, and what to mark once it is published.
#[derive(Debug, Default)]
pub(crate) struct OwnIndex {
    pub head: Option<AuthorityIndexHead>,
    /// Records this plan lists for the first time.
    pub newly: Vec<Sha256>,
    /// Records the published chain already lists but the ledger had not marked (a lost mark).
    pub adopt: Vec<Sha256>,
    pub note: Option<String>,
}

/// Mirrors every newly valid record into the own `authority\` and lists it in the own index
/// (contracts.md §1.6, §1.12 step 4 and repair). Only the own directory is ever written: a good
/// own copy is left alone, a missing one is written and a damaged one is repaired
/// (`put_record`). A failed repair keeps the record listed and says so. Index chunks are
/// immutable and written before the state that names them.
pub(crate) fn plan_own_index(
    plane: &Plane,
    state: &LocalState,
    provider: &ProviderRoot,
    group: &GroupId,
    own: &EnrollmentRef,
    published: Option<AuthorityIndexHead>,
) -> OwnIndex {
    let mut out = OwnIndex {
        head: published,
        ..OwnIndex::default()
    };
    let derived = derive(&plane.ledger, group, &plane.members);
    let mut candidates: Vec<&Entry> = plane
        .ledger
        .entries()
        .filter(|e| e.mirrored_seq.is_none() && derived.status_of(&e.hash) == Some(&Status::Valid))
        .collect();
    if candidates.is_empty() {
        return out;
    }
    let chain_head = match published {
        None => None,
        Some(h) => {
            let fetch = |c: &Sha256| {
                provider
                    .read(
                        &layout::authority_index_chunk(group, &own.computer_id, c),
                        MAX_INDEX_BYTES as u64,
                    )
                    .ok()
            };
            let read = read_index_chain(
                &IndexHead {
                    head: h.head,
                    total: h.total,
                },
                group,
                &own.enrollment_id,
                &Default::default(),
                &fetch,
            );
            if read.error.is_some() {
                out.note = Some("the own authority index could not be read; not extended".into());
                return out;
            }
            let listed: BTreeSet<Sha256> = read.refs.iter().map(|r| r.hash).collect();
            out.adopt = candidates
                .iter()
                .map(|e| e.hash)
                .filter(|h| listed.contains(h))
                .collect();
            candidates.retain(|e| !listed.contains(&e.hash));
            let bytes = fetch(&h.head);
            bytes
                .and_then(|b| decode::<AuthorityIndexChunk>(&b, MAX_INDEX_BYTES).ok())
                .map(|c| (h.head, c))
        }
    };
    if published.is_some() && chain_head.is_none() {
        out.note = Some("the own authority index could not be read; not extended".into());
        return out;
    }
    let mut refs = Vec::new();
    for e in candidates {
        let Some(bytes) = plane.bytes(state, &e.hash) else {
            continue;
        };
        let rel = layout::authority_record(group, &own.computer_id, &e.hash);
        if authority_store::put_record(
            provider,
            &rel,
            &bytes,
            AUTHORITY_LIMIT,
            own.computer_id.as_str(),
        )
        .is_err()
        {
            out.note = Some("A damaged copy could not be repaired".into());
        }
        refs.push(AuthorityRef {
            hash: e.hash,
            kind: e.kind,
            folders: e.folders.clone(),
        });
    }
    if refs.is_empty() {
        return out;
    }
    refs.sort_by_key(|r| r.hash);
    out.newly = refs.iter().map(|r| r.hash).collect();
    let total = published.map_or(0, |h| h.total);
    let (files, head) = append_index_refs(
        group,
        &own.enrollment_id,
        chain_head.as_ref().map(|(h, c)| (h, c)),
        total,
        refs,
    );
    for (h, bytes) in files {
        let rel = layout::authority_index_chunk(group, &own.computer_id, &h);
        if authority_store::put_record(
            provider,
            &rel,
            &bytes,
            MAX_INDEX_BYTES as u64,
            own.computer_id.as_str(),
        )
        .is_err()
        {
            out.newly.clear();
            out.note = Some("the authority index could not be written; will retry".into());
            return out;
        }
    }
    out.head = Some(AuthorityIndexHead {
        head: head.head,
        total: head.total,
    });
    out
}

/// Reads every member's `authority\` listing (own and mirrors, retired members and members whose
/// newest state is unusable included) into the plane (contracts.md §1.12 steps 1–3).
pub(crate) fn learn_from_directories(
    plane: &mut Plane,
    provider: &ProviderRoot,
    group: &GroupId,
    computers: &[ComputerId],
) {
    for c in computers {
        let Ok(names) = authority_store::list_names(provider, &layout::authority_dir(group, c))
        else {
            continue;
        };
        for n in names {
            let Some(stem) = n.strip_suffix(".json") else {
                continue;
            };
            let Ok(h) = Sha256::parse(stem) else { continue };
            if let Some(Body::Record(_)) = plane.ledger.body(&h) {
                plane.learn_bytes(h, Some(c), &[]);
                continue;
            }
            let Ok(bytes) = provider.read(
                &layout::authority_record(group, c, &h),
                wow_sync_core::kinds::MAX_RECORD_BYTES as u64,
            ) else {
                continue;
            };
            plane.learn_bytes(h, Some(c), &bytes);
        }
    }
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
