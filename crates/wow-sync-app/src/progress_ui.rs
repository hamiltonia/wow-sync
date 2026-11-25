//! Live progress (run-010 goal 2) and the live log lines (goal 3). The service emits typed
//! progress events; this turns them into the headline text, the throttled view the window
//! reads, and log lines that carry names, counts and sizes only.

use std::collections::BTreeSet;

use wow_sync_runtime::{ChangeRecord, ChangeSource, Phase, ProgressEvent};

use crate::history_ui::{done_note, group_digits, group_title, size_text};

/// The window and tray are refreshed at most about twice a second.
pub const MIN_GAP_MS: i64 = 500;
/// How long "Updated 15 add-ons … see Activity" stays in the headline.
pub const NOTE_MS: i64 = 20_000;

/// What the window shows while a pass works.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ProgressView {
    pub text: String,
    pub phase: Option<Phase>,
    /// "Incoming: 15 add-ons from RYZENFISH, 1.8 GB", kept until the pass ends.
    pub incoming: Option<String>,
    /// Packages that started applying: their "Waiting for your decision" text is stale.
    pub satisfied: BTreeSet<String>,
}

fn source_text(s: &Option<ChangeSource>) -> String {
    match s {
        Some(ChangeSource::Computer(n)) => format!(" from {n}"),
        None => " from several computers".into(),
        Some(_) => String::new(),
    }
}

fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{} {}", group_digits(n), if n == 1 { one } else { many })
}

/// "1.2 of 1.8 GB": both numbers in the unit of the total.
fn bytes_of(done: u64, total: u64) -> String {
    let t = size_text(total);
    let (_, unit) = t.rsplit_once(' ').unwrap_or((&t, ""));
    let d = size_text(done);
    let (dn, du) = d.rsplit_once(' ').unwrap_or((&d, ""));
    if du == unit {
        format!("{dn} of {t}")
    } else {
        format!("{d} of {t}")
    }
}

/// The headline for one event. `None` when the pass is idle.
pub fn text(e: &ProgressEvent) -> Option<String> {
    let cur = e
        .current
        .as_deref()
        .map(|c| format!(" ({c})"))
        .unwrap_or_default();
    Some(match e.phase {
        Phase::Idle => return None,
        Phase::Scanning if e.items_total > 0 => format!(
            "Checking files: {} of {}",
            group_digits(e.items_done),
            group_digits(e.items_total)
        ),
        Phase::Scanning => "Checking files".into(),
        Phase::Deciding => "Working out what to update".into(),
        Phase::Staging | Phase::Installing => {
            let verb = if e.phase == Phase::Staging {
                "Preparing"
            } else {
                "Installing"
            };
            // One package has one source; None only means several for a bigger batch.
            let from = if e.source.is_none() && e.items_total <= 1 {
                String::new()
            } else {
                source_text(&e.source)
            };
            let what = if e.items_total == 1 {
                match &e.current {
                    Some(c) => c.clone(),
                    None => "1 add-on".into(),
                }
            } else if e.items_total > 1 {
                plural(e.items_total, "add-on", "add-ons")
            } else {
                "add-ons".into()
            };
            let amount = if e.bytes_total > 0 {
                format!(": {}", bytes_of(e.bytes_done, e.bytes_total))
            } else if e.items_total > 1 {
                format!(": {} of {}", e.items_done, e.items_total)
            } else {
                String::new()
            };
            // One add-on already names itself.
            let cur = if e.items_total == 1 {
                String::new()
            } else {
                cur
            };
            format!("{verb} {what}{from}{amount}{cur}")
        }
        // The service switches to Removing per package, so a mixed pass reads honestly.
        Phase::Removing => match (&e.current, e.items_total) {
            (Some(c), n) if n <= 1 => format!("Removing {c}"),
            (Some(c), n) => format!(
                "Removing {}: {} of {} ({c})",
                plural(n, "add-on", "add-ons"),
                e.items_done,
                n
            ),
            (None, 1) => "Removing 1 add-on".into(),
            (None, n) if n > 1 => format!("Removing {}", plural(n, "add-on", "add-ons")),
            (None, _) => "Removing add-ons".into(),
        },
        Phase::Publishing if e.items_total > 0 => format!(
            "Publishing changes: {} of {}",
            group_digits(e.items_done),
            group_digits(e.items_total)
        ),
        Phase::Publishing => "Publishing changes".into(),
    })
}

