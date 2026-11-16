//! GPUI view: enrollment (G1/G2) and status (G3-G8) screens. It renders what the runtime
//! reports; it makes no sync decision. Every control has an AccessKit role and label and a tab
//! stop (G9).

use crate::backend::{lock, Cmd, SharedState};
use crate::consent::Consent;
use gpui::accesskit::{Role, Toggled};
use gpui::{
    actions, div, prelude::*, rgb, App, Context, FocusHandle, Render, SharedString, Window,
};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::Duration;
use wow_sync_runtime::core::ids::Sha256;
use wow_sync_runtime::core::variant::{consent_gap, ConsentAction, ConsentGap, GuidedConfirmation};
use wow_sync_runtime::{EnrollRequest, JoinChoice, PreviewKind, ReconfirmRequest};

actions!(wow_sync, [FocusNextStop, FocusPrevStop]);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        gpui::KeyBinding::new("tab", FocusNextStop, None),
        gpui::KeyBinding::new("shift-tab", FocusPrevStop, None),
    ]);
}

pub struct Root {
    shared: SharedState,
    tx: Option<Sender<Cmd>>,
    fatal: Option<String>,
    seen: u64,
    handles: HashMap<String, FocusHandle>,
    consent: Consent,
    note: String,
    banner: String,
    first: Option<FocusHandle>,
    focused_once: bool,
}

impl Root {
    pub fn new(
        shared: SharedState,
        tx: Option<Sender<Cmd>>,
        fatal: Option<String>,
        banner: String,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(250))
                .await;
            let alive = this.update(cx, |t, cx| {
                let v = lock(&t.shared).version;
                if v != t.seen {
                    t.seen = v;
                    cx.notify();
                }
            });
            if alive.is_err() {
                break;
            }
        })
        .detach();
        Root {
            shared,
            tx,
            fatal,
            seen: 0,
            handles: HashMap::new(),
            consent: Consent::default(),
            note: String::new(),
            banner,
            first: None,
            focused_once: false,
        }
    }

    pub fn focus_first(&self, window: &mut Window, cx: &mut App) {
        if let Some(h) = &self.first {
            window.focus(h, cx);
        }
    }

    fn handle(&mut self, key: &str, cx: &mut Context<Self>) -> FocusHandle {
        self.handles
            .entry(key.to_string())
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone()
    }

    fn send(&mut self, c: Cmd) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(c);
        }
    }

    /// The consent ticked in the window, for the preview it was shown with.
    fn given(&self, digest: Option<Sha256>) -> GuidedConfirmation {
        GuidedConfirmation {
            identity: self.consent.identity,
            scope: self.consent.scope,
            digest: digest.unwrap_or(Sha256([0; 32])),
        }
    }

    fn reconfirm(&mut self) {
        let digest = {
            let g = lock(&self.shared);
            match &g.status.reconfirm {
                Some(p) => p.digest,
                None => return,
            }
        };
        let gap = consent_gap(ConsentAction::Reconfirm, false, &self.given(digest), None);
        if gap.is_some() {
            self.note = "Confirm both the game version and the add-on scope to continue. \
                         Nothing has been changed."
                .into();
            return;
        }
        self.note.clear();
        self.consent.clear();
        self.send(Cmd::Reconfirm(ReconfirmRequest {
            identity_confirmed: true,
            scope_confirmed: true,
            digest,
        }));
    }

    fn enroll(&mut self, join: JoinChoice) {
        let (needs, digest) = {
            let g = lock(&self.shared);
            match &g.preview {
                Some(p) => (p.needs_guided_confirmation, p.digest),
                None => return,
            }
        };
        match consent_gap(
            ConsentAction::Enroll,
            needs,
            &self.given(digest),
            digest.as_ref(),
        ) {
            None => {}
            Some(ConsentGap::Scope) => {
                self.note = "Tick the agreement box first. Nothing has been written yet.".into();
                return;
            }
            Some(ConsentGap::Identity | ConsentGap::Stale) => {
                self.note = "This game version is unfamiliar: confirm the identity too. \
                             Agreeing to the scope alone is not enough."
                    .into();
                return;
            }
        }
        self.note.clear();
        let display_name = self.banner.clone();
        let req = EnrollRequest {
            join,
            identity_confirmed: self.consent.identity || !needs,
            scope_confirmed: self.consent.scope,
            digest,
            display_name,
        };
        self.consent.clear();
        self.send(Cmd::Enroll(req));
    }
}

