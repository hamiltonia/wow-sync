//! App state, actions, tray and the window shell. Pages live in `pages.rs` and `setup.rs`;
//! wording and derivations live in the pure `model.rs`; data comes from the runtime through
//! `backend.rs` and `world.rs`.

use crate::backend::{self, Cmd, SharedState, Target};
use crate::consent::Consent;
use crate::model::{
    self, banners, headline, nav_name, need_items, window_title, BannerAction, Detail, Filter,
    NeedTarget, Page, NAV,
};
use crate::theme::{Mode, Palette, ThemeChoice, Tone};
use crate::ui::{icon, u, zoom};
use crate::win;
use crate::world::{Busy, Inputs, Kind, World};
use gpui::accesskit::{Role, Toggled};
use gpui::{
    actions, div, prelude::*, rgb, transparent_black, AnyElement, App, Context, Div, FocusHandle,
    FontWeight, KeyDownEvent, Render, ScrollHandle, SharedString, Stateful, Window,
    WindowAppearance,
};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::time::Duration;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

actions!(
    wow_sync,
    [
        FocusNextStop,
        FocusPrevStop,
        RegionNext,
        RegionPrev,
        Go1,
        Go2,
        Go3,
        Go4,
        Go5,
        Go6,
        GoSettings,
        SyncKey,
        PauseKey,
        CloseKey,
        QuitKey,
        EscKey,
        BackKey
    ]
);

pub fn bind_keys(cx: &mut App) {
    use gpui::KeyBinding as K;
    cx.bind_keys([
        K::new("tab", FocusNextStop, None),
        K::new("shift-tab", FocusPrevStop, None),
        K::new("f6", RegionNext, None),
        K::new("shift-f6", RegionPrev, None),
        K::new("ctrl-1", Go1, None),
        K::new("ctrl-2", Go2, None),
        K::new("ctrl-3", Go3, None),
        K::new("ctrl-4", Go4, None),
        K::new("ctrl-5", Go5, None),
        K::new("ctrl-6", Go6, None),
        K::new("ctrl-,", GoSettings, None),
        K::new("f5", SyncKey, None),
        K::new("ctrl-r", SyncKey, None),
        K::new("ctrl-shift-p", PauseKey, None),
        K::new("ctrl-w", CloseKey, None),
        K::new("ctrl-q", QuitKey, None),
        K::new("escape", EscKey, None),
        K::new("alt-left", BackKey, None),
    ]);
}

