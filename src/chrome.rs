//! Window chrome: 48 px title bar, 52 px navigation rail, 28 px status bar.

use crate::i18n::RespT;
use crate::app::*;
use crate::icons::{self, Icon};
use crate::theme::{semibold, Ty};
use crate::ui::*;
use eframe::egui::{self, vec2, Align, Color32, FontId, Id, Layout, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder};

fn sub(ui: &mut Ui, rect: Rect, layout: Layout, f: impl FnOnce(&mut Ui)) {
    let mut c = ui.new_child(UiBuilder::new().max_rect(rect).layout(layout));
    c.spacing_mut().item_spacing = vec2(8.0, 0.0);
    f(&mut c);
}

/// Two-line picker button (caption above, value below).
fn picker(ui: &mut Ui, th: &crate::theme::Theme, icon: Icon, caps: &str, text: &str, mono: bool, max_w: f32) -> egui::Response {
    let p = &th.p;
    let f1 = FontId::new(10.0, semibold());
    let f2 = if mono { FontId::new(13.0, crate::theme::monobold()) } else { FontId::new(13.0, semibold()) };
    let tw = text_width(ui, text, &f2).min(max_w).max(text_width(ui, caps, &f1));
    let w = 10.0 + 16.0 + 8.0 + tw + 8.0 + 12.0 + 10.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 34.0), Sense::click());
    let painter = ui.painter_at(rect);
    if resp.hovered() {
        painter.rect_filled(rect, cr(6), p.bg3);
    }
    icons::paint_at(ui, Pos2::new(rect.left() + 10.0 + 8.0, rect.center().y), 16.0, icon, p.text1);
    let x = rect.left() + 10.0 + 16.0 + 8.0;
    draw_left(&painter, x, rect.center().y - 7.5, caps, f1, p.text3);
    draw_trunc(&painter, x, rect.center().y + 7.0, tw + 1.0, text, f2, p.text1);
    icons::paint_at(ui, Pos2::new(rect.right() - 10.0 - 6.0, rect.center().y), 12.0, Icon::Chev, p.text3);
    pointer(&resp);
    resp
}

fn win_btn(ui: &mut Ui, th: &crate::theme::Theme, kind: u8) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(36.0, 32.0), Sense::click());
    let p = &th.p;
    let hov = resp.hovered();
    let bg = if hov { if kind == 2 { Color32::from_rgb(0xC4, 0x2B, 0x1C) } else { p.bg3 } } else { Color32::TRANSPARENT };
    ui.painter().rect_filled(rect, 0.0, bg);
    let fg = if hov && kind == 2 { Color32::WHITE } else { p.text2 };
    let c = rect.center();
    let st = Stroke::new(1.3, fg);
    let painter = ui.painter();
    match kind {
        0 => {
            painter.line_segment([Pos2::new(c.x - 4.0, c.y), Pos2::new(c.x + 4.0, c.y)], st);
        }
        1 => {
            painter.rect_stroke(Rect::from_center_size(c, vec2(7.0, 7.0)), cr(1), st, StrokeKind::Middle);
        }
        _ => {
            painter.line_segment([Pos2::new(c.x - 3.5, c.y - 3.5), Pos2::new(c.x + 3.5, c.y + 3.5)], st);
            painter.line_segment([Pos2::new(c.x + 3.5, c.y - 3.5), Pos2::new(c.x - 3.5, c.y + 3.5)], st);
        }
    }
    resp
}

