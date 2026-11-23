//! First-run wizard (P1) and the re-confirmation page (P9).

use crate::app::{Act, Bk, ReconfAct, SetupAct, Shell};
use crate::theme::Tone;
use crate::ui::{icon, u};
use crate::world::SetupData;
use gpui::accesskit::Role;
use gpui::{div, prelude::*, rgb, AnyElement, Context, FontWeight, SharedString};
use std::path::Path;

const STEPS: [&str; 6] = [
    "Welcome",
    "Shared folder",
    "Game installation",
    "Confirm what to sync",
    "Sync group",
    "Review and set up",
];

pub const NOT_FOUND_TEXT: &str = "World of Warcraft was not found automatically. Choose the folder where it is installed (it contains folders like _retail_ or _forever_).";

/// Shown when a chosen folder is neither an install root nor a variant folder.
pub fn not_a_game_folder_text(path: &Path) -> String {
    format!(
        "{} is not a World of Warcraft folder. Choose the folder where it is installed (it contains folders like _retail_ or _forever_).",
        path.display()
    )
}

fn same_path(a: &Path, b: &Path) -> bool {
    let k = |p: &Path| p.to_string_lossy().to_lowercase().replace('/', "\\");
    k(a).trim_end_matches('\\') == k(b).trim_end_matches('\\')
}

/// Records a finished check in the row of the installation it was made for (matched by folder
/// and root, never by which row is selected) and returns that row's index. A result for an
/// installation that is not listed changes nothing.
pub fn fold_preview(
    d: &mut SetupData,
    p: &wow_sync_runtime::EnrollPreview,
    root: &Path,
    folder: &str,
) -> Option<usize> {
    use wow_sync_runtime::PreviewKind;
    let at = d
        .installs
        .iter()
        .position(|i| i.folder == folder && i.root == root)?;
    let ins = &mut d.installs[at];
    ins.inspected = true;
    ins.identity = if p.identity_line.is_empty() {
        ins.folder.clone()
    } else {
        p.identity_line.clone()
    };
    ins.recognised = p.kind == PreviewKind::Recognized;
    ins.refused = (p.kind == PreviewKind::Refused)
        .then(|| p.problem.clone().unwrap_or_else(|| p.identity_line.clone()));
    ins.can_retry = p.retryable;
    ins.test_mode_blocked = p.test_mode_blocked;
    ins.eligible = p.eligible.clone();
    ins.blocked = p.blocked.clone();
    Some(at)
}

/// Rows whose variant folder has a `.flavor.info` come first, so the row selected by default
/// is one the check can accept. A refused row stays listed, with its reason, after the
/// others; the order is otherwise kept.
pub fn usable_first(installs: &mut [crate::world::Install]) {
    installs.sort_by_key(|i| !i.root.join(&i.folder).join(".flavor.info").is_file());
}

/// "Checking add-ons… 12,000 files": progress of a long read-only check.
pub fn checking_text(files: u64) -> String {
    if files == 0 {
        return "Checking add-ons…".into();
    }
    let digits = files.to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(c);
    }
    format!("Checking add-ons… {grouped} files")
}

/// Warning shown for a chosen folder Dropbox does not report. It does not block: Dropbox may
/// be installed in an unusual way.
pub const NOT_DROPBOX_TEXT: &str = "This folder is not a Dropbox folder that WoW Sync can see. Sync only works if every computer uses the same Dropbox folder.";

/// What looking at a picked folder found. Built off the window thread, because the folder may
/// be on a slow disk or a network share.
pub enum Checked {
    Game(Option<(wow_sync_runtime::DetectedInstall, Option<String>)>),
    Dropbox { known: bool },
}

pub fn check_picked(game: bool, chosen: &Path) -> Checked {
    if game {
        Checked::Game(wow_sync_runtime::install_from_choice(chosen))
    } else {
        Checked::Dropbox {
            known: wow_sync_runtime::is_dropbox_folder(
                chosen,
                &wow_sync_runtime::dropbox_info_files(),
                &wow_sync_runtime::dropbox_defaults(),
            ),
        }
    }
}

