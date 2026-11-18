//! Conflict resolution: pick ours / theirs / both (or edit) per conflict, then mark the file resolved.

use crate::i18n::{PainterT};
use crate::app::*;
use crate::backend::repo;
use crate::diff::Seg;
use crate::icons::{self, Icon};
use crate::theme::{semibold, Ty};
use crate::ui::*;
use eframe::egui::{self, vec2, Align, Color32, FontId, Id, Layout, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder};

enum Act {
    File(String),
    Choose(usize, char),
    SetResult(usize, String),
    Save,
    Abort,
    Continue,
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let th = app.th.clone();
    let p = th.p.clone();
    let full = ui.max_rect();
    let Some(r) = app.repo.as_ref() else { return };
    let conflicts: Vec<String> = r.entries().iter().filter(|e| e.conflicted).map(|e| e.path.clone()).collect();
    let state = r.status.as_ref().map(|s| s.state.clone()).unwrap_or_default();
    let mut acts: Vec<Act> = vec![];
    let left = Rect::from_min_max(full.min, Pos2::new(full.left() + 300.0, full.bottom()));
    let main = Rect::from_min_max(Pos2::new(left.right(), full.top()), full.max);
    ui.painter().rect_filled(left, 0.0, p.bg1);
    vline(ui.painter(), left.right() - 0.5, left.top(), left.bottom(), p.line_soft);
    let painter = ui.painter().clone();
    draw_left(&painter, left.left() + 16.0, left.top() + 28.0, "Çatışmalar", FontId::new(15.0, semibold()), p.text1);
    draw_right(&painter, left.right() - 16.0, left.top() + 29.0, &format!("{} dosya", conflicts.len()), th.f(Ty::Small), p.text2);
    let mut y = left.top() + 52.0;
    for f in &conflicts {
        let rr = Rect::from_min_size(Pos2::new(left.left() + 6.0, y), vec2(left.width() - 13.0, 32.0));
        let resp = ui.interact(rr, Id::new(("cf", f)), Sense::click());
        let sel = r.conflict_file.as_deref() == Some(f.as_str());
        if sel {
            painter.rect_filled(rr, cr(6), p.bg_sel);
            painter.rect_stroke(rr, cr(6), Stroke::new(1.0, p.accent), StrokeKind::Inside);
        } else if resp.hovered() {
            painter.rect_filled(rr, cr(6), p.bg3);
        }
        badge(&painter, &p, Rect::from_center_size(Pos2::new(rr.left() + 18.0, rr.center().y), vec2(20.0, 20.0)), '!');
        draw_trunc(&painter, rr.left() + 36.0, rr.center().y, rr.width() - 44.0, f, th.f(Ty::Row), p.text1);
        pointer(&resp);
        if resp.clicked() {
            acts.push(Act::File(f.clone()));
        }
        y += 34.0;
    }
    if conflicts.is_empty() {
        let c = main.center();
        icons::paint_at(ui, c - vec2(0.0, 30.0), 28.0, Icon::Check, p.added);
        painter.text_t(c + vec2(0.0, 4.0), egui::Align2::CENTER_CENTER, "Tüm çatışmalar çözüldü", FontId::new(15.0, semibold()), p.text1);
        let br = Rect::from_center_size(c + vec2(0.0, 48.0), vec2(220.0, 34.0));
        let mut bui = ui.new_child(UiBuilder::new().max_rect(br).layout(Layout::left_to_right(Align::Center)));
        if !state.is_empty() && state != "none" && button(&mut bui, &th, Btn::new("İşleme devam et").primary().min_w(220.0).height(34.0)).clicked() {
            acts.push(Act::Continue);
        }
    } else {
        let head = Rect::from_min_size(main.min, vec2(main.width(), 56.0));
        painter.rect_filled(head, 0.0, p.bg1);
        hline(&painter, head.left(), head.right(), head.bottom() - 0.5, p.line_soft);
        let file = r.conflict_file.clone().unwrap_or_default();
        let (dir, name) = crate::util::split_path(&file);
        let x = draw_left(&painter, head.left() + 20.0, head.center().y, dir, th.f(Ty::Mono), p.text3);
        draw_left(&painter, x, head.center().y, name, th.f(Ty::Mono), p.text1);
        let total = r.conflict_segs.iter().filter(|s| matches!(s, Seg::Conflict(_))).count();
        let open = r.conflict_segs.iter().filter(|s| matches!(s, Seg::Conflict(c) if c.choice.is_none() && c.result.is_none())).count();
        let mut rx = head.right() - 20.0;
        let bw = 190.0;
        let br = Rect::from_min_size(Pos2::new(rx - bw, head.center().y - 16.0), vec2(bw, 32.0));
        let mut bui = ui.new_child(UiBuilder::new().max_rect(br).layout(Layout::left_to_right(Align::Center)));
        if button(&mut bui, &th, Btn::new("Çözüldü olarak işaretle").primary().min_w(bw).enabled(total > 0 && open == 0)).clicked() {
            acts.push(Act::Save);
        }
        rx = br.left() - 10.0;
        if !state.is_empty() && state != "none" {
            let ar = Rect::from_min_size(Pos2::new(rx - 110.0, head.center().y - 16.0), vec2(110.0, 32.0));
            let mut aui = ui.new_child(UiBuilder::new().max_rect(ar).layout(Layout::left_to_right(Align::Center)));
            if button(&mut aui, &th, Btn::new("İşlemi iptal et").danger().min_w(110.0)).clicked() {
                acts.push(Act::Abort);
            }
            rx = ar.left() - 12.0;
        }
        draw_right(&painter, rx, head.center().y, &format!("{open} / {total} çatışma bekliyor"), th.f(Ty::Small), if open == 0 { p.added } else { p.modified });

        let body = Rect::from_min_max(Pos2::new(main.left(), head.bottom()), main.max);
        let mut bui = ui.new_child(UiBuilder::new().max_rect(body.shrink2(vec2(16.0, 12.0))).layout(Layout::top_down(Align::Min)));
        let mono = th.f(Ty::Mono);
        let mw = ui.painter().layout_no_wrap("M".into(), mono.clone(), Color32::WHITE).size().x;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut bui, |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            let w = ui.available_width();
            let mut ci = 0usize;
            for (si, seg) in r.conflict_segs.iter().enumerate() {
                match seg {
                    Seg::Text(t) => {
                        let lines: Vec<&str> = t.lines().collect();
                        let show: Vec<String> = if lines.len() <= 6 { lines.iter().map(|s| s.to_string()).collect() } else {
                            let mut v: Vec<String> = lines[..2].iter().map(|s| s.to_string()).collect();
                            v.push(format!("⋯ {} satır değişmedi", lines.len() - 4));
                            v.extend(lines[lines.len() - 2..].iter().map(|s| s.to_string()));
                            v
                        };
                        let (rr, _) = ui.allocate_exact_size(vec2(w, show.len() as f32 * 20.0 + 8.0), Sense::hover());
                        for (i, l) in show.iter().enumerate() {
                            let is_gap = l.starts_with('⋯');
                            draw_trunc(ui.painter(), rr.left() + 12.0, rr.top() + 14.0 + i as f32 * 20.0, w - 24.0, &l.replace('\t', "    "), mono.clone(), if is_gap { p.text3 } else { p.text2 });
                        }
                    }
                    Seg::Conflict(c) => {
                        ci += 1;
                        let nl = |s: &str| s.lines().count().max(1);
                        let editing = c.choice == Some('m');
                        let h_lines = if editing { nl(c.result.as_deref().unwrap_or("")).max(3).min(16) } else { nl(&c.ours).max(nl(&c.theirs)).min(14) };
                        let col_h = h_lines as f32 * 20.0 + 16.0;
                        let card_h = 40.0 + col_h + 8.0;
                        let (rr, _) = ui.allocate_exact_size(vec2(w, card_h), Sense::hover());
                        let painter = ui.painter();
                        painter.rect_filled(rr, cr(10), p.bg1);
                        painter.rect_stroke(rr, cr(10), Stroke::new(1.0, if c.choice.is_some() { p.added.gamma_multiply(0.6) } else { p.accent_dim }), StrokeKind::Inside);
                        draw_left(painter, rr.left() + 14.0, rr.top() + 20.0, &format!("Çatışma {ci}"), FontId::new(13.0, semibold()), p.text1);
                        let mut bx = rr.right() - 12.0;
                        for (label, key) in [("Düzenle", 'm'), ("İkisi de", 'b'), ("Onlarınki", 't'), ("Bizimki", 'o')] {
                            let bw = text_width(ui, label, &th.f(Ty::Small)) + 22.0;
                            let br = Rect::from_min_size(Pos2::new(bx - bw, rr.top() + 8.0), vec2(bw, 24.0));
                            bx -= bw + 6.0;
                            let resp = ui.interact(br, Id::new(("choose", si, key)), Sense::click());
                            let on = c.choice == Some(key);
                            let painter = ui.painter();
                            painter.rect_filled(br, cr(6), if on { p.accent } else if resp.hovered() { p.bg_sel } else { p.bg3 });
                            painter.rect_stroke(br, cr(6), Stroke::new(1.0, if on { p.accent } else { p.line_strong }), StrokeKind::Inside);
                            centered(painter, br, label, if on { FontId::new(12.0, semibold()) } else { th.f(Ty::Small) }, if on { p.accent_fg } else { p.text1 });
                            pointer(&resp);
                            if resp.clicked() {
                                acts.push(Act::Choose(si, key));
                            }
                        }
                        if editing {
                            let er = Rect::from_min_size(Pos2::new(rr.left() + 14.0, rr.top() + 40.0), vec2(rr.width() - 28.0, col_h));
                            ui.painter().rect_filled(er, cr(6), p.bg0);
                            ui.painter().rect_stroke(er, cr(6), Stroke::new(1.0, p.accent), StrokeKind::Inside);
                            let mut txt = c.result.clone().unwrap_or_default();
                            let old = txt.clone();
                            let mut eui = ui.new_child(UiBuilder::new().max_rect(er.shrink2(vec2(10.0, 6.0))).layout(Layout::top_down(Align::Min)));
                            eui.add(egui::TextEdit::multiline(&mut txt).id(Id::new(("conf-edit", si))).font(mono.clone()).frame(false).desired_width(er.width() - 20.0).desired_rows(h_lines).text_color(p.text1));
                            if txt != old {
                                acts.push(Act::SetResult(si, txt));
                            }
                        } else {
                            let half = (rr.width() - 28.0 - 10.0) / 2.0;
                            for (k, (title, text, col)) in [("Bizim", &c.ours, p.added), ("Onların", &c.theirs, p.renamed)].into_iter().enumerate() {
                                let cr_ = Rect::from_min_size(Pos2::new(rr.left() + 14.0 + k as f32 * (half + 10.0), rr.top() + 40.0), vec2(half, col_h));
                                let painter = ui.painter();
                                painter.rect_filled(cr_, cr(6), p.tint_a(col, if p.dark { 0x14 } else { 0x10 }));
                                painter.rect_stroke(cr_, cr(6), Stroke::new(1.0, p.tint_a(col, 0x55)), StrokeKind::Inside);
                                let chosen = match c.choice { Some('o') => k == 0, Some('t') => k == 1, Some('b') => true, _ => false };
                                if chosen {
                                    painter.rect_filled(Rect::from_min_size(cr_.min, vec2(3.0, cr_.height())), 0.0, col);
                                }
                                let clip = painter.with_clip_rect(cr_.shrink(2.0));
                                for (i, l) in text.lines().take(14).enumerate() {
                                    let t = l.replace('\t', "    ");
                                    let maxc = ((half - 20.0) / mw) as usize;
                                    let shown: String = t.chars().take(maxc).collect();
                                    draw_left(&clip, cr_.left() + 10.0, cr_.top() + 18.0 + i as f32 * 20.0, &shown, mono.clone(), p.text1);
                                }
                                if text.is_empty() {
                                    draw_left(&clip, cr_.left() + 10.0, cr_.top() + 18.0, "(boş)", th.f(Ty::Mono), p.text3);
                                }
                                draw_right(painter, cr_.right() - 8.0, cr_.top() + 10.0, title, FontId::new(10.0, semibold()), col);
                            }
                        }
                    }
                }
            }
        });
    }
    for a in acts {
        match a {
            Act::File(f) => app.open_conflict_file(f),
            Act::Choose(si, k) => {
                if let Some(r) = app.repo.as_mut() {
                    if let Some(Seg::Conflict(c)) = r.conflict_segs.get_mut(si) {
                        c.choice = Some(k);
                        c.result = if k == 'm' { Some(format!("{}{}", c.ours, c.theirs)) } else { None };
                    }
                }
            }
            Act::SetResult(si, t) => {
                if let Some(r) = app.repo.as_mut() {
                    if let Some(Seg::Conflict(c)) = r.conflict_segs.get_mut(si) {
                        c.result = Some(t);
                        c.choice = Some('m');
                    }
                }
            }
            Act::Save => app.save_conflict_file(),
            Act::Abort => {
                let (path, kind) = (app.path().unwrap_or_default(), state.clone());
                app.confirm("İşlem iptal edilsin mi?", "Çözümlerin atılır ve depo işlem öncesi duruma döner.", "İşlemi iptal et", true, move |app| {
                    app.spawn_busy(move || repo::op_abort(path, kind), |app, r| {
                        match r {
                            Ok(()) => {
                                app.toast(ToastKind::Ok, "İşlem iptal edildi");
                                app.screen = Screen::Changes;
                                app.refresh_all();
                            }
                            Err(e) => app.toast(ToastKind::Err, e),
                        }
                    });
                });
            }
            Act::Continue => {
                let (path, kind) = (app.path().unwrap_or_default(), state.clone());
                app.spawn_busy(move || repo::op_continue(path, kind), |app, r| {
                    match r {
                        Ok(_) => {
                            app.toast(ToastKind::Ok, "İşlem tamamlandı");
                            app.screen = Screen::Changes;
                            app.refresh_all();
                        }
                        Err(e) => app.toast(ToastKind::Err, e),
                    }
                });
            }
        }
    }
}
