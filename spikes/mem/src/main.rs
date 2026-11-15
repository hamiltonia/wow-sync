// S-mem spike: minimal GPUI app that can run with its window closed.
// Usage: gpui_mem <hidden | show-hide SHOW_SECS | cycles N | show>
use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, Context, SharedString, Window,
    WindowBounds, WindowHandle, WindowOptions,
};
use std::time::Duration;

struct Root {
    clicks: usize,
}

impl Render for Root {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let button = |id: &'static str, label: &'static str| {
            div()
                .id(id)
                .px_3()
                .py_1()
                .bg(rgb(0x3b6ea5))
                .text_color(rgb(0xffffff))
                .rounded_md()
                .cursor_pointer()
                .child(label)
        };
        let rows = (0..200).map(|i| {
            div()
                .px_2()
                .py_1()
                .border_b_1()
                .border_color(rgb(0x333333))
                .child(SharedString::from(format!(
                    "Row {i}: synthetic addon entry v{}.{}.{} - up to date",
                    i % 7,
                    i % 13,
                    i % 5
                )))
        });
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xdddddd))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .p_2()
                    .child(button("a", "Sync now").on_click(cx.listener(|this, _, _, cx| {
                        this.clicks += 1;
                        cx.notify();
                    })))
                    .child(button("b", "Settings"))
                    .child(button("c", "Recover"))
                    .child(SharedString::from(format!("clicks: {}", self.clicks))),
            )
            .child(
                div()
                    .id("list")
                    .flex_1()
                    .overflow_y_scroll()
                    .children(rows),
            )
    }
}

fn open(cx: &mut App) -> WindowHandle<Root> {
    let bounds = Bounds::centered(None, size(px(640.), px(480.)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        |_, cx| cx.new(|_| Root { clicks: 0 }),
    )
    .expect("open window")
}

// GPUI 0.2.2 (Windows) posts WM_QUIT when the last window closes; an invisible
// anchor window keeps the process alive. Enabled with env WOWSYNC_ANCHOR=1.
fn open_anchor(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(1.), px(1.)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            show: false,
            focus: false,
            ..Default::default()
        },
        |_, cx| cx.new(|_| Root { clicks: 0 }),
    )
    .expect("open anchor");
}

fn close(cx: &mut App, handle: WindowHandle<Root>) {
    let _ = handle.update(cx, |_, window, _| window.remove_window());
}

fn mark(s: &str) {
    println!("MARK {} {s}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).cloned().unwrap_or_else(|| "hidden".into());
    let n: u64 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(0);
    println!("gpui_mem pid={} mode={mode} arg={n}", std::process::id());
    Application::new().run(move |cx: &mut App| {
        if std::env::var("WOWSYNC_ANCHOR").is_ok() {
            open_anchor(cx);
        }
        cx.spawn(async move |cx| {
            let sleep = |ms: u64| cx.background_executor().timer(Duration::from_millis(ms));
            match mode.as_str() {
                "hidden" => mark("hidden-idle"),
                "show" => {
                    cx.update(|cx| {
                        open(cx);
                    })
                    .unwrap();
                    mark("shown");
                }
                "show-hide" => {
                    let h = cx.update(open).unwrap();
                    mark("shown");
                    sleep(n * 1000).await;
                    cx.update(|cx| close(cx, h)).unwrap();
                    mark("hidden-idle");
                }
                "cycles" => {
                    for i in 0..n {
                        let h = cx.update(open).unwrap();
                        sleep(1500).await;
                        cx.update(|cx| close(cx, h)).unwrap();
                        sleep(1500).await;
                        mark(&format!("cycle {}", i + 1));
                    }
                    mark("hidden-idle");
                }
                other => eprintln!("unknown mode {other}"),
            }
        })
        .detach();
    });
}