pub fn incoming_text(e: &ProgressEvent) -> Option<String> {
    let i = e.incoming.as_ref()?;
    // A removal brings nothing in.
    if i.bytes == 0 {
        return None;
    }
    let from = source_text(&i.source);
    Some(format!(
        "Incoming: {}{from}, {}",
        plural(u64::from(i.addons), "add-on", "add-ons"),
        size_text(i.bytes)
    ))
}

fn phase_name(p: Phase) -> &'static str {
    match p {
        Phase::Scanning => "checking files",
        Phase::Deciding => "deciding",
        Phase::Staging => "preparing files",
        Phase::Installing => "installing",
        Phase::Removing => "removing",
        Phase::Publishing => "publishing",
        Phase::Idle => "idle",
    }
}

/// One log line for a finished package. No paths, no file names.
pub fn record_log(r: &ChangeRecord) -> String {
    let from = match &r.source {
        ChangeSource::ThisComputer => "this computer".to_string(),
        ChangeSource::Computer(n) => n.clone(),
    };
    format!(
        "{} {} (from {from}): {}, files {}, {} bytes, {} ms",
        r.kind.as_str(),
        r.name,
        crate::history_ui::what_text(r),
        {
            let f = &r.files;
            format!("+{} ~{} -{}", f.added, f.changed, f.removed)
        },
        r.byte_delta,
        r.ms
    )
}

pub enum Out {
    Log(String),
    /// The view the window should show now; `None` clears it.
    Show(Option<ProgressView>),
    /// The brief headline after a pass; `None` clears it.
    Note(Option<String>),
}

/// Turns events into outputs. Logging is immediate; window updates are throttled with a
/// trailing flush so the last state of a burst is never lost.
#[derive(Default)]
pub struct Pump {
    view: ProgressView,
    active: bool,
    last_phase: Option<Phase>,
    shown: Option<Phase>,
    last_shown_ms: i64,
    dirty: bool,
    records: Vec<ChangeRecord>,
    note_until: Option<i64>,
    pass_started_ms: i64,
}

impl Pump {
    pub fn on_event(&mut self, e: &ProgressEvent, now: i64) -> Vec<Out> {
        let mut out = Vec::new();
        if e.phase == Phase::Idle {
            if !self.active {
                return out;
            }
            self.active = false;
            self.dirty = false;
            self.last_phase = None;
            let records = std::mem::take(&mut self.records);
            let ms = (now - self.pass_started_ms).max(0);
            out.push(Out::Log(format!(
                "pass summary: {} ({} change(s), {ms} ms)",
                if records.is_empty() {
                    "nothing changed on this computer".to_string()
                } else {
                    group_title(&records)
                },
                records.len()
            )));
            out.push(Out::Show(None));
            self.view = ProgressView::default();
            if let Some(n) = done_note(&records) {
                self.note_until = Some(now + NOTE_MS);
                out.push(Out::Note(Some(n)));
            }
            return out;
        }
        if !self.active {
            self.active = true;
            self.pass_started_ms = now;
            self.records.clear();
            self.view = ProgressView::default();
            self.note_until = None;
            out.push(Out::Note(None));
        }
        if self.last_phase != Some(e.phase) {
            self.last_phase = Some(e.phase);
            let mut line = format!("phase: {}", phase_name(e.phase));
            if e.items_total > 0 {
                line.push_str(&format!(", {} item(s)", e.items_total));
            }
            if e.bytes_total > 0 {
                line.push_str(&format!(", {}", size_text(e.bytes_total)));
            }
            out.push(Out::Log(line));
        }
        if let Some(i) = incoming_text(e) {
            out.push(Out::Log(i.to_lowercase()));
            self.view.incoming = Some(i);
        }
        for p in &e.decision_satisfied {
            self.view.satisfied.insert(p.clone());
        }
        if let Some(r) = &e.completed {
            out.push(Out::Log(record_log(r)));
            self.records.push(r.clone());
        }
        self.view.phase = Some(e.phase);
        self.view.text = text(e).unwrap_or_default();
        self.dirty = true;
        // A new phase or a satisfied decision shows at once; byte counts wait their turn.
        let urgent = !e.decision_satisfied.is_empty()
            || e.incoming.is_some()
            || self.last_shown_ms == 0
            || self.view.phase != self.shown;
        if urgent || now - self.last_shown_ms >= MIN_GAP_MS {
            out.extend(self.flush(now));
        }
        out
    }