#[derive(Clone, Debug, PartialEq)]
pub enum Sheet {
    FirstClose,
    Quit(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    None,
    Nav(Page),
    Back,
    SyncNow,
    TogglePause,
    SetFilter(Filter),
    SetActFilter(u8),
    OpenAddon(String),
    OpenComputer(usize),
    OpenDecision(String),
    OpenNeedsHelp(String),
    OpenReconfirm,
    NeedsAddonsFiltered,
    TryAgainShared,
    ToggleExpand(String),
    Toggle(String),
    TurnOffTestMode,
    Theme(ThemeChoice),
    SheetHide,
    SheetCancel,
    SheetQuit,
    SheetKeepRunning,
    SheetQuitWhenSafe,
    Quit,
    ExplainCloseAgain,
    Setup(SetupAct),
    Reconfirm(ReconfAct),
    CopyVersion,
    /// Open this instance's data (state) folder in the file manager.
    OpenDataFolder,
    /// Open the shared folder in the file manager.
    OpenSharedFolder,
    DismissNotice,
    /// A group command from a feature screen, sent to the sync thread as it is.
    Group(crate::features::GroupAction),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SetupAct {
    Next,
    Back,
    ChooseFolder,
    /// Open the folder picker for the game folder (step 3).
    PickGameFolder,
    /// Open the folder picker for the Dropbox folder (step 2).
    PickDropboxFolder,
    TryAgain,
    /// Stop the read-only check of the selected installation.
    CancelCheck,
    TurnOffTestMode,
    SelectInstall(usize),
    IdentityToggle,
    ScopeToggle,
    CreateGroup,
    JoinGroup(usize),
    SetUp,
    Finish,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReconfAct {
    IdentityToggle,
    ScopeToggle,
    Confirm,
    NotNow,
}

#[derive(Clone, Debug, Default)]
pub struct SetupState {
    pub step: u8,
    pub install: usize,
    pub folder_chosen: bool,
    pub consent: Consent,
    pub group: Option<Option<usize>>,
    pub group_name: String,
    pub working: bool,
    pub finished: bool,
    /// Why the last folder a person chose was not accepted.
    pub pick_note: Option<String>,
    /// A chosen folder is being looked at off the window thread.
    pub checking: bool,
    /// The read-only check of an installation that is still running: its folder and how many
    /// files it has read so far.
    pub inspecting: Option<(std::path::PathBuf, String, u64)>,
}

#[derive(Clone, Debug, Default)]
pub struct ReconfState {
    pub consent: Consent,
    pub not_now: bool,
    pub note: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Bk {
    Primary,
    Secondary,
    Link,
}

pub struct TrayState {
    pub icon: TrayIcon,
    pub key: String,
    pub open: MenuId,
    pub sync: MenuId,
    pub pause: MenuId,
    pub needs: Option<MenuId>,
    pub quit: MenuId,
}

pub struct Args {
    pub theme: ThemeChoice,
    pub page: Option<Page>,
    pub detail: Option<String>,
    pub sheet: Option<String>,
    pub filter: Option<Filter>,
    pub setup_step: Option<u8>,
    pub start_hidden: bool,
    pub hide_after: Option<u64>,
    pub dev: Option<Target>,
}

pub struct Shell {
    pub w: World,
    pub boot: Option<Args>,
    pub shared: SharedState,
    pub tx: Option<Sender<Cmd>>,
    pub seen_version: u64,
    pub loaded: bool,
    pub message: String,
    seen_group_result: u64,
    seen_test_mode: u64,
    /// The last blocked-add-on list written to the log, so each check does not repeat it.
    logged_preview: String,
    pub diag: crate::about_ui::DiagState,
    pub preview_lines: Vec<String>,
    pub app_version: String,
    pub closing_quits: bool,
    pub quit_pending: bool,
    pub page: Page,
    pub detail: Detail,
    pub detail_stack: Vec<(Page, Detail)>,
    pub focus_stack: Vec<String>,
    pub filter: Filter,
    pub act_filter: u8,
    pub theme: ThemeChoice,
    pub system: Mode,
    pub sheet: Option<Sheet>,
    pub sheet_from: Option<String>,
    pub in_sheet: bool,
    pub first_close_done: bool,
    pub expanded: HashSet<String>,
    pub setup: SetupState,
    pub reconf: ReconfState,
    pub handles: HashMap<String, FocusHandle>,
    pub rov: HashMap<String, usize>,
    pub rov_keys: HashMap<String, Vec<String>>,
    pub scrolls: HashMap<String, ScrollHandle>,
    pub pending_focus: Option<String>,
    pub content_first: Option<String>,
    pub capture_first: bool,
    pub last_key: String,
    /// Key of the control that had focus in the latest frame with no sheet open.
    pub opener: String,
    pub hwnd: isize,
    pub visible: bool,
    pub tray: Option<TrayState>,
    pub last_title: String,
    pub notice: crate::notice::Notices,
    pub copied: bool,
    pub region: usize,
    pub window_w: f32,
    pub window_h: f32,
}

/// What the log may say about an action: its kind only, never the names, keys or paths in it.
pub fn act_label(act: &Act) -> String {
    if let Act::Group(a) = act {
        return format!("Group {:?}", a.kind());
    }
    let d = format!("{act:?}");
    d.split(['(', ' ', '{'])
        .next()
        .unwrap_or_default()
        .to_string()
}

pub fn log(msg: &str) {
    crate::diag_log::write(crate::diag_log::Level::Info, msg);
}

pub fn log_warn(msg: &str) {
    crate::diag_log::write(crate::diag_log::Level::Warn, msg);
}

pub fn log_error(msg: &str) {
    crate::diag_log::write(crate::diag_log::Level::Error, msg);
}

#[cfg(windows)]
pub fn hwnd_of(window: &Window) -> isize {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match HasWindowHandle::window_handle(window).map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Win32(h)) => h.hwnd.get(),
        _ => 0,
    }
}

#[cfg(not(windows))]
pub fn hwnd_of(_window: &Window) -> isize {
    0
}

fn mode_of(a: WindowAppearance) -> Mode {
    match a {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Mode::Dark,
        _ => Mode::Light,
    }
}

impl Shell {
    pub fn new(
        args: Args,
        shared: SharedState,
        tx: Option<Sender<Cmd>>,
        show_event: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let app_version = env!("CARGO_PKG_VERSION").to_string();
        let (start_hidden, hide_after, theme) = (args.start_hidden, args.hide_after, args.theme);
        let mut app = Shell {
            w: World::empty(&app_version),
            boot: Some(args),
            shared,
            tx,
            seen_version: u64::MAX,
            loaded: false,
            message: String::new(),
            seen_group_result: 0,
            seen_test_mode: 0,
            logged_preview: String::new(),
            diag: Default::default(),
            preview_lines: Vec::new(),
            app_version,
            closing_quits: false,
            quit_pending: false,
            page: Page::Overview,
            detail: Detail::None,
            detail_stack: Vec::new(),
            focus_stack: Vec::new(),
            filter: Filter::All,
            act_filter: 0,
            theme,
            system: mode_of(window.appearance()),
            sheet: None,
            sheet_from: None,
            in_sheet: false,
            first_close_done: false,
            expanded: HashSet::new(),
            setup: SetupState::default(),
            reconf: ReconfState::default(),
            handles: HashMap::new(),
            rov: HashMap::new(),
            rov_keys: HashMap::new(),
            scrolls: HashMap::new(),
            pending_focus: None,
            content_first: None,
            capture_first: false,
            last_key: String::new(),
            opener: String::new(),
            hwnd: hwnd_of(window),
            visible: true,
            tray: None,
            last_title: String::new(),
            notice: crate::notice::Notices::default(),
            copied: false,
            region: 0,
            window_w: 1040.0,
            window_h: 720.0,
        };
        app.reset_ui();
        app.pull(cx);
        app.pending_focus = Some("@content".into());

        let hwnd = app.hwnd;
        let entity = cx.entity();
        window.on_window_should_close(cx, move |_, cx| {
            entity.update(cx, |t, cx| t.request_close(cx));
            let _ = hwnd;
            false
        });
        cx.observe_window_appearance(window, |t, window, cx| {
            t.system = mode_of(window.appearance());
            log(&format!("appearance changed: system={:?}", t.system));
            cx.notify();
        })
        .detach();
        app.make_tray();
        if start_hidden {
            app.visible = false;
        }
        let started = std::time::Instant::now();
        cx.spawn(async move |this, cx| {
            let mut hid = false;
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                let mut evs: Vec<TrayEv> = Vec::new();
                while let Ok(e) = TrayIconEvent::receiver().try_recv() {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = e
                    {
                        evs.push(TrayEv::Click);
                    }
                }
                while let Ok(e) = MenuEvent::receiver().try_recv() {
                    evs.push(TrayEv::Menu(e.id));
                }
                if crate::win::show_requested(show_event) {
                    evs.push(TrayEv::Click);
                }
                let hide_now =
                    !hid && hide_after.is_some_and(|s| started.elapsed() >= Duration::from_secs(s));
                hid |= hide_now;
                if this
                    .update(cx, |t, cx| {
                        t.pull(cx);
                        for e in evs {
                            t.tray_event(e, cx);
                        }
                        if hide_now {
                            t.hide();
                        }
                    })
                    .is_err()
                {
                    break;
                }
                if let Some(show) = crate::win::take_visibility_request() {
                    cx.update(|app| {
                        if show {
                            app.activate(true);
                        } else {
                            app.hide();
                        }
                    });
                }
            }
        })
        .detach();
        app
    }

    pub fn pal(&self) -> Palette {
        Palette::of(self.theme.resolve(self.system))
    }

    // ---- runtime data ----

    pub fn reset_ui(&mut self) {
        self.page = Page::Overview;
        self.detail = Detail::None;
        self.detail_stack.clear();
        self.focus_stack.clear();
        self.filter = Filter::All;
        self.act_filter = 0;
        self.sheet = None;
        self.expanded.clear();
        self.rov.clear();
        self.notice.clear();
        self.reconf = ReconfState::default();
        self.setup = SetupState {
            step: 1,
            group_name: "My add-ons".into(),
            ..SetupState::default()
        };
        self.pending_focus = Some("@content".into());
    }

    pub fn send(&self, c: Cmd) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(c);
        }
    }

    /// Read what the sync thread has published and rebuild the view-model from it.
    pub fn pull(&mut self, cx: &mut Context<Self>) {
        let (w, msg, setup_ready, quit_ready, enrolled_now, settings, preview, inspecting) = {
            let g = backend::lock(&self.shared);
            if g.version == self.seen_version {
                return;
            }
            self.seen_version = g.version;
            let setup = self.w.setup.clone().or_else(|| g.setup.clone());
            let mut w = World::from_snapshot(Inputs {
                snap: &g.snapshot,
                paused_since_ms: g.paused_since,
                pause_damaged: g.pause_damaged,
                activity: &g.activity,
                syncing: g.busy,
                last_check_ms: g.last_check_ms,
                next_check_ms: g.next_check_ms,
                setup,
                app_version: &self.app_version,
            });
            w.confirm_enrolled(g.enrolled_now);
            w.shared_label = g
                .shared_root
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            w.features.caps = crate::features::Capabilities::runtime();
            w.features.decisions = g.decisions.clone();
            crate::features::apply_snapshot(&mut w.features, &g.snapshot);
            w.features.toggles = g.toggles;
            if let Some(r) = &g.group_result {
                if r.seq != self.seen_group_result {
                    self.seen_group_result = r.seq;
                    self.diag.absorb(r.kind, &r.result);
                    let text =
                        crate::features::result_text_named(r.kind, &r.result, r.subject.as_deref());
                    self.notice.result(self.page, text);
                }
            }
            if let Some((seq, on, applied)) = g.test_mode_result {
                if seq != self.seen_test_mode {
                    self.seen_test_mode = seq;
                    self.notice
                        .show(self.page, crate::backend::test_mode_notice(on, applied));
                }
            }
            (
                w,
                g.message.clone(),
                g.started,
                g.quit_ready,
                g.enrolled_now,
                g.settings,
                g.preview.clone().zip(g.preview_for.clone()),
                g.inspecting.as_ref().map(|i| {
                    (
                        i.root.clone(),
                        i.folder.clone(),
                        i.files.load(std::sync::atomic::Ordering::Relaxed),
                    )
                }),
            )
        };
        let first = !self.loaded && setup_ready;
        self.w = w;
        self.setup.inspecting = inspecting;
        let w = &self.w;
        self.notice
            .settle(|key, release| crate::rollback_ui::request_done(w, key, release));
        self.message = msg;
        self.closing_quits = settings.closing_quits;
        self.first_close_done = settings.first_close_seen;
        self.merge_preview(preview);
        self.mark_checking_rows();
        if enrolled_now && self.setup.working {
            self.setup.working = false;
            self.setup.finished = true;
            self.pending_focus = Some("@content".into());
        } else if self.setup.working && !self.message.is_empty() {
            self.setup.working = false;
        }
        if first {
            self.loaded = true;
            if let Some(b) = self.boot.take() {
                if let Some(p) = b.page {
                    self.page = p;
                }
                if let Some(f) = b.filter {
                    self.filter = f;
                }
                if let Some(d) = &b.detail {
                    self.open_detail_arg(d);
                }
                if let Some(s) = b.setup_step {
                    self.setup.step = s.clamp(1, 6);
                }
                match b.sheet.as_deref() {
                    Some("first-close") => self.sheet = Some(Sheet::FirstClose),
                    Some("quit") => self.sheet = Some(Sheet::Quit(self.quit_text())),
                    _ => {}
                }
            }
            self.pending_focus = Some("@content".into());
        }
        if quit_ready && self.quit_pending {
            cx.quit();
        }
        cx.notify();
    }

    /// A row whose check is running shows no earlier result, so it never offers Next on one.
    fn mark_checking_rows(&mut self) {
        let Some((root, folder, _)) = self.setup.inspecting.clone() else {
            return;
        };
        if let Some(ins) = self.w.setup.as_mut().and_then(|d| {
            d.installs
                .iter_mut()
                .find(|i| i.root == root && i.folder == folder)
        }) {
            ins.start_checking();
        }
    }

    /// Folds the read-only check into the setup row of the installation it was made for, which
    /// need not be the selected one: a late result never lands on another row.
    fn merge_preview(
        &mut self,
        preview: Option<(
            wow_sync_runtime::EnrollPreview,
            (std::path::PathBuf, String),
        )>,
    ) {
        let Some((p, (root, variant))) = preview else {
            self.preview_lines.clear();
            return;
        };
        let sel = self.setup.install;
        let mut seen: Option<(String, Option<String>, Vec<String>, usize)> = None;
        if let Some(d) = self.w.setup.as_mut() {
            let target = crate::setup::fold_preview(d, &p, &root, &variant);
            if target == Some(sel) {
                self.preview_lines = p.disclosures.clone();
            } else {
                self.preview_lines.clear();
            }
            if let Some(ins) = target.and_then(|i| d.installs.get(i)) {
                seen = Some((
                    ins.folder.clone(),
                    ins.refused.clone(),
                    ins.blocked.clone(),
                    ins.eligible.len(),
                ));
            }
            d.groups = p
                .groups
                .iter()
                .map(|g| crate::world::Group {
                    id: g.id.as_str().to_string(),
                    name: g.name.clone(),
                    identity: g.product.clone(),
                })
                .collect();
        }
        if let Some((folder, refused, blocked, eligible)) = seen {
            let sig = format!("{folder}|{refused:?}|{blocked:?}|{eligible}");
            if sig != self.logged_preview {
                self.logged_preview = sig;
                log(&format!(
                    "inspected {folder}: {eligible} add-on(s) eligible, {} blocked",
                    blocked.len()
                ));
                if let Some(r) = refused {
                    log_warn(&format!("installation {folder} refused: {r}"));
                }
                for b in blocked {
                    log_warn(&format!("blocked add-on {b}"));
                }
            }
        }
    }

    pub fn preview_disclosures(&self) -> Vec<String> {
        self.preview_lines.clone()
    }

    pub fn setup_message(&self) -> Option<String> {
        (!self.message.is_empty()).then(|| self.message.clone())
    }

    pub fn quit_text(&self) -> String {
        model::sheet_quit_text(&self.w).unwrap_or_else(|| "Quit WoW Sync?".into())
    }

    fn open_detail_arg(&mut self, d: &str) {
        let (kind, rest) = d.split_once(':').unwrap_or((d, ""));
        let find = |w: &World| {
            w.addons
                .iter()
                .find(|a| {
                    a.title
                        .to_lowercase()
                        .replace(' ', "")
                        .contains(&rest.to_lowercase().replace(' ', ""))
                })
                .map(|a| a.key.as_str().to_string())
        };
        match kind {
            "addon" => {
                if let Some(k) = find(&self.w) {
                    self.page = Page::Addons;
                    self.detail = Detail::Addon(k);
                }
            }
            "decision" => {
                if let Some(k) = find(&self.w) {
                    self.page = Page::Needs;
                    self.detail = Detail::Decision(k);
                }
            }
            "needshelp" => {
                if let Some(a) = self.w.addons.iter().find(|a| a.kind == Kind::NeedsHelp) {
                    self.page = Page::Needs;
                    self.detail = Detail::NeedsHelp(a.key.as_str().to_string());
                }
            }
            "computer" => {
                if let Some(i) = self.w.peers.iter().position(|p| {
                    p.name
                        .to_lowercase()
                        .replace(' ', "")
                        .contains(&rest.to_lowercase().replace(' ', ""))
                }) {
                    self.page = Page::Computers;
                    self.detail = Detail::Computer(i);
                }
            }
            "reconfirm" => {
                self.page = Page::Needs;
                self.detail = Detail::Reconfirm;
            }
            _ => {}
        }
    }

    // ---- actions ----

    pub fn do_act(&mut self, act: Act, window: &mut Window, cx: &mut Context<Self>) {
        if self.sheet.is_some()
            && !matches!(
                act,
                Act::SheetHide
                    | Act::SheetCancel
                    | Act::SheetQuit
                    | Act::SheetKeepRunning
                    | Act::SheetQuitWhenSafe
            )
        {
            return;
        }
        log(&format!("act {}", act_label(&act)));
        let moves = matches!(
            act,
            Act::Nav(_)
                | Act::Back
                | Act::OpenAddon(_)
                | Act::OpenComputer(_)
                | Act::OpenDecision(_)
                | Act::OpenNeedsHelp(_)
                | Act::OpenReconfirm
        );
        if !moves {
            self.notice.dismiss();
        }
        match act {
            Act::None => {}
            Act::Nav(p) => self.nav(p),
            Act::Back => self.back(),
            Act::SyncNow => self.sync_now(cx),
            Act::TogglePause => self.toggle_pause(cx),
            Act::SetFilter(f) => self.filter = f,
            Act::SetActFilter(f) => self.act_filter = f,
            Act::OpenAddon(k) => self.open(Page::Addons, Detail::Addon(k)),
            Act::OpenComputer(i) => self.open(Page::Computers, Detail::Computer(i)),
            Act::OpenDecision(k) => self.open(Page::Needs, Detail::Decision(k)),
            Act::OpenNeedsHelp(k) => self.open(Page::Needs, Detail::NeedsHelp(k)),
            Act::OpenReconfirm => self.open(Page::Needs, Detail::Reconfirm),
            Act::NeedsAddonsFiltered => {
                self.nav(Page::Addons);
                self.filter = Filter::NotUpToDate;
            }
            Act::TryAgainShared => {
                self.notice.show(self.page, "Checking the shared folder…");
                self.send(Cmd::SyncNow);
            }
            Act::ToggleExpand(k) => {
                let opened = !self.expanded.remove(&k);
                if let Some(f) = crate::model::focus_on_open(&k, opened) {
                    self.pending_focus = Some(f.into());
                }
                if opened {
                    self.expanded.insert(k);
                }
            }
            Act::TurnOffTestMode => {
                self.send(Cmd::SetTestMode(false, None));
            }
            Act::Toggle(k) => {
                if k == "test-mode" {
                    self.send(Cmd::SetTestMode(!self.w.test_mode, None));
                } else if k == "closing-quits" {
                    self.closing_quits = !self.closing_quits;
                    self.save_settings();
                    self.notice.show(self.page, "Saved");
                } else if let Some(t) = crate::about_ui::toggled(self.w.features.toggles, &k) {
                    self.w.features.toggles = t;
                    self.send(Cmd::SaveToggles(t));
                    self.notice.show(self.page, "Saved");
                } else if !self.diag.toggle(&k) {
                    self.notice
                        .show(self.page, "This setting isn't available yet.");
                }
            }
            Act::Theme(t) => {
                self.theme = t;
                log(&format!("theme set {:?}", t));
            }
            Act::SheetHide => {
                self.first_close_done = true;
                self.save_settings();
                self.close_sheet();
                self.hide();
            }
            Act::SheetCancel => self.close_sheet(),
            Act::SheetQuit => {
                self.first_close_done = true;
                self.closing_quits = true;
                self.save_settings();
                self.close_sheet();
                self.begin_quit();
            }
            Act::SheetQuitWhenSafe => {
                self.close_sheet();
                self.begin_quit();
            }
            Act::SheetKeepRunning => {
                backend::lock(&self.shared)
                    .abort_reads
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                self.send(Cmd::CancelQuit);
                self.quit_pending = false;
                self.close_sheet()
            }
            Act::Quit => self.request_quit(cx),
            Act::ExplainCloseAgain => {
                self.first_close_done = false;
                self.save_settings();
                self.notice
                    .show(self.page, "The close explanation will show next time.");
            }
            Act::Setup(a) => self.setup_act(a, cx),
            Act::Reconfirm(a) => self.reconf_act(a),
            Act::CopyVersion => self.copied = true,
            Act::OpenDataFolder => {
                let ok = crate::diag_log::state_dir().is_some_and(|d| win::open_folder(&d));
                if !ok {
                    log_warn("could not open the data folder");
                    self.notice
                        .show(self.page, "The data folder could not be opened.");
                }
            }
            Act::OpenSharedFolder => {
                let p = std::path::PathBuf::from(&self.w.shared_label);
                if !win::open_folder(&p) {
                    log_warn("could not open the shared folder");
                    self.notice
                        .show(self.page, "The shared folder could not be opened.");
                }
            }
            Act::DismissNotice => self.notice.dismiss(),
            Act::Group(a) => match crate::features::gate(&self.w, a.kind()) {
                Ok(()) => {
                    let scope = match &a {
                        crate::features::GroupAction::Rollback { package, recovered } => {
                            Some((package.as_str().to_string(), *recovered))
                        }
                        _ => None,
                    };
                    self.notice.action_started(self.page, scope);
                    self.send(Cmd::Group(a))
                }
                Err(why) => self.notice.show(self.page, why),
            },
        }
        let _ = window;
        cx.notify();
    }

    fn save_settings(&self) {
        self.send(Cmd::SaveSettings(wow_sync_runtime::AppSettings {
            closing_quits: self.closing_quits,
            first_close_seen: self.first_close_done,
        }));
    }

    fn begin_quit(&mut self) {
        self.quit_pending = true;
        backend::lock(&self.shared)
            .abort_reads
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.send(Cmd::QuitWhenSafe);
    }

    fn close_sheet(&mut self) {
        self.sheet = None;
        self.pending_focus = self.sheet_from.take();
    }

    pub fn nav(&mut self, p: Page) {
        self.page = p;
        self.detail = Detail::None;
        self.detail_stack.clear();
        self.focus_stack.clear();
        if let Some(i) = NAV.iter().position(|n| *n == p) {
            self.rov.insert("nav".into(), i);
        }
        self.pending_focus = Some("@content".into());
    }

    fn open(&mut self, page: Page, d: Detail) {
        self.focus_stack.push(self.last_key.clone());
        let prev = std::mem::replace(&mut self.detail, d);
        self.detail_stack.push((self.page, prev));
        self.page = page;
        self.pending_focus = Some("@content".into());
    }

    pub fn back(&mut self) {
        if let Some((page, prev)) = self.detail_stack.pop() {
            self.page = page;
            self.detail = prev;
            self.pending_focus = self.focus_stack.pop();
        } else if self.detail != Detail::None {
            self.detail = Detail::None;
        }
    }

    fn sync_now(&mut self, _cx: &mut Context<Self>) {
        if !self.w.enrolled || self.w.is_paused() || self.w.busy.is_some() {
            return;
        }
        self.w.busy = Some(Busy::Syncing { done: 0, total: 0 });
        self.send(Cmd::SyncNow);
    }

    fn toggle_pause(&mut self, _cx: &mut Context<Self>) {
        if self.w.is_paused() {
            self.send(Cmd::Resume);
        } else {
            if self.w.busy.is_some() {
                self.w.busy = Some(Busy::Pausing);
            }
            self.send(Cmd::Pause);
        }
    }

    fn request_quit(&mut self, cx: &mut Context<Self>) {
        match model::sheet_quit_text(&self.w) {
            Some(t) => {
                self.sheet_from = Some(self.opener.clone());
                self.sheet = Some(Sheet::Quit(t));
                self.show();
            }
            None => {
                self.begin_quit();
                cx.notify();
            }
        }
    }

    pub fn request_close(&mut self, cx: &mut Context<Self>) {
        log("close requested");
        if self.sheet.is_some() {
            return;
        }
        if self.w.enrolled && !self.first_close_done {
            self.sheet_from = Some(self.opener.clone());
            self.sheet = Some(Sheet::FirstClose);
            cx.notify();
        } else if self.w.enrolled && self.closing_quits {
            self.request_quit(cx);
        } else {
            self.hide();
        }
    }

    pub fn hide(&mut self) {
        self.visible = false;
        win::set_visible(self.hwnd, false);
        log("window hidden");
    }

    pub fn show(&mut self) {
        self.visible = true;
        win::set_visible(self.hwnd, true);
        log("window shown");
    }

    fn setup_target(&self) -> Option<Target> {
        let d = self.w.setup.as_ref()?;
        let ins = d.installs.get(self.setup.install)?;
        Some(Target {
            install_root: ins.root.clone(),
            variant_folder: ins.folder.clone(),
            shared_root: d.shared_root.clone()?,
        })
    }

    fn inspect_selected(&mut self) {
        let Some(t) = self.setup_target() else { return };
        let sel = self.setup.install;
        // The row is "checking" from now on, and no earlier result may fill it in meanwhile.
        if let Some(ins) = self.w.setup.as_mut().and_then(|d| d.installs.get_mut(sel)) {
            ins.start_checking();
        }
        {
            let mut g = backend::lock(&self.shared);
            g.preview = None;
            g.preview_for = None;
        }
        self.send(Cmd::Inspect(t));
    }

    /// Asks for a folder with the system dialog. Nothing is read or changed until the choice
    /// has been checked by `folder_picked`.
    fn pick_folder(&mut self, game: bool, cx: &mut Context<Self>) {
        if self.setup.checking {
            return;
        }
        let prompt = if game {
            "Choose the World of Warcraft folder"
        } else {
            "Choose your Dropbox folder"
        };
        log(&format!(
            "folder picker opened for {}",
            if game { "the game" } else { "Dropbox" }
        ));
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(prompt.into()),
        });
        cx.spawn(async move |this, cx| {
            let chosen = match rx.await {
                Ok(Ok(Some(v))) => v.into_iter().next(),
                _ => None,
            };
            let Some(p) = chosen else {
                log("folder picker closed without a choice");
                return;
            };
            this.update(cx, |s, cx| {
                s.setup.checking = true;
                s.setup.pick_note = None;
                cx.notify();
            })
            .ok();
            let looked = p.clone();
            let checked = cx
                .background_executor()
                .spawn(async move { crate::setup::check_picked(game, &looked) })
                .await;
            this.update(cx, |s, cx| {
                s.setup.checking = false;
                s.folder_picked(p, checked);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Takes the person's folder into setup once it has been looked at. A game folder joins the
    /// list and is then checked like any other; it is never trusted because it was chosen by
    /// hand. A folder Dropbox does not report is accepted with a warning, not refused.
    pub fn folder_picked(&mut self, path: std::path::PathBuf, checked: crate::setup::Checked) {
        self.setup.pick_note = None;
        let Some(d) = self.w.setup.as_mut() else {
            return;
        };
        match checked {
            crate::setup::Checked::Game(found) => {
                match crate::setup::add_found_install(d, &path, found) {
                    Ok(i) => {
                        log(&format!(
                            "game folder chosen: {} (accepted)",
                            path.display()
                        ));
                        self.setup_act_select(i);
                    }
                    Err(note) => {
                        log_warn(&format!(
                            "game folder chosen: {} (not a game folder)",
                            path.display()
                        ));
                        self.setup.pick_note = Some(note);
                    }
                }
            }
            crate::setup::Checked::Dropbox { known } => {
                crate::setup::apply_dropbox_choice(d, &path);
                self.setup.folder_chosen = false;
                if known {
                    log(&format!("Dropbox folder chosen: {}", path.display()));
                } else {
                    log_warn(&format!(
                        "Dropbox folder chosen: {} (not reported by Dropbox; warned)",
                        path.display()
                    ));
                    self.setup.pick_note = Some(crate::setup::NOT_DROPBOX_TEXT.into());
                }
            }
        }
        self.pending_focus = Some("@content".into());
    }

    fn setup_act_select(&mut self, i: usize) {
        self.setup.install = i;
        self.setup.consent.clear();
        self.inspect_selected();
    }

    fn setup_act(&mut self, a: SetupAct, cx: &mut Context<Self>) {
        match a {
            SetupAct::Next => {
                if self.setup.step < 6 {
                    self.setup.step += 1;
                    log(&format!("setup step {}", self.setup.step));
                    if self.setup.step == 3 {
                        self.inspect_selected();
                    }
                }
            }
            SetupAct::Back => {
                if self.setup.step == 3 {
                    self.send(Cmd::CancelInspect);
                }
                self.setup.step = self.setup.step.saturating_sub(1).max(1);
                log(&format!("setup step {} (back)", self.setup.step));
            }
            SetupAct::ChooseFolder => self.setup.folder_chosen = true,
            SetupAct::PickGameFolder => self.pick_folder(true, cx),
            SetupAct::PickDropboxFolder => self.pick_folder(false, cx),
            SetupAct::TryAgain => self.inspect_selected(),
            SetupAct::CancelCheck => self.send(Cmd::CancelInspect),
            SetupAct::TurnOffTestMode => {
                let t = self.setup_target();
                self.send(Cmd::SetTestMode(false, t));
                self.inspect_selected();
            }
            SetupAct::SelectInstall(i) => self.setup_act_select(i),
            SetupAct::IdentityToggle => self.setup.consent.identity = !self.setup.consent.identity,
            SetupAct::ScopeToggle => self.setup.consent.scope = !self.setup.consent.scope,
            SetupAct::CreateGroup => self.setup.group = Some(None),
            SetupAct::JoinGroup(i) => self.setup.group = Some(Some(i)),
            SetupAct::SetUp => self.enroll(),
            SetupAct::Finish => {
                self.setup.finished = false;
                self.setup.working = false;
                self.reset_ui();
            }
        }
        self.pending_focus = Some("@content".into());
    }

    fn enroll(&mut self) {
        use wow_sync_runtime::{EnrollRequest, JoinChoice};
        let (Some(t), Some(d)) = (self.setup_target(), self.w.setup.as_ref()) else {
            return;
        };
        let join = match self.setup.group {
            Some(Some(i)) => match d
                .groups
                .get(i)
                .and_then(|g| wow_sync_core::ids::GroupId::parse(&g.id).ok())
            {
                Some(id) => JoinChoice::Existing(id),
                None => return,
            },
            _ => JoinChoice::New {
                name: self.setup.group_name.clone(),
            },
        };
        let req = EnrollRequest {
            join,
            identity_confirmed: self.setup.consent.identity,
            scope_confirmed: self.setup.consent.scope,
            digest: backend::lock(&self.shared).digest_for(&t),
            display_name: d.computer.clone(),
        };
        self.setup.working = true;
        self.setup.consent.clear();
        self.send(Cmd::Enroll(t, req));
    }

    fn reconf_act(&mut self, a: ReconfAct) {
        let r = &mut self.reconf;
        match a {
            ReconfAct::IdentityToggle => r.consent.identity = !r.consent.identity,
            ReconfAct::ScopeToggle => r.consent.scope = !r.consent.scope,
            ReconfAct::NotNow => {
                r.not_now = true;
                r.note = Some("Syncing stays stopped until you confirm.".into());
            }
            ReconfAct::Confirm => {
                if !(r.consent.identity && r.consent.scope) {
                    r.note = Some(
                        "Confirm both the game version and the add-on list to continue. Nothing has been changed."
                            .into(),
                    );
                } else {
                    let digest = self.w.reconfirm.as_ref().and_then(|x| x.digest);
                    self.send(Cmd::Reconfirm(wow_sync_runtime::ReconfirmRequest {
                        identity_confirmed: true,
                        scope_confirmed: true,
                        digest,
                    }));
                    self.reconf = ReconfState::default();
                    self.detail = Detail::None;
                    self.detail_stack.clear();
                    self.notice.show(self.page, "Confirmed. Syncing continues.");
                }
            }
        }
        if self.reconf.note.is_some() || self.detail == Detail::Reconfirm {
            self.pending_focus = None;
        }
    }

    // ---- tray ----

    fn make_tray(&mut self) {
        let key = self.tray_key();
        match self.build_tray(&key) {
            Some(t) => self.tray = Some(t),
            None => log("tray creation failed"),
        }
    }

    fn tray_key(&self) -> String {
        let h = headline(&self.w);
        format!(
            "{:?}|{}|{}|{}|{}",
            model::tray_shape(&self.w),
            h.text,
            self.w.is_paused(),
            need_items(&self.w).len(),
            self.w.busy.is_some()
        )
    }

    fn menu_for(&self) -> (Menu, MenuId, MenuId, MenuId, Option<MenuId>, MenuId) {
        let h = headline(&self.w);
        let menu = Menu::new();
        let head = MenuItem::new(h.text.clone(), false, None);
        let open = MenuItem::new("Open WoW Sync", true, None);
        let can_sync = self.w.enrolled && !self.w.is_paused() && self.w.busy.is_none();
        let sync = MenuItem::new("Sync now", can_sync, None);
        let pause = MenuItem::new(
            if self.w.is_paused() {
                "Resume syncing"
            } else {
                "Pause syncing"
            },
            self.w.enrolled,
            None,
        );
        let n = need_items(&self.w).len();
        let needs = (n > 0).then(|| MenuItem::new(format!("Needs attention ({n})…"), true, None));
        let quit = MenuItem::new("Quit WoW Sync", true, None);
        let sep = PredefinedMenuItem::separator();
        let _ = menu.append_items(&[&head, &open, &sync, &pause]);
        if let Some(nm) = &needs {
            let _ = menu.append(nm);
        }
        let _ = menu.append_items(&[&sep, &quit]);
        (
            menu,
            open.id().clone(),
            sync.id().clone(),
            pause.id().clone(),
            needs.map(|n| n.id().clone()),
            quit.id().clone(),
        )
    }

    fn tray_image(&self) -> tray_icon::Icon {
        let px = model::tray_rgba(model::tray_shape(&self.w));
        tray_icon::Icon::from_rgba(px, 32, 32).expect("tray icon")
    }

    fn build_tray(&self, key: &str) -> Option<TrayState> {
        let (menu, open, sync, pause, needs, quit) = self.menu_for();
        let icon = TrayIconBuilder::new()
            .with_tooltip(model::tray_tooltip(&self.w))
            .with_icon(self.tray_image())
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .ok()?;
        Some(TrayState {
            icon,
            key: key.to_string(),
            open,
            sync,
            pause,
            needs,
            quit,
        })
    }

    fn refresh_tray(&mut self) {
        let key = self.tray_key();
        if self.tray.as_ref().map(|t| t.key != key).unwrap_or(false) {
            let (menu, open, sync, pause, needs, quit) = self.menu_for();
            let tip = model::tray_tooltip(&self.w);
            let img = self.tray_image();
            if let Some(t) = self.tray.as_mut() {
                let _ = t.icon.set_icon(Some(img));
                let _ = t.icon.set_tooltip(Some(tip.clone()));
                t.icon.set_menu(Some(Box::new(menu)));
                t.key = key;
                t.open = open;
                t.sync = sync;
                t.pause = pause;
                t.needs = needs;
                t.quit = quit;
                log(&format!("tray updated: {tip}"));
            }
        }
    }

    fn tray_event(&mut self, e: TrayEv, cx: &mut Context<Self>) {
        let Some(t) = &self.tray else { return };
        match e {
            TrayEv::Click => {
                // Q-39: a left click (and a second instance's request) always opens the window.
                log("tray: left click");
                self.show();
            }
            TrayEv::Menu(id) => {
                let item = if id == t.open {
                    "open"
                } else if id == t.sync {
                    "sync now"
                } else if id == t.pause {
                    "pause or resume"
                } else if Some(&id) == t.needs.as_ref() {
                    "needs attention"
                } else if id == t.quit {
                    "quit"
                } else {
                    "unknown"
                };
                log(&format!("tray: {item}"));
                if id == t.open {
                    self.show();
                } else if id == t.sync {
                    self.sync_now(cx);
                } else if id == t.pause {
                    self.toggle_pause(cx);
                } else if Some(&id) == t.needs.as_ref() {
                    self.show();
                    self.nav(Page::Needs);
                } else if id == t.quit {
                    self.request_quit(cx);
                }
            }
        }
        cx.notify();
    }

    // ---- focus plumbing ----

    pub fn handle(&mut self, key: &str, cx: &mut Context<Self>) -> FocusHandle {
        self.handles
            .entry(key.to_string())
            .or_insert_with(|| cx.focus_handle())
            .clone()
    }

    fn trapped(&self) -> bool {
        self.sheet.is_some() && !self.in_sheet
    }

    /// Base for every focusable control: id, focus handle, tab stop, visible focus ring.
    pub fn focusable(&mut self, key: &str, tab: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let h = self.handle(key, cx).tab_stop(tab && !self.trapped());
        if self.capture_first && !self.trapped() {
            self.content_first = Some(key.to_string());
            self.capture_first = false;
        }
        let pal = self.pal();
        div()
            .id(SharedString::from(key.to_string()))
            .track_focus(&h)
            .tab_stop(tab && !self.trapped())
            .border_2()
            .border_color(transparent_black())
            .rounded(u(6.))
            .focus_visible(move |s| s.border_color(rgb(pal.focus)))
    }

    pub fn clickable(&mut self, key: &str, act: Act, cx: &mut Context<Self>) -> Stateful<Div> {
        let k = key.to_string();
        self.focusable(key, true, cx)
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, window, cx| {
                this.last_key = k.clone();
                this.do_act(act.clone(), window, cx)
            }))
    }

    pub fn btn(
        &mut self,
        key: &str,
        label: &str,
        kind: Bk,
        act: Act,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let pal = self.pal();
        let base = self.clickable(key, act, cx);
        Self::style_btn(base, label, kind, true, pal)
            .role(Role::Button)
            .aria_label(SharedString::from(label.to_string()))
    }

    pub fn btn_off(
        &mut self,
        key: &str,
        label: &str,
        reason: &str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let pal = self.pal();
        let base = self.focusable(key, true, cx);
        Self::style_btn(base, label, Bk::Secondary, false, pal)
            .role(Role::Button)
            .aria_label(SharedString::from(format!("{label}, {reason}")))
    }

    fn style_btn(
        base: Stateful<Div>,
        label: &str,
        kind: Bk,
        enabled: bool,
        pal: Palette,
    ) -> Stateful<Div> {
        let b = base
            .flex()
            .items_center()
            .justify_center()
            .px(u(14.))
            .h(u(36.))
            .flex_none()
            .child(SharedString::from(label.to_string()));
        if !enabled {
            return b.text_color(rgb(pal.disabled)).bg(rgb(pal.surface));
        }
        match kind {
            Bk::Primary => b.bg(rgb(pal.accent)).text_color(rgb(pal.on_accent)),
            Bk::Secondary => b
                .bg(rgb(pal.surface))
                .border_color(rgb(pal.control_border))
                .text_color(rgb(pal.text))
                .hover(move |s| s.bg(rgb(pal.hover))),
            Bk::Link => b
                .px(u(4.))
                .text_color(rgb(pal.accent))
                .hover(move |s| s.bg(rgb(pal.hover))),
        }
    }

    /// A checkbox or switch row. `switch` only changes the role.
    pub fn toggle(
        &mut self,
        key: &str,
        label: &str,
        on: bool,
        switch: bool,
        act: Act,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let pal = self.pal();
        let boxed = div()
            .size(u(20.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .border_2()
            .border_color(rgb(pal.control_border))
            .rounded(u(if switch { 10. } else { 4. }))
            .bg(rgb(if on { pal.accent } else { pal.surface }))
            .child(if on {
                icon(model::Icon::Check, 14., pal.on_accent).into_any_element()
            } else {
                div().into_any_element()
            });
        let me = cx.entity();
        let a11y_act = act.clone();
        let a11y_key = key.to_string();
        self.clickable(key, act, cx)
            .on_a11y_action(gpui::accesskit::Action::Click, move |_, window, app| {
                // UIA TogglePattern arrives as Click without a pointer position; act directly.
                let act = a11y_act.clone();
                let key = a11y_key.clone();
                me.update(app, |this, cx| {
                    this.last_key = key;
                    this.do_act(act, window, cx)
                });
            })
            .role(if switch { Role::Switch } else { Role::CheckBox })
            .aria_label(SharedString::from(label.to_string()))
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .flex()
            .items_center()
            .gap(u(10.))
            .px(u(6.))
            .py(u(6.))
            .text_color(rgb(pal.text))
            .child(boxed)
            .child(div().flex_1().child(SharedString::from(label.to_string())))
    }

    pub fn disclosure(&mut self, key: &str, name: &str, cx: &mut Context<Self>) -> Stateful<Div> {
        let pal = self.pal();
        let open = self.expanded.contains(key);
        let label = format!("{name}, {}", if open { "expanded" } else { "collapsed" });
        self.clickable(
            &format!("disc-{key}"),
            Act::ToggleExpand(key.to_string()),
            cx,
        )
        .role(Role::Button)
        .aria_label(SharedString::from(label))
        .aria_expanded(open)
        .flex()
        .items_center()
        .gap(u(6.))
        .px(u(8.))
        .h(u(32.))
        .text_color(rgb(pal.accent))
        .child(icon(model::Icon::Chevron, 14., pal.accent).when(open, |s| {
            s.with_transformation(gpui::Transformation::rotate(gpui::radians(
                std::f32::consts::FRAC_PI_2,
            )))
        }))
        .child(SharedString::from(name.to_string()))
    }

    /// A roving-focus list row: one tab stop per group, arrows move, Enter/Space/click activate.
    #[allow(clippy::too_many_arguments)]
    pub fn rov_row(
        &mut self,
        group: &str,
        i: usize,
        n: usize,
        key: &str,
        a11y: String,
        act: Act,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let hkey = format!("{group}:{key}");
        self.rov_keys
            .entry(group.to_string())
            .or_default()
            .push(hkey.clone());
        let cur = self
            .rov
            .get(group)
            .copied()
            .unwrap_or(0)
            .min(n.saturating_sub(1));
        let (g1, g2, hk) = (group.to_string(), group.to_string(), hkey.clone());
        self.focusable(&hkey, i == cur, cx)
            .role(Role::ListItem)
            .aria_label(SharedString::from(a11y))
            .aria_position_in_set(i + 1)
            .aria_size_of_set(n)
            .cursor_pointer()
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, window, cx| {
                this.rov_key(&g1, i, ev, window, cx)
            }))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.rov.insert(g2.clone(), i);
                this.last_key = hk.clone();
                this.do_act(act.clone(), window, cx)
            }))
    }

    pub fn rov_key(
        &mut self,
        group: &str,
        i: usize,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let horizontal = matches!(group, "filter" | "theme" | "actfilter");
        let keys = self.rov_keys.get(group).cloned().unwrap_or_default();
        let n = keys.len();
        if n == 0 {
            return;
        }
        let page = 8usize;
        let k = ev.keystroke.key.as_str();
        let new = match (k, horizontal) {
            ("down", false) | ("right", true) => (i + 1).min(n - 1),
            ("up", false) | ("left", true) => i.saturating_sub(1),
            ("home", _) => 0,
            ("end", _) => n - 1,
            ("pagedown", false) => (i + page).min(n - 1),
            ("pageup", false) => i.saturating_sub(page),
            ("right", false) if group == "activity" => {
                if let Some(name) = keys
                    .get(i)
                    .and_then(|k| k.split_once(':').map(|(_, r)| r.to_string()))
                {
                    self.expanded.insert(name);
                    cx.notify();
                }
                cx.stop_propagation();
                return;
            }
            ("left", false) if group == "activity" => {
                if let Some(name) = keys
                    .get(i)
                    .and_then(|k| k.split_once(':').map(|(_, r)| r.to_string()))
                {
                    self.expanded.remove(&name);
                    cx.notify();
                }
                cx.stop_propagation();
                return;
            }
            _ => return,
        };
        self.rov.insert(group.to_string(), new);
        if let Some(h) = self.handles.get(&keys[new]).cloned() {
            window.focus(&h, cx);
        }
        if let Some(s) = self.scrolls.get(group) {
            s.scroll_to_item(new);
        }
        cx.stop_propagation();
        cx.notify();
    }

    pub fn scroll(&mut self, group: &str) -> ScrollHandle {
        self.scrolls.entry(group.to_string()).or_default().clone()
    }

    fn region_keys(&self) -> Vec<String> {
        let mut v = vec![
            format!("nav:{}", NAV[*self.rov.get("nav").unwrap_or(&0)].title()),
            "hdr-sync".to_string(),
        ];
        if !banners(&self.w).0.is_empty() {
            v.push("banner-0".into());
        }
        if let Some(c) = &self.content_first {
            v.push(c.clone());
        }
        v
    }

    fn move_region(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.sheet.is_some() {
            return;
        }
        let keys = self.region_keys();
        let n = keys.len() as isize;
        self.region = ((self.region as isize + delta).rem_euclid(n)) as usize;
        if let Some(h) = self.handles.get(&keys[self.region]).cloned() {
            window.focus(&h, cx);
        }
    }

    fn go(&mut self, p: Page, cx: &mut Context<Self>) {
        if self.sheet.is_some() || !self.w.enrolled {
            return;
        }
        self.nav(p);
        cx.notify();
    }
}

