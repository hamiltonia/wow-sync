//! First-run wizard (P1) and the re-confirmation page (P9).

use crate::app::{Act, Bk, ReconfAct, SetupAct, Shell};
use crate::theme::Tone;
use crate::ui::{icon, u};
use gpui::accesskit::Role;
use gpui::{div, prelude::*, rgb, AnyElement, Context, FontWeight, SharedString};

const STEPS: [&str; 6] = [
    "Welcome",
    "Shared folder",
    "Game installation",
    "Confirm what to sync",
    "Sync group",
    "Review and set up",
];

impl Shell {
    /// Consent needs both the exact game version and the add-on list; scope alone is not enough.
    pub fn setup_can_next(&self) -> bool {
        let Some(d) = &self.w.setup else { return true };
        let s = &self.setup;
        match s.step {
            2 => d.dropbox.is_some() && !d.online_only && s.folder_chosen,
            3 => d
                .installs
                .get(s.install)
                .is_some_and(|i| i.inspected && i.refused.is_none()),
            4 => s.consent.identity && s.consent.scope,
            5 => s.group.is_some(),
            _ => true,
        }
    }

    pub fn setup_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let d = self.w.setup.clone().unwrap_or_default();
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
                            d.computer
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
                    let build = if ins.build.is_empty() {
                        String::new()
                    } else {
                        format!(", build {}", ins.build)
                    };
                    let a = format!(
                        "{}{}, {}{}",
                        ins.identity,
                        build,
                        match (&ins.refused, ins.inspected) {
                            (Some(r), _) => r.clone(),
                            (None, true) => "can be synced".into(),
                            (None, false) => "not checked yet".into(),
                        },
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
                                    "{}{}",
                                    ins.identity,
                                    if ins.build.is_empty() {
                                        String::new()
                                    } else {
                                        format!(" · build {}", ins.build)
                                    }
                                ))),
                        )
                        .child(self.muted(ins.folder.clone()))
                        .child(match &ins.refused {
                            Some(r) => div()
                                .text_color(rgb(pal.fg(Tone::Help)))
                                .child(SharedString::from(format!("Can't be synced. {r}"))),
                            None if !ins.inspected => div()
                                .text_color(rgb(pal.muted))
                                .child("Checking this installation…"),
                            None if !ins.recognised => div()
                                .text_color(rgb(pal.fg(Tone::Waiting)))
                                .child("Not recognised yet: confirm it below"),
                            None => div()
                                .text_color(rgb(pal.fg(Tone::Ok)))
                                .child("Can be synced"),
                        });
                    body.push(row.into_any_element());
                }
                if let Some(ins) = d
                    .installs
                    .get(self.setup.install)
                    .filter(|i| i.refused.is_some())
                {
                    if ins.can_retry {
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
                    } else {
                        body.push(
                            self.muted("Trying again will not change this. Choose Back to pick another installation.")
                                .into_any_element(),
                        );
                    }
                }
            }
            4 => {
                if let Some(ins) = d.installs.get(self.setup.install).filter(|i| i.inspected) {
                    body.push(
                        div()
                            .child(SharedString::from(format!(
                                "Confirm this is the right game: {}{}.",
                                ins.identity,
                                if ins.build.is_empty() {
                                    String::new()
                                } else {
                                    format!(", build {}", ins.build)
                                }
                            )))
                            .into_any_element(),
                    );
                    let a = self.toggle(
                        "consent-identity",
                        &format!("This is my {} installation", ins.identity),
                        self.setup.consent.identity,
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
                        self.muted(format!(
                            "{} add-ons are eligible. {} cannot be synced.",
                            ins.eligible.len(),
                            ins.blocked.len()
                        ))
                        .into_any_element(),
                    );
                    for n in ins.blocked.iter().take(4) {
                        body.push(self.muted(format!("Not synced: {n}")).into_any_element());
                    }
                    if ins.blocked.len() > 4 {
                        body.push(
                            self.muted(format!(
                                "and {} more that cannot be synced",
                                ins.blocked.len() - 4
                            ))
                            .into_any_element(),
                        );
                    }
                    let b = self.toggle(
                        "consent-scope",
                        "These are the add-ons I want to sync",
                        self.setup.consent.scope,
                        false,
                        Act::Setup(SetupAct::ScopeToggle),
                        cx,
                    );
                    body.push(b.into_any_element());
                    if !(self.setup.consent.identity && self.setup.consent.scope) {
                        body.push(self.muted("Confirm both the game version and the add-on list to continue. Scope alone is not enough.").into_any_element());
                    }
                } else {
                    body.push(
                        self.muted("Reading this installation. Nothing is changed.")
                            .into_any_element(),
                    );
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
                                "Join {}, {}{}",
                                g.name,
                                g.identity,
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
                        .child(self.muted(g.identity.clone()))
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
                body.push(
                    self.muted(format!("{} (this computer's name)", d.computer))
                        .into_any_element(),
                );
            }
            _ => {
                let g = match self.setup.group {
                    Some(Some(i)) => format!(
                        "Join {}",
                        d.groups.get(i).map(|g| g.name.clone()).unwrap_or_default()
                    ),
                    _ => format!("Create a new group, {}", self.setup.group_name),
                };
                body.push(
                    self.muted(format!("Computer: {}. Group: {g}.", d.computer))
                        .into_any_element(),
                );
                for line in &self.preview_disclosures() {
                    body.push(self.muted(line.clone()).into_any_element());
                }
                if let Some(m) = self.setup_message() {
                    body.push(
                        div()
                            .id("setup-message")
                            .role(Role::Alert)
                            .aria_label(SharedString::from(m.clone()))
                            .text_color(rgb(pal.fg(Tone::Help)))
                            .child(SharedString::from(m))
                            .into_any_element(),
                    );
                }
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
            self.reconf.consent.identity,
            false,
            Act::Reconfirm(ReconfAct::IdentityToggle),
            cx,
        );
        col = col.child(a);
        col = col.child(self.muted(format!(
            "{} add-ons are eligible. {} cannot be synced.",
            r.eligible.len(),
            r.blocked.len()
        )));
        for n in r.blocked.iter().take(4) {
            col = col.child(self.muted(format!("Not synced: {n}")));
        }
        if r.blocked.len() > 4 {
            col = col.child(self.muted(format!(
                "and {} more that cannot be synced",
                r.blocked.len() - 4
            )));
        }
        let b = self.toggle(
            "rc-scope",
            "The add-on list is right",
            self.reconf.consent.scope,
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
