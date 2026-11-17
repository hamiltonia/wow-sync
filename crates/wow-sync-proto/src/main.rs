#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod assets;
mod model;
mod notify;
mod pages;
mod scenario;
mod setup;
mod theme;
mod ui;
mod win;

use app::{Args, ProtoApp};
use gpui::{px, size, AppContext, Bounds, QuitMode, TitlebarOptions, WindowBounds, WindowOptions};
use model::{Filter, Page, FILTERS};
use scenario::ScenarioId;
use theme::ThemeChoice;

fn parse() -> (Args, f32) {
    let v: Vec<String> = std::env::args().skip(1).collect();
    let val = |n: &str| {
        v.iter()
            .position(|a| a == n)
            .and_then(|i| v.get(i + 1))
            .cloned()
    };
    let zoom = val("--zoom")
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(1.0)
        .clamp(0.75, 3.0);
    let filter = val("--filter").and_then(|s| {
        let s = s.to_ascii_lowercase().replace(['-', ' '], "");
        FILTERS
            .iter()
            .copied()
            .find(|f: &Filter| f.label().to_ascii_lowercase().replace(['-', ' '], "") == s)
    });
    (
        Args {
            scenario: val("--scenario")
                .and_then(|s| ScenarioId::parse(&s))
                .unwrap_or(ScenarioId::S3),
            theme: val("--theme")
                .and_then(|s| ThemeChoice::parse(&s))
                .unwrap_or(ThemeChoice::System),
            page: val("--page").and_then(|s| Page::parse(&s)),
            detail: val("--detail"),
            sheet: val("--sheet"),
            filter,
            setup_step: val("--setup-step").and_then(|s| s.parse().ok()),
            start_hidden: v.iter().any(|a| a == "--start-hidden"),
        },
        zoom,
    )
}

fn main() {
    std::panic::set_hook(Box::new(|info| app::log(&format!("PANIC {info}"))));
    let (args, zoom) = parse();
    ui::set_zoom(zoom);
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
                        titlebar: Some(TitlebarOptions {
                            title: Some("WoW Sync".into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    move |window, cx| cx.new(|cx| ProtoApp::new(args, window, cx)),
                )
                .expect("open_window");
            if hidden {
                handle.update(cx, |app, _, _| app.hide()).ok();
            }
            cx.activate(true);
        });
}