enum TrayEv {
    Click,
    Menu(MenuId),
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vs = window.viewport_size();
        self.window_w = f32::from(vs.width) / zoom();
        self.window_h = f32::from(vs.height) / zoom();
        self.rov_keys.clear();
        self.content_first = None;
        let pal = self.pal();
        let setup_mode = !self.w.enrolled || self.setup.finished;
        let setup_panel = (setup_mode && self.setup.step >= 4).then(|| {
            let digest = self
                .setup_target()
                .and_then(|t| backend::lock(&self.shared).digest_for(&t));
            format!("setup|{}|{:?}", self.setup.install, digest)
        });
        self.setup.consent.observe(setup_panel);
        let reconf_panel = (!setup_mode && self.detail == Detail::Reconfirm).then(|| {
            format!(
                "reconfirm|{:?}",
                self.w.reconfirm.as_ref().and_then(|x| x.digest)
            )
        });
        self.reconf.consent.observe(reconf_panel);
        let title = if !self.loaded {
            window_title("Starting")
        } else if setup_mode {
            window_title("Set up this computer")
        } else {
            window_title(self.page.title())
        };
        if title != self.last_title {
            window.set_window_title(&title);
            self.last_title = title;
        }

        self.capture_first = true;
        let content = if !self.loaded {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child("Starting WoW Sync…")
                .into_any_element()
        } else if setup_mode {
            self.setup_view(cx)
        } else {
            self.content_view(cx)
        };
        let content_first = self.content_first.clone();
        self.capture_first = false;

