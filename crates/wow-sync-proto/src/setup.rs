//! First-run wizard (P1) and the re-confirmation page (P9).

use crate::app::{Act, Bk, ProtoApp, ReconfAct, SetupAct};
use crate::theme::Tone;
use crate::ui::{icon, u};
use gpui::accesskit::Role;
use gpui::{div, prelude::*, rgb, AnyElement, Context, FontWeight, KeyDownEvent, SharedString};

const STEPS: [&str; 6] = [
    "Welcome",
    "Shared folder",
    "Game installation",
    "Confirm what to sync",
    "Sync group and name",
    "Review and set up",
];

impl ProtoApp {
    /// Consent needs both the exact game version and the add-on list; scope alone is not enough.
    pub fn setup_can_next(&self) -> bool {
        let Some(d) = &self.w.setup else { return true };
        let s = &self.setup;
        match s.step {
            2 => d.dropbox.is_some() && !d.online_only && s.folder_chosen,
            3 => d
                .installs
                .get(s.install)
                .is_some_and(|i| i.refused.is_none()),
            4 => s.identity_ok && s.scope_ok,
            5 => s.group.is_some() && !s.name.trim().is_empty(),
            _ => true,
        }
    }

    pub fn setup_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let d = self.w.setup.clone().unwrap_or(crate::scenario::SetupData {
            dropbox: None,
            online_only: false,
            installs: vec![],
            groups: vec![],
        });
        let step = self.setup.step.clamp(1, 6);
        if self.setup.finished {
            let go = self.btn(
                "finish",
                "Finish and open WoW Sync",
                Bk::Primary,
                Act::Setup(SetupAct::Finish),
                cx,
            );
            return self
                .wizard_frame(vec![
                    self.heading("This computer is set up", 1)
                        .into_any_element(),
                    div()
                        .child(SharedString::from(format!(
                            "{} joined the group. The first check starts now.",
                            self.setup.name
                        )))
                        .into_any_element(),
                    go.into_any_element(),
                ])
                .into_any_element();
        }
        let mut body: Vec<AnyElement> = vec![
            div()
                .id("step-status")
                .role(Role::Status)
                .text_color(rgb(pal.muted))
                .child(SharedString::from(format!(
                    "Step {step} of 6: {}",
                    STEPS[step as usize - 1]
                )))
                .into_any_element(),
            self.heading(STEPS[step as usize - 1], 1).into_any_element(),
        ];
        match step {
            1 => {
                body.push(div().child("WoW Sync keeps your World of Warcraft add-ons the same on every computer you use. It shares them through your Dropbox folder.").into_any_element());
                body.push(
                    self.muted("Nothing changes in the game until you confirm each step.")
                        .into_any_element(),
                );
            }
            2 => match (&d.dropbox, d.online_only) {
                (None, _) => {
                    body.push(div().child("Dropbox was not found on this computer. Install Dropbox and sign in, then try again.").into_any_element());
                    body.push(
                        self.btn(
                            "try-again",
                            "Try again",
                            Bk::Secondary,
                            Act::Setup(SetupAct::TryAgain),
                            cx,
                        )
                        .into_any_element(),
                    );
                }
                (Some(p), true) => {
                    body.push(div().child(SharedString::from(format!("{p} is set to online-only. Make it available offline in Dropbox, then try again."))).into_any_element());
                    body.push(
                        self.btn(
                            "try-again",
                            "Try again",
                            Bk::Secondary,
                            Act::Setup(SetupAct::TryAgain),
                            cx,
                        )
                        .into_any_element(),
                    );
                }
                (Some(p), false) => {
                    body.push(
                        div()
                            .child(SharedString::from(format!("Dropbox folder found: {p}")))
                            .into_any_element(),
                    );
                    let (l, k) = if self.setup.folder_chosen {
                        ("Shared folder chosen", Bk::Secondary)
                    } else {
                        ("Use this folder", Bk::Primary)
                    };
                    body.push(
                        self.btn(
                            "choose-folder",
                            l,
                            k,
                            Act::Setup(SetupAct::ChooseFolder),
                            cx,
                        )
                        .into_any_element(),
                    );
                }
            },
            3 => {
                body.push(self.muted("Choose the game installation to sync. Each version of the game is separate.").into_any_element());
                let n = d.installs.len();
                for (i, ins) in d.installs.iter().enumerate() {
                    let sel = self.setup.install == i;
                    let a = format!(
                        "{}, build {}, {}{}",
                        ins.identity,
                        ins.build,
                        ins.refused
                            .clone()
                            .unwrap_or_else(|| "can be synced".into()),
                        if sel { ", selected" } else { "" }
                    );
                    let row = self
                        .rov_row(
                            "installs",
                            i,
                            n,
                            &format!("i{i}"),
                            a,
                            Act::Setup(SetupAct::SelectInstall(i)),
                            cx,
                        )
                        .flex()
                        .flex_col()
                        .p(u(10.))
                        .bg(rgb(if sel { pal.selected } else { pal.surface }))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(SharedString::from(format!(
                                    "{} · build {}",
                                    ins.identity, ins.build
                                ))),
                        )
                        .child(self.muted(ins.folder.clone()))
                        .child(match &ins.refused {
                            Some(r) => div()
                                .text_color(rgb(pal.fg(Tone::Help)))
                                .child(SharedString::from(format!("Can't be synced: {r}"))),
                            None if !ins.recognised => div()
                                .text_color(rgb(pal.fg(Tone::Waiting)))
                                .child("Not recognised yet: confirm it below"),
                            None => div()
                                .text_color(rgb(pal.fg(Tone::Ok)))
                                .child("Can be synced"),
                        });
                    body.push(row.into_any_element());
                }
                if d.installs
                    .get(self.setup.install)
                    .is_some_and(|i| i.refused.is_some())
                {
                    body.push(self.muted("This installation was refused. You can go Back and choose another, or Try again after fixing it.").into_any_element());
                    body.push(
                        self.btn(
                            "try-again",
                            "Try again",
                            Bk::Secondary,
                            Act::Setup(SetupAct::TryAgain),
                            cx,
                        )
                        .into_any_element(),
                    );
                }
            }
            4 => {
                if let Some(ins) = d.installs.get(self.setup.install) {
                    body.push(
                        div()
                            .child(SharedString::from(format!(
                                "Confirm this is the right game: {}, build {}.",
                                ins.identity, ins.build
                            )))
                            .into_any_element(),
                    );
                    let a = self.toggle(
                        "consent-identity",
                        &format!("This is my {} installation", ins.identity),
                        self.setup.identity_ok,
                        false,
                        Act::Setup(SetupAct::IdentityToggle),
                        cx,
                    );
                    body.push(a.into_any_element());
                    body.push(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Add-ons that will sync")
                            .into_any_element(),
                    );
                    body.push(
                        self.muted(format!("{} add-ons are eligible.", ins.eligible.len()))
                            .into_any_element(),
                    );
                    for (n, why) in ins.blocked.iter().take(4) {
                        body.push(
                            self.muted(format!("Not synced: {n} ({why})"))
                                .into_any_element(),
                        );
                    }
                    let b = self.toggle(
                        "consent-scope",
                        "These are the add-ons I want to sync",
                        self.setup.scope_ok,
                        false,
                        Act::Setup(SetupAct::ScopeToggle),
                        cx,
                    );
                    body.push(b.into_any_element());
                    if !(self.setup.identity_ok && self.setup.scope_ok) {
                        body.push(self.muted("Confirm both the game version and the add-on list to continue. Scope alone is not enough.").into_any_element());
                    }
                }
            }
            5 => {
                body.push(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Sync group")
                        .into_any_element(),
                );
                let n = d.groups.len() + 1;
                let sel_new = self.setup.group == Some(None);
                body.push(
                    self.rov_row(
                        "groups",
                        0,
                        n,
                        "new",
                        format!(
                            "Create a new group{}",
                            if sel_new { ", selected" } else { "" }
                        ),
                        Act::Setup(SetupAct::CreateGroup),
                        cx,
                    )
                    .p(u(10.))
                    .bg(rgb(if sel_new { pal.selected } else { pal.surface }))
                    .child("Create a new group")
                    .into_any_element(),
                );
                for (i, g) in d.groups.iter().enumerate() {
                    let sel = self.setup.group == Some(Some(i));
                    body.push(
                        self.rov_row(
                            "groups",
                            i + 1,
                            n,
                            &format!("g{i}"),
                            format!(
                                "Join {}, {}, last activity {}{}",
                                g.name,
                                g.identity,
                                g.last_activity,
                                if sel { ", selected" } else { "" }
                            ),
                            Act::Setup(SetupAct::JoinGroup(i)),
                            cx,
                        )
                        .p(u(10.))
                        .flex()
                        .flex_col()
                        .bg(rgb(if sel { pal.selected } else { pal.surface }))
                        .child(SharedString::from(format!("Join {}", g.name)))
                        .child(self.muted(format!(
                            "{} · last activity {}",
                            g.identity, g.last_activity
                        )))
                        .into_any_element(),
                    );
                }
                if matches!(self.setup.group, Some(Some(_))) {
                    body.push(self.muted("Joining adds this computer to an existing group. Its add-ons will be compared before anything is replaced.").into_any_element());
                }
                body.push(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Computer name")
                        .into_any_element(),
                );
                body.push(self.name_field(cx).into_any_element());
            }
            _ => {
                let g = match self.setup.group {
                    Some(Some(i)) => format!(
                        "Join {}",
                        d.groups.get(i).map(|g| g.name.clone()).unwrap_or_default()
                    ),
                    _ => "Create a new group".into(),
                };
                body.push(
                    self.muted(format!("Computer: {}. Group: {g}.", self.setup.name))
                        .into_any_element(),
                );
                if self.setup.working {
                    body.push(
                        div()
                            .flex()
                            .items_center()
                            .gap(u(8.))
                            .text_color(rgb(pal.fg(Tone::Working)))
                            .child(icon(crate::model::Icon::Sync, 16., pal.fg(Tone::Working)))
                            .child("Setting up…")
                            .into_any_element(),
                    );
                } else {
                    body.push(
                        self.btn(
                            "setup",
                            "Set up this computer",
                            Bk::Primary,
                            Act::Setup(SetupAct::SetUp),
                            cx,
                        )
                        .into_any_element(),
                    );
                }
            }
        }
        let working = self.setup.working;
        let mut nav = div().flex().gap(u(10.));
        if step > 1 && !working {
            nav = nav.child(self.btn(
                "wiz-back",
                "Back",
                Bk::Secondary,
                Act::Setup(SetupAct::Back),
                cx,
            ));
        }
        if step < 6 {
            nav = nav.child(if self.setup_can_next() {
                self.btn(
                    "wiz-next",
                    "Next",
                    Bk::Primary,
                    Act::Setup(SetupAct::Next),
                    cx,
                )
            } else {
                self.btn_off("wiz-next", "Next", "complete this step first", cx)
            });
        }
        nav = nav.child(self.btn(
            "wiz-scen",
            "Switch scenario",
            Bk::Link,
            Act::Nav(crate::model::Page::Scenarios),
            cx,
        ));
        body.push(nav.into_any_element());
        self.wizard_frame(body).into_any_element()
    }

    fn wizard_frame(&self, children: Vec<AnyElement>) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .gap(u(14.))
            .max_w(u(640.))
            .mx_auto()
            .py(u(24.))
            .children(children)
    }

    fn name_field(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let shown = if self.setup.name.is_empty() {
            "Type a name".to_string()
        } else {
            self.setup.name.clone()
        };
        self.focusable("name-field", true, cx)
            .role(Role::TextInput)
            .aria_label(SharedString::from(format!(
                "Computer name, {}",
                if self.setup.name.is_empty() {
                    "empty"
                } else {
                    &self.setup.name
                }
            )))
            .px(u(10.))
            .h(u(36.))
            .flex()
            .items_center()
            .bg(rgb(pal.surface))
            .border_color(rgb(pal.control_border))
            .text_color(rgb(if self.setup.name.is_empty() {
                pal.muted
            } else {
                pal.text
            }))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                let k = &ev.keystroke;
                if k.key == "backspace" {
                    this.setup.name.pop();
                } else if let Some(c) = &k.key_char {
                    if !k.modifiers.control
                        && !k.modifiers.platform
                        && this.setup.name.chars().count() < 40
                        && !c.chars().any(|ch| ch.is_control())
                    {
                        this.setup.name.push_str(c);
                    } else {
                        return;
                    }
                } else {
                    return;
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .child(SharedString::from(format!(
                "{shown}{}",
                if self.setup.name.is_empty() { "" } else { "|" }
            )))
            .into_any_element()
    }

    pub fn reconfirm_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(r) = self.w.reconfirm.clone() else {
            return self
                .body_col_pub()
                .child("Nothing to confirm.")
                .into_any_element();
        };
        let back = self.back_link("Overview", cx);
        let mut col = self
            .body_col_pub()
            .child(back)
            .child(self.heading("Confirm the game version", 1))
            .child(div().child(SharedString::from(r.what_changed.clone())))
            .child(div().child(SharedString::from(format!(
                "Installation is now: {}.",
                r.identity
            ))));
        let a = self.toggle(
            "rc-identity",
            &format!("This is still the right game: {}", r.identity),
            self.reconf.identity_ok,
            false,
            Act::Reconfirm(ReconfAct::IdentityToggle),
            cx,
        );
        col = col.child(a);
        col = col.child(self.muted(format!(
            "{} add-ons are eligible, {} newly eligible.",
            r.eligible.len(),
            r.newly_eligible.len()
        )));
        for (n, why) in r.blocked.iter().take(4) {
            col = col.child(self.muted(format!("Not synced: {n} ({why})")));
        }
        let b = self.toggle(
            "rc-scope",
            "The add-on list is right",
            self.reconf.scope_ok,
            false,
            Act::Reconfirm(ReconfAct::ScopeToggle),
            cx,
        );
        col = col.child(b);
        if let Some(n) = self.reconf.note.clone() {
            col = col.child(
                div()
                    .id("rc-note")
                    .role(Role::Status)
                    .aria_label(SharedString::from(n.clone()))
                    .child(SharedString::from(n)),
            );
        }
        let c = self.btn(
            "rc-confirm",
            "Confirm and continue",
            Bk::Primary,
            Act::Reconfirm(ReconfAct::Confirm),
            cx,
        );
        let n = self.btn(
            "rc-notnow",
            "Not now",
            Bk::Secondary,
            Act::Reconfirm(ReconfAct::NotNow),
            cx,
        );
        col.child(div().flex().gap(u(10.)).child(c).child(n))
            .into_any_element()
    }

    pub fn body_col_pub(&self) -> gpui::Div {
        div().flex().flex_col().gap(u(14.)).max_w(u(880.))
    }
}
