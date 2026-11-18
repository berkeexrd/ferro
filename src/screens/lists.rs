//! Branches, Stash and Operation log pages.

use crate::i18n::{PainterT};
use crate::app::*;
use crate::icons::{self, Icon};
use crate::theme::{semibold, Ty};
use crate::ui::*;
use eframe::egui::{self, vec2, Align, FontId, Id, Layout, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder};

pub fn show(app: &mut App, ui: &mut Ui) {
    let mut cmds: Vec<Cmd> = vec![];
    let mut sel_stash: Option<usize> = None;
    let mut sel_file: Option<String> = None;
    let mut filter: Option<String> = None;
    match app.screen {
        Screen::Branches => branches(app, ui, &mut cmds, &mut filter),
        Screen::Stash => stash(app, ui, &mut cmds, &mut sel_stash, &mut sel_file),
        _ => oplog(app, ui, &mut cmds),
    }
    if let Some(f) = filter {
        ui.ctx().data_mut(|d| d.insert_temp(Id::new("branch-filter"), f));
    }
    if let Some(i) = sel_stash {
        app.select_stash(i);
    }
    if let Some(f) = sel_file {
        app.select_stash_file(f);
    }
    if let Some((n, op)) = ui.ctx().data_mut(|d| d.remove_temp::<(String, String)>(Id::new("stash-act"))) {
        app.stash_apply(n, &op);
    }
    for c in cmds {
        app.run(c);
    }
}

fn page_header(app: &App, ui: &mut Ui, rect: Rect, title: &str, sub: &str) -> f32 {
    let p = &app.th.p;
    draw_left(ui.painter(), rect.left() + 24.0, rect.top() + 32.0, title, FontId::new(20.0, semibold()), p.text1);
    draw_left(ui.painter(), rect.left() + 24.0, rect.top() + 56.0, sub, app.th.f(Ty::Small), p.text2);
    76.0
}

