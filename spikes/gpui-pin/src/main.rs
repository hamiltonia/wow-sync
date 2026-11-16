// Feasibility spike (run-002 R2-pin): GPUI on a pinned Zed release tag. Not product code.
// Tray + hide/show/close->hide + single instance + keep-alive without a window +
// AccessKit-labelled controls, in one console-less (windows subsystem) binary.
// Lifecycle logic is Windows-only; other targets only need to compile (build-only).
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::cell::{Cell, RefCell};
use std::io::Write;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::accesskit::{Role, Toggled};
use gpui::{
    actions, div, prelude::*, px, rgb, size, App, AsyncApp, Bounds, Context, FocusHandle,
    KeyBinding, QuitMode, Render, SharedString, TitlebarOptions, Window, WindowBounds,
    WindowHandle, WindowOptions,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

actions!(spike_pin, [FocusNextStop, FocusPrevStop]);

thread_local! {
    static START: Instant = Instant::now();
}

// Windows-subsystem builds have no console: log to the file named by WOWSYNC_LOG.
fn log(msg: &str) {
    let ms = START.with(|s| s.elapsed().as_millis());
    let line = format!("[{:>6} ms] pid={} {}\n", ms, std::process::id(), msg);
    if let Ok(path) = std::env::var("WOWSYNC_LOG") {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = f.write_all(line.as_bytes());
        }
    }
}

fn mark(s: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    log(&format!("MARK {now} {s}"));
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    #[link(name = "user32")]
    extern "system" {
        pub fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
        pub fn IsWindowVisible(hwnd: isize) -> i32;
        pub fn SetForegroundWindow(hwnd: isize) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        pub fn CreateMutexW(attr: *const c_void, owner: i32, name: *const u16) -> isize;
        pub fn CreateEventW(attr: *const c_void, manual: i32, init: i32, name: *const u16)
            -> isize;
        pub fn SetEvent(h: isize) -> i32;
        pub fn WaitForSingleObject(h: isize, ms: u32) -> u32;
        pub fn GetLastError() -> u32;
    }
    pub const SW_HIDE: i32 = 0;
    pub const SW_RESTORE: i32 = 9;
    pub fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(windows)]
const MUTEX_NAME: &str = "Local\\WowSyncSpikeGpuiPinSingleInstance";
#[cfg(windows)]
const EVENT_NAME: &str = "Local\\WowSyncSpikeGpuiPinShow";

struct Root {
    checked: bool,
    clicks: [u32; 2],
    b1: FocusHandle,
    b2: FocusHandle,
    toggle: FocusHandle,
    items: Vec<FocusHandle>,
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ring = |on: bool| if on { rgb(0xffcc00) } else { rgb(0x475569) };
        let button = |id: &'static str,
                      label: String,
                      h: &FocusHandle,
                      idx: isize,
                      which: usize,
                      w: &Window,
                      cx: &mut Context<Self>| {
            div()
                .id(id)
                .role(Role::Button)
                .aria_label(SharedString::from(label.clone()))
                .track_focus(h)
                .tab_index(idx)
                .px_3()
                .py_1()
                .border_2()
                .border_color(ring(h.is_focused(w)))
                .bg(rgb(0x334155))
                .child(SharedString::from(label))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.clicks[which] += 1;
                    cx.notify();
                }))
        };
        let list = (0..10).map(|i| {
            let h = &self.items[i];
            div()
                .id(("item", i))
                .role(Role::ListItem)
                .aria_label(SharedString::from(format!("List item {}", i + 1)))
                .aria_position_in_set(i + 1)
                .aria_size_of_set(10)
                .track_focus(h)
                .tab_index(10 + i as isize)
                .px_2()
                .border_1()
                .border_color(ring(h.is_focused(window)))
                .child(SharedString::from(format!("List item {}", i + 1)))
        });
        let on = self.checked;
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .size_full()
            .on_action(cx.listener(|_, _: &FocusNextStop, w, cx| w.focus_next(cx)))
            .on_action(cx.listener(|_, _: &FocusPrevStop, w, cx| w.focus_prev(cx)))
            .bg(rgb(0x1e293b))
            .text_color(rgb(0xffffff))
            .child(
                div()
                    .id("heading")
                    .text_xl()
                    .role(Role::Heading)
                    .aria_level(1)
                    .aria_label("WoW Sync Pin Heading")
                    .child("WoW Sync Pin Heading"),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(button(
                        "btn1",
                        format!("Sync now ({})", self.clicks[0]),
                        &self.b1,
                        2,
                        0,
                        window,
                        cx,
                    ))
                    .child(button(
                        "btn2",
                        format!("Settings ({})", self.clicks[1]),
                        &self.b2,
                        3,
                        1,
                        window,
                        cx,
                    )),
            )
            .child(
                div()
                    .id("toggle")
                    .role(Role::CheckBox)
                    .aria_label("Enable sync")
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .track_focus(&self.toggle)
                    .tab_index(4)
                    .px_2()
                    .border_2()
                    .border_color(ring(self.toggle.is_focused(window)))
                    .child(SharedString::from(format!(
                        "[{}] Enable sync",
                        if on { "x" } else { " " }
                    )))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.checked = !this.checked;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("items")
                    .flex()
                    .flex_col()
                    .role(Role::List)
                    .aria_label("Items")
                    .children(list),
            )
    }
}

