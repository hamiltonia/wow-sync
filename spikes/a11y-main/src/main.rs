// Feasibility spike: what does GPUI expose to Windows UIA? Not product code.
use gpui::accesskit::{Role, Toggled};
use gpui::{
    actions, div, prelude::*, px, rgb, size, App, Bounds, Context, FocusHandle, KeyBinding,
    KeyDownEvent, Render, SharedString, TitlebarOptions, Window, WindowBounds, WindowOptions,
};

actions!(a11y_spike, [FocusNextStop, FocusPrevStop]);

struct Root {
    text: String,
    checked: bool,
    clicks: [u32; 2],
    field: FocusHandle,
    b1: FocusHandle,
    b2: FocusHandle,
    toggle: FocusHandle,
    items: Vec<FocusHandle>,
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_ring = |h: &FocusHandle, w: &Window| h.is_focused(w);
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
                .border_color(ring(focus_ring(h, w)))
                .bg(rgb(0x334155))
                .child(SharedString::from(label))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.clicks[which] += 1;
                    cx.notify();
                }))
        };

        let field_focused = focus_ring(&self.field, window);
        let text = self.text.clone();
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
                .border_color(ring(focus_ring(h, window)))
                .child(SharedString::from(format!("List item {}", i + 1)))
        });

        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .size_full()
            .on_action(cx.listener(|_, _: &FocusNextStop, w, cx| {
                w.focus_next(cx);
            }))
            .on_action(cx.listener(|_, _: &FocusPrevStop, w, cx| {
                w.focus_prev(cx);
            }))
            .bg(rgb(0x1e293b))
            .text_color(rgb(0xffffff))
            .child(
                div()
                    .id("heading")
                    .text_xl()
                    .role(Role::Heading)
                    .aria_level(1)
                    .aria_label("A11y Spike Heading")
                    .child("A11y Spike Heading"),
            )
            .child(
                div()
                    .id("field")
                    .role(Role::TextInput)
                    .aria_label("Name")
                    .aria_value(SharedString::from(self.text.clone()))
                    .track_focus(&self.field)
                    .tab_index(1)
                    .px_2()
                    .border_2()
                    .border_color(ring(field_focused))
                    .child(SharedString::from(format!("Name: {text}")))
                    .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                        if e.keystroke.key == "backspace" {
                            this.text.pop();
                        } else if let Some(c) = &e.keystroke.key_char {
                            this.text.push_str(c);
                        }
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(button(
                        "btn1",
                        format!("Save ({})", self.clicks[0]),
                        &self.b1,
                        2,
                        0,
                        window,
                        cx,
                    ))
                    .child(button(
                        "btn2",
                        format!("Cancel ({})", self.clicks[1]),
                        &self.b2,
                        3,
                        1,
                        window,
                        cx,
                    )),
            )
            .child({
                let on = self.checked;
                div()
                    .id("toggle")
                    .role(Role::CheckBox)
                    .aria_label("Enable sync")
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .track_focus(&self.toggle)
                    .tab_index(4)
                    .px_2()
                    .border_2()
                    .border_color(ring(focus_ring(&self.toggle, window)))
                    .child(SharedString::from(format!(
                        "[{}] Enable sync",
                        if on { "x" } else { " " }
                    )))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.checked = !this.checked;
                        cx.notify();
                    }))
            })
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

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        // GPUI has no default Tab handling: Tab/Shift+Tab must be bound explicitly.
        cx.bind_keys([
            KeyBinding::new("tab", FocusNextStop, None),
            KeyBinding::new("shift-tab", FocusPrevStop, None),
        ]);
        let bounds = Bounds::centered(None, size(px(480.), px(520.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("A11y Spike".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let mk = || cx.focus_handle().tab_stop(true);
                    let r = Root {
                        text: String::new(),
                        checked: false,
                        clicks: [0, 0],
                        field: mk(),
                        b1: mk(),
                        b2: mk(),
                        toggle: mk(),
                        items: (0..10).map(|_| mk()).collect(),
                    };
                    window.focus(&r.field, cx);
                    r
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
