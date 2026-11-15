// Feasibility spike: GPUI window + tray-icon lifecycle. Not product code.
// Lifecycle logic is Windows-only; other targets only need to compile (build-only).
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    div, prelude::*, px, rgb, size, App, Application, AsyncApp, Bounds, Context, Render, Window,
    WindowBounds, WindowHandle, WindowOptions,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

thread_local! {
    static START: Instant = Instant::now();
}

fn log(msg: &str) {
    let ms = START.with(|s| s.elapsed().as_millis());
    println!("[{:>6} ms] pid={} {}", ms, std::process::id(), msg);
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    #[link(name = "user32")]
    extern "system" {
        pub fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
        pub fn IsWindowVisible(hwnd: isize) -> i32;
        pub fn SetForegroundWindow(hwnd: isize) -> i32;
        pub fn PostMessageW(hwnd: isize, msg: u32, w: usize, l: isize) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        pub fn CreateMutexW(attr: *const c_void, owner: i32, name: *const u16) -> isize;
        pub fn CreateEventW(
            attr: *const c_void,
            manual: i32,
            init: i32,
            name: *const u16,
        ) -> isize;
        pub fn SetEvent(h: isize) -> i32;
        pub fn WaitForSingleObject(h: isize, ms: u32) -> u32;
        pub fn GetLastError() -> u32;
    }
    pub const SW_HIDE: i32 = 0;
    pub const SW_RESTORE: i32 = 9;
    pub const WM_CLOSE: u32 = 0x0010;
    pub fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(windows)]
const MUTEX_NAME: &str = "Local\\WowSyncSpikeGpuiSingleInstance";
#[cfg(windows)]
const EVENT_NAME: &str = "Local\\WowSyncSpikeGpuiShow";

struct Root;

impl Render for Root {
    fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .justify_center()
            .items_center()
            .bg(rgb(0x1e293b))
            .text_color(rgb(0xffffff))
            .child("WoW Sync GPUI spike window")
    }
}

fn hwnd_of(window: &Window) -> isize {
    match HasWindowHandle::window_handle(window).map(|h| h.as_raw()) {
        #[cfg(windows)]
        Ok(RawWindowHandle::Win32(h)) => h.hwnd.get(),
        _ => 0,
    }
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
    unsafe {
        let _mutex = win::CreateMutexW(std::ptr::null(), 1, win::wide(MUTEX_NAME).as_ptr());
        let existed = win::GetLastError() == 183; // ERROR_ALREADY_EXISTS
        let ev = win::CreateEventW(std::ptr::null(), 0, 0, win::wide(EVENT_NAME).as_ptr());
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

fn main() {
    START.with(|_| {});
    let Some(show_event) = single_instance() else {
        log("second instance: signalled first instance to show; exiting");
        return;
    };
    let selftest = std::env::var("WOWSYNC_SPIKE_SELFTEST").is_ok();
    log(&format!("start selftest={selftest}"));

    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(480.), px(240.)), cx);
        let handle: WindowHandle<Root> = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| Root),
            )
            .expect("open_window");

        let hwnd = Rc::new(Cell::new(0isize));
        {
            let hwnd = hwnd.clone();
            handle
                .update(cx, |_, window, cx| {
                    hwnd.set(hwnd_of(window));
                    match window.gpu_specs() {
                        Some(s) => log(&format!(
                            "gpu_specs: device={} driver={} software_emulated={}",
                            s.device_name, s.driver_name, s.is_software_emulated
                        )),
                        None => log("gpu_specs: none"),
                    }
                    let h = hwnd.get();
                    // Close button: intercept, hide instead of exit.
                    window.on_window_should_close(cx, move |_, _| {
                        log("close requested -> intercepted, hiding to tray");
                        set_visible(h, false);
                        false
                    });
                })
                .ok();
        }
        log(&format!("window opened hwnd=0x{:x}", hwnd.get()));

        let menu = Menu::new();
        let show_item = MenuItem::new("Show", true, None);
        let quit_item = MenuItem::new("Quit", true, None);
        menu.append_items(&[&show_item, &quit_item]).unwrap();
        let tray = TrayIconBuilder::new()
            .with_tooltip("WoW Sync spike")
            .with_icon(tray_icon_image())
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .expect("tray");
        log("tray icon created");
        let (show_id, quit_id) = (show_item.id().clone(), quit_item.id().clone());

        // tray-icon delivers via channels from its hidden window, which runs on
        // GPUI's main-thread Win32 message loop; poll them from a GPUI task.
        let hwnd_p = hwnd.clone();
        cx.spawn(async move |cx: &mut AsyncApp| {
            let _tray = tray; // keep alive for the app lifetime
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                let mut want_show = false;
                let mut want_quit = false;
                while let Ok(e) = TrayIconEvent::receiver().try_recv() {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = e
                    {
                        log("tray left-click event");
                        if visible(hwnd_p.get()) {
                            set_visible(hwnd_p.get(), false);
                            log("tray click: hid window");
                        } else {
                            want_show = true;
                        }
                    }
                }
                while let Ok(e) = MenuEvent::receiver().try_recv() {
                    if e.id == show_id {
                        log("tray menu: Show");
                        want_show = true;
                    } else if e.id == quit_id {
                        log("tray menu: Quit");
                        want_quit = true;
                    }
                }
                #[cfg(windows)]
                if unsafe { win::WaitForSingleObject(show_event, 0) } == 0 {
                    log("second-instance signal received");
                    want_show = true;
                }
                #[cfg(not(windows))]
                let _ = show_event;
                if want_show {
                    set_visible(hwnd_p.get(), true);
                    log(&format!("shown, visible={}", visible(hwnd_p.get())));
                }
                if want_quit {
                    log("quitting via cx.quit()");
                    cx.update(|cx| cx.quit()).ok();
                    return;
                }
            }
        })
        .detach();

        if selftest {
            let hwnd = hwnd.clone();
            cx.spawn(async move |cx: &mut AsyncApp| {
                let step = |ms| cx.background_executor().timer(Duration::from_millis(ms));
                step(1500).await;
                log(&format!("SELFTEST open: visible={}", visible(hwnd.get())));
                set_visible(hwnd.get(), false);
                step(500).await;
                log(&format!(
                    "SELFTEST hide: visible={} (process+tray alive)",
                    visible(hwnd.get())
                ));
                set_visible(hwnd.get(), true);
                step(500).await;
                log(&format!("SELFTEST reopen: visible={}", visible(hwnd.get())));
                #[cfg(windows)]
                unsafe {
                    win::PostMessageW(hwnd.get(), win::WM_CLOSE, 0, 0);
                }
                step(800).await;
                log(&format!(
                    "SELFTEST WM_CLOSE: visible={} (expect false, process alive)",
                    visible(hwnd.get())
                ));
                set_visible(hwnd.get(), true);
                step(500).await;
                log(&format!("SELFTEST reopen2: visible={}", visible(hwnd.get())));
                log("SELFTEST quit");
                cx.update(|cx| cx.quit()).ok();
            })
            .detach();
        }
    });
    log("run() returned; process exiting");
}