type Slot = Rc<RefCell<Option<WindowHandle<Root>>>>;

fn hwnd_of(window: &Window) -> isize {
    match HasWindowHandle::window_handle(window).map(|h| h.as_raw()) {
        #[cfg(windows)]
        Ok(RawWindowHandle::Win32(h)) => h.hwnd.get(),
        _ => 0,
    }
}

fn hwnd_of_slot(cx: &mut App, slot: &Slot) -> Option<isize> {
    let h = (*slot.borrow())?;
    h.update(cx, |_, w, _| hwnd_of(w)).ok()
}

fn visible(hwnd: isize) -> bool {
    #[cfg(windows)]
    unsafe {
        win::IsWindowVisible(hwnd) != 0
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        false
    }
}

fn set_visible(hwnd: isize, show: bool) {
    #[cfg(windows)]
    unsafe {
        if show {
            win::ShowWindow(hwnd, win::SW_RESTORE);
            win::SetForegroundWindow(hwnd);
        } else {
            win::ShowWindow(hwnd, win::SW_HIDE);
        }
    }
    #[cfg(not(windows))]
    let _ = (hwnd, show);
}

// `destroy`: the close button really closes the window (no windows left alive);
// otherwise close is intercepted and the window is hidden.
fn open(cx: &mut App, slot: &Slot, destroy: bool) {
    let bounds = Bounds::centered(None, size(px(480.), px(520.)), cx);
    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("WoW Sync Pin Spike".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let mk = || cx.focus_handle().tab_stop(true);
                    let r = Root {
                        checked: false,
                        clicks: [0, 0],
                        b1: mk(),
                        b2: mk(),
                        toggle: mk(),
                        items: (0..10).map(|_| mk()).collect(),
                    };
                    window.focus(&r.b1, cx);
                    r
                })
            },
        )
        .expect("open_window");
    handle
        .update(cx, |_, window, cx| {
            let h = hwnd_of(window);
            if let Some(s) = window.gpu_specs() {
                log(&format!(
                    "gpu_specs: device={} driver={} software_emulated={}",
                    s.device_name, s.driver_name, s.is_software_emulated
                ));
            }
            log(&format!("window opened hwnd=0x{h:x}"));
            window.on_window_should_close(cx, move |_, _| {
                if destroy {
                    log("close requested -> window destroyed (keep-alive test)");
                    true
                } else {
                    log("close requested -> intercepted, hiding to tray");
                    set_visible(h, false);
                    false
                }
            });
        })
        .ok();
    *slot.borrow_mut() = Some(handle);
}

// Show: restore the hidden window, or open a fresh one if it was destroyed.
fn show(cx: &mut App, slot: &Slot, destroy: bool) {
    match hwnd_of_slot(cx, slot) {
        Some(h) => set_visible(h, true),
        None => open(cx, slot, destroy),
    }
    if let Some(h) = hwnd_of_slot(cx, slot) {
        log(&format!("shown, visible={}", visible(h)));
    }
}

// Hide-like action for memory scenarios: SW_HIDE (window kept) or real destroy.
fn hide(cx: &mut App, slot: &Slot, destroy: bool) {
    if destroy {
        let h = slot.borrow_mut().take();
        if let Some(h) = h {
            let _ = h.update(cx, |_, window, _| window.remove_window());
        }
    } else if let Some(h) = hwnd_of_slot(cx, slot) {
        set_visible(h, false);
    }
}

fn tray_icon_image() -> Icon {
    let (w, h) = (32u32, 32u32);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..(w * h) {
        px.extend_from_slice(&[0x2b, 0x7f, 0xff, 0xff]);
    }
    Icon::from_rgba(px, w, h).expect("icon")
}

/// Returns the show-event handle if this is the first instance; otherwise signals
/// the first instance and returns None.
#[cfg(windows)]
fn single_instance() -> Option<isize> {
    // WOWSYNC_INSTANCE only lets the measurement harness run independent copies side by side.
    let suffix = std::env::var("WOWSYNC_INSTANCE").unwrap_or_default();
    unsafe {
        let _mutex =
            win::CreateMutexW(std::ptr::null(), 1, win::wide(&format!("{MUTEX_NAME}{suffix}")).as_ptr());
        let existed = win::GetLastError() == 183; // ERROR_ALREADY_EXISTS
        let ev =
            win::CreateEventW(std::ptr::null(), 0, 0, win::wide(&format!("{EVENT_NAME}{suffix}")).as_ptr());
        if existed {
            win::SetEvent(ev);
            None
        } else {
            Some(ev)
        }
    }
}
#[cfg(not(windows))]
fn single_instance() -> Option<isize> {
    Some(0)
}