pub fn title_bar(app: &mut App, ctx: &egui::Context) {
    let th_p = app.th.p.clone();
    egui::TopBottomPanel::top("title")
        .exact_height(48.0)
        .frame(egui::Frame::new().fill(th_p.bg0))
        .show_separator_line(false)
        .show(ctx, |ui| {
            let full = ui.max_rect();
            let p = &th_p;
            // drag + maximize on the empty background
            let bg = ui.interact(full, Id::new("title-drag"), Sense::click_and_drag());
            if bg.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if bg.double_clicked() {
                let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
            }
            hline(ui.painter(), full.left(), full.right(), full.bottom() - 0.5, p.line_soft);

            let mut cmds: Vec<Cmd> = vec![];
            let mut open_repo_menu: Option<Pos2> = None;
            let mut open_sync_menu: Option<Pos2> = None;
            let mut win: Option<u8> = None;

            let left = Rect::from_min_max(Pos2::new(full.left() + 12.0, full.top()), Pos2::new(full.left() + 12.0 + 520.0, full.bottom()));
            sub(ui, left, Layout::left_to_right(Align::Center), |ui| {
                let name = app.repo.as_ref().map(|r| r.name.clone()).unwrap_or_else(|| "Depo seç".into());
                let r = picker(ui, &app.th, Icon::Folder, "DEPO", &name, false, 170.0);
                if r.clicked() {
                    open_repo_menu = Some(Pos2::new(r.rect.left(), r.rect.bottom() + 4.0));
                }
                if app.repo.is_some() {
                    let (sr, _) = ui.allocate_exact_size(vec2(1.0, 20.0), Sense::hover());
                    ui.painter().rect_filled(sr, 0.0, p.line_soft);
                    let br = app.repo.as_ref().map(|r| r.branch()).unwrap_or_default();
                    let r = picker(ui, &app.th, Icon::Branch, "DAL", &br, true, 200.0);
                    if r.clicked() {
                        cmds.push(Cmd::Switcher);
                    }
                }
            });

            let right = Rect::from_min_max(Pos2::new(full.right() - 560.0, full.top()), Pos2::new(full.right(), full.bottom()));
            sub(ui, right, Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if win_btn(ui, &app.th, 2).clicked() {
                    win = Some(2);
                }
                if win_btn(ui, &app.th, 1).clicked() {
                    win = Some(1);
                }
                if win_btn(ui, &app.th, 0).clicked() {
                    win = Some(0);
                }
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing.x = 8.0;
                let (ar, aresp) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
                let who = app.s.account.as_ref().map(|a| if a.name.is_empty() { a.login.clone() } else { a.name.clone() }).unwrap_or_else(|| if app.git_user.0.is_empty() { "?".into() } else { app.git_user.0.clone() });
                avatar(ui.painter(), p, ar.center(), 28.0, &initials_of(&who));
                if aresp.clicked() {
                    cmds.push(Cmd::Settings);
                }
                pointer(&aresp);
                if icon_button(ui, &app.th, Icon::Gear, 32.0, false).on_hover_text_t("Ayarlar (Ctrl ,)").clicked() {
                    cmds.push(Cmd::Settings);
                }
                if app.repo.is_some() {
                    let st = app.repo.as_ref().and_then(|r| r.status.clone());
                    let (label, icon, count, cmd) = match &st {
                        Some(s) if s.upstream.is_none() && s.branch.is_some() => ("Branch'i yayınla".to_string(), Icon::Up, None, Cmd::Push),
                        Some(s) if s.ahead > 0 => ("Push origin".into(), Icon::Up, Some(s.ahead.to_string()), Cmd::Push),
                        Some(s) if s.behind > 0 => ("Pull origin".into(), Icon::Down, Some(s.behind.to_string()), Cmd::Pull),
                        _ => ("Fetch origin".into(), Icon::Sync, None, Cmd::Fetch),
                    };
                    let busy = app.sync_label.is_some();
                    let label = if let Some(l) = app.sync_label { format!("{l}…") } else { label };
                    let w = 12.0 * 2.0 + 14.0 + 6.0 + text_width(ui, &label, &FontId::new(13.0, semibold())) + count.as_ref().map(|c| 6.0 + text_width(ui, c, &app.th.f(Ty::MonoXs)) + 10.0).unwrap_or(0.0) + 28.0;
                    let (rect, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
                    let mut r = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::left_to_right(Align::Center)));
                    r.spacing_mut().item_spacing.x = 0.0;
                    let (m, c) = split_button(&mut r, &app.th, &label, icon, count, !busy);
                    if m.clicked() && !busy {
                        cmds.push(cmd);
                    }
                    if c.clicked() {
                        open_sync_menu = Some(Pos2::new(rect.right(), rect.bottom() + 4.0));
                    }
                }
            });

            let cw = 380.0;
            let srect = Rect::from_center_size(Pos2::new(full.center().x, full.center().y), vec2(cw, 32.0));
            if full.width() > 1180.0 {
                let resp = ui.interact(srect, Id::new("title-search"), Sense::click());
                let painter = ui.painter();
                painter.rect_filled(srect, cr(6), if resp.hovered() { p.bg2 } else { p.bg1 });
                painter.rect_stroke(srect, cr(6), Stroke::new(1.0, if resp.hovered() { p.line_strong } else { p.line }), StrokeKind::Inside);
                icons::paint_at(ui, Pos2::new(srect.left() + 18.0, srect.center().y), 14.0, Icon::Search, p.text3);
                draw_trunc(painter, srect.left() + 34.0, srect.center().y, srect.width() - 34.0 - 72.0, "Komut, dosya, dal ve commit ara", app.th.f(Ty::Row), p.text3);
                draw_right(painter, srect.right() - 10.0, srect.center().y, "Ctrl K", app.th.f(Ty::MonoXs), p.text3);
                pointer(&resp);
                if resp.clicked() {
                    cmds.push(Cmd::Palette);
                }
            }

            if let Some(pos) = open_repo_menu {
                let mut items: Vec<MenuItem> = app
                    .s
                    .recents
                    .iter()
                    .take(8)
                    .map(|r| MenuItem { label: r.name.clone(), icon: Some(Icon::Folder), kbd: None, cmd: Cmd::OpenRecent(r.path.clone()), danger: false, enabled: true })
                    .collect();
                items.push(MenuItem { label: "Yerel depo aç…".into(), icon: Some(Icon::Plus), kbd: Some("Ctrl O"), cmd: Cmd::OpenFolder, danger: false, enabled: true });
                items.push(MenuItem { label: "Depo klonla…".into(), icon: Some(Icon::Clone), kbd: Some("Ctrl Shift O"), cmd: Cmd::CloneRepo, danger: false, enabled: true });
                if app.repo.is_some() {
                    items.push(MenuItem { label: "Editörde aç".into(), icon: Some(Icon::Edit), kbd: None, cmd: Cmd::OpenEditor, danger: false, enabled: true });
                    items.push(MenuItem { label: "Terminalde aç".into(), icon: Some(Icon::Terminal), kbd: None, cmd: Cmd::OpenTerminal, danger: false, enabled: true });
                    items.push(MenuItem { label: "Dosya Gezgini'nde göster".into(), icon: Some(Icon::Folder), kbd: None, cmd: Cmd::Reveal, danger: false, enabled: true });
                }
                app.menu = Some(Menu { pos, right_align: false, items });
            }
            if let Some(pos) = open_sync_menu {
                let mk = |label: &str, icon, cmd: Cmd, danger: bool| MenuItem { label: label.into(), icon: Some(icon), kbd: cmd.shortcut(), cmd, danger, enabled: true };
                app.menu = Some(Menu {
                    pos,
                    right_align: true,
                    items: vec![mk("Fetch origin", Icon::Sync, Cmd::Fetch, false), mk("Pull", Icon::Down, Cmd::Pull, false), mk("Push", Icon::Up, Cmd::Push, false), mk("Force push (güvenli)", Icon::Alert, Cmd::ForcePush, true)],
                });
            }
            match win {
                Some(0) => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                Some(1) => {
                    let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
                }
                Some(2) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                _ => {}
            }
            for c in cmds {
                app.run(c);
            }
        });
}

