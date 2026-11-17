//! Prototype state, actions, timers, tray and the window shell. Pages live in `pages.rs` and
//! `setup.rs`; wording and derivations live in the pure `model.rs`.

use crate::model::{
    self, banners, headline, need_items, window_title, BannerAction, Detail, Filter, NeedTarget,
    Page, NAV,
};
use crate::scenario::{Busy, Kind, ScenarioId, Step, World};
use crate::theme::{Mode, Palette, ThemeChoice, Tone};
use crate::ui::{icon, u, zoom};
use crate::win;
use gpui::accesskit::{Role, Toggled};
use gpui::{
    actions, div, prelude::*, rgb, transparent_black, AnyElement, App, Context, Div, FocusHandle,
    FontWeight, KeyDownEvent, Render, ScrollHandle, SharedString, Stateful, Window,
    WindowAppearance,
};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

actions!(
    wow_sync_proto,
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
        BackKey,
        NextScenario
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
        K::new("ctrl-shift-n", NextScenario, None),
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
    Keep(String, usize),
    TryAgainShared,
    NeedsHelpChoose,
    ToggleExpand(String),
    Toggle(String),
    Theme(ThemeChoice),
    CheckUpdate,
    Scenario(ScenarioId),
    SimWowClosed,
    SimDen,
    SimGame,
    SheetHide,
    SheetQuit,
    SheetKeepRunning,
    SheetQuitWhenSafe,
    Quit,
    ExplainCloseAgain,
    Setup(SetupAct),
    Reconfirm(ReconfAct),
    CopyVersion,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SetupAct {
    Next,
    Back,
    ChooseFolder,
    TryAgain,
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
    pub identity_ok: bool,
    pub scope_ok: bool,
    pub group: Option<Option<usize>>,
    pub name: String,
    pub working: bool,
    pub finished: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ReconfState {
    pub identity_ok: bool,
    pub scope_ok: bool,
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
    pub scenario: ScenarioId,
    pub theme: ThemeChoice,
    pub page: Option<Page>,
    pub detail: Option<String>,
    pub sheet: Option<String>,
    pub filter: Option<Filter>,
    pub setup_step: Option<u8>,
    pub start_hidden: bool,
}

pub struct ProtoApp {
    pub w: World,
    pub epoch: u64,
    pub sync_epoch: u64,
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
    pub toggles: HashMap<String, bool>,
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
    pub hwnd: isize,
    pub visible: bool,
    pub tray: Option<TrayState>,
    pub last_title: String,
    pub notice: Option<String>,
    pub nh_working: bool,
    pub copied: bool,
    pub region: usize,
    pub window_w: f32,
    pub window_h: f32,
}

pub fn log(msg: &str) {
    if let Ok(path) = std::env::var("WOWSYNC_PROTO_LOG") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "pid={} {}", std::process::id(), msg);
        }
    }
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

impl ProtoApp {
    pub fn new(args: Args, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut toggles = HashMap::new();
        for k in ["signin", "hidden", "summary", "needsme"] {
            toggles.insert(k.to_string(), true);
        }
        let mut app = ProtoApp {
            w: World::new(args.scenario),
            epoch: 0,
            sync_epoch: 0,
            page: Page::Overview,
            detail: Detail::None,
            detail_stack: Vec::new(),
            focus_stack: Vec::new(),
            filter: Filter::All,
            act_filter: 0,
            theme: args.theme,
            system: mode_of(window.appearance()),
            sheet: None,
            sheet_from: None,
            in_sheet: false,
            first_close_done: false,
            toggles,
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
            hwnd: hwnd_of(window),
            visible: true,
            tray: None,
            last_title: String::new(),
            notice: None,
            nh_working: false,
            copied: false,
            region: 0,
            window_w: 1040.0,
            window_h: 720.0,
        };
        app.start_world(cx);
        if let Some(p) = args.page {
            app.page = p;
        }
        if let Some(f) = args.filter {
            app.filter = f;
        }
        if let Some(d) = &args.detail {
            app.open_detail_arg(d);
        }
        if let Some(s) = args.setup_step {
            app.setup.step = s.clamp(1, 6);
        }
        match args.sheet.as_deref() {
            Some("first-close") => app.sheet = Some(Sheet::FirstClose),
            Some("quit") => {
                app.sheet = Some(Sheet::Quit(
                    "WoW Sync is updating Bagnon. Quitting waits for the current step to finish safely."
                        .into(),
                ))
            }
            _ => {}
        }
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
        if args.start_hidden {
            app.visible = false;
        }
        cx.spawn(async move |this, cx| loop {
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
            if this
                .update(cx, |t, cx| {
                    for e in evs {
                        t.tray_event(e, cx);
                    }
                })
                .is_err()
            {
                break;
            }
        })
        .detach();
        app
    }