    /// Called on a timer: sends the held update once its time has come, ends the note.
    pub fn tick(&mut self, now: i64) -> Vec<Out> {
        let mut out = Vec::new();
        if self.dirty && now - self.last_shown_ms >= MIN_GAP_MS {
            out.extend(self.flush(now));
        }
        if self.note_until.is_some_and(|t| now >= t) {
            self.note_until = None;
            out.push(Out::Note(None));
        }
        out
    }

    fn flush(&mut self, now: i64) -> Vec<Out> {
        self.dirty = false;
        self.last_shown_ms = now.max(1);
        self.shown = self.view.phase;
        vec![Out::Show(Some(self.view.clone()))]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_sync_runtime::{ChangeKind, ChangeReason, FileCounts, IncomingSummary, VersionLabel};

    fn ev(seq: u64, phase: Phase) -> ProgressEvent {
        ProgressEvent {
            seq,
            pass_id: 1,
            phase,
            current: None,
            items_done: 0,
            items_total: 0,
            bytes_done: 0,
            bytes_total: 0,
            source: None,
            incoming: None,
            decision_satisfied: Vec::new(),
            completed: None,
        }
    }

    fn record(name: &str) -> ChangeRecord {
        ChangeRecord {
            unix_ms: 5_000,
            pass_id: 1,
            kind: ChangeKind::Applied,
            name: name.into(),
            folders: vec![name.into()],
            source: ChangeSource::Computer("RYZENFISH".into()),
            old_version: VersionLabel::Toc("1".into()),
            new_version: VersionLabel::Toc("2".into()),
            files: FileCounts {
                added: 1,
                changed: 2,
                removed: 0,
            },
            byte_delta: 100,
            reason: ChangeReason::Choice(None),
            paths: vec!["Core.lua".into()],
            paths_more: 0,
            ms: 0,
        }
    }

    #[test]
    fn headline_text_for_each_phase() {
        let mut e = ev(1, Phase::Scanning);
        e.items_done = 12_000;
        e.items_total = 36_000;
        assert_eq!(text(&e).unwrap(), "Checking files: 12,000 of 36,000");
        let mut e = ev(2, Phase::Installing);
        e.items_total = 15;
        e.bytes_done = 1_288_490_189;
        e.bytes_total = 1_932_735_283;
        e.source = Some(ChangeSource::Computer("RYZENFISH".into()));
        e.current = Some("SpokenQuestsAudioShared".into());
        assert_eq!(
            text(&e).unwrap(),
            "Installing 15 add-ons from RYZENFISH: 1.2 of 1.8 GB (SpokenQuestsAudioShared)"
        );
        let mut e = ev(3, Phase::Publishing);
        e.items_done = 3;
        e.items_total = 7;
        assert_eq!(text(&e).unwrap(), "Publishing changes: 3 of 7");
        assert_eq!(text(&ev(4, Phase::Idle)), None);
    }

    #[test]
    fn a_removal_never_reads_as_an_install() {
        let mut e = ev(1, Phase::Removing);
        e.items_total = 1;
        e.current = Some("Atlas".into());
        assert_eq!(text(&e).unwrap(), "Removing Atlas");
        e.current = None;
        assert_eq!(text(&e).unwrap(), "Removing 1 add-on");
        e.items_total = 3;
        e.items_done = 1;
        e.current = Some("Atlas".into());
        assert_eq!(text(&e).unwrap(), "Removing 3 add-ons: 1 of 3 (Atlas)");
    }

    #[test]
    fn several_sources_never_read_as_this_computer() {
        let mut e = ev(1, Phase::Installing);
        e.items_total = 15;
        e.source = None;
        assert_eq!(
            text(&e).unwrap(),
            "Installing 15 add-ons from several computers: 0 of 15"
        );
        e.source = Some(ChangeSource::ThisComputer);
        assert!(!text(&e).unwrap().contains("this computer"));
    }

    #[test]
    fn incoming_summary_is_shown_before_the_apply() {
        let mut e = ev(1, Phase::Staging);
        e.incoming = Some(IncomingSummary {
            addons: 15,
            source: Some(ChangeSource::Computer("RYZENFISH".into())),
            bytes: 1_932_735_283,
        });
        let mut p = Pump::default();
        let outs = p.on_event(&e, 10_000);
        let shown = outs.iter().find_map(|o| match o {
            Out::Show(Some(v)) => Some(v.clone()),
            _ => None,
        });
        assert_eq!(
            shown.unwrap().incoming.as_deref(),
            Some("Incoming: 15 add-ons from RYZENFISH, 1.8 GB")
        );
    }

    #[test]
    fn window_updates_are_throttled_and_the_last_one_is_not_lost() {
        let mut p = Pump::default();
        let mut shows = 0;
        let count = |outs: &[Out]| {
            outs.iter()
                .filter(|o| matches!(o, Out::Show(Some(_))))
                .count()
        };
        // 100 byte updates in one second.
        for i in 0..100u64 {
            let mut e = ev(i + 1, Phase::Installing);
            e.bytes_total = 1_000;
            e.bytes_done = i;
            shows += count(&p.on_event(&e, 10_000 + (i as i64) * 10));
        }
        assert!(shows <= 3, "{shows} updates in one second");
        // The held last update is sent once its time has come.
        let after = p.tick(11_100);
        assert_eq!(count(&after), 1);
        assert_eq!(count(&p.tick(11_700)), 0);
    }

    #[test]
    fn a_decision_is_satisfied_at_once_even_inside_the_throttle_gap() {
        let mut p = Pump::default();
        p.on_event(&ev(1, Phase::Installing), 10_000);
        let mut e = ev(2, Phase::Installing);
        e.decision_satisfied = vec!["hideanything".into()];
        let outs = p.on_event(&e, 10_050);
        let v = outs
            .into_iter()
            .find_map(|o| match o {
                Out::Show(Some(v)) => Some(v),
                _ => None,
            })
            .expect("shown at once");
        assert!(v.satisfied.contains("hideanything"));
    }

    #[test]
    fn logs_carry_phase_package_and_summary_without_paths() {
        let mut p = Pump::default();
        let mut logs: Vec<String> = Vec::new();
        let mut take = |outs: Vec<Out>| {
            for o in outs {
                if let Out::Log(l) = o {
                    logs.push(l);
                }
            }
        };
        take(p.on_event(&ev(1, Phase::Scanning), 1_000));
        let mut e = ev(2, Phase::Installing);
        e.items_total = 1;
        e.completed = Some(record("Bagnon"));
        take(p.on_event(&e, 2_000));
        take(p.on_event(&ev(3, Phase::Idle), 3_000));
        assert_eq!(logs[0], "phase: checking files");
        assert!(logs[1].starts_with("phase: installing"));
        assert!(logs
            .iter()
            .any(|l| l.starts_with("applied Bagnon (from RYZENFISH): 1 \u{2192} 2")));
        let last = logs.last().unwrap();
        assert!(last.starts_with("pass summary: Bagnon updated from RYZENFISH"));
        for l in &logs {
            assert!(
                !l.contains('\\') && !l.contains(":/") && !l.contains("Core.lua"),
                "{l}"
            );
        }
    }

    #[test]
    fn the_pass_ends_with_a_brief_note_that_then_expires() {
        let mut p = Pump::default();
        let mut e = ev(1, Phase::Installing);
        e.completed = Some(record("Bagnon"));
        p.on_event(&e, 1_000);
        let outs = p.on_event(&ev(2, Phase::Idle), 2_000);
        assert!(outs.iter().any(|o| matches!(o, Out::Show(None))));
        assert!(outs.iter().any(
            |o| matches!(o, Out::Note(Some(n)) if n == "Updated Bagnon from RYZENFISH \u{2014} see Activity")
        ));
        assert!(p.tick(2_000 + NOTE_MS - 1).is_empty());
        assert!(matches!(p.tick(2_000 + NOTE_MS)[..], [Out::Note(None)]));
    }
}