pub fn nav_rail(app: &mut App, ctx: &egui::Context) {
    let p = app.th.p.clone();
    egui::SidePanel::left("rail")
        .exact_width(52.0)
        .resizable(false)
        .frame(egui::Frame::new().fill(p.bg0).inner_margin(egui::Margin::symmetric(0, 8)))
        .show_separator_line(false)
        .show(ctx, |ui| {
            let full = ui.max_rect();
            vline(ui.painter(), full.right() - 0.5, full.top() - 8.0, full.bottom() + 8.0, p.line_soft);
            let changes = app.repo.as_ref().map(|r| r.entries().len()).unwrap_or(0);
            let items: [(Screen, Icon, &str, Option<(usize, bool)>); 5] = [
                (Screen::Changes, Icon::Diff, "Değişiklikler", Some((changes, true))),
                (Screen::History, Icon::History, "Geçmiş", None),
                (Screen::Branches, Icon::Branch, "Dallar", None),
                (Screen::Pr, Icon::Pr, "Pull request'ler", None),
                (Screen::Stash, Icon::Stash, "Stash", app.repo.as_ref().map(|r| (r.stashes.len(), false))),
            ];
            ui.spacing_mut().item_spacing.y = 4.0;
            let enabled = app.repo.is_some();
            for (s, icon, tip, badge_n) in items {
                let active = app.screen == s || (s == Screen::Changes && app.screen == Screen::Conflict);
                let (rect, resp) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::click());
                let rect = Rect::from_center_size(Pos2::new(full.center().x, rect.center().y), rect.size());
                if active {
                    ui.painter().rect_filled(rect, cr(8), p.bg3);
                } else if resp.hovered() && enabled {
                    ui.painter().rect_filled(rect, cr(8), p.bg3);
                }
                let fg = if !enabled { p.text3.gamma_multiply(0.5) } else if active { p.accent } else if resp.hovered() { p.text1 } else { p.text2 };
                icons::paint_at(ui, rect.center(), 18.0, icon, fg);
                if let Some((n, accent)) = badge_n {
                    if n > 0 && enabled {
                        let txt = if n > 99 { "99+".to_string() } else { n.to_string() };
                        let tw = text_width(ui, &txt, &FontId::new(10.0, semibold()));
                        let w = (tw + 6.0).max(16.0);
                        let br = Rect::from_min_size(Pos2::new(rect.right() - w + 2.0, rect.top() + 3.0), vec2(w, 16.0));
                        ui.painter().rect_filled(br, cr(8), if accent { p.accent } else { Color32::from_rgb(0x3A, 0x37, 0x31) });
                        centered(ui.painter(), br, &txt, FontId::new(10.0, semibold()), if accent { p.accent_fg } else { p.text1 });
                    }
                }
                let resp = resp.on_hover_text_t(tip);
                pointer(&resp);
                if resp.clicked() && enabled {
                    app.goto(s);
                }
            }
            // bottom: operation log
            let bottom = Rect::from_min_size(Pos2::new(full.left(), full.bottom() - 40.0), vec2(52.0, 40.0));
            sub(ui, bottom, Layout::top_down(Align::Center), |ui| {
                let (rect, resp) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::click());
                let active = app.screen == Screen::Log;
                if active || resp.hovered() {
                    ui.painter().rect_filled(rect, cr(8), p.bg3);
                }
                icons::paint_at(ui, rect.center(), 18.0, Icon::Undo, if active { p.accent } else if resp.hovered() { p.text1 } else { p.text2 });
                let resp = resp.on_hover_text_t("İşlem günlüğü");
                pointer(&resp);
                if resp.clicked() && app.repo.is_some() {
                    app.goto(Screen::Log);
                }
            });
        });
}