/// Adds the game folder a person chose to the list, as an install root or as one of its
/// `_variant_` folders, and returns the row to select. The row is only a candidate: it still
/// goes through the read-only check and the identity and scope confirmation.
pub fn add_found_install(
    d: &mut SetupData,
    chosen: &Path,
    found: Option<(wow_sync_runtime::DetectedInstall, Option<String>)>,
) -> Result<usize, String> {
    let (found, variant) = found.ok_or_else(|| not_a_game_folder_text(chosen))?;
    for v in &found.variants {
        if !d
            .installs
            .iter()
            .any(|i| same_path(&i.root, &found.root) && &i.folder == v)
        {
            d.installs.push(crate::install_row(&found.root, v));
        }
    }
    let in_root = |i: &crate::world::Install| same_path(&i.root, &found.root);
    if variant.is_none() {
        // A chosen root selects a row the check can accept before a refused one.
        let usable = d
            .installs
            .iter()
            .position(|i| in_root(i) && i.root.join(&i.folder).join(".flavor.info").is_file());
        if let Some(at) = usable {
            return Ok(at);
        }
    }
    d.installs
        .iter()
        .position(|i| in_root(i) && variant.as_ref().is_none_or(|v| &i.folder == v))
        .ok_or_else(|| not_a_game_folder_text(chosen))
}

/// Uses a Dropbox folder the person chose. The shared folder stays `<Dropbox>\WoW Sync`.
pub fn apply_dropbox_choice(d: &mut SetupData, chosen: &Path) {
    d.dropbox = Some(chosen.display().to_string());
    d.shared_root = Some(chosen.join("WoW Sync"));
    d.online_only = false;
}