    pub fn pal(&self) -> Palette {
        Palette::of(self.theme.resolve(self.system))
    }

    pub fn mode(&self) -> Mode {
        self.theme.resolve(self.system)
    }

    // ---- world lifecycle ----

    pub fn start_world(&mut self, cx: &mut Context<Self>) {
        self.epoch += 1;
        self.sync_epoch += 1;
        self.page = Page::Overview;
        self.detail = Detail::None;
        self.detail_stack.clear();
        self.focus_stack.clear();
        self.filter = Filter::All;
        self.act_filter = 0;
        self.sheet = None;
        self.expanded.clear();
        self.rov.clear();
        self.notice = None;
        self.nh_working = false;
        self.reconf = ReconfState::default();
        self.setup = SetupState {
            step: 1,
            name: "Desk PC".into(),
            ..SetupState::default()
        };
        self.pending_focus = Some("@content".into());
        match self.w.scenario {
            ScenarioId::S4 => self.after(6.0, Step::FinishSyncing, true, cx),
            ScenarioId::S7 => self.after(4.0, Step::ApplyDetails, false, cx),
            _ => {}
        }
        log(&format!("scenario loaded {}", self.w.scenario.code()));
    }

    pub fn load_scenario(&mut self, id: ScenarioId, cx: &mut Context<Self>) {
        self.w = World::new(id);
        self.start_world(cx);
        cx.notify();
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

    pub fn after(&mut self, secs: f32, step: Step, sync: bool, cx: &mut Context<Self>) {
        let (epoch, sync_epoch) = (self.epoch, self.sync_epoch);
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis((secs * 1000.0) as u64))
                .await;
            this.update(cx, |t, cx| {
                if t.epoch == epoch && (!sync || t.sync_epoch == sync_epoch) {
                    t.apply_step(step, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    fn apply_step(&mut self, step: Step, cx: &mut Context<Self>) {
        match &step {
            Step::FinishSyncing if self.w.is_paused() => return,
            Step::SetupDone => {
                self.w.step(step);
                self.setup.working = false;
                self.setup.finished = true;
                self.w.enrolled = false;
                cx.notify();
                return;
            }
            _ => {}
        }
        let keep = if let Step::DecisionSaved(k) = &step {
            Some(k.clone())
        } else {
            None
        };
        self.w.step(step);
        if let Some(k) = keep {
            self.after(3.0, Step::DecisionApplied(k), false, cx);
        }
        if self.w.needs_help_resolved && self.nh_working {
            self.nh_working = false;
            self.detail = Detail::None;
            self.detail_stack.clear();
        }
        cx.notify();
    }

    // ---- actions ----

    pub fn do_act(&mut self, act: Act, window: &mut Window, cx: &mut Context<Self>) {
        if self.sheet.is_some()
            && !matches!(
                act,
                Act::SheetHide | Act::SheetQuit | Act::SheetKeepRunning | Act::SheetQuitWhenSafe
            )
        {
            return;
        }
        log(&format!("act {act:?}"));
        self.notice = None;
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
            Act::Keep(k, c) => {
                self.w.choose_copy(&k, c);
                self.w.recompute();
                self.after(1.5, Step::DecisionSaved(k), false, cx);
            }
            Act::TryAgainShared => {
                self.w.last_check_secs = Some(0);
                self.notice = Some("Checking the shared folder…".into());
                self.after(2.0, Step::TryAgainDone, false, cx);
            }
            Act::NeedsHelpChoose => {
                self.nh_working = true;
                self.after(2.0, Step::NeedsHelpResolved, false, cx);
            }
            Act::ToggleExpand(k) => {
                if !self.expanded.remove(&k) {
                    self.expanded.insert(k);
                }
            }
            Act::Toggle(k) => {
                let v = self.toggles.get(&k).copied().unwrap_or(false);
                self.toggles.insert(k, !v);
                self.notice = Some("Saved".into());
            }
            Act::Theme(t) => {
                self.theme = t;
                log(&format!("theme set {:?}", t));
            }
            Act::CheckUpdate => {
                self.w.update.checking = true;
                self.after(2.0, Step::UpdateChecked, false, cx);
            }
            Act::Scenario(id) => {
                self.load_scenario(id, cx);
                self.pending_focus = Some("@content".into());
            }
            Act::SimWowClosed => {
                self.w.simulate_wow_closed();
                self.notice = Some("WoW closed: held notifications were released.".into());
            }
            Act::SimDen => self.w.simulate_den_reports(),
            Act::SimGame => self.w.simulate_game_patched(),
            Act::SheetHide => {
                self.first_close_done = true;
                self.close_sheet();
                self.hide();
            }
            Act::SheetQuit | Act::SheetQuitWhenSafe => {
                self.close_sheet();
                cx.quit();
            }
            Act::SheetKeepRunning => self.close_sheet(),
            Act::Quit => self.request_quit(cx),
            Act::ExplainCloseAgain => {
                self.first_close_done = false;
                self.notice = Some("The close explanation will show next time.".into());
            }
            Act::Setup(a) => self.setup_act(a, cx),
            Act::Reconfirm(a) => self.reconf_act(a),
            Act::CopyVersion => self.copied = true,
        }
        let _ = window;
        cx.notify();
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

    fn sync_now(&mut self, cx: &mut Context<Self>) {
        if !self.w.enrolled || self.w.is_paused() || self.w.busy.is_some() {
            return;
        }
        self.w.busy = Some(Busy::Syncing { done: 0, total: 0 });
        self.w.sync_now();
        self.w.recompute();
        self.after(2.0, Step::FinishSync, true, cx);
    }

    fn toggle_pause(&mut self, cx: &mut Context<Self>) {
        self.sync_epoch += 1;
        if self.w.is_paused() {
            self.w.resume();
            self.w.busy = None;
            let n = self
                .w
                .addons
                .iter()
                .filter(|a| a.kind == Kind::Available)
                .count() as u32;
            if n > 0 {
                self.w.busy = Some(Busy::Syncing {
                    done: 2.min(n),
                    total: n,
                });
                self.w.recompute();
                self.after(6.0, Step::FinishSyncing, true, cx);
            } else {
                self.w.recompute();
            }
        } else {
            let was_busy = self.w.busy.is_some();
            self.w.pause();
            if was_busy {
                self.w.busy = Some(Busy::Pausing);
                self.after(1.5, Step::FinishSync, true, cx);
            }
            self.w.recompute();
        }
    }

    fn request_quit(&mut self, cx: &mut Context<Self>) {
        match model::sheet_quit_text(&self.w) {
            Some(t) => {
                self.sheet_from = Some(self.last_key.clone());
                self.sheet = Some(Sheet::Quit(t));
                self.show();
            }
            None => cx.quit(),
        }
    }

    pub fn request_close(&mut self, cx: &mut Context<Self>) {
        log("close requested");
        if self.sheet.is_some() {
            return;
        }
        if self.w.enrolled && !self.first_close_done {
            self.sheet_from = Some(self.last_key.clone());
            self.sheet = Some(Sheet::FirstClose);
            cx.notify();
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

    fn setup_act(&mut self, a: SetupAct, cx: &mut Context<Self>) {
        let data = self.w.setup.clone();
        let s = &mut self.setup;
        match a {
            SetupAct::Next => {
                if s.step < 6 {
                    s.step += 1;
                }
            }
            SetupAct::Back => {
                s.step = s.step.saturating_sub(1).max(1);
            }
            SetupAct::ChooseFolder => s.folder_chosen = true,
            SetupAct::TryAgain => {}
            SetupAct::SelectInstall(i) => {
                s.install = i;
                s.identity_ok = false;
                s.scope_ok = false;
            }
            SetupAct::IdentityToggle => s.identity_ok = !s.identity_ok,
            SetupAct::ScopeToggle => s.scope_ok = !s.scope_ok,
            SetupAct::CreateGroup => s.group = Some(None),
            SetupAct::JoinGroup(i) => s.group = Some(Some(i)),
            SetupAct::SetUp => {
                s.working = true;
                self.after(2.0, Step::SetupDone, false, cx);
            }
            SetupAct::Finish => {
                let name = self.setup.name.clone();
                let id = self.w.scenario;
                self.w = World::new(ScenarioId::S3);
                self.w.computer = name;
                let _ = (id, data);
                self.start_world(cx);
                self.first_close_done = false;
            }
        }
        self.pending_focus = Some("@content".into());
    }

    fn reconf_act(&mut self, a: ReconfAct) {
        let r = &mut self.reconf;
        match a {
            ReconfAct::IdentityToggle => r.identity_ok = !r.identity_ok,
            ReconfAct::ScopeToggle => r.scope_ok = !r.scope_ok,
            ReconfAct::NotNow => {
                r.not_now = true;
                r.note = Some("Syncing stays stopped until you confirm.".into());
            }
            ReconfAct::Confirm => {
                if !(r.identity_ok && r.scope_ok) {
                    r.note = Some(
                        "Confirm both the game version and the add-on list to continue. Nothing has been changed."
                            .into(),
                    );
                } else {
                    self.w.reconfirm = None;
                    self.w.activity.insert(
                        0,
                        crate::scenario::ActivityItem {
                            title: "Game version confirmed; syncing continues".into(),
                            items: vec![],
                            problem: false,
                            day: "Today".into(),
                            when: "21:11".into(),
                        },
                    );
                    self.w.recompute();
                    self.reconf = ReconfState::default();
                    self.detail = Detail::None;
                    self.detail_stack.clear();
                    self.notice = Some("Confirmed. Syncing continues.".into());
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
                log("tray left click");
                if self.visible {
                    self.hide();
                } else {
                    self.show();
                }
            }
            TrayEv::Menu(id) => {
                log("tray menu item");
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
        self.clickable(key, act, cx)
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

impl Render for ProtoApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vs = window.viewport_size();
        self.window_w = f32::from(vs.width) / zoom();
        self.window_h = f32::from(vs.height) / zoom();
        self.rov_keys.clear();
        self.content_first = None;
        let pal = self.pal();
        let setup_mode = !self.w.enrolled;

        let title = if setup_mode {
            window_title("Set up this computer")
        } else {
            window_title(self.page.title())
        };
        if title != self.last_title {
            window.set_window_title(&title);
            self.last_title = title;
        }

        self.capture_first = true;
        let content = if setup_mode {
            self.setup_view(cx)
        } else {
            self.content_view(cx)
        };
        let content_first = self.content_first.clone();
        self.capture_first = false;

        let body: AnyElement = if setup_mode {
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

        if let Some(k) = self.pending_focus.take() {
            let key = if k == "@content" {
                content_first.unwrap_or_default()
            } else {
                k
            };
            if let Some(h) = self.handles.get(&key).cloned() {
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

        let strip = div()
            .id("proto-strip")
            .h(u(26.))
            .flex_none()
            .flex()
            .items_center()
            .px(u(12.))
            .bg(rgb(pal.divider))
            .text_color(rgb(pal.text))
            .text_size(u(12.))
            .role(Role::Status)
            .aria_label("Prototype: fake data, nothing is synced")
            .child("Prototype: fake data, nothing is synced")
            .child(div().flex_1())
            .child(SharedString::from(format!(
                "Scenario {} · {} · {}{}",
                self.w.scenario.code(),
                self.w.scenario.title(),
                if self.mode() == Mode::Dark {
                    "dark"
                } else {
                    "light"
                },
                if (zoom() - 1.0).abs() > 0.01 {
                    format!(" · app zoom {:.0}%", zoom() * 100.0)
                } else {
                    String::new()
                }
            )));

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
            .on_action(cx.listener(|t, _: &NextScenario, w, cx| {
                let n = t.w.scenario.next();
                t.do_act(Act::Scenario(n), w, cx)
            }))
            .child(div().flex_1().min_h_0().child(body))
            .child(strip)
    }
}

impl ProtoApp {
    fn in_sheet_key(&self, h: &FocusHandle) -> bool {
        ["sheet-default", "sheet-other"]
            .iter()
            .any(|k| self.handles.get(*k).map(|x| x == h).unwrap_or(false))
    }

    fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.sheet.clone() {
            Some(Sheet::FirstClose) => self.do_act(Act::SheetHide, window, cx),
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
            if matches!(p, Page::AppSettings | Page::Scenarios) {
                col = col.child(div().h(u(1.)).my(u(6.)).bg(rgb(pal.divider)));
            }
            if p == Page::Scenarios {
                col = col.child(
                    div()
                        .px(u(8.))
                        .pb(u(2.))
                        .text_size(u(11.))
                        .text_color(rgb(pal.muted))
                        .child(if collapsed { "Proto" } else { "Prototype" }),
                );
            }
            let current = self.page == p;
            let mut name = p.title().to_string();
            if p == Page::Needs && needs > 0 {
                name = format!("{name}, {needs} items");
            }
            if p == Page::SettingsSync {
                name = format!("{name}, coming later");
            }
            if current {
                name = format!("{name}, current page");
            }
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
                        .when(p == Page::SettingsSync, |d| {
                            d.child(
                                div()
                                    .text_size(u(11.))
                                    .text_color(rgb(pal.muted))
                                    .child("Coming later"),
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
                .child(div().flex_1().child(SharedString::from(msg)))
                .child(a)
                .child(b)
                .into_any_element(),
        )
    }

    pub fn needs_target_act(t: &NeedTarget) -> Act {
        match t {
            NeedTarget::Error => Act::Nav(Page::Needs),
            NeedTarget::Reconfirm => Act::OpenReconfirm,
            NeedTarget::NeedsHelp(k) => Act::OpenNeedsHelp(k.clone()),
            NeedTarget::Decision(k) => Act::OpenDecision(k.clone()),
            NeedTarget::Computer(i) => Act::OpenComputer(*i),
        }
    }
}