        let body: AnyElement = if setup_mode || !self.loaded {
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(content)
                .into_any_element()
        } else {
            let header = self.header_view(cx);
            let sidebar = self.sidebar_view(cx);
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(header)
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .flex_row()
                        .child(sidebar)
                        .child(content),
                )
                .into_any_element()
        };

        if self.sheet.is_none() {
            if let Some((k, _)) = self
                .handles
                .iter()
                .find(|(k, h)| h.is_focused(window) && !k.starts_with("sheet-"))
            {
                self.opener = k.clone();
            }
        }
        if let Some(k) = self.pending_focus.take() {
            let key = if k == "@content" {
                content_first.clone().unwrap_or_default()
            } else {
                k
            };
            let h = self
                .handles
                .get(&key)
                .or_else(|| content_first.as_ref().and_then(|c| self.handles.get(c)))
                .cloned();
            if let Some(h) = h {
                window.focus(&h, cx);
            }
        }
        if self.sheet.is_some() && !self.handles.is_empty() {
            if let Some(h) = self.handles.get("sheet-default").cloned() {
                if !h.is_focused(window)
                    && !self
                        .handles
                        .values()
                        .any(|x| x.is_focused(window) && self.in_sheet_key(x))
                {
                    window.focus(&h, cx);
                }
            }
        }
        self.refresh_tray();

        div()
            .id("root")
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(pal.bg))
            .text_color(rgb(pal.text))
            .text_size(u(14.))
            .on_action(cx.listener(|_, _: &FocusNextStop, w, cx| w.focus_next(cx)))
            .on_action(cx.listener(|_, _: &FocusPrevStop, w, cx| w.focus_prev(cx)))
            .on_action(cx.listener(|t, _: &RegionNext, w, cx| t.move_region(1, w, cx)))
            .on_action(cx.listener(|t, _: &RegionPrev, w, cx| t.move_region(-1, w, cx)))
            .on_action(cx.listener(|t, _: &Go1, _, cx| t.go(Page::Overview, cx)))
            .on_action(cx.listener(|t, _: &Go2, _, cx| t.go(Page::Computers, cx)))
            .on_action(cx.listener(|t, _: &Go3, _, cx| t.go(Page::Addons, cx)))
            .on_action(cx.listener(|t, _: &Go4, _, cx| t.go(Page::Needs, cx)))
            .on_action(cx.listener(|t, _: &Go5, _, cx| t.go(Page::Activity, cx)))
            .on_action(cx.listener(|t, _: &Go6, _, cx| t.go(Page::SettingsSync, cx)))
            .on_action(cx.listener(|t, _: &GoSettings, _, cx| t.go(Page::AppSettings, cx)))
            .on_action(cx.listener(|t, _: &SyncKey, w, cx| {
                if t.sheet.is_none() {
                    t.do_act(Act::SyncNow, w, cx)
                }
            }))
            .on_action(cx.listener(|t, _: &PauseKey, w, cx| {
                if t.w.enrolled {
                    t.do_act(Act::TogglePause, w, cx)
                }
            }))
            .on_action(cx.listener(|t, _: &CloseKey, _, cx| t.request_close(cx)))
            .on_action(cx.listener(|t, _: &QuitKey, w, cx| t.do_act(Act::Quit, w, cx)))
            .on_action(cx.listener(|t, _: &EscKey, w, cx| t.escape(w, cx)))
            .on_action(cx.listener(|t, _: &BackKey, w, cx| {
                if t.sheet.is_none() {
                    t.do_act(Act::Back, w, cx)
                }
            }))
            .child(div().flex_1().min_h_0().child(body))
    }
}