pub fn status_bar(app: &mut App, ctx: &egui::Context) {
    let p = app.th.p.clone();
    // cache the last git command (the activity log clones a Vec)
    let last: Option<(String, u128, bool)> = ctx.data_mut(|d| {
        let key = Id::new("lastcmd");
        let cached: Option<(std::time::Instant, Option<(String, u128, bool)>)> = d.get_temp(key);
        match cached {
            Some((t, v)) if t.elapsed().as_millis() < 400 => v,
            _ => {
                let v = crate::backend::stage::last_log();
                d.insert_temp(key, (std::time::Instant::now(), v.clone()));
                v
            }
        }
    });
    let mut cmds: Vec<Cmd> = vec![];
    egui::TopBottomPanel::bottom("status")
        .exact_height(28.0)
        .frame(egui::Frame::new().fill(p.bg0))
        .show_separator_line(false)
        .show(ctx, |ui| {
            let full = ui.max_rect();
            hline(ui.painter(), full.left(), full.right(), full.top() + 0.5, p.line_soft);
            let painter = ui.painter().clone();
            let cy = full.center().y;
            let mut x = full.left() + 12.0;
            let busy = app.busy > 0 || app.sync_label.is_some();
            let (dot, label) = if let Some(l) = app.sync_label {
                (p.accent, format!("{l} sürüyor"))
            } else if app.busy > 0 {
                (p.accent, "Çalışıyor".to_string())
            } else {
                (p.added, "Hazır".to_string())
            };
            painter.circle_filled(Pos2::new(x + 3.5, cy), 3.5, dot);
            x = draw_left(&painter, x + 7.0 + 6.0, cy, &label, app.th.f(Ty::Small), p.text2) + 16.0;
            if busy {
                ctx.request_repaint_after(std::time::Duration::from_millis(120));
            }
            let txt = if !app.sync_text.is_empty() && app.sync_label.is_some() {
                app.sync_text.clone()
            } else if let Some((c, ms, _)) = &last {
                format!("git {c} · {ms} ms")
            } else {
                String::new()
            };
            if !txt.is_empty() {
                icons::paint_at(ui, Pos2::new(x + 6.0, cy), 12.0, Icon::Terminal, p.text2);
                let max = (full.right() - x - 420.0).max(100.0);
                draw_trunc(&painter, x + 18.0, cy, max, &txt, app.th.f(Ty::MonoXs), p.text2);
            }
            let mut rx = full.right() - 12.0;
            if !app.git_version.is_empty() {
                rx = draw_right(&painter, rx, cy, &format!("{} · UTF-8", app.git_version), app.th.f(Ty::MonoXs), p.text3) - 16.0;
            }
            if let Some(op) = app.oplog.first() {
                let can = app.can_undo();
                if can {
                    let bw = 74.0;
                    let br = Rect::from_min_size(Pos2::new(rx - bw, cy - 10.0), vec2(bw, 20.0));
                    let resp = ui.interact(br, Id::new("status-undo"), Sense::click());
                    if resp.hovered() {
                        painter.rect_filled(br, cr(4), p.bg3);
                    }
                    icons::paint_at(ui, Pos2::new(br.left() + 11.0, cy), 11.0, Icon::Undo, p.accent_hi);
                    draw_left(&painter, br.left() + 22.0, cy, "Geri al", FontId::new(12.0, semibold()), p.accent_hi);
                    pointer(&resp);
                    if resp.clicked() {
                        cmds.push(Cmd::Undo);
                    }
                    rx = br.left() - 8.0;
                }
                let txt = format!("Son işlem: {}", op.title);
                let g = painter.layout_no_wrap(txt.clone(), app.th.f(Ty::Small), p.text2);
                let w = g.size().x.min((rx - x - 40.0).max(60.0));
                draw_trunc(&painter, rx - w, cy, w, &txt, app.th.f(Ty::Small), p.text2);
            }
        });
    for c in cmds {
        app.run(c);
    }
}