const BG: u32 = 0x1e293b;
const PANEL: u32 = 0x334155;
const MUTED: u32 = 0xcbd5e1;

fn ring(on: bool) -> gpui::Rgba {
    if on {
        rgb(0xffcc00)
    } else {
        rgb(0x475569)
    }
}

fn text(
    id: impl Into<SharedString>,
    role: Role,
    s: impl Into<String>,
) -> gpui::Stateful<gpui::Div> {
    let s: SharedString = SharedString::from(s.into());
    div()
        .id(id.into())
        .role(role)
        .aria_label(s.clone())
        .child(s)
}

fn heading(id: &str, s: &str) -> impl IntoElement {
    text(SharedString::from(id.to_string()), Role::Heading, s)
        .text_xl()
        .aria_level(1)
}

fn section(id: &str, title: &str, rows: Vec<String>) -> impl IntoElement {
    let items: Vec<_> = rows
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            text(format!("{id}-{i}"), Role::ListItem, r)
                .text_color(rgb(MUTED))
                .px_2()
        })
        .collect();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            text(format!("{id}-title"), Role::Heading, title)
                .aria_level(2)
                .font_weight(gpui::FontWeight::BOLD),
        )
        .child(
            div()
                .id(SharedString::from(id.to_string()))
                .role(Role::List)
                .aria_label(SharedString::from(title.to_string()))
                .flex()
                .flex_col()
                .children(items),
        )
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut idx = 1isize;
        let mut next = || {
            idx += 1;
            idx
        };
        let mut body = div().flex().flex_col().gap_3().p_4();
        let mut first: Option<FocusHandle> = None;

        body = body.child(heading("title", "WoW Sync"));
        body =
            body.child(text("banner", Role::Paragraph, self.banner_text()).text_color(rgb(MUTED)));

        if let Some(f) = self.fatal.clone() {
            body = body.child(text("fatal", Role::Paragraph, f));
        } else {
            let (enrolled, preview, status, message, lines, busy) = {
                let g = lock(&self.shared);
                (
                    g.status.enrolled,
                    g.preview.clone(),
                    g.status.clone(),
                    g.message.clone(),
                    g.report_lines.clone(),
                    g.busy,
                )
            };
            let panel = match (&status.reconfirm, &preview) {
                (Some(p), _) => Some(format!(
                    "reconfirm|{}|{}|{:?}",
                    p.identity_line, p.scope_line, p.digest
                )),
                (None, Some(p)) if !enrolled => Some(format!(
                    "enroll|{}|{}|{:?}",
                    p.identity_line, p.scope_line, p.digest
                )),
                _ => None,
            };
            self.consent.observe(panel);
            if !message.is_empty() {
                body =
                    body.child(text("message", Role::Paragraph, message).text_color(rgb(0xfca5a5)));
            }
            if !self.note.is_empty() {
                body = body.child(
                    text("note", Role::Paragraph, self.note.clone()).text_color(rgb(0xfca5a5)),
                );
            }
            if !enrolled {
                if let Some(p) = preview {
                    body = body.child(heading("enroll-heading", "Set up this computer"));
                    body = body.child(text("identity", Role::Paragraph, p.identity_line.clone()));
                    if !p.scope_line.is_empty() {
                        body = body.child(text("scope", Role::Paragraph, p.scope_line.clone()));
                    }
                    body = body.child(text(
                        "scope-note",
                        Role::Paragraph,
                        "Only add-on code (Interface/AddOns) is synced. Game settings, saved variables and accounts are excluded.",
                    ));
                    if !p.disclosures.is_empty() {
                        body = body.child(section(
                            "disclosures",
                            "What setting this up does",
                            p.disclosures.clone(),
                        ));
                    }
                    if let Some(prob) = &p.problem {
                        body = body.child(
                            text("problem", Role::Paragraph, prob.clone())
                                .text_color(rgb(0xfca5a5)),
                        );
                    }
                    if !p.blocked.is_empty() {
                        body = body.child(section(
                            "blocked",
                            "Blocked folders (not synced)",
                            p.blocked.clone(),
                        ));
                    }
                    if !p.eligible.is_empty() {
                        body = body.child(section(
                            "eligible",
                            "Add-on folders that would sync",
                            p.eligible.clone(),
                        ));
                    }
                    if p.kind != PreviewKind::Refused && p.problem.is_none() {
                        if p.needs_guided_confirmation {
                            let h = self.handle("identity-ok", cx);
                            first.get_or_insert(h.clone());
                            body = body.child(checkbox(
                                "identity-ok",
                                "I confirm this is the game version shown above",
                                self.consent.identity,
                                &h,
                                next(),
                                window,
                                cx,
                                |t| t.consent.identity = !t.consent.identity,
                            ));
                        }
                        let h = self.handle("scope-ok", cx);
                        first.get_or_insert(h.clone());
                        body = body.child(checkbox(
                            "scope-ok",
                            "I agree to sync the add-on code listed above for this installation",
                            self.consent.scope,
                            &h,
                            next(),
                            window,
                            cx,
                            |t| t.consent.scope = !t.consent.scope,
                        ));
                        let h = self.handle("create", cx);
                        body = body.child(button(
                            "create",
                            "Create a new sync group",
                            &h,
                            next(),
                            window,
                            cx,
                            |t| {
                                t.enroll(JoinChoice::New {
                                    name: "Add-on sync group".into(),
                                })
                            },
                        ));
                        for (i, g) in p.groups.iter().enumerate() {
                            let key = format!("join-{i}");
                            let h = self.handle(&key, cx);
                            let gid = g.id.clone();
                            body = body.child(button(
                                &key,
                                &format!("Join group \"{}\" ({})", g.name, g.product),
                                &h,
                                next(),
                                window,
                                cx,
                                move |t| t.enroll(JoinChoice::Existing(gid.clone())),
                            ));
                        }
                    }
                } else {
                    body = body.child(text(
                        "checking",
                        Role::Paragraph,
                        "Checking this installation...",
                    ));
                }
            } else {
                body = body.child(heading("status-heading", &status.headline));
                if let Some(who) = status.who_line() {
                    body = body.child(text("who", Role::Paragraph, who));
                }
                if let Some(p) = &status.reconfirm {
                    body = body.child(heading("reconfirm-heading", "Confirm the game version"));
                    body = body.child(text("identity", Role::Paragraph, p.identity_line.clone()));
                    if !p.scope_line.is_empty() {
                        body = body.child(text("scope", Role::Paragraph, p.scope_line.clone()));
                    }
                    body = body.child(section(
                        "disclosures",
                        "What syncing does",
                        p.disclosures.clone(),
                    ));
                    let h = self.handle("identity-ok", cx);
                    first.get_or_insert(h.clone());
                    body = body.child(checkbox(
                        "identity-ok",
                        "I confirm this is the game version shown above",
                        self.consent.identity,
                        &h,
                        next(),
                        window,
                        cx,
                        |t| t.consent.identity = !t.consent.identity,
                    ));
                    let h = self.handle("scope-ok", cx);
                    body = body.child(checkbox(
                        "scope-ok",
                        "I agree to sync the add-on code listed above",
                        self.consent.scope,
                        &h,
                        next(),
                        window,
                        cx,
                        |t| t.consent.scope = !t.consent.scope,
                    ));
                    let h = self.handle("reconfirm", cx);
                    body = body.child(button(
                        "reconfirm",
                        "Confirm and continue",
                        &h,
                        next(),
                        window,
                        cx,
                        |t| t.reconfirm(),
                    ));
                }
                if !status.activity.is_empty() {
                    body = body.child(
                        text(
                            "activity",
                            Role::Paragraph,
                            format!("Game activity: {}", status.activity),
                        )
                        .text_color(rgb(MUTED)),
                    );
                }
                if let Some(e) = &status.provider_error {
                    body = body.child(
                        text("provider-error", Role::Paragraph, format!("Problem: {e}"))
                            .text_color(rgb(0xfca5a5)),
                    );
                }
                let h = self.handle("sync-now", cx);
                first.get_or_insert(h.clone());
                let label = if busy { "Syncing..." } else { "Sync now" };
                body = body.child(button("sync-now", label, &h, next(), window, cx, |t| {
                    t.send(Cmd::SyncNow)
                }));
                if !status.blocks.is_empty() {
                    body = body.child(section("blocks", "Waiting", status.blocks.clone()));
                }
                let pk: Vec<String> = status.packages.iter().map(|p| p.text.clone()).collect();
                body = body.child(section("packages", "Add-ons", pk));
                if let Some(n) = &status.recovery_note {
                    body = body.child(text("recovery", Role::Paragraph, format!("Recovery: {n}")));
                }
                body = body.child(section(
                    "members",
                    "Other computers",
                    status.members.clone(),
                ));
                if let Some(lp) = &status.last_publish {
                    body = body.child(text("last-publish", Role::Paragraph, lp.clone()));
                }
                if !lines.is_empty() {
                    body = body.child(section("last-sync", "Last sync", lines));
                }
            }
        }
        if !self.focused_once {
            if let Some(h) = &first {
                window.focus(h, cx);
                self.focused_once = true;
            }
        }
        self.first = first;
        div()
            .id("root")
            .size_full()
            .overflow_y_scroll()
            .bg(rgb(BG))
            .text_color(rgb(0xffffff))
            .on_action(cx.listener(|_, _: &FocusNextStop, w, cx| w.focus_next(cx)))
            .on_action(cx.listener(|_, _: &FocusPrevStop, w, cx| w.focus_prev(cx)))
            .child(body)
    }
}

