//! WoW Sync shell (thin slice). Developer launch mode works on synthetic installs only.
//!
//!   wow-sync --dev --install <root> [--provider <dir>] [--state <dir>] [--name N]
//!            [--folder _forever_] [--start-hidden] [--interval-ms N] [--recent-ms N]
//!            [--crash-after N] [--hide-after-secs N]
//!
//! Provider and state default to fresh temp directories. Logging goes to $WOWSYNC_LOG.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod backend;
mod ui;
mod win;

use std::cell::{Cell, RefCell};
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{
    px, size, App, AppContext, AsyncApp, Bounds, QuitMode, TitlebarOptions, Window, WindowBounds,
    WindowHandle, WindowOptions,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use wow_sync_runtime::ServiceConfig;

use backend::{BackendOptions, Cmd, Shared, SharedState};
use ui::Root;

fn log(msg: &str) {
    if let Ok(path) = std::env::var("WOWSYNC_LOG") {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "pid={} {}", std::process::id(), msg);
        }
    }
}

struct Args {
    dev: bool,
    install: Option<PathBuf>,
    provider: Option<PathBuf>,
    state: Option<PathBuf>,
    name: String,
    folder: String,
    start_hidden: bool,
    interval_ms: u64,
    recent_ms: u64,
    crash_after: Option<usize>,
    hide_after: Option<u64>,
}