fn branches(app: &App, ui: &mut Ui, cmds: &mut Vec<Cmd>, filter: &mut Option<String>) {
    let p = &app.th.p;
    let full = ui.max_rect();
    let r = app.repo.as_ref().unwrap();
    let h = page_header(app, ui, full, "Dallar", &format!("{} yerel · {} uzak · {} etiket", r.branches.iter().filter(|b| !b.is_remote).count(), r.branches.iter().filter(|b| b.is_remote && !b.name.ends_with("/HEAD")).count(), r.tags.len()));
    let nb = Rect::from_min_size(Pos2::new(full.right() - 24.0 - 120.0, full.top() + 22.0), vec2(120.0, 32.0));
    let mut bui = ui.new_child(UiBuilder::new().max_rect(nb).layout(Layout::left_to_right(Align::Center)));
    if button(&mut bui, &app.th, Btn::new("Yeni dal").icon(Icon::Plus).primary().min_w(120.0)).clicked() {
        cmds.push(Cmd::NewBranch);
    }
    let sr = Rect::from_min_size(Pos2::new(nb.left() - 12.0 - 280.0, full.top() + 22.0), vec2(280.0, 32.0));
    let mut sui = ui.new_child(UiBuilder::new().max_rect(sr).layout(Layout::left_to_right(Align::Center)));
    let mut q: String = ui.ctx().data_mut(|d| d.get_temp(Id::new("branch-filter"))).unwrap_or_default();
    let old = q.clone();
    text_input(&mut sui, &app.th, &mut q, InputOpts::new(Id::new("branch-filter-input"), "Dal ara").width(280.0).height(32.0));
    if q != old {
        *filter = Some(q.clone());
    }
    let area = Rect::from_min_max(Pos2::new(full.left() + 16.0, full.top() + h), full.max - vec2(16.0, 8.0));
    let mut aui = ui.new_child(UiBuilder::new().max_rect(area).layout(Layout::top_down(Align::Min)));
    let cur = r.branch();
    let list: Vec<&crate::backend::repo::Branch> = r.branches.iter().filter(|b| !b.name.ends_with("/HEAD")).filter(|b| q.is_empty() || b.name.to_lowercase().contains(&q.to_lowercase())).collect();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut aui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for (title, remote) in [("YEREL", false), ("UZAK", true)] {
            let group: Vec<_> = list.iter().filter(|b| b.is_remote == remote).collect();
            if group.is_empty() {
                continue;
            }
            let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
            draw_left(ui.painter(), hr.left() + 8.0, hr.center().y + 4.0, &format!("{title} · {}", group.len()), FontId::new(11.0, semibold()), p.text3);
            for b in group {
                let (rr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
                let painter = ui.painter_at(rr);
                let is_cur = b.current;
                if is_cur {
                    painter.rect_filled(rr, cr(8), p.bg_sel);
                    painter.rect_stroke(rr, cr(8), Stroke::new(1.0, p.accent_dim), StrokeKind::Inside);
                } else if resp.hovered() {
                    painter.rect_filled(rr, cr(8), p.bg3);
                }
                icons::paint_at(ui, Pos2::new(rr.left() + 22.0, rr.center().y), 16.0, if remote { Icon::Link } else { Icon::Branch }, if is_cur { p.accent } else { p.text2 });
                let mut rx = rr.right() - 14.0;
                if resp.hovered() && !is_cur {
                    let mk = |ui: &mut Ui, label: &str, id: &str, rx: &mut f32, danger: bool| -> bool {
                        let w = text_width(ui, label, &app.th.f(Ty::Small)) + 22.0;
                        let br = Rect::from_min_size(Pos2::new(*rx - w, rr.center().y - 12.0), vec2(w, 24.0));
                        *rx -= w + 6.0;
                        let rs = ui.interact(br, Id::new((id, &b.name)), Sense::click());
                        let c = if danger { p.deleted } else { p.text1 };
                        ui.painter().rect_filled(br, cr(6), if rs.hovered() { p.bg_sel } else { p.bg1 });
                        ui.painter().rect_stroke(br, cr(6), Stroke::new(1.0, if danger { p.danger_line } else { p.line_strong }), StrokeKind::Inside);
                        centered(ui.painter(), br, label, app.th.f(Ty::Small), c);
                        pointer(&rs);
                        rs.clicked()
                    };
                    let target = if remote { b.name.split_once('/').map(|x| x.1.to_string()).unwrap_or(b.name.clone()) } else { b.name.clone() };
                    if mk(ui, "Geç", "sw", &mut rx, false) {
                        cmds.push(Cmd::SwitchTo(target.clone()));
                    }
                    if mk(ui, &format!("{cur} içine birleştir"), "mg", &mut rx, false) {
                        cmds.push(Cmd::MergeInto(b.name.clone()));
                    }
                    if !remote && mk(ui, "Sil", "rm", &mut rx, true) {
                        cmds.push(Cmd::DeleteBranch(b.name.clone()));
                    }
                } else {
                    rx = draw_right(&painter, rx, rr.center().y, &crate::util::rel(crate::util::parse_ts(&b.date)), app.th.f(Ty::Small), p.text3) - 12.0;
                    if b.behind > 0 {
                        rx = draw_right(&painter, rx, rr.center().y, &format!("↓{}", b.behind), app.th.f(Ty::MonoXs), p.renamed) - 8.0;
                    }
                    if b.ahead > 0 {
                        rx = draw_right(&painter, rx, rr.center().y, &format!("↑{}", b.ahead), app.th.f(Ty::MonoXs), p.added) - 8.0;
                    }
                    if is_cur {
                        rx = draw_right(&painter, rx, rr.center().y, "mevcut", FontId::new(11.0, semibold()), p.accent_hi) - 10.0;
                    }
                }
                let tx = rr.left() + 44.0;
                draw_trunc(&painter, tx, rr.center().y - 8.0, rx - tx, &b.name, FontId::new(13.0, crate::theme::monobold()), p.text1);
                draw_trunc(&painter, tx, rr.center().y + 9.0, rx - tx, &format!("{} · {}", b.author, b.subject), app.th.f(Ty::Small), p.text3);
                pointer(&resp);
                if resp.double_clicked() && !is_cur {
                    let target = if remote { b.name.split_once('/').map(|x| x.1.to_string()).unwrap_or(b.name.clone()) } else { b.name.clone() };
                    cmds.push(Cmd::SwitchTo(target));
                }
            }
            ui.add_space(8.0);
        }
        if !r.tags.is_empty() && q.is_empty() {
            let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
            draw_left(ui.painter(), hr.left() + 8.0, hr.center().y + 4.0, &format!("ETİKETLER · {}", r.tags.len()), FontId::new(11.0, semibold()), p.text3);
            for t in r.tags.iter().take(30) {
                let (rr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
                if resp.hovered() {
                    ui.painter().rect_filled(rr, cr(8), p.bg3);
                }
                icons::paint_at(ui, Pos2::new(rr.left() + 22.0, rr.center().y), 14.0, Icon::Tag, p.modified);
                draw_left(ui.painter(), rr.left() + 44.0, rr.center().y, &t.name, app.th.f(Ty::Mono), p.text1);
                draw_right(ui.painter(), rr.right() - 14.0, rr.center().y, &crate::util::rel(crate::util::parse_ts(&t.date)), app.th.f(Ty::Small), p.text3);
            }
        }
    });
}

