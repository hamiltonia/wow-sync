//! WoW Sync. Normal start reads the remembered setup, or detects the game and Dropbox
//! read-only and shows the setup guide.
//!
//! Developer mode works on synthetic installs only:
//!   wow-sync --dev --install <root> [--provider <dir>] [--state <dir>] [--name N]
//!            [--folder _forever_] [--start-hidden] [--interval-ms N] [--recent-ms N]
//!            [--crash-after N] [--hide-after-secs N]
//! Window-state options for evidence: --theme --page --detail --sheet --setup-step --filter --zoom.
//! Logging goes to $WOWSYNC_LOG.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod assets;
mod backend;
mod brand;
mod consent;
mod model;
mod pages;
mod setup;
mod theme;
mod ui;
mod win;
mod world;

use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{px, size, AppContext, Bounds, QuitMode, TitlebarOptions, WindowBounds, WindowOptions};

use app::{Args, Shell};
use backend::{BackendOptions, Shared, SharedState, Target};
use model::{Filter, Page, FILTERS};
use theme::ThemeChoice;
use world::{Install, SetupData};

struct Cli {
    args: Args,
    zoom: f32,
    dev: bool,
    install: Option<PathBuf>,
    provider: Option<PathBuf>,
    state: Option<PathBuf>,
    name: Option<String>,
    folder: String,
    interval_ms: u64,
    recent_ms: u64,
    crash_after: Option<usize>,
}

fn parse() -> Cli {
    let v: Vec<String> = std::env::args().skip(1).collect();
    let val = |n: &str| {
        v.iter()
            .position(|a| a == n)
            .and_then(|i| v.get(i + 1))
            .cloned()
    };
    let filter = val("--filter").and_then(|s| {
        let s = s.to_ascii_lowercase().replace(['-', ' '], "");
        FILTERS
            .iter()
            .copied()
            .find(|f: &Filter| f.label().to_ascii_lowercase().replace(['-', ' '], "") == s)
    });
    Cli {
        args: Args {
            theme: val("--theme")
                .and_then(|s| ThemeChoice::parse(&s))
                .unwrap_or(ThemeChoice::System),
            page: val("--page").and_then(|s| Page::parse(&s)),
            detail: val("--detail"),
            sheet: val("--sheet"),
            filter,
            setup_step: val("--setup-step").and_then(|s| s.parse().ok()),
            start_hidden: v.iter().any(|a| a == "--start-hidden"),
            hide_after: val("--hide-after-secs").and_then(|s| s.parse().ok()),
            dev: None,
        },
        zoom: val("--zoom")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1.0)
            .clamp(0.75, 3.0),
        dev: v.iter().any(|a| a == "--dev"),
        install: val("--install").map(PathBuf::from),
        provider: val("--provider").map(PathBuf::from),
        state: val("--state").map(PathBuf::from),
        name: val("--name"),
        folder: val("--folder").unwrap_or_else(|| "_forever_".into()),
        interval_ms: val("--interval-ms")
            .and_then(|s| s.parse().ok())
            .unwrap_or(5000),
        recent_ms: val("--recent-ms")
            .and_then(|s| s.parse().ok())
            .unwrap_or(2000),
        crash_after: val("--crash-after").and_then(|s| s.parse().ok()),
    }
}

fn computer_name() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "This computer".into())
}

fn state_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("WoWSync").join("state")
}

fn install_row(root: &std::path::Path, folder: &str) -> Install {
    Install {
        identity: folder.trim_matches('_').replace('_', " "),
        folder: folder.to_string(),
        build: String::new(),
        root: root.to_path_buf(),
        recognised: false,
        inspected: false,
        refused: None,
        eligible: Vec::new(),
        blocked: Vec::new(),
    }
}

/// Read-only detection of the game installs and of Dropbox (UG-24).
fn detect(computer: &str) -> SetupData {
    let installs = wow_sync_runtime::find_wow(&wow_sync_runtime::wow_candidates())
        .into_iter()
        .flat_map(|d| {
            d.variants
                .iter()
                .map(|v| install_row(&d.root, v))
                .collect::<Vec<_>>()
        })
        .collect();
    let dropbox = wow_sync_runtime::find_dropbox(
        &wow_sync_runtime::dropbox_info_files(),
        &wow_sync_runtime::dropbox_defaults(),
    )
    .into_iter()
    .next();
    SetupData {
        dropbox: dropbox.as_ref().map(|p| p.display().to_string()),
        shared_root: dropbox.map(|p| p.join("WoW Sync")),
        online_only: false,
        installs,
        groups: Vec::new(),
        computer: computer.to_string(),
    }
}

fn main() {
    std::panic::set_hook(Box::new(|info| app::log(&format!("PANIC {info}"))));
    let mut cli = parse();
    ui::set_zoom(cli.zoom);

    let developer = cli.dev && cli.install.is_some();
    let computer = cli.name.clone().unwrap_or_else(computer_name);
    let state = cli.state.clone().unwrap_or_else(|| {
        if developer {
            std::env::temp_dir().join(format!("wowsync-state-{}", std::process::id()))
        } else {
            state_dir()
        }
    });
    let _ = std::fs::create_dir_all(&state);
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
        app::log("second instance signalled the first; exiting");
        return;
    };

    let (target, setup) = if developer {
        let root = cli.install.clone().unwrap_or_default();
        let provider = cli.provider.clone().unwrap_or_else(|| {
            std::env::temp_dir().join(format!("wowsync-provider-{}", std::process::id()))
        });
        let _ = std::fs::create_dir_all(&provider);
        let t = Target {
            install_root: root.clone(),
            variant_folder: cli.folder.clone(),
            shared_root: provider.clone(),
        };
        let data = SetupData {
            dropbox: Some(provider.display().to_string()),
            shared_root: Some(provider),
            online_only: false,
            installs: vec![install_row(&root, &cli.folder)],
            groups: Vec::new(),
            computer: computer.clone(),
        };
        (Some(t), Some(data))
    } else {
        (backend::load_target(&state), Some(detect(&computer)))
    };
    cli.args.dev = target.clone().filter(|_| developer);

    let shared: SharedState = Arc::new(Mutex::new(Shared::default()));
    let (wake_tx, _wake_rx) = channel::<()>();
    let (cmd_tx, cmd_rx) = channel();
    backend::spawn(
        target,
        setup,
        BackendOptions {
            interval: Duration::from_millis(cli.interval_ms),
            crash_after: cli.crash_after,
            state_dir: state,
            display_name: computer,
            app_version: env!("CARGO_PKG_VERSION").into(),
            recent_ms: cli.recent_ms,
            save_target: !developer,
        },
        cmd_rx,
        shared.clone(),
        wake_tx,
    );
    app::log(&format!("start dev={developer}"));

    let args = cli.args;
    gpui_platform::application()
        .with_assets(assets::Assets)
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx| {
            app::bind_keys(cx);
            let bounds = Bounds::centered(None, size(px(1040.), px(720.)), cx);
            let hidden = args.start_hidden;
            let handle = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        window_min_size: Some(size(px(760.), px(520.))),
                        show: !hidden,
                        titlebar: Some(TitlebarOptions {
                            title: Some("WoW Sync".into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    move |window, cx| {
                        cx.new(|cx| Shell::new(args, shared, Some(cmd_tx), show_event, window, cx))
                    },
                )
                .expect("open_window");
            if hidden {
                handle.update(cx, |app, _, _| app.hide()).ok();
            } else {
                cx.activate(true);
            }
        });
}
