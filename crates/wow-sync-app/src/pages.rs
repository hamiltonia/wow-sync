//! Main-window pages (P2 to P5, P7 to P11, P14).

use crate::app::{Act, Bk, Shell};
use crate::model::{
    self, addon_rows, addon_story, counts_sentence, filter_count, headline, need_items,
    paused_hint, peer_view, unconfirmed_addons, Detail, Icon, Label, Page, FILTERS,
};
use crate::theme::{Mode, Palette, ThemeChoice, Tone};
use crate::ui::{icon, u};
use crate::world::{rel, Kind};
use gpui::accesskit::Role;
use gpui::{div, prelude::*, rgb, AnyElement, Context, Div, FontWeight, SharedString};
use wow_sync_core::status::MemberSummary;

impl Shell {
    pub fn heading(&self, text: &str, level: usize) -> gpui::Stateful<Div> {
        let pal = self.pal();
        div()
            .id(SharedString::from(format!("h{level}-{text}")))
            .role(Role::Heading)
            .aria_level(level)
            .aria_label(SharedString::from(text.to_string()))
            .text_size(u(if level == 1 { 22. } else { 16. }))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(pal.text))
            .child(SharedString::from(text.to_string()))
    }

    pub fn panel(&self, title: &str, children: Vec<AnyElement>) -> Div {
        let pal = self.pal();
        div()
            .flex()
            .flex_col()
            .gap(u(8.))
            .p(u(20.))
            .rounded(u(8.))
            .bg(rgb(pal.surface))
            .border_1()
            .border_color(rgb(pal.divider))
            .child(self.heading(title, 2))
            .children(children)
    }

    pub fn muted(&self, s: impl Into<SharedString>) -> Div {
        div().text_color(rgb(self.pal().muted)).child(s.into())
    }

    pub fn chip(&self, l: &Label) -> Div {
        let pal = self.pal();
        div()
            .flex()
            .items_center()
            .gap(u(6.))
            .text_color(rgb(pal.fg(l.tone)))
            .child(icon(l.icon, 16., pal.fg(l.tone)))
            .child(SharedString::from(l.text.clone()))
    }

    fn kv(&self, label: &str, value: String, a11y: String) -> gpui::Stateful<Div> {
        let pal = self.pal();
        div()
            .id(SharedString::from(format!("kv-{label}")))
            .role(Role::Group)
            .aria_label(SharedString::from(a11y))
            .flex()
            .gap(u(12.))
            .child(
                div()
                    .w(u(130.))
                    .flex_none()
                    .text_color(rgb(pal.muted))
                    .child(SharedString::from(label.to_string())),
            )
            .child(div().flex_1().min_w_0().child(SharedString::from(value)))
    }

    fn body_col(&self) -> Div {
        div().flex().flex_col().gap(u(14.)).max_w(u(880.))
    }

    pub fn back_link(&mut self, label: &str, cx: &mut Context<Self>) -> AnyElement {
        self.btn("back", &format!("← {label}"), Bk::Link, Act::Back, cx)
            .self_start()
            .aria_label(SharedString::from(format!("Back to {label}")))
            .into_any_element()
    }

    pub fn page_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match self.detail.clone() {
            Detail::Addon(k) => return self.addon_detail(&k, cx),
            Detail::Computer(i) => return self.computer_detail(i, cx),
            Detail::Decision(k) => return self.decision_view(&k, cx),
            Detail::NeedsHelp(k) => return self.needs_help_view(&k, cx),
            Detail::Reconfirm => return self.reconfirm_view(cx),
            Detail::None => {}
        }
        match self.page {
            Page::Overview => self.overview(cx),
            Page::Computers => self.computers(cx),
            Page::Addons => self.addons(cx),
            Page::Needs => self.needs(cx),
            Page::Activity => self.activity(cx),
            Page::SettingsSync => self.settings_sync(cx),
            Page::AppSettings => self.app_settings(cx),
            Page::About => self.about(cx),
        }
    }

    // ---------------------------------------------------------------- Overview

    fn overview(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let wide = self.window_w - if self.window_w < 900.0 { 64.0 } else { 220.0 } - 40.0 >= 820.0;
        let items = need_items(&self.w);
        let mut left: Vec<AnyElement> = Vec::new();
        if !items.is_empty() {
            let mut rows = Vec::new();
            for (i, it) in items.iter().take(3).enumerate() {
                let act = Self::needs_target_act(&it.target);
                let b = self
                    .btn(&format!("need-{i}"), it.button, Bk::Secondary, act, cx)
                    .aria_label(SharedString::from(format!("{}: {}", it.button, it.title)));
                rows.push(self.need_row(it, b));
            }
            if items.len() > 3 {
                rows.push(
                    self.btn(
                        "need-all",
                        &format!("See all {}", items.len()),
                        Bk::Link,
                        Act::Nav(Page::Needs),
                        cx,
                    )
                    .into_any_element(),
                );
            }
            left.push(self.panel("Needs attention", rows).into_any_element());
        }
        let c = counts_sentence(&self.w);
        let mut tc: Vec<AnyElement> = Vec::new();
        let addons_link = self
            .clickable("tc-addons", Act::NeedsAddonsFiltered, cx)
            .role(Role::Button)
            .aria_label(SharedString::from(format!(
                "Add-ons, {c}, open the list of add-ons that are not up to date"
            )))
            .flex()
            .gap(u(12.))
            .child(
                div()
                    .w(u(130.))
                    .flex_none()
                    .text_color(rgb(pal.muted))
                    .child("Add-ons"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(rgb(pal.accent))
                    .child(SharedString::from(c)),
            );
        tc.push(addons_link.into_any_element());
        if let Some(h) = paused_hint(&self.w) {
            tc.push(self.muted(h).into_any_element());
        }
        tc.push(
            self.kv(
                "Game",
                format!("{} · {}", model::game_state(&self.w), self.w.build),
                format!("Game, {}", model::game_state(&self.w)),
            )
            .into_any_element(),
        );
        let shared = match &self.w.provider_error {
            Some(e) => format!("{} · can't be read: {e}", self.w.shared_label),
            None => format!("{} · readable", self.w.shared_label),
        };
        tc.push(
            self.kv(
                "Shared folder",
                shared.clone(),
                format!("Shared folder, {shared}"),
            )
            .into_any_element(),
        );
        let ls = self
            .w
            .last_shared_secs
            .map(|s| format!("{} ({} add-ons)", rel(s), self.w.synced_total()))
            .unwrap_or_else(|| "Not yet".into());
        tc.push(
            self.kv("Last shared", ls.clone(), format!("Last shared, {ls}"))
                .into_any_element(),
        );
        let lc = model::last_check_text(&self.w);
        tc.push(
            self.kv("Last check", lc.clone(), format!("Last check, {lc}"))
                .into_any_element(),
        );
        if self.w.recovery_extra > 0 {
            let n = self.w.recovery_extra;
            let r = format!("{n} extra {} kept", if n == 1 { "copy" } else { "copies" });
            tc.push(
                self.kv(
                    "Recovery copies",
                    r.clone(),
                    format!("Recovery copies, {r}"),
                )
                .into_any_element(),
            );
        }
        left.push(self.panel("This computer", tc).into_any_element());

        let mut rc: Vec<AnyElement> = Vec::new();
        if self.w.activity.is_empty() {
            rc.push(self.muted("Nothing has changed yet.").into_any_element());
        }
        let acts: Vec<_> = self.w.activity.iter().take(5).cloned().collect();
        for (i, a) in acts.iter().enumerate() {
            let label = format!("{}, {} {}", a.title, a.day, a.when);
            let r = self
                .clickable(&format!("recent-{i}"), Act::Nav(Page::Activity), cx)
                .role(Role::Button)
                .aria_label(SharedString::from(label))
                .flex()
                .gap(u(10.))
                .px(u(6.))
                .py(u(4.))
                .child(
                    div()
                        .w(u(90.))
                        .flex_none()
                        .text_color(rgb(pal.muted))
                        .child(SharedString::from(format!("{} {}", a.day, a.when))),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(SharedString::from(a.title.clone())),
                );
            rc.push(r.into_any_element());
        }
        if !acts.is_empty() {
            rc.push(
                self.btn(
                    "see-activity",
                    "See all activity",
                    Bk::Link,
                    Act::Nav(Page::Activity),
                    cx,
                )
                .into_any_element(),
            );
        }
        left.push(self.panel("Recent changes", rc).into_any_element());

        let mut right: Vec<AnyElement> = Vec::new();
        let mut oc: Vec<AnyElement> = Vec::new();
        if self.w.peers.is_empty() {
            oc.push(self.muted("No other computers have joined this group yet. Set up WoW Sync on another computer and choose Join to add it.").into_any_element());
        }
        for i in 0..self.w.peers.len() {
            let pv = peer_view(&self.w, i);
            let card = self
                .clickable(&format!("pc-{i}"), Act::OpenComputer(i), cx)
                .role(Role::Button)
                .aria_label(SharedString::from(pv.a11y.clone()))
                .flex()
                .flex_col()
                .gap(u(2.))
                .p(u(10.))
                .border_1()
                .border_color(rgb(pal.divider))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(u(8.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(icon(pv.icon, 16., pal.fg(pv.tone)))
                        .child(SharedString::from(pv.name.clone()))
                        .child(
                            div()
                                .font_weight(FontWeight::NORMAL)
                                .text_color(rgb(pal.fg(pv.tone)))
                                .child(SharedString::from(pv.status.clone())),
                        ),
                )
                .child(self.muted(pv.confirmed.clone()))
                .child(self.muted(pv.received.clone()));
            oc.push(card.into_any_element());
        }
        right.push(self.panel("Other computers", oc).into_any_element());

        let title = self.heading("Overview", 1);
        let cols = if wide {
            div()
                .flex()
                .gap(u(14.))
                .items_start()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(u(14.))
                        .children(left),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(u(14.))
                        .children(right),
                )
        } else {
            div()
                .flex()
                .flex_col()
                .gap(u(14.))
                .children(left)
                .children(right)
        };
        div()
            .flex()
            .flex_col()
            .gap(u(14.))
            .child(title)
            .child(cols)
            .into_any_element()
    }

    fn need_row(&self, it: &model::NeedItem, button: gpui::Stateful<Div>) -> AnyElement {
        let pal = self.pal();
        div()
            .flex()
            .items_center()
            .gap(u(10.))
            .child(icon(model::tone_icon(it.tone), 20., pal.fg(it.tone)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(SharedString::from(it.title.clone())),
                    )
                    .child(self.muted(it.reason.clone()))
                    .children(
                        (!it.affects.is_empty())
                            .then(|| self.muted(format!("Affects: {}", it.affects))),
                    ),
            )
            .child(button)
            .into_any_element()
    }

    // --------------------------------------------------------------- Computers

    fn computers(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let n = self.w.peers.len() + 1;
        let mut rows: Vec<AnyElement> = Vec::new();
        let paused = self.w.is_paused();
        let me_status = if paused {
            "Paused: not sharing or applying".to_string()
        } else if self.w.busy.is_some() {
            "Sharing now…".to_string()
        } else {
            headline(&self.w).text
        };
        let a = format!(
            "{} (this computer), {}",
            self.w.computer,
            me_status.to_lowercase()
        );
        let me = self
            .rov_row("computers", 0, n, "me", a, Act::None, cx)
            .flex()
            .gap(u(10.))
            .px(u(8.))
            .py(u(8.))
            .child(
                div()
                    .w(u(170.))
                    .flex_none()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(SharedString::from(format!(
                        "{} (This computer)",
                        self.w.computer
                    ))),
            )
            .child(div().w(u(170.)).flex_none().child("—"))
            .child(div().w(u(200.)).flex_none().child("This computer"))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(SharedString::from(me_status)),
            );
        rows.push(me.into_any_element());
        for i in 0..self.w.peers.len() {
            let pv = peer_view(&self.w, i);
            let r = self
                .rov_row(
                    "computers",
                    i + 1,
                    n,
                    &format!("pc{i}"),
                    pv.a11y.clone(),
                    Act::OpenComputer(i),
                    cx,
                )
                .flex()
                .gap(u(10.))
                .px(u(8.))
                .py(u(8.))
                .when(pv.stale, |d| d.bg(rgb(pal.fill(Tone::Unknown))))
                .child(
                    div()
                        .w(u(170.))
                        .flex_none()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(SharedString::from(pv.name.clone())),
                )
                .child(div().w(u(170.)).flex_none().child(SharedString::from(
                    pv.received.replace("Report received ", ""),
                )))
                .child(
                    div()
                        .w(u(200.))
                        .flex_none()
                        .child(SharedString::from(format!(
                            "{} of {} current add-ons",
                            self.w.confirmed_by(i),
                            self.w.synced_total()
                        ))),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(u(6.))
                        .text_color(rgb(pal.fg(pv.tone)))
                        .child(icon(pv.icon, 16., pal.fg(pv.tone)))
                        .child(SharedString::from(pv.status.clone())),
                );
            rows.push(r.into_any_element());
        }
        let sh = self.scroll("computers");
        let list = div()
            .id("list-computers")
            .role(Role::List)
            .aria_label(SharedString::from(model::list_a11y("Computers", n, None)))
            .overflow_y_scroll()
            .track_scroll(&sh)
            .rounded(u(8.))
            .bg(rgb(pal.surface))
            .border_1()
            .border_color(rgb(pal.divider))
            .children(rows);
        let header = div()
            .flex()
            .gap(u(10.))
            .px(u(8.))
            .text_size(u(12.))
            .text_color(rgb(pal.muted))
            .child(div().w(u(170.)).flex_none().child("Name"))
            .child(
                div()
                    .w(u(170.))
                    .flex_none()
                    .child("Last report received here"),
            )
            .child(div().w(u(200.)).flex_none().child("Last confirmed"))
            .child(div().flex_1().child("Status"));
        let mut col = self.body_col().child(self.heading("Computers", 1)).child(self.muted("Based on each computer's last report. What a computer has right now is not known until it reports again."));
        if self.w.peers.is_empty() {
            col = col.child(self.muted("No other computers have joined this group yet. To add one, install WoW Sync on it and choose Join when it asks for a sync group."));
        }
        if let Some((h, sub)) = crate::computers_ui::standing_headline(&self.w.features.standing) {
            col = col.child(self.heading(&h, 2)).child(self.muted(sub));
            if let Ok(a) = crate::computers_ui::join_again(&self.w) {
                col = col.child(self.btn(
                    "join-again",
                    "Join again…",
                    Bk::Primary,
                    Act::Group(a),
                    cx,
                ));
            }
        }
        let mut col = col.child(header).child(list);
        if !self.w.features.retired.is_empty() {
            col = col.child(self.heading("Retired", 2));
            for r in self.w.features.retired.clone() {
                col = col.child(
                    div()
                        .flex()
                        .gap(u(10.))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(SharedString::from(r.name.clone())),
                        )
                        .child(self.muted(crate::computers_ui::retired_line(&r))),
                );
            }
        }
        for (j, row) in self.w.features.allow_again.clone().into_iter().enumerate() {
            let label = crate::computers_ui::allow_again_label(&row);
            col = col.child(self.muted(label));
            let key = format!("allow-again-{j}");
            col = match crate::computers_ui::allow_again(&self.w, &row) {
                Ok(a) => {
                    col.child(self.btn(&key, "Allow again…", Bk::Secondary, Act::Group(a), cx))
                }
                Err(why) => col.child(self.btn_off(&key, "Allow again…", &why, cx)),
            };
        }
        col.into_any_element()
    }

    fn computer_detail(&mut self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        if i >= self.w.peers.len() {
            return self
                .body_col()
                .child(self.back_link("Computers", cx))
                .child("This computer is no longer in the group.")
                .into_any_element();
        }
        let pv = peer_view(&self.w, i);
        let peer = self.w.peers[i].clone();
        let back = self.back_link("Computers", cx);
        let mut know: Vec<AnyElement> = vec![
            self.muted(pv.confirmed.clone()).into_any_element(),
            self.muted(format!("Game: {}", peer.game))
                .into_any_element(),
        ];
        let un = unconfirmed_addons(&self.w, i);
        if !un.is_empty() && matches!(peer.state, MemberSummary::Usable | MemberSummary::NoState) {
            know.push(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Not confirmed yet:")
                    .into_any_element(),
            );
            let n = un.len();
            for (j, (key, label)) in un.into_iter().take(8).enumerate() {
                know.push(
                    self.btn(
                        &format!("unc-{j}"),
                        &label,
                        Bk::Link,
                        Act::OpenAddon(key),
                        cx,
                    )
                    .into_any_element(),
                );
            }
            if n > 8 {
                know.push(self.muted(format!("and {} more", n - 8)).into_any_element());
            }
        }
        know.push(self.muted(format!("This is what {} last reported. What it has right now is not known until it reports again.", peer.name)).into_any_element());
        let mut details: Vec<AnyElement> = vec![self
            .disclosure("computer-details", "Details", cx)
            .into_any_element()];
        if self.expanded.contains("computer-details") {
            details.push(
                self.muted(format!("Computer ID: {}", peer.id))
                    .into_any_element(),
            );
            details.push(
                self.muted(format!(
                    "App version: {}",
                    if peer.app.is_empty() {
                        "not reported"
                    } else {
                        &peer.app
                    }
                ))
                .into_any_element(),
            );
        }
        let retire = match crate::computers_ui::retire_sheet(&self.w, i) {
            Err(why) => self.btn_off("retire", "Retire this computer…", &why, cx),
            Ok(_) => self.btn(
                "retire",
                "Retire this computer…",
                Bk::Secondary,
                Act::ToggleExpand("retire-open".into()),
                cx,
            ),
        };
        let mut retire_panel: Vec<AnyElement> = Vec::new();
        if self.expanded.contains("retire-open") {
            if let Ok(s) = crate::computers_ui::retire_sheet(&self.w, i) {
                retire_panel.push(self.heading(&s.title, 2).into_any_element());
                for f in s.facts {
                    retire_panel.push(div().child(SharedString::from(f)).into_any_element());
                }
                for extra in s.stale_warning.into_iter().chain(s.master_note) {
                    retire_panel.push(
                        div()
                            .text_color(rgb(pal.fg(Tone::Help)))
                            .child(SharedString::from(extra))
                            .into_any_element(),
                    );
                }
                let acked = self.expanded.contains("retire-ack");
                retire_panel.push(
                    self.btn(
                        "retire-ack",
                        &format!("{} {}", if acked { "[x]" } else { "[ ]" }, s.ack_label),
                        Bk::Link,
                        Act::ToggleExpand("retire-ack".into()),
                        cx,
                    )
                    .into_any_element(),
                );
                let go = match crate::computers_ui::retire_action(&self.w, i, acked) {
                    Ok(a) => self.btn("retire-go", "Retire", Bk::Primary, Act::Group(a), cx),
                    Err(why) => self.btn_off("retire-go", "Retire", &why, cx),
                };
                retire_panel.push(go.into_any_element());
                retire_panel.push(
                    self.btn(
                        "retire-cancel",
                        "Cancel",
                        Bk::Secondary,
                        Act::ToggleExpand("retire-open".into()),
                        cx,
                    )
                    .into_any_element(),
                );
            }
        }
        self.body_col()
            .child(back)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(u(10.))
                    .child(icon(pv.icon, 24., pal.fg(pv.tone)))
                    .child(self.heading(&peer.name, 1))
                    .child(
                        div()
                            .text_color(rgb(pal.fg(pv.tone)))
                            .child(SharedString::from(pv.status.clone())),
                    )
                    .child(self.muted(pv.received.clone())),
            )
            .child(self.panel("What we know", know))
            .child(self.panel("More", details))
            .child(retire)
            .children(retire_panel)
            .into_any_element()
    }

    // ----------------------------------------------------------------- Add-ons

    fn filter_group(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let mut row = div()
            .id("filter-addons")
            .flex()
            .gap(u(8.))
            .role(Role::Group)
            .aria_label("Filter add-ons");
        for (i, f) in FILTERS.iter().copied().enumerate() {
            let on = self.filter == f;
            let n = filter_count(&self.w, f);
            let a = format!(
                "{}, {} add-ons{}",
                f.label(),
                n,
                if on { ", selected" } else { "" }
            );
            let b = self
                .rov_row(
                    "filter",
                    i,
                    FILTERS.len(),
                    f.label(),
                    a,
                    Act::SetFilter(f),
                    cx,
                )
                .role(Role::Button)
                .aria_toggled(if on {
                    gpui::accesskit::Toggled::True
                } else {
                    gpui::accesskit::Toggled::False
                })
                .px(u(12.))
                .h(u(34.))
                .flex()
                .items_center()
                .bg(rgb(if on { pal.selected } else { pal.surface }))
                .when(on, |d| d.font_weight(FontWeight::SEMIBOLD))
                .child(SharedString::from(format!("{} ({})", f.label(), n)));
            row = row.child(b);
        }
        row.into_any_element()
    }

    fn addons(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let total = self.w.view.packages.len();
        let mut col = self.body_col().child(self.heading("Add-ons", 1));
        if let Some(s) = self.w.stale_secs {
            col = col.child(self.muted(format!("Last known add-on data, as of {}", rel(s))));
        }
        if total == 0 {
            return col
                .child(self.muted("No add-ons found in this installation yet."))
                .into_any_element();
        }
        let filter = self.filter;
        col = col.child(self.filter_group(cx));
        let rows = addon_rows(&self.w, filter);
        let n = rows.len();
        let mut els: Vec<AnyElement> = Vec::new();
        for (i, r) in rows.iter().enumerate() {
            let choose = r.label.text == "Needs your decision";
            let el = self
                .rov_row(
                    "addons",
                    i,
                    n,
                    &r.key,
                    r.a11y.clone(),
                    Act::OpenAddon(r.key.clone()),
                    cx,
                )
                .flex()
                .items_center()
                .gap(u(10.))
                .px(u(10.))
                .h(u(40.))
                .border_b_1()
                .border_color(rgb(pal.divider))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(SharedString::from(r.title.clone())),
                )
                .child(
                    div()
                        .w(u(90.))
                        .flex_none()
                        .child(SharedString::from(r.version.clone())),
                )
                .child(
                    div()
                        .w(u(250.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(u(8.))
                        .child(self.chip(&r.label))
                        .when(choose, |d| {
                            d.child(div().text_color(rgb(pal.accent)).child("Choose…"))
                        }),
                )
                .child(
                    div()
                        .w(u(160.))
                        .flex_none()
                        .truncate()
                        .text_color(rgb(pal.muted))
                        .child(SharedString::from(r.confirmed_by.clone())),
                );
            els.push(el.into_any_element());
        }
        let header = div()
            .flex()
            .gap(u(10.))
            .px(u(10.))
            .text_size(u(12.))
            .text_color(rgb(pal.muted))
            .child(div().flex_1().child("Name"))
            .child(div().w(u(90.)).flex_none().child("Version here"))
            .child(div().w(u(250.)).flex_none().child("Status"))
            .child(div().w(u(160.)).flex_none().child("Confirmed by"));
        let sh = self.scroll("addons");
        let max_h = (self.window_h - 76.0 - 26.0 - 200.0).max(200.0);
        let list = div()
            .id("list-addons")
            .role(Role::List)
            .aria_label(SharedString::from(model::list_a11y(
                "Add-ons",
                n,
                Some(filter),
            )))
            .max_h(u(max_h))
            .overflow_y_scroll()
            .track_scroll(&sh)
            .rounded(u(8.))
            .bg(rgb(pal.surface))
            .border_1()
            .border_color(rgb(pal.divider))
            .children(els);
        col = col.child(header);
        if n == 0 {
            col = col.child(self.muted("No add-ons match this filter."));
        }
        col = col.child(list);
        let removed = crate::removed_ui::lines(&self.w.features.removed);
        col = col.child(self.heading("Removed", 2));
        col = col.child(self.muted(crate::removed_ui::intro(removed.len())));
        for (i, r) in removed.into_iter().enumerate() {
            let mut body: Vec<AnyElement> = vec![
                self.muted(format!("{} ({})", r.title, r.folders))
                    .into_any_element(),
                self.muted(&r.provenance).into_any_element(),
            ];
            if let Some(a) = &r.awaiting {
                body.push(self.muted(a).into_any_element());
            }
            body.push(match crate::removed_ui::restore_action(&self.w, i) {
                Ok(a) => self
                    .btn(
                        &format!("removed-restore-{i}"),
                        "Restore",
                        Bk::Secondary,
                        Act::Group(a),
                        cx,
                    )
                    .into_any_element(),
                Err(why) => self
                    .btn_off(&format!("removed-restore-{i}"), "Restore", &why, cx)
                    .into_any_element(),
            });
            col = col.child(self.panel(&r.title, body));
        }
        col.into_any_element()
    }

    fn addon_detail(&mut self, key: &str, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let Some(s) = addon_story(&self.w, key) else {
            return self
                .body_col()
                .child(self.back_link("Add-ons", cx))
                .child("This add-on is no longer in the group.")
                .into_any_element();
        };
        let back = self.back_link("Add-ons", cx);
        let mut next: Vec<AnyElement> = vec![div()
            .child(SharedString::from(s.next.clone()))
            .into_any_element()];
        if let Some((label, d)) = s.next_button.clone() {
            let act = match d {
                Detail::Decision(k) => Act::OpenDecision(k),
                Detail::NeedsHelp(k) => Act::OpenNeedsHelp(k),
                _ => Act::None,
            };
            next.push(
                self.btn("next-step", label, Bk::Primary, act, cx)
                    .into_any_element(),
            );
        }
        let mut comps: Vec<AnyElement> = Vec::new();
        for (j, (name, line, tone, ic)) in s.computers.iter().enumerate() {
            let peer = self.w.peers.iter().position(|p| p.name == *name);
            let name_el: AnyElement = match peer {
                Some(pi) => self
                    .btn(
                        &format!("oc-{j}"),
                        name,
                        Bk::Link,
                        Act::OpenComputer(pi),
                        cx,
                    )
                    .into_any_element(),
                None => div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .px(u(4.))
                    .child(SharedString::from(format!("{name} (this computer)")))
                    .into_any_element(),
            };
            comps.push(
                div()
                    .flex()
                    .items_center()
                    .gap(u(8.))
                    .child(icon(*ic, 16., pal.fg(*tone)))
                    .child(div().w(u(180.)).flex_none().child(name_el))
                    .child(div().flex_1().child(SharedString::from(line.clone())))
                    .into_any_element(),
            );
        }
        let mut folders: Vec<AnyElement> = vec![div()
            .child(SharedString::from(s.folders.clone()))
            .into_any_element()];
        if let Some(g) = &s.grouped {
            folders.push(self.muted(g.clone()).into_any_element());
        }
        let restore = self.btn_off("restore", "Restore…", "coming later", cx);
        let mut rec: Vec<AnyElement> = vec![restore.into_any_element()];
        let installed = s.versions.clone();
        let rows: Vec<crate::rollback_ui::ReleaseRow> = self
            .w
            .features
            .history
            .iter()
            .filter(|r| r.package.as_str() == key)
            .cloned()
            .collect();
        for (j, r) in rows.into_iter().enumerate() {
            let mut line = format!("Version {}", r.version);
            if !r.held_by.is_empty() {
                line.push_str(&format!(", held by {}", r.held_by));
            }
            if r.current {
                line.push_str(" (installed now)");
            }
            if let Some(chip) = crate::rollback_ui::blocked_chip(&r) {
                line.push_str(&format!(" [{chip}]"));
            }
            rec.push(div().child(SharedString::from(line)).into_any_element());
            let k = format!("rb-{j}");
            match crate::rollback_ui::button(&self.w, &r) {
                crate::rollback_ui::HistoryButton::None => {}
                crate::rollback_ui::HistoryButton::Disabled { label, reason } => {
                    rec.push(self.btn_off(&k, label, &reason, cx).into_any_element());
                    rec.push(self.muted(reason).into_any_element());
                }
                crate::rollback_ui::HistoryButton::AllowAgain(a) => {
                    rec.push(
                        self.btn(
                            &k,
                            crate::rollback_ui::ALLOW_AGAIN,
                            Bk::Secondary,
                            Act::Group(a),
                            cx,
                        )
                        .into_any_element(),
                    );
                }
                crate::rollback_ui::HistoryButton::GoBack(a) => {
                    let open = format!("{k}-open");
                    if self.expanded.contains(&open) {
                        for t in crate::rollback_ui::confirm_text(&installed, &r) {
                            rec.push(div().child(SharedString::from(t)).into_any_element());
                        }
                        rec.push(
                            self.btn(
                                &format!("{k}-go"),
                                "Go back",
                                Bk::Primary,
                                Act::Group(a),
                                cx,
                            )
                            .into_any_element(),
                        );
                        rec.push(
                            self.btn(
                                &format!("{k}-no"),
                                "Cancel",
                                Bk::Secondary,
                                Act::ToggleExpand(open),
                                cx,
                            )
                            .into_any_element(),
                        );
                    } else {
                        rec.push(
                            self.btn(
                                &k,
                                crate::rollback_ui::GO_BACK,
                                Bk::Secondary,
                                Act::ToggleExpand(open),
                                cx,
                            )
                            .into_any_element(),
                        );
                    }
                }
            }
        }
        let mut det: Vec<AnyElement> = vec![self
            .disclosure("addon-details", "Details", cx)
            .into_any_element()];
        if self.expanded.contains("addon-details") {
            for d in &s.details {
                det.push(self.muted(d.clone()).into_any_element());
            }
        }
        let why: Vec<AnyElement> = s
            .why
            .iter()
            .map(|w| {
                div()
                    .child(SharedString::from(w.clone()))
                    .into_any_element()
            })
            .collect();
        self.body_col()
            .child(back)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(u(12.))
                    .child(self.heading(&s.title, 1))
                    .child(self.chip(&s.label)),
            )
            .child(self.panel(
                "Versions",
                vec![div().child(SharedString::from(s.versions.clone())).into_any_element()],
            ))
            .child(self.panel(
                "What happened",
                vec![div().child(SharedString::from(s.what.clone())).into_any_element()],
            ))
            .child(self.panel("Why this version", why))
            .child(self.panel("Next step", next))
            .child(self.panel("On each computer", comps))
            .child(self.panel("Folders", folders))
            .child(self.panel("Recovery copies", rec))
            .child(self.panel("Details", det))
            .into_any_element()
    }

    // ----------------------------------------------------------- Needs attention

    fn needs(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let items = need_items(&self.w);
        let mut col = self.body_col().child(self.heading("Needs attention", 1));
        if items.is_empty() {
            return col
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(u(8.))
                        .child(icon(Icon::Check, 20., pal.fg(Tone::Ok)))
                        .child("Nothing needs your attention."),
                )
                .child(self.btn(
                    "see-waiting",
                    "See what is waiting",
                    Bk::Link,
                    Act::NeedsAddonsFiltered,
                    cx,
                ))
                .into_any_element();
        }
        let n = items.len();
        let mut rows: Vec<AnyElement> = Vec::new();
        for (i, it) in items.iter().enumerate() {
            let act = Self::needs_target_act(&it.target);
            let a = if it.affects.is_empty() {
                format!("{}, {}, {}", it.title, it.reason, it.button)
            } else {
                format!(
                    "{}, {}, affects {}, {}",
                    it.title, it.reason, it.affects, it.button
                )
            };
            let r = self
                .rov_row("needs", i, n, &format!("n{i}"), a, act, cx)
                .flex()
                .items_center()
                .gap(u(10.))
                .p(u(12.))
                .border_b_1()
                .border_color(rgb(pal.divider))
                .child(icon(model::tone_icon(it.tone), 20., pal.fg(it.tone)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(SharedString::from(it.title.clone())),
                        )
                        .child(self.muted(it.reason.clone()))
                        .children(
                            (!it.affects.is_empty())
                                .then(|| self.muted(format!("Affects: {}", it.affects))),
                        ),
                )
                .child(
                    div()
                        .px(u(12.))
                        .h(u(32.))
                        .flex()
                        .items_center()
                        .text_color(rgb(pal.accent))
                        .child(it.button),
                );
            rows.push(r.into_any_element());
        }
        let list = div()
            .id("list-needs")
            .role(Role::List)
            .aria_label(SharedString::from(model::list_a11y(
                "Needs attention",
                n,
                None,
            )))
            .rounded(u(8.))
            .bg(rgb(pal.surface))
            .border_1()
            .border_color(rgb(pal.divider))
            .children(rows);
        col = col.child(list);
        col.into_any_element()
    }

    fn decision_card_view(
        &mut self,
        card: crate::decisions_ui::DecisionCard,
        back: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pal = self.pal();
        let mut col = self
            .body_col()
            .child(back)
            .child(self.heading(&format!("{}: choose what to keep", card.title), 1))
            .child(div().child(SharedString::from(card.why.clone())));
        if let Some(line) = card.hold_line.clone() {
            col = col.child(self.muted(line));
        }
        if let Some(note) = crate::decisions_ui::standing_note(&self.w) {
            col = col.child(self.muted(note));
        }
        if let Some(note) = card.no_options_note.clone() {
            col = col.child(self.muted(note));
        }
        let gate = crate::decisions_ui::choosing_gate(&self.w);
        for o in card.options.clone() {
            let ack_key = format!("ack-{}", o.id.0);
            let acked = self.expanded.contains(&ack_key);
            let mut inner: Vec<AnyElement> = vec![div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(SharedString::from(o.label.clone()))
                .into_any_element()];
            if !o.holders.is_empty() {
                inner.push(self.muted(o.holders.clone()).into_any_element());
            }
            for e in &o.effects {
                inner.push(
                    div()
                        .child(SharedString::from(e.clone()))
                        .into_any_element(),
                );
            }
            if let Some(wn) = &o.warning {
                inner.push(
                    div()
                        .text_color(rgb(pal.fg(Tone::Help)))
                        .child(SharedString::from(wn.clone()))
                        .into_any_element(),
                );
                inner.push(
                    self.btn(
                        &ack_key,
                        if acked {
                            "Understood (selected)"
                        } else {
                            "I understand this"
                        },
                        Bk::Link,
                        Act::ToggleExpand(ack_key.clone()),
                        cx,
                    )
                    .into_any_element(),
                );
            }
            let key = format!("choose-{}", o.id.0);
            let btn = match crate::decisions_ui::choose(&self.w, &card, o.id, acked) {
                Ok(a) => self.btn(&key, "Keep this", Bk::Primary, Act::Group(a), cx),
                Err(why) => self.btn_off(&key, "Keep this", &why, cx),
            };
            inner.push(btn.into_any_element());
            if gate.is_ok() && o.needs_confirm && !acked {
                inner.push(
                    self.muted("Tick the box above to continue.")
                        .into_any_element(),
                );
            }
            col = col.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(u(6.))
                    .p(u(16.))
                    .rounded(u(8.))
                    .bg(rgb(pal.surface))
                    .border_1()
                    .border_color(rgb(pal.divider))
                    .children(inner),
            );
        }
        if let Err(why) = gate {
            col = col.child(self.muted(why));
        }
        col = col.child(self.muted(card.reach.clone()));
        col = col.child(self.btn("decide-later", "Decide later", Bk::Secondary, Act::Back, cx));
        col.into_any_element()
    }

    fn decision_view(&mut self, key: &str, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let back = self.back_link(
            if self.detail_stack.last().map(|d| d.0) == Some(Page::Addons) {
                "Add-on"
            } else {
                "Needs attention"
            },
            cx,
        );
        let cards = crate::decisions_ui::cards(&self.w);
        if let Some(card) = cards.into_iter().find(|c| match &c.id.0 {
            wow_sync_core::decision::DecisionScope::Folders(ks) => {
                ks.iter().any(|k| k.as_str() == key)
            }
            _ => false,
        }) {
            return self.decision_card_view(card, back, cx);
        }
        let Some(a) = self.w.addon(key).cloned() else {
            return self
                .body_col()
                .child(back)
                .child("This decision is no longer needed.")
                .into_any_element();
        };
        let reason = match &a.kind {
            Kind::NeedsDecision(r) => model::decision_reason(&self.w, &a, *r),
            _ => String::new(),
        };
        let mut col = self
            .body_col()
            .child(back)
            .child(self.heading(&format!("{}: choose which copy to keep", a.title), 1));
        if !matches!(a.kind, Kind::NeedsDecision(_)) {
            col = col
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(u(8.))
                        .child(icon(Icon::Check, 18., pal.fg(Tone::Ok)))
                        .child(SharedString::from(format!(
                            "{} no longer needs a decision.",
                            a.title
                        ))),
                )
                .child(self.btn(
                    "see-addon",
                    &format!("Open {}", a.title),
                    Bk::Link,
                    Act::OpenAddon(key.to_string()),
                    cx,
                ));
            return col.into_any_element();
        }
        col = col.child(div().child(SharedString::from(reason)));
        for (ci, c) in a.candidates.iter().enumerate() {
            let holders = if c.holders.len() == 1 && c.holders[0] == self.w.computer {
                "This computer".to_string()
            } else {
                c.holders
                    .iter()
                    .map(|h| {
                        if *h == self.w.computer {
                            "This computer".to_string()
                        } else {
                            h.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" and ")
            };
            let only_silent = c.holders.len() == 1
                && self
                    .w
                    .peers
                    .iter()
                    .position(|p| p.name == c.holders[0])
                    .map(|pi| peer_view(&self.w, pi).stale)
                    .unwrap_or(false);
            let keep = self.btn_off(
                &format!("keep-{ci}"),
                "Keep this copy",
                "choosing a copy is not available yet",
                cx,
            );
            let dk = format!("cand-{ci}");
            let mut inner: Vec<AnyElement> = vec![
                div()
                    .flex()
                    .gap(u(10.))
                    .items_center()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(SharedString::from(format!(
                                "Version {}",
                                model::version_label(c)
                            ))),
                    )
                    .child(self.muted(format!("Held by {holders}")))
                    .into_any_element(),
                div()
                    .child(SharedString::from(c.differences.clone()))
                    .into_any_element(),
            ];
            if only_silent {
                inner.push(self.muted(format!("Only on {}, not heard from for a long time. Its files may not be downloadable now.", c.holders[0])).into_any_element());
            }
            inner.push(keep.into_any_element());
            inner.push(self.disclosure(&dk, "Details", cx).into_any_element());
            if self.expanded.contains(&dk) {
                inner.push(
                    self.muted(format!("Folder key: {}", a.key.as_str()))
                        .into_any_element(),
                );
            }
            col = col.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(u(6.))
                    .p(u(16.))
                    .rounded(u(8.))
                    .bg(rgb(pal.surface))
                    .border_1()
                    .border_color(rgb(pal.divider))
                    .children(inner),
            );
        }
        col = col.child(self.muted("The chosen copy is installed on every computer in this group. Other copies are kept as recovery copies."));
        col = col.child(self.btn("decide-later", "Decide later", Bk::Secondary, Act::Back, cx));
        col.into_any_element()
    }

    fn needs_help_view(&mut self, key: &str, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let back = self.back_link("Needs attention", cx);
        let title = self
            .w
            .addon(key)
            .map(|a| a.title.clone())
            .unwrap_or_else(|| "Add-on".into());
        let (try_again, keep) = (
            self.btn_off("nh-try", "Try again", "not available yet", cx),
            self.btn_off(
                "nh-keep",
                "Keep the previous version",
                "not available yet",
                cx,
            ),
        );
        let working = false;
        let mut det: Vec<AnyElement> = vec![self
            .disclosure("nh-details", "Details", cx)
            .into_any_element()];
        if self.expanded.contains("nh-details") {
            det.push(
                self.muted(format!(
                    "Add-on: {key} · the old and new files are both kept."
                ))
                .into_any_element(),
            );
        }
        let what = model::addon_story(&self.w, key)
            .map(|s| s.what)
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| format!("An update of {title} stopped unexpectedly."));
        let mut col = self
            .body_col()
            .child(back)
            .child(self.heading(&format!("{title}: an update stopped unexpectedly"), 1))
            .child(self.panel(
                "What happened",
                vec![div().child(SharedString::from(what)).into_any_element()],
            ))
            .child(div().flex().gap(u(10.)).child(try_again).child(keep));
        if working {
            col = col.child(
                div()
                    .flex()
                    .items_center()
                    .gap(u(8.))
                    .text_color(rgb(pal.fg(Tone::Working)))
                    .child(icon(Icon::Sync, 16., pal.fg(Tone::Working)))
                    .child("Working…"),
            );
        }
        col.child(self.panel("More", det)).into_any_element()
    }

    // ---------------------------------------------------------------- Activity

    fn activity(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let mut col = self.body_col().child(self.heading("Activity", 1));
        let labels = ["All", "Changes", "Problems"];
        let mut f = div()
            .id("filter-activity")
            .flex()
            .gap(u(8.))
            .role(Role::Group)
            .aria_label("Filter activity");
        for (i, l) in labels.iter().enumerate() {
            let on = self.act_filter as usize == i;
            let b = self
                .rov_row(
                    "actfilter",
                    i,
                    3,
                    l,
                    format!("{l}{}", if on { ", selected" } else { "" }),
                    Act::SetActFilter(i as u8),
                    cx,
                )
                .role(Role::Button)
                .aria_toggled(if on {
                    gpui::accesskit::Toggled::True
                } else {
                    gpui::accesskit::Toggled::False
                })
                .px(u(12.))
                .h(u(34.))
                .flex()
                .items_center()
                .bg(rgb(if on { pal.selected } else { pal.surface }))
                .child(SharedString::from(l.to_string()));
            f = f.child(b);
        }
        col = col.child(f);
        let acts: Vec<_> = self
            .w
            .activity
            .iter()
            .filter(|a| match self.act_filter {
                1 => !a.problem,
                2 => a.problem,
                _ => true,
            })
            .cloned()
            .collect();
        if acts.is_empty() {
            col = col.child(self.muted(if self.w.activity.is_empty() {
                "Nothing has happened yet. Changes appear here after the first sync."
            } else {
                "Nothing matches this filter."
            }));
        }
        let n = acts.len();
        let mut rows: Vec<AnyElement> = Vec::new();
        for (i, a) in acts.iter().enumerate() {
            let name = format!("act-{i}");
            let open = self.expanded.contains(&name);
            let label = format!(
                "{}, {} {}{}",
                a.title,
                a.day,
                a.when,
                if a.items.is_empty() {
                    String::new()
                } else if open {
                    ", expanded".into()
                } else {
                    ", collapsed".into()
                }
            );
            let tone = if a.problem { Tone::Help } else { Tone::Ok };
            let mut r = self
                .rov_row(
                    "activity",
                    i,
                    n,
                    &name,
                    label,
                    if a.items.is_empty() {
                        Act::None
                    } else {
                        Act::ToggleExpand(name.clone())
                    },
                    cx,
                )
                .flex()
                .flex_col()
                .p(u(10.))
                .border_b_1()
                .border_color(rgb(pal.divider))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(u(10.))
                        .child(icon(
                            if a.problem { Icon::Alert } else { Icon::Check },
                            16.,
                            pal.fg(tone),
                        ))
                        .child(
                            div()
                                .w(u(110.))
                                .flex_none()
                                .text_color(rgb(pal.muted))
                                .child(SharedString::from(format!("{} {}", a.day, a.when))),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(SharedString::from(a.title.clone())),
                        )
                        .when(!a.items.is_empty(), |d| {
                            d.child(self.muted(if open { "▾" } else { "▸" }))
                        }),
                );
            if open {
                for it in &a.items {
                    r = r.child(
                        div()
                            .pl(u(150.))
                            .text_color(rgb(pal.muted))
                            .child(SharedString::from(it.clone())),
                    );
                }
            }
            rows.push(r.into_any_element());
        }
        col = col.child(
            div()
                .id("list-activity")
                .role(Role::List)
                .aria_label(SharedString::from(model::list_a11y("Activity", n, None)))
                .rounded(u(8.))
                .bg(rgb(pal.surface))
                .border_1()
                .border_color(rgb(pal.divider))
                .children(rows),
        );
        let _ = pal;
        col.into_any_element()
    }

    // ----------------------------------------------------------- Settings sync

    fn settings_sync(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let h = self.handle("ss-banner", cx).tab_stop(true);
        if self.capture_first {
            self.content_first = Some("ss-banner".into());
            self.capture_first = false;
        }
        let banner_text = match (
            &self.w.features.settings.master,
            self.w.features.settings.enabled,
            self.w.features.settings.policy_conflict,
        ) {
            (_, _, true) => "The settings master was changed on two computers. Nothing is chosen for you; choose a master to share settings again.".to_string(),
            (Some(_), _, _) => format!(
                "{} shares its add-on settings with the group. Settings are never merged.",
                self.w.features.master.clone().unwrap_or_else(|| "The settings master".into())
            ),
            (None, true, _) => "No settings master is chosen, so settings are not shared yet.".to_string(),
            (None, false, _) => "Settings sync is off. Add-on files are synced; your add-on settings stay as they are on each computer.".to_string(),
        };
        let banner = div()
            .id("ss-banner")
            .track_focus(&h)
            .tab_stop(true)
            .role(Role::Status)
            .aria_label(SharedString::from(banner_text.clone()))
            .border_2()
            .border_color(gpui::transparent_black())
            .focus_visible(move |s| s.border_color(rgb(pal.focus)))
            .p(u(14.))
            .rounded(u(8.))
            .bg(rgb(pal.fill(Tone::Unknown)))
            .child(SharedString::from(banner_text));
        let mut col = self
            .body_col()
            .child(self.heading("Settings sync", 1))
            .child(banner);

        // Master
        let mut m: Vec<AnyElement> = vec![div()
            .child("One computer shares its add-on settings with the group automatically. Choosing a master shares settings for every add-on, including add-ons found later.")
            .into_any_element()];
        if self.expanded.contains("ss-master-open") {
            let cs = crate::settings_ui::master_controls(&self.w);
            m.extend(self.control_buttons("ss-master", cs, cx));
            m.push(
                self.btn(
                    "ss-master-close",
                    "Close",
                    Bk::Secondary,
                    Act::ToggleExpand("ss-master-open".into()),
                    cx,
                )
                .into_any_element(),
            );
        } else {
            m.push(
                self.btn(
                    "ss-master-open",
                    "Change master…",
                    Bk::Secondary,
                    Act::ToggleExpand("ss-master-open".into()),
                    cx,
                )
                .into_any_element(),
            );
        }
        col = col.child(self.panel("Settings master", m));

        // Exceptions, per add-on
        let mut e: Vec<AnyElement> = vec![self
            .muted("Per add-on, take settings from a different computer than the master, or don't share it.")
            .into_any_element()];
        let keys: Vec<(String, String)> = self
            .w
            .addons
            .iter()
            .map(|a| (a.key.clone(), a.title.clone()))
            .collect();
        for (key, title) in keys {
            let Ok(folder) = wow_sync_core::paths::validate_folder_name(&key) else {
                continue;
            };
            let open = format!("ss-exc-{key}");
            e.push(div().child(SharedString::from(title)).into_any_element());
            if self.expanded.contains(&open) {
                let cs = crate::settings_ui::exception_controls(&self.w, &folder);
                e.extend(self.control_buttons(&open, cs, cx));
                e.push(
                    self.btn(
                        &format!("{open}-close"),
                        "Close",
                        Bk::Secondary,
                        Act::ToggleExpand(open.clone()),
                        cx,
                    )
                    .into_any_element(),
                );
            } else {
                e.push(
                    self.btn(
                        &open,
                        "Add an exception…",
                        Bk::Secondary,
                        Act::ToggleExpand(open.clone()),
                        cx,
                    )
                    .into_any_element(),
                );
            }
        }
        col = col.child(self.panel("Exceptions", e));

        // Settings for each account or character
        let units = self.w.features.settings.units.clone();
        let mut s: Vec<AnyElement> = Vec::new();
        if units.is_empty() {
            s.push(self.muted(crate::settings_ui::NO_UNITS).into_any_element());
        }
        for (i, row) in units.iter().enumerate() {
            s.push(
                div()
                    .child(SharedString::from(format!(
                        "{}, {}: {}",
                        row.addon,
                        crate::settings_ui::identity_words(&row.unit),
                        crate::settings_ui::status_text(&row.status)
                    )))
                    .into_any_element(),
            );
            let cs = crate::settings_ui::unit_controls(&self.w, row);
            s.extend(self.control_buttons(&format!("ss-u{i}"), cs, cx));
            for (j, (c, sentence)) in crate::settings_ui::restore_controls(&self.w, row)
                .into_iter()
                .enumerate()
            {
                let open = format!("ss-u{i}-r{j}");
                if self.expanded.contains(&open) {
                    s.push(div().child(SharedString::from(sentence)).into_any_element());
                    s.extend(self.control_buttons(
                        &format!("{open}-go"),
                        vec![crate::settings_ui::Control {
                            label: "Restore".into(),
                            action: c.action,
                        }],
                        cx,
                    ));
                    s.push(
                        self.btn(
                            &format!("{open}-no"),
                            "Cancel",
                            Bk::Secondary,
                            Act::ToggleExpand(open),
                            cx,
                        )
                        .into_any_element(),
                    );
                } else {
                    s.push(
                        self.btn(
                            &open,
                            &c.label,
                            Bk::Secondary,
                            Act::ToggleExpand(open.clone()),
                            cx,
                        )
                        .into_any_element(),
                    );
                }
            }
        }
        col = col.child(self.panel("Settings choices and sharing", s));

        // Pause and resume
        let mut p: Vec<AnyElement> = vec![div()
            .child("Protected settings are not shared or replaced. When you resume, you'll see any differences first.")
            .into_any_element()];
        let paused = self.w.features.settings.paused.clone();
        let folders: Vec<wow_sync_core::paths::FolderKey> = units
            .iter()
            .map(|r| r.unit.folder.clone())
            .chain(paused.iter().map(|p| p.folder.clone()))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        if folders.is_empty() {
            p.push(self.muted(crate::settings_ui::NO_UNITS).into_any_element());
        }
        for f in folders {
            p.push(
                div()
                    .child(SharedString::from(f.as_str().to_string()))
                    .into_any_element(),
            );
            if let Some(pf) = paused.iter().find(|x| x.folder == f) {
                let open = format!("ss-resume-{}", f.as_str());
                if self.expanded.contains(&open) {
                    let cs = crate::settings_ui::resume_controls(
                        &self.w,
                        &f,
                        pf.here_changed,
                        pf.shared_newer,
                    );
                    p.extend(self.control_buttons(&open, cs, cx));
                    p.push(
                        self.btn(
                            &format!("{open}-stay"),
                            "Stay paused",
                            Bk::Secondary,
                            Act::ToggleExpand(open),
                            cx,
                        )
                        .into_any_element(),
                    );
                } else {
                    p.push(
                        self.btn(
                            &open,
                            "Resume…",
                            Bk::Secondary,
                            Act::ToggleExpand(open.clone()),
                            cx,
                        )
                        .into_any_element(),
                    );
                }
            } else {
                let c = crate::settings_ui::pause_control(&self.w, &f);
                p.extend(self.control_buttons(&format!("ss-pause-{}", f.as_str()), vec![c], cx));
            }
        }
        col = col.child(self.panel("Pause settings", p));
        col.into_any_element()
    }

    /// A button per control; an unavailable one is disabled and says why.
    fn control_buttons(
        &mut self,
        prefix: &str,
        cs: Vec<crate::settings_ui::Control>,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut out = Vec::new();
        for (i, c) in cs.into_iter().enumerate() {
            let k = format!("{prefix}-{i}");
            match c.action {
                Ok(a) => out.push(
                    self.btn(&k, &c.label, Bk::Secondary, Act::Group(a), cx)
                        .into_any_element(),
                ),
                Err(reason) => {
                    out.push(self.btn_off(&k, &c.label, &reason, cx).into_any_element());
                    out.push(self.muted(reason).into_any_element());
                }
            }
        }
        out
    }

    // ------------------------------------------------------------ App settings

    fn app_settings(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let mut col = self.body_col().child(self.heading("App settings", 1));
        let _ = pal;
        let signin = self.btn_off(
            "signin",
            "Start WoW Sync when I sign in",
            "not available yet",
            cx,
        );
        col = col.child(self.panel("Startup", vec![signin.into_any_element(), self.muted("WoW Sync starts only when you open it. Starting at sign-in is not available yet.").into_any_element()]));
        let paused = self.w.is_paused();
        let pb = self.btn(
            "as-pause",
            if paused {
                "Resume syncing"
            } else {
                "Pause syncing"
            },
            Bk::Secondary,
            Act::TogglePause,
            cx,
        );
        let sn = if !paused && self.w.busy.is_none() && self.w.enrolled {
            self.btn("as-sync", "Sync now", Bk::Secondary, Act::SyncNow, cx)
        } else {
            self.btn_off(
                "as-sync",
                "Sync now",
                "unavailable while paused or syncing",
                cx,
            )
        };
        let status = if self.w.pause_damaged {
            "The pause setting can't be read. Syncing is held until you resume.".to_string()
        } else if paused {
            format!(
                "Paused since {}",
                self.w.paused_since.clone().unwrap_or_default()
            )
        } else {
            "Syncing is on".into()
        };
        col = col.child(self.panel(
            "Syncing",
            vec![
                    self.muted(status).into_any_element(),
                    div()
                        .flex()
                        .gap(u(10.))
                        .child(pb)
                        .child(sn)
                        .into_any_element(),
                ],
        ));
        let mut themes = div()
            .id("theme-group")
            .flex()
            .gap(u(8.))
            .role(Role::RadioGroup)
            .aria_label("Theme");
        for (i, tc) in [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark]
            .into_iter()
            .enumerate()
        {
            let on = self.theme == tc;
            let el = self
                .rov_row(
                    "theme",
                    i,
                    3,
                    tc.label(),
                    format!(
                        "Theme: {}{}",
                        tc.label(),
                        if on { ", selected" } else { "" }
                    ),
                    Act::Theme(tc),
                    cx,
                )
                .role(Role::RadioButton)
                .aria_toggled(if on {
                    gpui::accesskit::Toggled::True
                } else {
                    gpui::accesskit::Toggled::False
                })
                .px(u(12.))
                .h(u(34.))
                .flex()
                .items_center()
                .bg(rgb(if on { pal.selected } else { pal.surface }))
                .when(on, |d| d.font_weight(FontWeight::SEMIBOLD))
                .child(SharedString::from(tc.label()));
            themes = themes.child(el);
        }
        let sys = if self.system == Mode::Dark {
            "dark"
        } else {
            "light"
        };
        col = col.child(self.panel(
            "Appearance",
            vec![
                    self.muted(format!("The system is currently set to {sys}."))
                        .into_any_element(),
                    themes.into_any_element(),
                ],
        ));
        let tg = self.w.features.toggles;
        let n1 = self.toggle(
            "notify-summaries",
            "Tell me when changes were applied",
            tg.notify_summaries,
            true,
            Act::Toggle("notify-summaries".into()),
            cx,
        );
        let n2 = self.toggle(
            "notify-needs-me",
            "Tell me when something needs me",
            tg.notify_needs_me,
            true,
            Act::Toggle("notify-needs-me".into()),
            cx,
        );
        col = col.child(self.panel(
            "Notifications",
            vec![
                self.muted("Problems always show here and in the tray icon, whatever you choose.")
                    .into_any_element(),
                n1.into_any_element(),
                n2.into_any_element(),
            ],
        ));
        let cq = self.closing_quits;
        let cqt = self.toggle(
            "closing-quits",
            "Closing the window quits WoW Sync",
            cq,
            true,
            Act::Toggle("closing-quits".into()),
            cx,
        );
        let wb = self.btn(
            "explain-close",
            "Explain close-to-tray again next time",
            Bk::Secondary,
            Act::ExplainCloseAgain,
            cx,
        );
        col = col.child(self.panel(
            "Window",
            vec![cqt.into_any_element(), wb.into_any_element()],
        ));
        let crash = self.btn_off("crash", "Send anonymous crash reports", "coming later", cx);
        let what = self.btn_off("what-sent", "What is sent?", "coming later", cx);
        col = col.child(self.panel(
            "Privacy",
            vec![crash.into_any_element(), what.into_any_element()],
        ));
        let o1 = self.btn_off("open-shared", "Open shared folder", "not available yet", cx);
        let o2 = self.btn_off("open-data", "Open data folder", "not available yet", cx);
        let o3 = self.btn(
            "export-diag",
            "Export diagnostics…",
            Bk::Secondary,
            Act::ToggleExpand("diag-open".into()),
            cx,
        );
        let shared = if self.w.shared_label.is_empty() {
            "Shared folder: not set up yet.".to_string()
        } else {
            format!("Shared folder: {}", self.w.shared_label)
        };
        let mut data = vec![
            self.muted(shared).into_any_element(),
            div()
                .flex()
                .gap(u(10.))
                .child(o1)
                .child(o2)
                .child(o3)
                .into_any_element(),
        ];
        if self.expanded.contains("diag-open") {
            data.extend(self.diag_sheet(cx));
        }
        col = col.child(self.panel("Data", data));
        let q = self.btn("quit", "Quit WoW Sync", Bk::Secondary, Act::Quit, cx);
        col.child(self.panel("Quit", vec![q.into_any_element()]))
            .into_any_element()
    }

    // ------------------------------------------------------------------- About

    /// The export sheet: options, what is never included, the exact preview, then Save.
    fn diag_sheet(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        use crate::features::{gate, ActionKind, GroupAction};
        let pal = self.pal();
        let opts = self.diag.options;
        let mut v: Vec<AnyElement> = Vec::new();
        for w in crate::about_ui::warnings() {
            v.push(self.muted(w).into_any_element());
        }
        v.push(
            self.toggle(
                "diag-names",
                "Include add-on folder names",
                opts.include_folder_names,
                false,
                Act::Toggle("diag-names".into()),
                cx,
            )
            .into_any_element(),
        );
        v.push(
            self.toggle(
                "diag-fingerprints",
                "Include file fingerprints",
                opts.include_fingerprints,
                false,
                Act::Toggle("diag-fingerprints".into()),
                cx,
            )
            .into_any_element(),
        );
        let preview = match gate(&self.w, ActionKind::ExportDiagnostics) {
            Ok(()) => self.btn(
                "diag-preview",
                "Preview",
                Bk::Secondary,
                Act::Group(GroupAction::ExportDiagnostics { options: opts }),
                cx,
            ),
            Err(why) => self.btn_off("diag-preview", "Preview", &why, cx),
        };
        v.push(preview.into_any_element());
        if let Some(p) = self.diag.preview.clone() {
            v.push(self.muted(p.summary.clone()).into_any_element());
            v.push(
                div()
                    .id("diag-text")
                    .max_h(u(200.))
                    .overflow_y_scroll()
                    .p(u(8.))
                    .rounded(u(6.))
                    .bg(rgb(pal.surface))
                    .text_size(u(12.))
                    .child(SharedString::from(p.text))
                    .into_any_element(),
            );
            let dest = crate::about_ui::default_dest();
            v.push(
                self.muted(format!("It will be saved as {}", dest.display()))
                    .into_any_element(),
            );
            let save = match gate(&self.w, ActionKind::SaveDiagnostics) {
                Ok(()) => self.btn(
                    "diag-save",
                    "Save",
                    Bk::Primary,
                    Act::Group(GroupAction::SaveDiagnostics {
                        options: opts,
                        dest,
                    }),
                    cx,
                ),
                Err(why) => self.btn_off("diag-save", "Save", &why, cx),
            };
            v.push(save.into_any_element());
        } else {
            v.push(
                self.muted("Preview first. Only what you see is saved.")
                    .into_any_element(),
            );
        }
        if let Some(m) = self.diag.message.clone() {
            v.push(self.muted(m).into_any_element());
        }
        v
    }

    fn about(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let pal = self.pal();
        let u_info = self.w.update.clone();
        let check = match crate::features::gate(&self.w, crate::features::ActionKind::CheckUpdates)
        {
            Ok(()) => self.btn(
                "check-update",
                "Check for updates",
                Bk::Secondary,
                Act::Group(crate::features::GroupAction::CheckUpdates),
                cx,
            ),
            Err(why) => self.btn_off("check-update", "Check for updates", &why, cx),
        };
        let auto = self.toggle(
            "update-check",
            "Check for updates automatically",
            self.w.features.toggles.update_check,
            true,
            Act::Toggle("update-check".into()),
            cx,
        );
        let up: Vec<AnyElement> = vec![
            self.muted(crate::about_ui::update_text(self.w.features.update.as_ref()))
                .into_any_element(),
            check.into_any_element(),
            auto.into_any_element(),
            self.muted("WoW Sync never replaces itself. To update, download the new version and install it; your sync data and recovery copies are kept.").into_any_element(),
        ];
        let copy = self.btn(
            "copy-version",
            if self.copied {
                "Copied version info"
            } else {
                "Copy version info"
            },
            Bk::Secondary,
            Act::CopyVersion,
            cx,
        );
        let ver = vec![
            div().child(format!("WoW Sync {}", u_info.current)).into_any_element(),
            div().text_color(rgb(pal.fg(Tone::Waiting))).child("This build is not signed by a publisher. Your system may warn you when installing it. Do not turn off system protections; follow the install guide.").into_any_element(),
            copy.into_any_element(),
        ];
        self.body_col()
            .child(self.heading("About", 1))
            .child(self.panel("Version", ver))
            .child(self.panel("Updates", up))
            .child(self.panel(
                "Licence",
                vec![self.muted("MIT licence, with third-party notices.").into_any_element()],
            ))
            .into_any_element()
    }
}

#[allow(dead_code)]
fn _palette_used(_: Palette) {}