impl Shell {
    fn in_sheet_key(&self, h: &FocusHandle) -> bool {
        ["sheet-default", "sheet-other"]
            .iter()
            .any(|k| self.handles.get(*k).map(|x| x == h).unwrap_or(false))
    }

    fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.sheet.clone() {
            Some(Sheet::FirstClose) => self.do_act(Act::SheetCancel, window, cx),
            Some(Sheet::Quit(_)) => self.do_act(Act::SheetKeepRunning, window, cx),
            None => {
                if self.detail != Detail::None {
                    self.do_act(Act::Back, window, cx)
                } else if self.w.enrolled {
                    // Esc on a top-level page does nothing.
                } else if self.setup.step > 1 && !self.setup.working {
                    self.do_act(Act::Setup(SetupAct::Back), window, cx)
                }
            }
        }
    }

    fn header_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let h = headline(&self.w);
        let paused = self.w.is_paused();
        let busy = self.w.busy.is_some();
        let can_sync = self.w.enrolled && !paused && !busy && self.w.reconfirm.is_none();
        let sync_label = if matches!(self.w.busy, Some(Busy::Syncing { .. })) {
            "Syncing…"
        } else {
            "Sync now"
        };
        let sync = if can_sync {
            self.btn("hdr-sync", "Sync now", Bk::Primary, Act::SyncNow, cx)
        } else {
            let reason = if paused {
                "unavailable while paused"
            } else if busy {
                "unavailable while syncing"
            } else {
                "unavailable until the game version is confirmed"
            };
            self.btn_off("hdr-sync", sync_label, reason, cx)
        };
        let pause_label = if paused {
            "Resume syncing"
        } else {
            "Pause syncing"
        };
        let pause = self.btn(
            "hdr-pause",
            pause_label,
            Bk::Secondary,
            Act::TogglePause,
            cx,
        );
        let pause = pause.border_color(rgb(pal.control_border));
        let sync = if can_sync {
            sync
        } else {
            sync.border_color(rgb(pal.divider))
        };
        let mut head = div()
            .id("header")
            .role(Role::Banner)
            .aria_label("Status")
            .h(u(76.))
            .flex_none()
            .flex()
            .items_center()
            .gap(u(14.))
            .px(u(20.))
            .bg(rgb(pal.surface))
            .border_b_1()
            .border_color(rgb(pal.divider));
        let tone_bg = pal.fill(h.tone);
        head = head
            .child(
                div()
                    .size(u(40.))
                    .flex_none()
                    .rounded(u(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgb(tone_bg))
                    .child(icon(h.icon, 24., pal.fg(h.tone))),
            )
            .child(
                div()
                    .id("headline")
                    .role(Role::Status)
                    .aria_label(SharedString::from(model::headline_a11y(&self.w)))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(u(2.))
                    .child(
                        div()
                            .text_size(u(20.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(pal.fg(h.tone)))
                            .child(SharedString::from(h.text.clone())),
                    )
                    .child(
                        div()
                            .text_size(u(13.))
                            .text_color(rgb(pal.muted))
                            .child(SharedString::from(model::context_line(&self.w))),
                    )
                    .when(model::headline_sub(&self.w).is_some(), |d| {
                        d.child(div().text_size(u(13.)).text_color(rgb(pal.muted)).child(
                            SharedString::from(model::headline_sub(&self.w).unwrap_or_default()),
                        ))
                    })
                    .when(matches!(self.w.busy, Some(Busy::Syncing { .. })), |d| {
                        let (done, total) = match self.w.busy {
                            Some(Busy::Syncing { done, total }) => (done, total),
                            _ => (0, 0),
                        };
                        let name = if total > 0 {
                            format!("Applying {done} of {total}")
                        } else {
                            "Syncing".into()
                        };
                        d.child(
                            div()
                                .id("sync-progress")
                                .role(Role::ProgressIndicator)
                                .aria_label(SharedString::from(name))
                                .mt(u(4.))
                                .h(u(4.))
                                .w(u(240.))
                                .rounded(u(2.))
                                .bg(rgb(pal.divider))
                                .child(
                                    div()
                                        .h_full()
                                        .w(u(if total > 0 {
                                            240.0 * done as f32 / total as f32
                                        } else {
                                            90.0
                                        }))
                                        .rounded(u(2.))
                                        .bg(rgb(pal.accent)),
                                ),
                        )
                    }),
            )
            .child(sync)
            .child(pause);
        head.into_any_element()
    }

    fn sidebar_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let collapsed = self.window_w < 900.0;
        let needs = need_items(&self.w).len();
        let mut col = div()
            .id("sidebar")
            .role(Role::Navigation)
            .aria_label("Sections")
            .w(u(if collapsed { 64. } else { 220. }))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .gap(u(2.))
            .p(u(8.))
            .bg(rgb(pal.sidebar))
            .border_r_1()
            .border_color(rgb(pal.divider));
        for (i, p) in NAV.iter().copied().enumerate() {
            if p == Page::AppSettings {
                col = col.child(div().h(u(1.)).my(u(6.)).bg(rgb(pal.divider)));
            }
            let current = self.page == p;
            let name = nav_name(p, needs, current);
            let key = format!("nav:{}", p.title());
            let cur_idx = self.rov.get("nav").copied().unwrap_or(0);
            self.rov_keys
                .entry("nav".into())
                .or_default()
                .push(key.clone());
            let g = "nav".to_string();
            let k2 = key.clone();
            let row = self
                .focusable(&key, i == cur_idx, cx)
                .role(Role::Button)
                .aria_label(SharedString::from(name))
                .when(current, |d| d.aria_selected(true))
                .cursor_pointer()
                .h(u(38.))
                .flex()
                .items_center()
                .gap(u(8.))
                .px(u(8.))
                .text_color(rgb(pal.text))
                .when(current, |d| {
                    d.bg(rgb(pal.selected)).font_weight(FontWeight::SEMIBOLD)
                })
                .hover(move |s| s.bg(rgb(pal.hover)))
                .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, window, cx| {
                    this.rov_key(&g, i, ev, window, cx)
                }))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.last_key = k2.clone();
                    this.do_act(Act::Nav(p), window, cx)
                }))
                .child(
                    div()
                        .size(u(24.))
                        .flex_none()
                        .rounded(u(6.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(rgb(if current { pal.accent } else { pal.divider }))
                        .text_color(rgb(if current { pal.on_accent } else { pal.text }))
                        .text_size(u(12.))
                        .child(SharedString::from(
                            p.title().chars().next().unwrap_or('?').to_string(),
                        )),
                )
                .when(!collapsed, |d| {
                    d.child(div().flex_1().child(SharedString::from(p.title())))
                        .when(p == Page::Needs && needs > 0, |d| {
                            d.child(
                                div()
                                    .px(u(7.))
                                    .rounded(u(9.))
                                    .bg(rgb(pal.fill(Tone::Help)))
                                    .text_color(rgb(pal.fg(Tone::Help)))
                                    .text_size(u(12.))
                                    .child(SharedString::from(needs.to_string())),
                            )
                        })
                });
            col = col.child(row);
        }
        col.into_any_element()
    }

    fn banner_view(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let pal = self.pal();
        let (list, more) = banners(&self.w);
        let mut out = Vec::new();
        for (i, b) in list.iter().enumerate() {
            let alert = matches!(b.tone, Tone::Error | Tone::Help);
            let mut row = div()
                .id(SharedString::from(format!("banner-row-{}", b.text)))
                .role(if alert { Role::Alert } else { Role::Status })
                .aria_label(SharedString::from(b.text.clone()))
                .flex()
                .items_center()
                .gap(u(12.))
                .p(u(12.))
                .rounded(u(8.))
                .bg(rgb(pal.fill(b.tone)))
                .border_1()
                .border_color(rgb(pal.fg(b.tone)))
                .child(icon(model::tone_icon(b.tone), 18., pal.fg(b.tone)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_color(rgb(pal.text))
                        .child(SharedString::from(b.text.clone())),
                );
            if b.action != BannerAction::None {
                let act = match b.action {
                    BannerAction::Resume => Act::TogglePause,
                    BannerAction::TryAgain => Act::TryAgainShared,
                    BannerAction::TurnOffTestMode => Act::TurnOffTestMode,
                    _ => Act::OpenReconfirm,
                };
                let btn = self.btn(
                    &format!("banner-{i}"),
                    b.action_label,
                    Bk::Secondary,
                    act,
                    cx,
                );
                row = row.child(btn);
            } else {
                let h = self.handle(&format!("banner-{i}"), cx);
                row = row.child(
                    div()
                        .id(SharedString::from(format!("banner-{i}")))
                        .track_focus(&h)
                        .tab_stop(false)
                        .size(u(1.)),
                );
            }
            out.push(row.into_any_element());
        }
        if more > 0 {
            let l = self.btn(
                "banner-more",
                &format!("and {more} more"),
                Bk::Link,
                Act::Nav(Page::Needs),
                cx,
            );
            out.push(l.into_any_element());
        }
        out
    }

    fn content_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let mut banners = self.banner_view(cx);
        let sheet = self.sheet_view(cx);
        let page = self.page_view(cx);
        let width = self.window_w - if self.window_w < 900.0 { 64.0 } else { 220.0 };
        let mut col = div()
            .id("content-scroll")
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .p(u(20.))
            .flex()
            .flex_col()
            .gap(u(14.));
        let _ = width;
        if let Some(s) = sheet {
            col = col.child(s);
        }
        for b in banners.drain(..) {
            col = col.child(b);
        }
        if let Some(n) = self
            .notice
            .for_page(
                self.page,
                match &self.detail {
                    Detail::Addon(k) => Some(k.as_str()),
                    _ => None,
                },
            )
            .map(str::to_string)
        {
            let dismiss = self.btn(
                "notice-dismiss",
                "Dismiss",
                Bk::Secondary,
                Act::DismissNotice,
                cx,
            );
            col = col.child(
                div()
                    .id("action-notice")
                    .role(Role::Status)
                    .aria_label(SharedString::from(n.clone()))
                    .p(u(12.))
                    .rounded(u(8.))
                    .bg(rgb(pal.surface))
                    .border_1()
                    .border_color(rgb(pal.divider))
                    .flex()
                    .items_center()
                    .gap(u(12.))
                    .child(div().flex_1().min_w_0().child(SharedString::from(n)))
                    .child(dismiss),
            );
        }
        col = col.child(page);
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(rgb(pal.bg))
            .child(col)
            .into_any_element()
    }

    fn sheet_view(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pal = self.pal();
        let sheet = self.sheet.clone()?;
        let (msg, first, other) = match &sheet {
            Sheet::FirstClose => (
                "WoW Sync keeps running in the tray and keeps your add-ons in sync. To stop it, choose Quit WoW Sync from the tray menu.".to_string(),
                ("Hide window", Act::SheetHide),
                ("Quit WoW Sync", Act::SheetQuit),
            ),
            Sheet::Quit(t) => (
                t.clone(),
                ("Quit when safe", Act::SheetQuitWhenSafe),
                ("Keep running", Act::SheetKeepRunning),
            ),
        };
        self.in_sheet = true;
        let a = self.btn("sheet-default", first.0, Bk::Primary, first.1, cx);
        let b = self.btn("sheet-other", other.0, Bk::Secondary, other.1, cx);
        self.in_sheet = false;
        Some(
            div()
                .id("sheet")
                .role(Role::Dialog)
                .aria_label(SharedString::from(msg.clone()))
                .p(u(16.))
                .rounded(u(8.))
                .bg(rgb(pal.surface))
                .border_2()
                .border_color(rgb(pal.accent))
                .flex()
                .items_center()
                .gap(u(16.))
                .child(div().flex_1().min_w_0().child(SharedString::from(msg)))
                .child(a)
                .child(b)
                .into_any_element(),
        )
    }

    pub fn needs_target_act(t: &NeedTarget) -> Act {
        match t {
            NeedTarget::Error | NeedTarget::Risk => Act::Nav(Page::Needs),
            NeedTarget::Restore => Act::Nav(Page::Addons),
            NeedTarget::Reconfirm => Act::OpenReconfirm,
            NeedTarget::TestMode => Act::TurnOffTestMode,
            NeedTarget::NeedsHelp(k) => Act::OpenNeedsHelp(k.clone()),
            NeedTarget::Decision(k) => Act::OpenDecision(k.clone()),
            NeedTarget::Computer(i) => Act::OpenComputer(*i),
        }
    }
}

#[cfg(test)]
mod log_tests {
    use super::*;
    use crate::features::GroupAction;
    use wow_sync_core::ids::Sha256;

    #[test]
    fn the_action_log_names_only_the_kind_and_never_a_name_or_path() {
        let secret = "Account#Secret";
        let acts = [
            Act::OpenAddon(secret.into()),
            Act::OpenDecision(secret.into()),
            Act::ToggleExpand(secret.into()),
            Act::Toggle(secret.into()),
            Act::OpenNeedsHelp("C:\\Users\\Someone\\save.zip".into()),
            Act::Setup(SetupAct::JoinGroup(3)),
        ];
        for a in &acts {
            let l = act_label(a);
            assert!(!l.contains(secret) && !l.contains("Someone"), "{l}");
            assert!(!l.contains('(') && !l.contains('3'), "{l}");
        }
        assert_eq!(act_label(&acts[0]), "OpenAddon");
        let g = Act::Group(GroupAction::Rollback {
            package: wow_sync_core::paths::validate_folder_name("SecretAddon").unwrap(),
            recovered: Sha256([7; 32]),
        });
        let l = act_label(&g);
        assert_eq!(l, "Group Rollback");
        assert!(!l.contains("SecretAddon"));
    }
}