impl Root {
    fn banner_text(&self) -> String {
        format!(
            "Developer build - synthetic installs only. This computer: {}",
            self.banner
        )
    }
}

fn button(
    id: &str,
    label: &str,
    h: &FocusHandle,
    tab: isize,
    window: &Window,
    cx: &mut Context<Root>,
    f: impl Fn(&mut Root) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.to_string()))
        .role(Role::Button)
        .aria_label(SharedString::from(label.to_string()))
        .track_focus(h)
        .tab_index(tab)
        .px_3()
        .py_1()
        .border_2()
        .border_color(ring(h.is_focused(window)))
        .bg(rgb(PANEL))
        .child(SharedString::from(label.to_string()))
        .on_click(cx.listener(move |this, _, _, cx| {
            f(this);
            cx.notify();
        }))
}

#[allow(clippy::too_many_arguments)]
fn checkbox(
    id: &str,
    label: &str,
    on: bool,
    h: &FocusHandle,
    tab: isize,
    window: &Window,
    cx: &mut Context<Root>,
    f: impl Fn(&mut Root) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.to_string()))
        .role(Role::CheckBox)
        .aria_label(SharedString::from(label.to_string()))
        .aria_toggled(if on { Toggled::True } else { Toggled::False })
        .track_focus(h)
        .tab_index(tab)
        .px_2()
        .border_2()
        .border_color(ring(h.is_focused(window)))
        .child(SharedString::from(format!(
            "[{}] {label}",
            if on { "x" } else { " " }
        )))
        .on_click(cx.listener(move |this, _, _, cx| {
            f(this);
            cx.notify();
        }))
}