fn parse() -> Args {
    let v: Vec<String> = std::env::args().skip(1).collect();
    let val = |n: &str| {
        v.iter()
            .position(|a| a == n)
            .and_then(|i| v.get(i + 1))
            .cloned()
    };
    Args {
        dev: v.iter().any(|a| a == "--dev"),
        install: val("--install").map(PathBuf::from),
        provider: val("--provider").map(PathBuf::from),
        state: val("--state").map(PathBuf::from),
        name: val("--name").unwrap_or_else(|| "This computer".into()),
        folder: val("--folder").unwrap_or_else(|| "_forever_".into()),
        start_hidden: v.iter().any(|a| a == "--start-hidden"),
        interval_ms: val("--interval-ms")
            .and_then(|s| s.parse().ok())
            .unwrap_or(5000),
        recent_ms: val("--recent-ms")
            .and_then(|s| s.parse().ok())
            .unwrap_or(2000),
        crash_after: val("--crash-after").and_then(|s| s.parse().ok()),
        hide_after: val("--hide-after-secs").and_then(|s| s.parse().ok()),
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

struct Ctx {
    shared: SharedState,
    tx: Option<std::sync::mpsc::Sender<Cmd>>,
    fatal: Option<String>,
    name: String,
}

fn open(cx: &mut App, slot: &Slot, c: &Ctx) {
    let bounds = Bounds::centered(None, size(px(640.), px(720.)), cx);
    let (shared, tx, fatal, name) = (
        c.shared.clone(),
        c.tx.clone(),
        c.fatal.clone(),
        c.name.clone(),
    );
    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("WoW Sync".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |window, cx| {
                cx.new(|cx| {
                    let r = Root::new(shared, tx, fatal, name, cx);
                    r.focus_first(window, cx);
                    r
                })
            },
        )
        .expect("open_window");
    handle
        .update(cx, |_, window, cx| {
            let h = hwnd_of(window);
            window.on_window_should_close(cx, move |_, _| {
                win::set_visible(h, false);
                false
            });
        })
        .ok();
    *slot.borrow_mut() = Some(handle);
}

fn show(cx: &mut App, slot: &Slot, c: &Ctx) {
    match hwnd_of_slot(cx, slot) {
        Some(h) => win::set_visible(h, true),
        None => open(cx, slot, c),
    }
}

fn tray_icon_image() -> Icon {
    let mut px = Vec::with_capacity(32 * 32 * 4);
    for _ in 0..(32 * 32) {
        px.extend_from_slice(&[0x2b, 0x7f, 0xff, 0xff]);
    }
    Icon::from_rgba(px, 32, 32).expect("icon")
}

fn make_dir(p: Option<PathBuf>, tag: &str) -> PathBuf {
    p.unwrap_or_else(|| std::env::temp_dir().join(format!("wowsync-{tag}-{}", std::process::id())))
}

fn main() {
    let a = parse();
    let (state, provider) = (
        make_dir(a.state.clone(), "state"),
        make_dir(a.provider.clone(), "provider"),
    );
    let key = format!(
        "{:x}{}",
        state
            .to_string_lossy()
            .bytes()
            .fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64)
                .wrapping_mul(0x100000001b3)),
        std::env::var("WOWSYNC_INSTANCE").unwrap_or_default()
    );
    let Some(show_event) = win::single_instance(&key) else {
        log("second instance signalled the first; exiting");
        return;
    };

    let shared: SharedState = Arc::new(Mutex::new(Shared::default()));
    let (wake_tx, _wake_rx) = channel::<()>();
    let (tx, fatal) = match (&a.install, a.dev) {
        (Some(root), true) => {
            let (cmd_tx, cmd_rx) = channel();
            std::fs::create_dir_all(&state).ok();
            let cfg = ServiceConfig {
                install_root: root.clone(),
                variant_folder: a.folder.clone(),
                shared_root: provider.clone(),
                state_dir: state.clone(),
                display_name: a.name.clone(),
                app_version: env!("CARGO_PKG_VERSION").into(),
                recent_ms: a.recent_ms,
            };
            backend::spawn(
                cfg,
                BackendOptions {
                    interval: Duration::from_millis(a.interval_ms),
                    crash_after: a.crash_after,
                },
                cmd_rx,
                shared.clone(),
                wake_tx,
            );
            (Some(cmd_tx), None)
        }
        _ => (
            None,
            Some(
                "This build works on synthetic installs only. Start it with --dev --install \
                 <synthetic install folder>."
                    .to_string(),
            ),
        ),
    };
    log(&format!("start dev={}", a.dev));

    let ctx = Rc::new(Ctx {
        shared,
        tx: tx.clone(),
        fatal,
        name: a.name.clone(),
    });
    let (start_hidden, hide_after) = (a.start_hidden, a.hide_after);

    gpui_platform::application()
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx: &mut App| {
            ui::bind_keys(cx);
            let slot: Slot = Rc::new(RefCell::new(None));

            let menu = Menu::new();
            let show_item = MenuItem::new("Show", true, None);
            let quit_item = MenuItem::new("Quit", true, None);
            menu.append_items(&[&show_item, &quit_item]).unwrap();
            let tray = TrayIconBuilder::new()
                .with_tooltip("WoW Sync")
                .with_icon(tray_icon_image())
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()
                .expect("tray");
            let (show_id, quit_id) = (show_item.id().clone(), quit_item.id().clone());

            if !start_hidden {
                open(cx, &slot, &ctx);
                cx.activate(true);
            }

            let (slot_p, ctx_p) = (slot.clone(), ctx.clone());
            cx.spawn(async move |cx: &mut AsyncApp| {
                let _tray = tray;
                let started = std::time::Instant::now();
                let mut hid = false;
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(50))
                        .await;
                    let want_show = Cell::new(false);
                    let want_quit = Cell::new(false);
                    while let Ok(e) = TrayIconEvent::receiver().try_recv() {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = e
                        {
                            let sp = slot_p.clone();
                            let vis = cx.update(|cx| {
                                hwnd_of_slot(cx, &sp).map(win::is_visible).unwrap_or(false)
                            });
                            if vis {
                                cx.update(|cx| {
                                    if let Some(h) = hwnd_of_slot(cx, &sp) {
                                        win::set_visible(h, false);
                                    }
                                });
                            } else {
                                want_show.set(true);
                            }
                        }
                    }
                    while let Ok(e) = MenuEvent::receiver().try_recv() {
                        if e.id == show_id {
                            want_show.set(true);
                        } else if e.id == quit_id {
                            want_quit.set(true);
                        }
                    }
                    if win::show_requested(show_event) {
                        want_show.set(true);
                    }
                    if let Some(s) = hide_after {
                        if !hid && started.elapsed() >= Duration::from_secs(s) {
                            hid = true;
                            let sp = slot_p.clone();
                            cx.update(|cx| {
                                if let Some(h) = hwnd_of_slot(cx, &sp) {
                                    win::set_visible(h, false);
                                }
                            });
                        }
                    }
                    if want_show.get() {
                        let (sp, c) = (slot_p.clone(), ctx_p.clone());
                        cx.update(|cx| show(cx, &sp, &c));
                    }
                    if want_quit.get() {
                        if let Some(t) = &ctx_p.tx {
                            let _ = t.send(Cmd::Quit);
                        }
                        cx.update(|cx| cx.quit());
                        return;
                    }
                }
            })
            .detach();
        });
}