fn stash(app: &App, ui: &mut Ui, cmds: &mut Vec<Cmd>, sel_stash: &mut Option<usize>, sel_file: &mut Option<String>) {
    let p = &app.th.p;
    let full = ui.max_rect();
    let r = app.repo.as_ref().unwrap();
    let lw = 380.0f32;
    let left = Rect::from_min_max(full.min, Pos2::new(full.left() + lw, full.bottom()));
    let right = Rect::from_min_max(Pos2::new(left.right(), full.top()), full.max);
    ui.painter().rect_filled(left, 0.0, p.bg1);
    vline(ui.painter(), left.right() - 0.5, left.top(), left.bottom(), p.line_soft);
    draw_left(ui.painter(), left.left() + 16.0, left.top() + 28.0, "Stash", FontId::new(15.0, semibold()), p.text1);
    draw_right(ui.painter(), left.right() - 16.0, left.top() + 29.0, &format!("{} kayıt", r.stashes.len()), app.th.f(Ty::Small), p.text2);
    let sb = Rect::from_min_size(Pos2::new(left.left() + 14.0, left.top() + 48.0), vec2(lw - 28.0, 32.0));
    let mut bui = ui.new_child(UiBuilder::new().max_rect(sb).layout(Layout::left_to_right(Align::Center)));
    if button(&mut bui, &app.th, Btn::new("Değişiklikleri stash'le").icon(Icon::Stash).min_w(lw - 28.0).enabled(r.dirty())).clicked() {
        cmds.push(Cmd::StashAll);
    }
    let area = Rect::from_min_max(Pos2::new(left.left() + 6.0, left.top() + 92.0), left.max - vec2(7.0, 0.0));
    if r.stashes.is_empty() {
        ui.painter().text_t(area.center(), egui::Align2::CENTER_CENTER, "Stash boş", app.th.f(Ty::Row), p.text3);
    }
    let mut lui = ui.new_child(UiBuilder::new().max_rect(area).layout(Layout::top_down(Align::Min)));
    egui::ScrollArea::vertical().id_salt("stash-list").auto_shrink([false, false]).show(&mut lui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for (i, s) in r.stashes.iter().enumerate() {
            let (rr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::click());
            let sel = r.sel_stash == Some(i);
            if sel {
                ui.painter().rect_filled(rr, cr(8), p.bg_sel);
            } else if resp.hovered() {
                ui.painter().rect_filled(rr, cr(8), p.bg3);
            }
            icons::paint_at(ui, Pos2::new(rr.left() + 20.0, rr.center().y), 16.0, Icon::Stash, if sel { p.accent } else { p.text2 });
            draw_trunc(ui.painter(), rr.left() + 40.0, rr.center().y - 8.0, rr.width() - 56.0, &s.subject, app.th.f(Ty::Row), p.text1);
            draw_left(ui.painter(), rr.left() + 40.0, rr.center().y + 9.0, &format!("{} · {}", s.name, crate::util::rel(crate::util::parse_ts(&s.date))), app.th.f(Ty::MonoXs), p.text3);
            pointer(&resp);
            if resp.clicked() {
                *sel_stash = Some(i);
            }
        }
    });
    let Some(s) = r.sel_stash.and_then(|i| r.stashes.get(i)) else {
        ui.painter().text_t(right.center(), egui::Align2::CENTER_CENTER, "Ayrıntı için bir stash seç", FontId::new(14.0, semibold()), p.text2);
        return;
    };
    draw_trunc(ui.painter(), right.left() + 20.0, right.top() + 32.0, right.width() - 40.0, &s.subject, FontId::new(18.0, semibold()), p.text1);
    let ar = Rect::from_min_size(Pos2::new(right.left() + 20.0, right.top() + 56.0), vec2(right.width() - 40.0, 28.0));
    let mut aui = ui.new_child(UiBuilder::new().max_rect(ar).layout(Layout::left_to_right(Align::Center)));
    aui.spacing_mut().item_spacing.x = 8.0;
    if button(&mut aui, &app.th, Btn::new("Geri yükle (pop)").primary().small()).clicked() {
        cmds.push(Cmd::StashPop(s.name.clone()));
    }
    if button(&mut aui, &app.th, Btn::new("Uygula").small()).clicked() {
        ui.ctx().data_mut(|d| d.insert_temp(Id::new("stash-act"), (s.name.clone(), "apply".to_string())));
    }
    if button(&mut aui, &app.th, Btn::new("Sil").danger().small()).clicked() {
        ui.ctx().data_mut(|d| d.insert_temp(Id::new("stash-act"), (s.name.clone(), "drop".to_string())));
    }
    let fy = right.top() + 100.0;
    let nf = r.stash_files.len().min(5);
    let frect = Rect::from_min_size(Pos2::new(right.left() + 14.0, fy), vec2(right.width() - 28.0, nf as f32 * 34.0));
    let mut fui = ui.new_child(UiBuilder::new().max_rect(frect).layout(Layout::top_down(Align::Min)));
    egui::ScrollArea::vertical().id_salt("stash-files").auto_shrink([false, false]).show(&mut fui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for f in &r.stash_files {
            let (rr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
            let sel = r.stash_diff.as_ref().map(|d| d.0 == f.path).unwrap_or(false);
            if sel {
                ui.painter().rect_filled(rr, cr(6), p.bg3);
            } else if resp.hovered() {
                ui.painter().rect_filled(rr, cr(6), p.bg1);
            }
            badge(ui.painter(), p, Rect::from_center_size(Pos2::new(rr.left() + 16.0, rr.center().y), vec2(20.0, 20.0)), f.status.chars().next().unwrap_or('M'));
            draw_trunc(ui.painter(), rr.left() + 36.0, rr.center().y, rr.width() - 50.0, &f.path, app.th.f(Ty::Row), p.text1);
            pointer(&resp);
            if resp.clicked() {
                *sel_file = Some(f.path.clone());
            }
        }
    });
    let box_rect = Rect::from_min_max(Pos2::new(right.left() + 20.0, fy + nf as f32 * 34.0 + 12.0), right.max - vec2(20.0, 16.0));
    if box_rect.height() > 60.0 {
        ui.painter().rect_filled(box_rect, cr(8), p.bg2);
        ui.painter().rect_stroke(box_rect, cr(8), Stroke::new(1.0, p.diff_head_line), StrokeKind::Inside);
        if let Some((file, fd)) = r.stash_diff.as_ref() {
            let mut dui = ui.new_child(UiBuilder::new().max_rect(box_rect.shrink(1.0)).layout(Layout::top_down(Align::Min)));
            dui.set_clip_rect(box_rect.shrink(1.0));
            crate::screens::history::readonly_diff(app, &mut dui, box_rect.shrink(1.0), fd, file);
        }
    }
}