impl Shell {
    /// "Checking…" while a picked folder is being looked at, or what was found about it.
    fn pick_status(&self, pal: &crate::theme::Palette) -> Vec<AnyElement> {
        if self.setup.checking {
            return vec![div()
                .id("pick-checking")
                .role(Role::Status)
                .aria_label("Checking the folder you chose")
                .text_color(rgb(pal.muted))
                .child("Checking…")
                .into_any_element()];
        }
        self.setup
            .pick_note
            .clone()
            .map(|note| {
                div()
                    .id("pick-note")
                    .role(Role::Alert)
                    .aria_label(SharedString::from(note.clone()))
                    .text_color(rgb(pal.fg(Tone::Help)))
                    .child(SharedString::from(note))
                    .into_any_element()
            })
            .into_iter()
            .collect()
    }
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
        body.extend(self.pick_status(&pal));
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
                    body.push(div().child("Dropbox was not found on this computer. Install Dropbox and sign in, then try again. If Dropbox is somewhere unusual, choose its folder.").into_any_element());
                    body.push(
                        div()
                            .flex()
                            .gap(u(10.))
                            .child(self.btn(
                                "try-again",
                                "Try again",
                                Bk::Secondary,
                                Act::Setup(SetupAct::TryAgain),
                                cx,
                            ))
                            .child(self.btn(
                                "pick-dropbox",
                                "Choose your Dropbox folder…",
                                Bk::Secondary,
                                Act::Setup(SetupAct::PickDropboxFolder),
                                cx,
                            ))
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
                            None if !ins.inspected => {
                                let live = self
                                    .setup
                                    .inspecting
                                    .as_ref()
                                    .filter(|(r, f, _)| *r == ins.root && *f == ins.folder);
                                match live {
                                    Some((_, _, n)) => div()
                                        .text_color(rgb(pal.muted))
                                        .child(SharedString::from(checking_text(*n))),
                                    None => div()
                                        .text_color(rgb(pal.muted))
                                        .child("Not checked yet. Choose Check again."),
                                }
                            }
                            None if !ins.recognised => div()
                                .text_color(rgb(pal.fg(Tone::Waiting)))
                                .child("Not recognised yet: confirm it below"),
                            None => div()
                                .text_color(rgb(pal.fg(Tone::Ok)))
                                .child("Can be synced"),
                        });
                    body.push(row.into_any_element());
                }
                if d.installs.is_empty() {
                    body.push(
                        div()
                            .id("no-installs")
                            .role(Role::Status)
                            .aria_label(SharedString::from(NOT_FOUND_TEXT))
                            .child(NOT_FOUND_TEXT)
                            .into_any_element(),
                    );
                }
                body.push(
                    self.btn(
                        "pick-game",
                        "Choose folder…",
                        if d.installs.is_empty() {
                            Bk::Primary
                        } else {
                            Bk::Secondary
                        },
                        Act::Setup(SetupAct::PickGameFolder),
                        cx,
                    )
                    .into_any_element(),
                );
                if let Some(ins) = d.installs.get(self.setup.install).filter(|i| !i.inspected) {
                    let running = self
                        .setup
                        .inspecting
                        .as_ref()
                        .is_some_and(|(r, f, _)| *r == ins.root && *f == ins.folder);
                    let (id, label, act) = if running {
                        ("cancel-check", "Cancel check", SetupAct::CancelCheck)
                    } else {
                        ("check-again", "Check again", SetupAct::TryAgain)
                    };
                    body.push(
                        self.btn(id, label, Bk::Secondary, Act::Setup(act), cx)
                            .into_any_element(),
                    );
                }
                if let Some(ins) = d
                    .installs
                    .get(self.setup.install)
                    .filter(|i| i.refused.is_some())
                {
                    if ins.test_mode_blocked {
                        body.push(
                            self.btn(
                                "turn-off-test-mode",
                                "Turn off test mode",
                                Bk::Secondary,
                                Act::Setup(SetupAct::TurnOffTestMode),
                                cx,
                            )
                            .into_any_element(),
                        );
                    }
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
                    } else if !ins.test_mode_blocked {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn lab(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("wowsync-setup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn empty() -> SetupData {
        SetupData::default()
    }

    fn add_chosen_install(d: &mut SetupData, chosen: &Path) -> Result<usize, String> {
        match check_picked(true, chosen) {
            Checked::Game(found) => add_found_install(d, chosen, found),
            Checked::Dropbox { .. } => unreachable!(),
        }
    }

    #[test]
    fn the_empty_state_says_what_to_do_in_plain_words() {
        assert_eq!(
            NOT_FOUND_TEXT,
            "World of Warcraft was not found automatically. Choose the folder where it is installed (it contains folders like _retail_ or _forever_)."
        );
        let p = PathBuf::from("G:\\Stuff");
        let t = not_a_game_folder_text(&p);
        assert!(t.starts_with("G:\\Stuff is not a World of Warcraft folder."));
        assert!(t.contains("_retail_ or _forever_"));
    }

    #[test]
    fn a_chosen_install_root_adds_its_variants_and_selects_the_first() {
        let t = lab("root");
        let wow = t.join("World of Warcraft");
        std::fs::create_dir_all(wow.join("_classic_era_")).unwrap();
        std::fs::create_dir_all(wow.join("_forever_")).unwrap();
        let mut d = empty();
        let i = add_chosen_install(&mut d, &wow).unwrap();
        assert_eq!(d.installs.len(), 2);
        assert_eq!(d.installs[i].folder, "_classic_era_");
        assert_eq!(d.installs[i].root, wow);
        assert!(!d.installs[i].inspected, "the choice is only a candidate");
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn a_chosen_variant_folder_uses_its_parent_and_selects_that_variant() {
        let t = lab("variant");
        let wow = t.join("World of Warcraft");
        std::fs::create_dir_all(wow.join("_classic_era_")).unwrap();
        std::fs::create_dir_all(wow.join("_forever_")).unwrap();
        let mut d = empty();
        let i = add_chosen_install(&mut d, &wow.join("_forever_")).unwrap();
        assert_eq!(d.installs[i].folder, "_forever_");
        assert_eq!(d.installs[i].root, wow);
        let again = add_chosen_install(&mut d, &wow.join("_forever_")).unwrap();
        assert_eq!((again, d.installs.len()), (i, 2), "no duplicate rows");
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn a_folder_with_no_game_in_it_is_refused_with_its_path() {
        let t = lab("refuse");
        std::fs::create_dir_all(t.join("Data")).unwrap();
        let mut d = empty();
        let e = add_chosen_install(&mut d, &t).unwrap_err();
        assert!(e.contains(&t.display().to_string()) && e.contains("not a World of Warcraft"));
        assert!(add_chosen_install(&mut d, &t.join("missing")).is_err());
        assert!(add_chosen_install(&mut d, &t.join("Data")).is_err());
        assert!(d.installs.is_empty());
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn a_picked_folder_not_reported_by_dropbox_is_a_warning_not_a_refusal() {
        let t = lab("notdropbox");
        let Checked::Dropbox { known } = check_picked(false, &t.join("Elsewhere")) else {
            panic!("a Dropbox check was asked for");
        };
        assert!(!known);
        assert!(NOT_DROPBOX_TEXT.starts_with("This folder is not a Dropbox folder"));
        let mut d = empty();
        apply_dropbox_choice(&mut d, &t.join("Elsewhere"));
        assert!(d.dropbox.is_some() && !d.online_only);
    }

    #[test]
    fn a_chosen_root_selects_its_usable_variant_not_a_refused_one_listed_first() {
        let t = lab("pickroot");
        let wow = t.join("World of Warcraft");
        std::fs::create_dir_all(wow.join("_classic_beta_")).unwrap();
        std::fs::create_dir_all(wow.join("_forever_")).unwrap();
        std::fs::write(wow.join("_forever_").join(".flavor.info"), "x").unwrap();
        let mut d = empty();
        let found = wow_sync_runtime::DetectedInstall {
            root: wow.clone(),
            variants: vec!["_classic_beta_".into(), "_forever_".into()],
        };
        let at = add_found_install(&mut d, &wow, Some((found, None))).unwrap();
        assert_eq!(d.installs[at].folder, "_forever_");
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn a_result_lands_only_on_its_own_row_and_an_unlisted_target_changes_nothing() {
        let t = lab("fold");
        let wow = t.join("World of Warcraft");
        let mut d = empty();
        d.installs = vec![
            crate::install_row(&wow, "_classic_beta_"),
            crate::install_row(&wow, "_forever_"),
        ];
        let preview = wow_sync_runtime::EnrollPreview {
            kind: wow_sync_runtime::PreviewKind::Recognized,
            identity_line: "Forever".into(),
            scope_line: String::new(),
            eligible: vec!["Alpha".into()],
            blocked: vec![],
            digest: None,
            needs_guided_confirmation: false,
            groups: vec![],
            problem: None,
            retryable: false,
            test_mode_blocked: false,
            disclosures: vec![],
        };
        // Row 0 is the selected one when the result for row 1 arrives.
        assert_eq!(fold_preview(&mut d, &preview, &wow, "_forever_"), Some(1));
        assert!(!d.installs[0].inspected && d.installs[0].eligible.is_empty());
        assert!(d.installs[1].inspected && d.installs[1].recognised);
        assert_eq!(d.installs[1].eligible, ["Alpha"]);
        let before = format!("{:?}", d.installs);
        assert_eq!(fold_preview(&mut d, &preview, &wow, "_retail_"), None);
        assert_eq!(
            fold_preview(&mut d, &preview, &t.join("other"), "_forever_"),
            None
        );
        assert_eq!(format!("{:?}", d.installs), before);
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn a_row_that_starts_a_new_check_drops_what_the_old_one_said() {
        let t = lab("recheck");
        let wow = t.join("World of Warcraft");
        let mut row = crate::install_row(&wow, "_forever_");
        row.inspected = true;
        row.refused = Some("old reason".into());
        row.can_retry = true;
        row.test_mode_blocked = true;
        row.start_checking();
        assert!(!row.inspected && row.refused.is_none());
        assert!(!row.can_retry && !row.test_mode_blocked);
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn a_usable_install_is_listed_and_selected_before_a_refused_one() {
        let t = lab("usable");
        let wow = t.join("World of Warcraft");
        std::fs::create_dir_all(wow.join("_classic_beta_")).unwrap();
        std::fs::create_dir_all(wow.join("_forever_")).unwrap();
        std::fs::write(wow.join("_forever_").join(".flavor.info"), "Product: wow\n").unwrap();
        // Detection lists variant folders alphabetically, so the refused one comes first.
        let mut rows = vec![
            crate::install_row(&wow, "_classic_beta_"),
            crate::install_row(&wow, "_forever_"),
        ];
        usable_first(&mut rows);
        assert_eq!(rows[0].folder, "_forever_", "selected by default (row 0)");
        assert_eq!(
            rows[1].folder, "_classic_beta_",
            "the refused row stays listed"
        );
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn long_checks_show_how_many_files_were_read() {
        assert_eq!(checking_text(0), "Checking add-ons…");
        assert_eq!(checking_text(950), "Checking add-ons… 950 files");
        assert_eq!(checking_text(12_000), "Checking add-ons… 12,000 files");
        assert_eq!(
            checking_text(1_234_567),
            "Checking add-ons… 1,234,567 files"
        );
    }

    #[test]
    fn a_chosen_dropbox_folder_keeps_the_shared_folder_inside_it() {
        let mut d = empty();
        d.online_only = true;
        let root = Path::new("dropbox").join("My Dropbox");
        apply_dropbox_choice(&mut d, &root);
        assert_eq!(d.dropbox.as_deref(), root.to_str());
        assert_eq!(d.shared_root, Some(root.join("WoW Sync")));
        assert!(!d.online_only);
    }
}
