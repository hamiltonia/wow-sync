//! Notification planner (ops.md §3.2, A-27). Pure: time, game state, preferences and policy are
//! inputs; the ledger is a value the runtime persists. The planner decides what to say and when;
//! it never delivers anything.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// `notices.json` limit (technical cap, contracts.md §8).
pub const MAX_LEDGER_BYTES: usize = 64 * 1024;
/// Cleared keys remembered for recurrence damping.
pub const CLEARED_CAP: usize = 100;
/// Names kept per held kind; counts are kept for the rest.
pub const HELD_NAMES_CAP: usize = 5;
/// Held non-summary keys kept; more are counted and assumed still open.
pub const HELD_KEYS_CAP: usize = 100;
/// Names shown in one notification before "and N more".
pub const NAMES_SHOWN: usize = 3;
/// Q-85 default: 6 hours.
pub const DEFAULT_RECURRENCE_QUIET_MS: i64 = 6 * 60 * 60 * 1000;
const NAME_CHARS: usize = 64;
const LEDGER_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NoticeKind {
    Summary,
    ActionNeeded,
    DataAtRisk,
    AppUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NoticeScope {
    Addon(String),
    Computer(String),
    SharedFolder,
    Update(String),
    Batch,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NoticeKey {
    pub kind: NoticeKind,
    pub scope: NoticeScope,
}

pub type Fingerprint = [u8; 32];

/// Hash of whatever makes a condition "the same" (for example the candidate set of a decision).
pub fn fingerprint(content: &[u8]) -> Fingerprint {
    crate::hash::sha256(content).0
}

/// One observation. `name` is the display subject (add-on, tag text); it is shown, never parsed.
/// Non-summary events are the full set of conditions open now, each cycle. Summary events are
/// one-shot: one per item applied in a completed batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeEvent {
    pub key: NoticeKey,
    pub fingerprint: Fingerprint,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameActivity {
    Running,
    NotRunning,
    /// "Can't tell" is treated as "maybe playing" (ops.md §3.2 rule 5).
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoticePrefs {
    pub summaries: bool,
    pub needs_me: bool,
}

impl Default for NoticePrefs {
    fn default() -> Self {
        NoticePrefs {
            summaries: true,
            needs_me: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoticePolicy {
    /// Q-85. `None` = no suppression of recurrences.
    pub recurrence_quiet_ms: Option<i64>,
}

impl Default for NoticePolicy {
    fn default() -> Self {
        NoticePolicy {
            recurrence_quiet_ms: Some(DEFAULT_RECURRENCE_QUIET_MS),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OpenEntry {
    fp: Fingerprint,
    last_notified_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClearedEntry {
    key: NoticeKey,
    fp: Fingerprint,
    last_notified_ms: i64,
}

/// Nonurgent notices waiting for the game to close. Bounded: counts, at most five names per kind
/// and at most [`HELD_KEYS_CAP`] keys.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeldSet {
    summary_count: u32,
    summary_names: Vec<String>,
    items: BTreeMap<NoticeKey, String>,
    overflow: u32,
}

impl HeldSet {
    fn is_empty(&self) -> bool {
        self.summary_count == 0 && self.items.is_empty() && self.overflow == 0
    }

    fn add_summary(&mut self, name: &str) {
        self.summary_count = self.summary_count.saturating_add(1);
        if self.summary_names.len() < HELD_NAMES_CAP {
            self.summary_names.push(name.to_string());
        }
    }

    fn add_item(&mut self, key: &NoticeKey, name: &str) {
        if let Some(slot) = self.items.get_mut(key) {
            *slot = name.to_string();
        } else if self.items.len() >= HELD_KEYS_CAP {
            self.overflow = self.overflow.saturating_add(1);
        } else {
            let kept = if self.items.len() < HELD_NAMES_CAP {
                name.to_string()
            } else {
                String::new()
            };
            self.items.insert(key.clone(), kept);
        }
    }

    pub fn held_keys(&self) -> usize {
        self.items.len()
    }

    pub fn held_summaries(&self) -> u32 {
        self.summary_count
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NoticeLedger {
    open: BTreeMap<NoticeKey, OpenEntry>,
    cleared: VecDeque<ClearedEntry>,
    held: HeldSet,
}

impl NoticeLedger {
    pub fn open_len(&self) -> usize {
        self.open.len()
    }

    pub fn cleared_len(&self) -> usize {
        self.cleared.len()
    }

    pub fn held(&self) -> &HeldSet {
        &self.held
    }

    fn push_cleared(&mut self, e: ClearedEntry) {
        self.cleared.retain(|c| c.key != e.key);
        self.cleared.push_back(e);
        while self.cleared.len() > CLEARED_CAP {
            self.cleared.pop_front();
        }
    }

    fn take_cleared(&mut self, key: &NoticeKey) -> Option<ClearedEntry> {
        let i = self.cleared.iter().position(|c| &c.key == key)?;
        self.cleared.remove(i)
    }

    /// Bounded JSON for `notices.json`. Over the limit it sheds the oldest cleared keys, then
    /// held names; open conditions are never shed because they are bounded by reality.
    pub fn encode(&self) -> Vec<u8> {
        let mut l = self.clone();
        loop {
            let bytes = serde_json::to_vec(&LedgerDto::from(&l)).unwrap_or_default();
            if bytes.len() <= MAX_LEDGER_BYTES {
                return bytes;
            }
            if l.cleared.pop_front().is_some() {
                continue;
            }
            if l.held.items.values().any(|n| !n.is_empty()) || !l.held.summary_names.is_empty() {
                l.held.items.values_mut().for_each(String::clear);
                l.held.summary_names.clear();
                continue;
            }
            // Last resort: forget the least recently notified open key. It will notify again
            // after a restart, which errs toward telling the user.
            let oldest = l
                .open
                .iter()
                .min_by_key(|(_, e)| e.last_notified_ms)
                .map(|(k, _)| k.clone());
            match oldest {
                Some(k) => {
                    l.open.remove(&k);
                }
                None => return bytes,
            }
        }
    }

    /// `None` for anything unreadable, oversize or of another version; callers start empty.
    pub fn decode(bytes: &[u8]) -> Option<NoticeLedger> {
        if bytes.len() > MAX_LEDGER_BYTES {
            return None;
        }
        let dto: LedgerDto = serde_json::from_slice(bytes).ok()?;
        if dto.v != LEDGER_VERSION {
            return None;
        }
        let mut l = NoticeLedger::default();
        for r in dto.open {
            l.open.insert(
                r.key,
                OpenEntry {
                    fp: unhex(&r.fp)?,
                    last_notified_ms: r.last,
                },
            );
        }
        for r in dto.cleared {
            l.push_cleared(ClearedEntry {
                key: r.key,
                fp: unhex(&r.fp)?,
                last_notified_ms: r.last,
            });
        }
        l.held.summary_count = dto.held.summary_count;
        l.held.summary_names = dto
            .held
            .summary_names
            .into_iter()
            .take(HELD_NAMES_CAP)
            .collect();
        for (key, name) in dto.held.items.into_iter().take(HELD_KEYS_CAP) {
            l.held.items.insert(key, name);
        }
        l.held.overflow = dto.held.overflow;
        Some(l)
    }
}

/// One open or cleared condition on disk.
#[derive(Serialize, Deserialize)]
struct KeyRow {
    key: NoticeKey,
    fp: String,
    last: i64,
}

#[derive(Serialize, Deserialize)]
struct HeldDto {
    summary_count: u32,
    summary_names: Vec<String>,
    items: Vec<(NoticeKey, String)>,
    overflow: u32,
}

#[derive(Serialize, Deserialize)]
struct LedgerDto {
    v: u32,
    open: Vec<KeyRow>,
    cleared: Vec<KeyRow>,
    held: HeldDto,
}

impl From<&NoticeLedger> for LedgerDto {
    fn from(l: &NoticeLedger) -> Self {
        LedgerDto {
            v: LEDGER_VERSION,
            open: l
                .open
                .iter()
                .map(|(k, e)| KeyRow {
                    key: k.clone(),
                    fp: hex(&e.fp),
                    last: e.last_notified_ms,
                })
                .collect(),
            cleared: l
                .cleared
                .iter()
                .map(|c| KeyRow {
                    key: c.key.clone(),
                    fp: hex(&c.fp),
                    last: c.last_notified_ms,
                })
                .collect(),
            held: HeldDto {
                summary_count: l.held.summary_count,
                summary_names: l.held.summary_names.clone(),
                items: l
                    .held
                    .items
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                overflow: l.held.overflow,
            },
        }
    }
}

fn hex(b: &[u8]) -> String {
    const D: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push(D[(x >> 4) as usize] as char);
        s.push(D[(x & 15) as usize] as char);
    }
    s
}

fn unhex(s: &str) -> Option<Fingerprint> {
    let b = s.as_bytes();
    if b.len() != 64 {
        return None;
    }
    let nib = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = nib(b[2 * i])? << 4 | nib(b[2 * i + 1])?;
    }
    Some(out)
}

/// What to tell the user. Text comes from [`Notice::title`] and [`Notice::body`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub kind: NoticeKind,
    /// At most [`NAMES_SHOWN`] names.
    pub names: Vec<String>,
    /// Items beyond the shown names.
    pub more: u32,
    /// Released after the game closed: "While you played: …".
    pub while_playing: bool,
}

impl Notice {
    pub fn count(&self) -> u32 {
        self.names.len() as u32 + self.more
    }

    pub fn title(&self) -> &'static str {
        match self.kind {
            NoticeKind::DataAtRisk => "WoW Sync: attention needed now",
            NoticeKind::AppUpdate => "WoW Sync update",
            NoticeKind::Summary | NoticeKind::ActionNeeded => "WoW Sync",
        }
    }

    pub fn body(&self) -> String {
        let n = self.count();
        let list = name_list(&self.names, self.more);
        let first = self.names.first().map(String::as_str).unwrap_or("");
        match self.kind {
            NoticeKind::Summary => {
                let lead = if self.while_playing {
                    "While you played: "
                } else {
                    ""
                };
                if n == 1 && !first.is_empty() {
                    format!("{lead}{first} was updated.")
                } else {
                    format!("{lead}{n} add-ons updated: {list}.")
                }
            }
            NoticeKind::ActionNeeded => {
                if n == 1 && !first.is_empty() {
                    format!("{first} needs your decision. Open WoW Sync to choose.")
                } else {
                    format!("{n} add-ons need your decision. Open WoW Sync to choose.")
                }
            }
            NoticeKind::DataAtRisk => {
                if n == 1 && !first.is_empty() {
                    format!("{first}. Open WoW Sync to see what to do.")
                } else {
                    format!(
                        "{n} problems could put your data at risk. Open WoW Sync to see what to do."
                    )
                }
            }
            NoticeKind::AppUpdate => {
                format!("WoW Sync {first} is available. Open WoW Sync to download it.")
            }
        }
    }
}

fn name_list(names: &[String], more: u32) -> String {
    let mut s = names.join(", ");
    if more > 0 {
        s = if let (true, Some(i)) = (names.len() > 1, s.rfind(", ")) {
            format!("{}, {} and {more} more", &s[..i], &s[i + 2..])
        } else {
            format!("{s} and {more} more")
        };
    } else if let Some(i) = s.rfind(", ") {
        s = format!("{} and {}", &s[..i], &s[i + 2..]);
    }
    s
}

fn clean_name(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_control())
        .take(NAME_CHARS)
        .collect()
}

fn recurrence_allows(
    kind: NoticeKind,
    last_notified_ms: i64,
    now_ms: i64,
    policy: NoticePolicy,
) -> bool {
    if kind == NoticeKind::DataAtRisk {
        return true;
    }
    match policy.recurrence_quiet_ms {
        None => true,
        Some(quiet) => now_ms < last_notified_ms || now_ms - last_notified_ms >= quiet,
    }
}

fn group(kind: NoticeKind, names: Vec<String>, total: u32, while_playing: bool) -> Notice {
    let shown: Vec<String> = names
        .into_iter()
        .filter(|n| !n.is_empty())
        .take(NAMES_SHOWN)
        .collect();
    let more = total.saturating_sub(shown.len() as u32);
    Notice {
        kind,
        names: shown,
        more,
        while_playing,
    }
}

/// Plans the notifications for one observation. `events` are described on [`NoticeEvent`];
/// `open_now` is every non-summary key open right now (a key absent from it is cleared or
/// resolved). Returns the notices to deliver and the next ledger.
pub fn plan_notices(
    ledger: &NoticeLedger,
    events: &[NoticeEvent],
    open_now: &BTreeSet<NoticeKey>,
    game: GameActivity,
    prefs: NoticePrefs,
    policy: NoticePolicy,
    now_unix_ms: i64,
) -> (Vec<Notice>, NoticeLedger) {
    let mut l = ledger.clone();
    let deferring = game != GameActivity::NotRunning;

    let gone: Vec<NoticeKey> = l
        .open
        .keys()
        .filter(|k| !open_now.contains(*k))
        .cloned()
        .collect();
    for k in gone {
        if let Some(e) = l.open.remove(&k) {
            l.push_cleared(ClearedEntry {
                key: k,
                fp: e.fp,
                last_notified_ms: e.last_notified_ms,
            });
        }
    }
    l.held.items.retain(|k, _| open_now.contains(k));

    let mut now_summary: Vec<String> = Vec::new();
    let mut now_action: Vec<String> = Vec::new();
    let mut now_risk: Vec<String> = Vec::new();
    let mut now_update: Vec<String> = Vec::new();

    for e in events {
        let name = clean_name(&e.name);
        if e.key.kind == NoticeKind::Summary {
            if !prefs.summaries {
                continue;
            }
            if deferring {
                l.held.add_summary(&name);
            } else {
                now_summary.push(name);
            }
            continue;
        }
        if !open_now.contains(&e.key) {
            continue;
        }
        let (notify, last) = match l.open.get(&e.key) {
            Some(o) if o.fp == e.fingerprint => continue,
            Some(_) => (true, now_unix_ms),
            None => match l.take_cleared(&e.key) {
                Some(c) if c.fp == e.fingerprint => {
                    let ok = recurrence_allows(e.key.kind, c.last_notified_ms, now_unix_ms, policy);
                    (ok, if ok { now_unix_ms } else { c.last_notified_ms })
                }
                _ => (true, now_unix_ms),
            },
        };
        l.open.insert(
            e.key.clone(),
            OpenEntry {
                fp: e.fingerprint,
                last_notified_ms: last,
            },
        );
        if !notify || !prefs.needs_me {
            continue;
        }
        match e.key.kind {
            NoticeKind::DataAtRisk => now_risk.push(name),
            _ if deferring => l.held.add_item(&e.key, &name),
            NoticeKind::AppUpdate => now_update.push(name),
            _ => now_action.push(name),
        }
    }

    let mut out = Vec::new();
    if !now_risk.is_empty() {
        let total = now_risk.len() as u32;
        out.push(group(NoticeKind::DataAtRisk, now_risk, total, false));
    }

    if !deferring {
        let held = std::mem::take(&mut l.held);
        let released = !held.is_empty();
        let mut actions: Vec<String> = Vec::new();
        let mut updates: Vec<String> = Vec::new();
        let mut held_actions = 0u32;
        if prefs.needs_me {
            for (k, n) in &held.items {
                if k.kind == NoticeKind::AppUpdate {
                    updates.push(n.clone());
                } else {
                    actions.push(n.clone());
                    held_actions += 1;
                }
            }
            held_actions += held.overflow;
        }
        let mut action_total = held_actions + now_action.len() as u32;
        actions.extend(now_action);
        let update_total = updates.len() as u32 + now_update.len() as u32;
        updates.extend(now_update);
        if released && action_total > 0 {
            // At most two notices after a release: an update joins the action notice.
            action_total += update_total;
            actions.append(&mut updates);
        }
        if action_total > 0 {
            out.push(group(
                NoticeKind::ActionNeeded,
                actions,
                action_total,
                released,
            ));
        }
        for u in updates.into_iter().filter(|u| !u.is_empty()) {
            out.push(group(NoticeKind::AppUpdate, vec![u], 1, false));
        }
        let held_count = held.summary_count;
        let mut names = held.summary_names.clone();
        let total = held_count + now_summary.len() as u32;
        names.extend(now_summary);
        if total > 0 {
            out.push(group(NoticeKind::Summary, names, total, held_count > 0));
        }
    }
    (out, l)
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: i64 = 60 * 60 * 1000;

    fn key(kind: NoticeKind, name: &str) -> NoticeKey {
        NoticeKey {
            kind,
            scope: NoticeScope::Addon(name.into()),
        }
    }

    fn ev(kind: NoticeKind, name: &str, content: &str) -> NoticeEvent {
        NoticeEvent {
            key: key(kind, name),
            fingerprint: fingerprint(content.as_bytes()),
            name: name.into(),
        }
    }

    fn summary(name: &str, batch: &str) -> NoticeEvent {
        NoticeEvent {
            key: NoticeKey {
                kind: NoticeKind::Summary,
                scope: NoticeScope::Batch,
            },
            fingerprint: fingerprint(batch.as_bytes()),
            name: name.into(),
        }
    }

    fn open(events: &[NoticeEvent]) -> BTreeSet<NoticeKey> {
        events
            .iter()
            .filter(|e| e.key.kind != NoticeKind::Summary)
            .map(|e| e.key.clone())
            .collect()
    }

    struct Run {
        ledger: NoticeLedger,
    }

    impl Run {
        fn new() -> Run {
            Run {
                ledger: NoticeLedger::default(),
            }
        }

        fn step(
            &mut self,
            events: &[NoticeEvent],
            game: GameActivity,
            prefs: NoticePrefs,
            policy: NoticePolicy,
            now: i64,
        ) -> Vec<Notice> {
            let (n, l) = plan_notices(
                &self.ledger,
                events,
                &open(events),
                game,
                prefs,
                policy,
                now,
            );
            self.ledger = l;
            n
        }

        fn quick(&mut self, events: &[NoticeEvent], game: GameActivity, now: i64) -> Vec<Notice> {
            self.step(
                events,
                game,
                NoticePrefs::default(),
                NoticePolicy::default(),
                now,
            )
        }
    }

    use GameActivity::{NotRunning, Running, Unknown};

    #[test]
    fn five_applied_items_make_one_summary_with_three_names_and_a_count() {
        let evs: Vec<NoticeEvent> = ["A", "B", "C", "D", "E"]
            .iter()
            .map(|n| summary(n, "b1"))
            .collect();
        let mut r = Run::new();
        let out = r.quick(&evs, NotRunning, 0);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, NoticeKind::Summary);
        assert_eq!(out[0].names, ["A", "B", "C"]);
        assert_eq!(out[0].more, 2);
        assert_eq!(out[0].body(), "5 add-ons updated: A, B, C and 2 more.");
        let one = r.quick(&[summary("A", "b2")], NotRunning, 1);
        assert_eq!(one[0].body(), "A was updated.");
    }

    #[test]
    fn several_new_action_items_become_one_notification_with_a_count() {
        let evs = vec![
            ev(NoticeKind::ActionNeeded, "A", "1"),
            ev(NoticeKind::ActionNeeded, "B", "1"),
        ];
        let out = Run::new().quick(&evs, NotRunning, 0);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].count(), 2);
        assert!(out[0].body().starts_with("2 add-ons need your decision"));
    }

    #[test]
    fn a_conflict_while_gaming_waits_and_notifies_once_on_game_exit() {
        let evs = vec![ev(NoticeKind::ActionNeeded, "WeakAuras", "1")];
        let mut r = Run::new();
        assert!(r.quick(&evs, Running, 0).is_empty());
        assert!(r.quick(&evs, Running, 1000).is_empty());
        assert_eq!(r.ledger.held().held_keys(), 1);
        let out = r.quick(&evs, NotRunning, 2000);
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].while_playing);
        assert!(out[0].body().contains("WeakAuras needs your decision"));
        assert!(r.quick(&evs, NotRunning, 3000).is_empty());
        assert_eq!(r.ledger.held().held_keys(), 0);
    }

    #[test]
    fn an_unchanged_condition_over_100_cycles_notifies_once() {
        let evs = vec![ev(NoticeKind::ActionNeeded, "A", "same")];
        let mut r = Run::new();
        let total: usize = (0..100)
            .map(|i| r.quick(&evs, NotRunning, i * 1000).len())
            .sum();
        assert_eq!(total, 1);
        let changed = vec![ev(NoticeKind::ActionNeeded, "A", "new candidate")];
        assert_eq!(r.quick(&changed, NotRunning, 200_000).len(), 1);
    }

    #[test]
    fn recurrence_after_clearing_waits_for_the_quiet_period() {
        let evs = vec![ev(NoticeKind::ActionNeeded, "A", "x")];
        let changed = vec![ev(NoticeKind::ActionNeeded, "A", "y")];
        let none = NoticePolicy {
            recurrence_quiet_ms: None,
        };
        let go = |gap: i64, again: &[NoticeEvent], policy: NoticePolicy| {
            let mut r = Run::new();
            let p = NoticePrefs::default();
            assert_eq!(r.step(&evs, NotRunning, p, policy, 0).len(), 1);
            r.step(&[], NotRunning, p, policy, 1);
            r.step(again, NotRunning, p, policy, gap).len()
        };
        let d = NoticePolicy::default();
        assert_eq!(go(H, &evs, d), 0, "same fingerprint after 1 h");
        assert_eq!(go(6 * H, &evs, d), 1, "same fingerprint after 6 h");
        assert_eq!(
            go(H, &changed, d),
            1,
            "a changed fingerprint always notifies"
        );
        assert_eq!(go(H, &evs, none), 1, "no quiet period configured");
        assert_eq!(
            go(-5, &evs, d),
            1,
            "a clock that went back errs toward notifying"
        );

        // A suppressed recurrence does not restart the quiet period.
        let mut r = Run::new();
        r.quick(&evs, NotRunning, 0);
        r.quick(&[], NotRunning, 1);
        assert!(r.quick(&evs, NotRunning, 2 * H).is_empty());
        r.quick(&[], NotRunning, 3 * H);
        assert_eq!(r.quick(&evs, NotRunning, 6 * H).len(), 1);
    }

    #[test]
    fn held_summaries_release_as_at_most_two_notices_and_resolved_items_drop() {
        let mut r = Run::new();
        for i in 0..8 {
            let evs = vec![
                summary(&format!("S{i}"), &format!("b{i}")),
                ev(NoticeKind::ActionNeeded, "Keep", "1"),
                ev(NoticeKind::ActionNeeded, "Gone", "1"),
            ];
            assert!(r.quick(&evs, Running, i * 10).is_empty());
        }
        let still = vec![ev(NoticeKind::ActionNeeded, "Keep", "1")];
        let out = r.quick(&still, NotRunning, 1000);
        assert_eq!(out.len(), 2, "{out:?}");
        let s = out.iter().find(|n| n.kind == NoticeKind::Summary).unwrap();
        assert_eq!(s.count(), 8);
        assert_eq!(s.names.len(), 3);
        assert!(s.body().starts_with("While you played: 8 add-ons updated"));
        let a = out
            .iter()
            .find(|n| n.kind == NoticeKind::ActionNeeded)
            .unwrap();
        assert_eq!(a.names, ["Keep"], "the resolved item is dropped");
        assert_eq!(a.count(), 1);
    }

    #[test]
    fn unknown_game_state_is_treated_as_gaming() {
        let evs = vec![ev(NoticeKind::ActionNeeded, "A", "1")];
        let mut r = Run::new();
        assert!(r.quick(&evs, Unknown, 0).is_empty());
        assert_eq!(r.ledger.held().held_keys(), 1);
        let with_summary = vec![evs[0].clone(), summary("S", "b")];
        assert!(r.quick(&with_summary, Unknown, 1).is_empty());
        assert_eq!(r.quick(&evs, NotRunning, 2).len(), 2);
    }

    #[test]
    fn data_at_risk_is_sent_at_once_while_gaming_and_recurs_after_an_hour() {
        let risk = vec![ev(NoticeKind::DataAtRisk, "Disk", "x")];
        let mut r = Run::new();
        let out = r.quick(&risk, Running, 0);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, NoticeKind::DataAtRisk);
        assert!(
            r.quick(&risk, Running, 10).is_empty(),
            "unchanged: not again"
        );
        r.quick(&[], Running, 20);
        assert_eq!(
            r.quick(&risk, Unknown, H).len(),
            1,
            "recurrence is not damped"
        );
    }

    #[test]
    fn needs_me_off_silences_every_os_notice_kind_without_a_burst_later() {
        let evs = vec![
            ev(NoticeKind::ActionNeeded, "A", "1"),
            ev(NoticeKind::AppUpdate, "0.2.0", "1"),
            ev(NoticeKind::DataAtRisk, "Disk", "1"),
        ];
        let off = NoticePrefs {
            summaries: true,
            needs_me: false,
        };
        let mut r = Run::new();
        let out = r.step(&evs, NotRunning, off, NoticePolicy::default(), 0);
        assert!(out.is_empty(), "{out:?}");
        assert_eq!(r.ledger.open_len(), 3, "recorded as notified");
        assert!(
            r.quick(&evs, NotRunning, 10).is_empty(),
            "no burst on re-enable"
        );
        let s = r.step(
            &[summary("A", "b")],
            NotRunning,
            off,
            NoticePolicy::default(),
            20,
        );
        assert_eq!(s.len(), 1, "summaries have their own toggle");
        let no_summary = NoticePrefs {
            summaries: false,
            needs_me: true,
        };
        let held = r.step(
            &[summary("A", "b2")],
            Running,
            no_summary,
            NoticePolicy::default(),
            30,
        );
        assert!(held.is_empty());
        assert_eq!(r.ledger.held().held_summaries(), 0);
    }

    #[test]
    fn needs_me_off_at_release_drops_what_was_held() {
        let evs = vec![ev(NoticeKind::ActionNeeded, "A", "1")];
        let mut r = Run::new();
        r.quick(&evs, Running, 0);
        let off = NoticePrefs {
            summaries: true,
            needs_me: false,
        };
        let out = r.step(&evs, NotRunning, off, NoticePolicy::default(), 1);
        assert!(out.is_empty());
        assert_eq!(r.ledger.held().held_keys(), 0);
    }

    #[test]
    fn ledger_survives_a_restart_without_repeating() {
        let evs = vec![
            ev(NoticeKind::ActionNeeded, "A", "1"),
            ev(NoticeKind::DataAtRisk, "Disk", "1"),
        ];
        let mut r = Run::new();
        assert_eq!(r.quick(&evs, NotRunning, 0).len(), 2);
        let bytes = r.ledger.encode();
        let mut restarted = Run::new();
        restarted.ledger = NoticeLedger::decode(&bytes).expect("decodes");
        assert_eq!(restarted.ledger, r.ledger);
        assert!(restarted.quick(&evs, NotRunning, 10).is_empty());
        assert!(NoticeLedger::decode(b"{not json").is_none());
        assert!(NoticeLedger::decode(&vec![b' '; MAX_LEDGER_BYTES + 1]).is_none());
        let other_version = br#"{"v":2,"open":[],"cleared":[],"held":{"summary_count":0,"summary_names":[],"items":[],"overflow":0}}"#;
        assert!(NoticeLedger::decode(other_version).is_none());
    }

    #[test]
    fn ledger_and_held_set_are_bounded_and_an_evicted_key_notifies_again() {
        let mut r = Run::new();
        let first = vec![ev(NoticeKind::ActionNeeded, "k0", "x")];
        assert_eq!(r.quick(&first, NotRunning, 0).len(), 1);
        for i in 1..10_000i64 {
            let evs = vec![ev(NoticeKind::ActionNeeded, &format!("k{i}"), "x")];
            r.quick(&evs, NotRunning, i);
        }
        assert!(r.ledger.cleared_len() <= CLEARED_CAP);
        assert!(r.ledger.open_len() <= 1);
        assert!(r.ledger.encode().len() <= MAX_LEDGER_BYTES);
        // k0 was evicted from the cleared list: its recurrence (same fingerprint, 1 s later)
        // notifies instead of being damped.
        assert_eq!(r.quick(&first, NotRunning, 10_001).len(), 1);

        let mut g = Run::new();
        for i in 0..10_000i64 {
            let evs: Vec<NoticeEvent> = vec![
                ev(NoticeKind::ActionNeeded, &format!("h{i}"), "x"),
                summary(&format!("s{i}"), &format!("b{i}")),
            ];
            g.quick(&evs, Running, i);
        }
        assert!(g.ledger.held().held_keys() <= 1);
        let many: Vec<NoticeEvent> = (0..500)
            .map(|i| ev(NoticeKind::ActionNeeded, &format!("m{i}"), "x"))
            .collect();
        g.quick(&many, Running, 20_000);
        assert!(g.ledger.held().held_keys() <= HELD_KEYS_CAP);
        assert!(g.ledger.encode().len() <= MAX_LEDGER_BYTES);
        let out = g.quick(&many, NotRunning, 30_000);
        let a = out
            .iter()
            .find(|n| n.kind == NoticeKind::ActionNeeded)
            .unwrap();
        assert_eq!(a.count(), 500, "overflowed keys are counted, not dropped");
        assert!(a.names.len() <= NAMES_SHOWN);
        let s = out.iter().find(|n| n.kind == NoticeKind::Summary).unwrap();
        assert_eq!(s.count(), 10_000);
    }

    #[test]
    fn app_update_is_one_notice_per_tag_and_defers_while_gaming() {
        let tag1 = vec![ev(NoticeKind::AppUpdate, "0.2.0", "v0.2.0")];
        let mut r = Run::new();
        assert!(r.quick(&tag1, Running, 0).is_empty());
        let out = r.quick(&tag1, NotRunning, 1);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, NoticeKind::AppUpdate);
        assert!(out[0].body().contains("0.2.0 is available"));
        assert!(
            r.quick(&tag1, NotRunning, 24 * H).is_empty(),
            "same tag next day"
        );
        let tag2 = vec![ev(NoticeKind::AppUpdate, "0.3.0", "v0.3.0")];
        assert_eq!(r.quick(&tag2, NotRunning, 48 * H).len(), 1);
    }

    #[test]
    fn names_are_cleaned_and_bounded() {
        let evs = vec![ev(
            NoticeKind::ActionNeeded,
            &format!("A\n{}", "x".repeat(500)),
            "1",
        )];
        let out = Run::new().quick(&evs, NotRunning, 0);
        assert!(!out[0].names[0].contains('\n'));
        assert!(out[0].names[0].chars().count() <= 64);
    }
}