fn oplog(app: &App, ui: &mut Ui, cmds: &mut Vec<Cmd>) {
    let p = &app.th.p;
    let full = ui.max_rect();
    let h = page_header(app, ui, full, "İşlem günlüğü", "Her yazma işlemi anlık görüntüyle saklanır; uzağa giden işlemler geri alınamaz.");
    let area = Rect::from_min_max(Pos2::new(full.left() + 24.0, full.top() + h), full.max - vec2(24.0, 8.0));
    if app.oplog.is_empty() {
        ui.painter().text_t(area.center(), egui::Align2::CENTER_CENTER, "Bu oturumda henüz işlem yok", app.th.f(Ty::Row), p.text3);
        return;
    }
    let mut aui = ui.new_child(UiBuilder::new().max_rect(area).layout(Layout::top_down(Align::Min)));
    let first_undoable = if app.can_undo() { Some(0) } else { None };
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut aui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for (i, o) in app.oplog.iter().enumerate() {
            let (rr, _) = ui.allocate_exact_size(vec2(ui.available_width().min(900.0), 46.0), Sense::hover());
            let top = Some(i) == first_undoable;
            if top {
                ui.painter().rect_filled(rr, cr(6), p.bg3);
            }
            let tm = crate::util::clock(o.at);
            draw_left(ui.painter(), rr.left() + 12.0, rr.center().y, &tm, app.th.f(Ty::MonoXs), p.text2);
            draw_trunc(ui.painter(), rr.left() + 72.0, rr.center().y - 8.0, rr.width() - 260.0, &o.title, app.th.f(Ty::Row), p.text1);
            draw_trunc(ui.painter(), rr.left() + 72.0, rr.center().y + 9.0, rr.width() - 260.0, &o.cmd, app.th.f(Ty::MonoXs), p.text3);
            match &o.undo {
                Undo::None(why) => {
                    draw_right(ui.painter(), rr.right() - 12.0, rr.center().y, if why.is_empty() { "geri alınamaz" } else { why }, app.th.f(Ty::Small), p.text3);
                }
                _ if top => {
                    let br = Rect::from_min_size(Pos2::new(rr.right() - 12.0 - 78.0, rr.center().y - 13.0), vec2(78.0, 26.0));
                    let mut bui = ui.new_child(UiBuilder::new().max_rect(br).layout(Layout::left_to_right(Align::Center)));
                    if button(&mut bui, &app.th, Btn::new("Geri al").primary().small().min_w(78.0)).clicked() {
                        cmds.push(Cmd::Undo);
                    }
                }
                _ => {
                    let reachable = app.oplog[..i].iter().all(|o| !matches!(o.undo, Undo::None(_)));
                    if reachable {
                        let br = Rect::from_min_size(Pos2::new(rr.right() - 12.0 - 96.0, rr.center().y - 13.0), vec2(96.0, 26.0));
                        let mut bui = ui.new_child(UiBuilder::new().max_rect(br).layout(Layout::left_to_right(Align::Center)));
                        if button(&mut bui, &app.th, Btn::new("Buraya dön").small().min_w(96.0)).clicked() {
                            cmds.push(Cmd::UndoTo(i - 1));
                        }
                    } else {
                        draw_right(ui.painter(), rr.right() - 12.0, rr.center().y, "araya geri alınamaz işlem girmiş", app.th.f(Ty::Small), p.text3);
                    }
                }
            }
        }
    });
}