// Usage: spike-gpui-pin [normal | destroy | hidden | show-hide SECS | cycles N]
//   normal:      window shown; close hides to tray (default)
//   destroy:     window shown; close really destroys it; tray Show reopens
//   hidden:      no window at start, tray + keep-alive only
//   show-hide:   show SECS, then hide (WOWSYNC_HIDE=sw: SW_HIDE, else destroy)
//   cycles N:    N times 1.5 s shown / 1.5 s hidden, then idle
fn main() {
    START.with(|_| {});
    let Some(show_event) = single_instance() else {
        log("second instance: signalled first instance to show; exiting");
        return;
    };
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).cloned().unwrap_or_else(|| "normal".into());
    let n: u64 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(0);
    let sw_hide = std::env::var("WOWSYNC_HIDE").map(|v| v == "sw").unwrap_or(false);
    log(&format!("start mode={mode} n={n} sw_hide={sw_hide}"));

    gpui_platform::application()
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx: &mut App| {
            cx.bind_keys([
                KeyBinding::new("tab", FocusNextStop, None),
                KeyBinding::new("shift-tab", FocusPrevStop, None),
            ]);
            let slot: Slot = Rc::new(RefCell::new(None));
            let destroy_close = mode == "destroy" || mode == "hidden";
            let hide_destroys = !sw_hide;

            let menu = Menu::new();
            let show_item = MenuItem::new("Show", true, None);
            let quit_item = MenuItem::new("Quit", true, None);
            menu.append_items(&[&show_item, &quit_item]).unwrap();
            let tray = TrayIconBuilder::new()
                .with_tooltip("WoW Sync pin spike")
                .with_icon(tray_icon_image())
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()
                .expect("tray");
            log("tray icon created");
            let (show_id, quit_id) = (show_item.id().clone(), quit_item.id().clone());

            if mode == "normal" || mode == "destroy" {
                open(cx, &slot, destroy_close);
                cx.activate(true);
            }

            // tray-icon delivers via channels from its hidden window on GPUI's main
            // thread message loop; poll them from a GPUI task.
            let slot_p = slot.clone();
            cx.spawn(async move |cx: &mut AsyncApp| {
                let _tray = tray;
                loop {
                    cx.background_executor().timer(Duration::from_millis(50)).await;
                    let want_show = Cell::new(false);
                    let want_quit = Cell::new(false);
                    while let Ok(e) = TrayIconEvent::receiver().try_recv() {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = e
                        {
                            log("tray left-click event");
                            let sp = slot_p.clone();
                            let was_visible =
                                cx.update(|cx| hwnd_of_slot(cx, &sp).map(visible).unwrap_or(false));
                            if was_visible {
                                cx.update(|cx| {
                                    if let Some(h) = hwnd_of_slot(cx, &sp) {
                                        set_visible(h, false);
                                    }
                                });
                                log("tray click: hid window");
                            } else {
                                want_show.set(true);
                            }
                        }
                    }
                    while let Ok(e) = MenuEvent::receiver().try_recv() {
                        if e.id == show_id {
                            log("tray menu: Show");
                            want_show.set(true);
                        } else if e.id == quit_id {
                            log("tray menu: Quit");
                            want_quit.set(true);
                        }
                    }
                    #[cfg(windows)]
                    if unsafe { win::WaitForSingleObject(show_event, 0) } == 0 {
                        log("second-instance signal received");
                        want_show.set(true);
                    }
                    #[cfg(not(windows))]
                    let _ = show_event;
                    if want_show.get() {
                        let sp = slot_p.clone();
                        cx.update(|cx| show(cx, &sp, destroy_close));
                    }
                    if want_quit.get() {
                        log("quitting via cx.quit()");
                        cx.update(|cx| cx.quit());
                        return;
                    }
                }
            })
            .detach();

            let slot_m = slot.clone();
            cx.spawn(async move |cx: &mut AsyncApp| {
                let sleep = |ms: u64| cx.background_executor().timer(Duration::from_millis(ms));
                match mode.as_str() {
                    "hidden" => mark("hidden-idle"),
                    "show-hide" => {
                        let s = slot_m.clone();
                        cx.update(|cx| open(cx, &s, false));
                        mark("shown");
                        sleep(n * 1000).await;
                        let s = slot_m.clone();
                        cx.update(|cx| hide(cx, &s, hide_destroys));
                        mark("hidden-idle");
                    }
                    "cycles" => {
                        for i in 0..n {
                            let s = slot_m.clone();
                            cx.update(|cx| show(cx, &s, hide_destroys));
                            sleep(1500).await;
                            let s = slot_m.clone();
                            cx.update(|cx| hide(cx, &s, hide_destroys));
                            sleep(1500).await;
                            mark(&format!("cycle {}", i + 1));
                        }
                        mark("hidden-idle");
                    }
                    _ => {}
                }
            })
            .detach();
        });
    log("run() returned; process exiting");
}
