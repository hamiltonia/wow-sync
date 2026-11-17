//! Notification preview rules from docs/design/ui/interaction.md. Nothing is posted to the OS:
//! the prototype only shows what would be sent and when (Q-30).

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotifKind {
    Summary,
    Action,
    Urgent,
    AppUpdate,
}

impl NotifKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Summary => "Batch summary",
            Self::Action => "Action needed",
            Self::Urgent => "Data at risk",
            Self::AppUpdate => "App update",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotifState {
    WouldSend,
    Held,
    /// Folded into a released notification.
    Combined,
    /// Resolved while held.
    Dropped,
    /// Unchanged condition, not repeated.
    Suppressed,
}

impl NotifState {
    pub fn label(self) -> &'static str {
        match self {
            Self::WouldSend => "Would be sent now",
            Self::Held => "Held while WoW runs",
            Self::Combined => "Combined into a summary",
            Self::Dropped => "Dropped: resolved meanwhile",
            Self::Suppressed => "Not repeated: unchanged condition",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notif {
    pub kind: NotifKind,
    pub key: String,
    pub text: String,
    pub state: NotifState,
    pub repeats: u32,
}

const MAX_HELD_PER_KIND: usize = 5;
const MAX_LOG: usize = 40;

#[derive(Clone, Debug, Default)]
pub struct NotifEngine {
    game_running: bool,
    pub log: Vec<Notif>,
    seen: BTreeMap<String, String>,
}

impl NotifEngine {
    pub fn set_game_running(&mut self, running: bool) {
        self.game_running = running;
    }

    pub fn held(&self) -> Vec<&Notif> {
        self.log
            .iter()
            .filter(|n| n.state == NotifState::Held)
            .collect()
    }

    fn push(&mut self, n: Notif) {
        self.log.insert(0, n);
        self.log.truncate(MAX_LOG);
    }

    /// A condition appears or is re-observed. Unchanged conditions never notify again.
    pub fn raise(&mut self, kind: NotifKind, key: &str, text: &str) {
        if self.seen.get(key).map(String::as_str) == Some(text) {
            if let Some(n) = self
                .log
                .iter_mut()
                .find(|n| n.key == key && n.state == NotifState::Suppressed)
            {
                n.repeats += 1;
                return;
            }
            self.push(Notif {
                kind,
                key: key.into(),
                text: text.into(),
                state: NotifState::Suppressed,
                repeats: 1,
            });
            return;
        }
        self.seen.insert(key.to_string(), text.to_string());
        let state = if self.game_running && kind != NotifKind::Urgent {
            NotifState::Held
        } else {
            NotifState::WouldSend
        };
        self.push(Notif {
            kind,
            key: key.into(),
            text: text.into(),
            state,
            repeats: 0,
        });
        if state == NotifState::Held {
            self.bound_held();
        }
    }

    fn bound_held(&mut self) {
        for kind in [NotifKind::Summary, NotifKind::Action, NotifKind::AppUpdate] {
            let mut count = 0;
            for n in self.log.iter_mut() {
                if n.kind == kind && n.state == NotifState::Held {
                    count += 1;
                    if count > MAX_HELD_PER_KIND {
                        n.state = NotifState::Dropped;
                    }
                }
            }
        }
    }

    /// The game was observed not running: release at most two notifications, one combined
    /// summary and one action. `open_keys` are the conditions still open.
    pub fn release(&mut self, open_keys: &[String]) {
        let mut summaries: Vec<String> = Vec::new();
        let mut actions: Vec<String> = Vec::new();
        for n in self.log.iter_mut().rev() {
            if n.state != NotifState::Held {
                continue;
            }
            match n.kind {
                NotifKind::Action => {
                    if open_keys.contains(&n.key) {
                        actions.push(n.text.trim_end_matches('.').to_string());
                        n.state = NotifState::Combined;
                    } else {
                        n.state = NotifState::Dropped;
                    }
                }
                _ => {
                    summaries.push(n.text.trim_end_matches('.').to_string());
                    n.state = NotifState::Combined;
                }
            }
        }
        if !actions.is_empty() {
            let text = format!("{}.", actions.join("; "));
            self.push(Notif {
                kind: NotifKind::Action,
                key: "released-action".into(),
                text,
                state: NotifState::WouldSend,
                repeats: 0,
            });
        }
        if !summaries.is_empty() {
            let text = format!("While you played: {}.", summaries.join("; "));
            self.push(Notif {
                kind: NotifKind::Summary,
                key: "released-summary".into(),
                text,
                state: NotifState::WouldSend,
                repeats: 0,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonurgent_held_while_game_runs_urgent_sent() {
        let mut e = NotifEngine::default();
        e.set_game_running(true);
        e.raise(NotifKind::Summary, "a", "1 add-on updated.");
        e.raise(NotifKind::Urgent, "u", "Disk almost full.");
        assert_eq!(e.held().len(), 1);
        assert_eq!(e.log[0].state, NotifState::WouldSend);
    }

    #[test]
    fn release_sends_at_most_two() {
        let mut e = NotifEngine::default();
        e.set_game_running(true);
        for i in 0..4 {
            e.raise(
                NotifKind::Summary,
                &format!("s{i}"),
                &format!("{i} updated."),
            );
        }
        e.raise(NotifKind::Action, "act", "1 add-on needs your decision.");
        e.raise(NotifKind::Action, "gone", "Resolved thing needs you.");
        e.set_game_running(false);
        e.release(&["act".to_string()]);
        let sent: Vec<_> = e
            .log
            .iter()
            .filter(|n| n.state == NotifState::WouldSend)
            .collect();
        assert_eq!(sent.len(), 2);
        assert!(e.held().is_empty());
        assert!(e
            .log
            .iter()
            .any(|n| n.key == "gone" && n.state == NotifState::Dropped));
    }

    #[test]
    fn unchanged_condition_is_not_repeated_but_changed_content_notifies() {
        let mut e = NotifEngine::default();
        e.raise(
            NotifKind::Action,
            "decisions",
            "3 add-ons need your decision.",
        );
        e.raise(
            NotifKind::Action,
            "decisions",
            "3 add-ons need your decision.",
        );
        e.raise(
            NotifKind::Action,
            "decisions",
            "3 add-ons need your decision.",
        );
        assert_eq!(
            e.log
                .iter()
                .filter(|n| n.state == NotifState::WouldSend)
                .count(),
            1
        );
        let sup = e
            .log
            .iter()
            .find(|n| n.state == NotifState::Suppressed)
            .unwrap();
        assert_eq!(sup.repeats, 2);
        e.raise(
            NotifKind::Action,
            "decisions",
            "2 add-ons need your decision.",
        );
        assert_eq!(
            e.log
                .iter()
                .filter(|n| n.state == NotifState::WouldSend)
                .count(),
            2
        );
    }

    #[test]
    fn held_set_is_bounded_per_kind() {
        let mut e = NotifEngine::default();
        e.set_game_running(true);
        for i in 0..20 {
            e.raise(
                NotifKind::Summary,
                &format!("s{i}"),
                &format!("{i} updated."),
            );
        }
        assert_eq!(e.held().len(), 5);
    }
}
