//! Changes screen: file list and commit box on the left, virtualised diff with hunk / line staging on the right.

use crate::i18n::{PainterT, RespT};
use crate::app::*;
use crate::backend::repo::Entry;
use crate::diff::LineKind;
use crate::highlight;
use crate::icons::{self, Icon};
use crate::theme::{semibold, Ty};
use crate::ui::*;
use eframe::egui::{self, vec2, Align, Color32, FontId, Id, Layout, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder};

const SIDE_W: f32 = 340.0;
const LINE_H: f32 = 22.0;
const HEAD_H: f32 = 32.0;

enum Act {
    Select(String),
    ToggleFile(Entry),
    Filter(usize),
    LineToggle(Block, usize, bool),
    StageSel(Block),
    StageHunk(Block),
    DiscardHunk(Block),
    StageWhole(Entry),
    FileMenu(Entry, Pos2),
    More(Pos2),
    Cmd(Cmd),
    AddCoauthor,
    SetSplit(bool),
    FullCtx,
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let full = ui.max_rect();
    let p = app.th.p.clone();
    let left = Rect::from_min_max(full.min, Pos2::new(full.left() + SIDE_W, full.bottom()));
    let main = Rect::from_min_max(Pos2::new(left.right(), full.top()), full.max);
    ui.painter().rect_filled(left, 0.0, p.bg1);
    vline(ui.painter(), left.right() - 0.5, left.top(), left.bottom(), p.line_soft);
    let mut acts: Vec<Act> = vec![];
    {
        let mut lui = ui.new_child(UiBuilder::new().max_rect(left).layout(Layout::top_down(Align::Min)));
        side(app, &mut lui, left, &mut acts);
        let mut mui = ui.new_child(UiBuilder::new().max_rect(main).layout(Layout::top_down(Align::Min)));
        main_view(app, &mut mui, main, &mut acts);
    }
    for a in acts {
        match a {
            Act::Select(f) => app.select_file(f),
            Act::ToggleFile(e) => app.toggle_file(&e),
            Act::Filter(f) => {
                if let Some(r) = app.repo.as_mut() {
                    r.filter = f;
                }
            }
            Act::LineToggle(b, i, shift) => line_toggle(app, b, i, shift),
            Act::StageSel(b) => {
                let lines: Vec<usize> = app.repo.as_ref().map(|r| r.line_sel.iter().filter(|(s, h, _)| *s == b.staged && *h == b.hunk).map(|(_, _, l)| *l).collect()).unwrap_or_default();
                if !lines.is_empty() {
                    app.apply_hunk_lines(b, lines);
                }
            }
            Act::StageHunk(b) => app.apply_hunk(b),
            Act::DiscardHunk(b) => app.discard_hunk(b),
            Act::StageWhole(e) => app.toggle_file(&e),
            Act::FileMenu(e, pos) => {
                let (st, un) = staged_flags(&e);
                let mk = |l: &str, i: Icon, c: Cmd, d: bool| MenuItem { label: l.into(), icon: Some(i), kbd: None, cmd: c, danger: d, enabled: true };
                let mut items = vec![];
                items.push(mk(if st && !un { "Hazırlıktan çıkar" } else { "Hazırla" }, Icon::Check, Cmd::StageFile(e.path.clone()), false));
                items.push(mk("Değişiklikleri at…", Icon::Trash, Cmd::DiscardFile(e.path.clone()), true));
                items.push(mk("Yolu kopyala", Icon::Copy, Cmd::CopyPath(e.path.clone()), false));
                items.push(mk("Dosya Gezgini'nde göster", Icon::Folder, Cmd::Reveal, false));
                app.menu = Some(Menu { pos, right_align: false, items });
            }
            Act::More(pos) => {
                let sel = app.repo.as_ref().and_then(|r| r.sel_file.clone()).unwrap_or_default();
                let mk = |l: &str, i: Icon, c: Cmd, d: bool| MenuItem { label: l.into(), icon: Some(i), kbd: None, cmd: c, danger: d, enabled: true };
                app.menu = Some(Menu {
                    pos,
                    right_align: true,
                    items: vec![mk("Değişiklikleri at…", Icon::Trash, Cmd::DiscardFile(sel.clone()), true), mk("Yolu kopyala", Icon::Copy, Cmd::CopyPath(sel), false), mk("Editörde aç", Icon::Edit, Cmd::OpenEditor, false), mk("Dosya Gezgini'nde göster", Icon::Folder, Cmd::Reveal, false)],
                });
            }
            Act::Cmd(c) => app.run(c),
            Act::SetSplit(v) => {
                app.s.split_diff = v;
                app.save_settings();
            }
            Act::FullCtx => {
                if let Some(r) = app.repo.as_mut() {
                    r.full_ctx = !r.full_ctx;
                }
                app.load_file_view();
            }
            Act::AddCoauthor => {
                if let Some(r) = app.repo.as_mut() {
                    r.coauthor_open = !r.coauthor_open;
                }
            }
        }
    }
}

fn line_toggle(app: &mut App, b: Block, i: usize, shift: bool) {
    let Some(r) = app.repo.as_mut() else { return };
    // already staged line → unstage that line right away
    if b.staged {
        app.apply_hunk_lines(b, vec![i]);
        return;
    }
    let key = (b.staged, b.hunk, i);
    if shift {
        if let (Some((ab, ai)), Some(fv)) = (r.line_anchor, r.file_view.as_ref()) {
            if ab == b {
                let (lo, hi) = (ai.min(i), ai.max(i));
                let h = fv.hunk(b);
                for k in lo..=hi {
                    if h.lines.get(k).map(|l| l.kind != LineKind::Ctx).unwrap_or(false) {
                        r.line_sel.insert((b.staged, b.hunk, k));
                    }
                }
                return;
            }
        }
    }
    if !r.line_sel.remove(&key) {
        r.line_sel.insert(key);
    }
    r.line_anchor = Some((b, i));
}

fn side(app: &App, ui: &mut Ui, rect: Rect, acts: &mut Vec<Act>) {
    let p = &app.th.p;
    let r = app.repo.as_ref().unwrap();
    let entries: Vec<&Entry> = r.entries().iter().filter(|e| !e.conflicted).collect();
    let conflicts = r.entries().iter().filter(|e| e.conflicted).count();
    let n_all = entries.len();
    let n_staged = entries.iter().filter(|e| staged_flags(e).0).count();
    let n_partial = entries.iter().filter(|e| { let (s, u) = staged_flags(e); s && u && e.kind != "untracked" }).count();
    let n_untracked = entries.iter().filter(|e| e.kind == "untracked").count();
    let painter = ui.painter().clone();

    let head_h = 14.0 + 20.0 + 10.0 + 4.0 + 10.0 + 24.0 + 10.0;
    let head = Rect::from_min_size(rect.min, vec2(SIDE_W - 1.0, head_h));
    let x0 = head.left() + 14.0;
    let x1 = head.right() - 14.0;
    let cy = head.top() + 14.0 + 10.0;
    draw_left(&painter, x0, cy, "Değişiklikler", FontId::new(15.0, semibold()), p.text1);
    let sum = if n_all == 0 { "değişiklik yok".to_string() } else {
        let mut s = format!("{n_all} dosya · {n_staged} hazır");
        if n_partial > 0 { s.push_str(&format!(" · {n_partial} kısmen")); }
        s
    };
    draw_right(&painter, x1, cy + 1.0, &sum, app.th.f(Ty::Small), p.text2);
    let by = head.top() + 14.0 + 20.0 + 10.0;
    let bar = Rect::from_min_size(Pos2::new(x0, by), vec2(x1 - x0, 4.0));
    painter.rect_filled(bar, cr(2), p.line_soft);
    if n_all > 0 {
        let full_n = n_staged.saturating_sub(n_partial) as f32;
        let w1 = bar.width() * full_n / n_all as f32;
        let w2 = bar.width() * n_partial as f32 / n_all as f32;
        if w1 > 0.5 { painter.rect_filled(Rect::from_min_size(bar.min, vec2(w1, 4.0)), cr(2), p.accent); }
        if w2 > 0.5 { painter.rect_filled(Rect::from_min_size(bar.min + vec2(w1 + if w1 > 0.5 { 2.0 } else { 0.0 }, 0.0), vec2(w2, 4.0)), cr(2), p.accent_dim); }
    }
    let pills = Rect::from_min_size(Pos2::new(x0, by + 4.0 + 10.0), vec2(x1 - x0, 24.0));
    let mut pui = ui.new_child(UiBuilder::new().max_rect(pills).layout(Layout::left_to_right(Align::Center)));
    pui.spacing_mut().item_spacing.x = 6.0;
    for (i, (label, n)) in [("Tümü", n_all), ("Hazır", n_staged), ("Takipsiz", n_untracked)].into_iter().enumerate() {
        if pill(&mut pui, &app.th, &format!("{label} {n}"), r.filter == i).clicked() {
            acts.push(Act::Filter(i));
        }
    }
    if conflicts > 0 {
        let b = Btn::new(format!("{conflicts} çatışma")).icon(Icon::Alert).danger().tiny();
        if button(&mut pui, &app.th, b).clicked() {
            acts.push(Act::Cmd(Cmd::Goto(Screen::Conflict)));
        }
    }

    let co_h = if r.coauthor_open { 44.0 } else { 0.0 };
    let box_h = 14.0 + 34.0 + 10.0 + 70.0 + 10.0 + 26.0 + co_h + 10.0 + 38.0 + 14.0;
    let box_rect = Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - box_h), Pos2::new(rect.right() - 1.0, rect.bottom()));
    let list_rect = Rect::from_min_max(Pos2::new(rect.left() + 6.0, head.bottom()), Pos2::new(rect.right() - 7.0, box_rect.top()));

    let visible: Vec<&Entry> = entries
        .iter()
        .copied()
        .filter(|e| match r.filter {
            1 => staged_flags(e).0,
            2 => e.kind == "untracked",
            _ => true,
        })
        .collect();
    let mut lui = ui.new_child(UiBuilder::new().max_rect(list_rect).layout(Layout::top_down(Align::Min)));
    if visible.is_empty() {
        empty_list(app, &mut lui, list_rect, n_all == 0);
    } else {
        lui.spacing_mut().item_spacing.y = 2.0;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(&mut lui, app.th.row_h(), visible.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for i in range {
                file_row(app, ui, visible[i], acts);
            }
        });
    }

    hline(&painter, box_rect.left(), box_rect.right(), box_rect.top() + 0.5, p.line_soft);
    let bx = Rect::from_min_max(box_rect.min + vec2(14.0, 14.0), box_rect.max - vec2(14.0, 14.0));
    let mut cui = ui.new_child(UiBuilder::new().max_rect(bx).layout(Layout::top_down(Align::Min)));
    commit_box(app, &mut cui, acts);
}

fn empty_list(app: &App, ui: &mut Ui, rect: Rect, clean: bool) {
    let p = &app.th.p;
    let c = rect.center() - vec2(0.0, 10.0);
    icons::paint_at(ui, c - vec2(0.0, 22.0), 26.0, if clean { Icon::Check } else { Icon::Search }, p.text3);
    ui.painter().text_t(c + vec2(0.0, 6.0), egui::Align2::CENTER_CENTER, if clean { "Çalışma alanı temiz" } else { "Bu filtrede dosya yok" }, FontId::new(13.0, semibold()), p.text1);
    if clean {
        ui.painter().text_t(c + vec2(0.0, 26.0), egui::Align2::CENTER_CENTER, "Bir dosyayı değiştirince burada görünür", app.th.f(Ty::Small), p.text3);
    }
}

fn file_row(app: &App, ui: &mut Ui, e: &Entry, acts: &mut Vec<Act>) {
    let p = &app.th.p;
    let r = app.repo.as_ref().unwrap();
    let h = app.th.row_h();
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let selected = r.sel_file.as_deref() == Some(e.path.as_str());
    let painter = ui.painter_at(rect);
    if selected {
        painter.rect_filled(rect, cr(6), p.bg_sel);
        painter.rect_stroke(rect, cr(6), Stroke::new(1.0, p.accent), StrokeKind::Inside);
    } else if resp.hovered() {
        painter.rect_filled(rect, cr(6), p.bg3);
    }
    let (st, un) = staged_flags(e);
    let state = if st && !un { Cb::On } else if st && un && e.kind != "untracked" { Cb::Partial } else { Cb::Off };
    let cb = Rect::from_center_size(Pos2::new(rect.left() + 8.0 + 7.5, rect.center().y), vec2(15.0, 15.0));
    let cb_hit = ui.interact(cb.expand(5.0), Id::new(("cb", &e.path)), Sense::click());
    paint_checkbox(ui, &app.th, cb, state, cb_hit.hovered());
    pointer(&cb_hit);
    let letter = letter_of(e);
    let bd = Rect::from_center_size(Pos2::new(cb.right() + 8.0 + 10.0, rect.center().y), vec2(20.0, 20.0));
    badge(&painter, p, bd, letter);
    let mut right = rect.right() - 8.0;
    let stats = r.numstat.get(&e.path).copied();
    if e.kind == "untracked" {
        right = draw_right(&painter, right, rect.center().y, "yeni", app.th.f(Ty::Small), p.text2) - 8.0;
    } else if let Some((a, d)) = stats {
        if d > 0 { right = draw_right(&painter, right, rect.center().y, &format!("−{d}"), app.th.f(Ty::MonoXs), p.deleted) - 6.0; }
        if a > 0 { right = draw_right(&painter, right, rect.center().y, &format!("+{a}"), app.th.f(Ty::MonoXs), p.added) - 6.0; }
    }
    let x = bd.right() + 8.0;
    let (dir, name) = crate::util::split_path(&e.path);
    let deleted = e.kind == "deleted";
    let name_c = if deleted { p.text2 } else { p.text1 };
    let avail = (right - x).max(20.0);
    let nw = text_width(ui, name, &app.th.f(Ty::Row)).min(avail);
    let dir_w = (avail - nw).max(0.0);
    let mut x2 = x;
    if !dir.is_empty() && dir_w > 12.0 {
        x2 = draw_trunc(&painter, x, rect.center().y, dir_w, dir, app.th.f(Ty::Row), p.text3);
    }
    let end = draw_trunc(&painter, x2, rect.center().y, right - x2, name, app.th.f(Ty::Row), name_c);
    if deleted {
        painter.line_segment([Pos2::new(x, rect.center().y), Pos2::new(end, rect.center().y)], Stroke::new(1.0, p.code_gutter));
    }
    pointer(&resp);
    if cb_hit.clicked() {
        acts.push(Act::ToggleFile(e.clone()));
    } else if resp.clicked() {
        acts.push(Act::Select(e.path.clone()));
    }
    if resp.secondary_clicked() {
        let pos = resp.interact_pointer_pos().unwrap_or(rect.center());
        acts.push(Act::Select(e.path.clone()));
        acts.push(Act::FileMenu(e.clone(), pos));
    }
}

fn commit_box(app: &App, ui: &mut Ui, acts: &mut Vec<Act>) {
    let p = &app.th.p;
    let r = app.repo.as_ref().unwrap();
    let w = ui.available_width();
    ui.spacing_mut().item_spacing.y = 10.0;
    let id_sum = Id::new("commit-summary");
    let mut sum = ctx_buf(ui, "commit-sum-buf", &r.summary);
    let n = sum.chars().count();
    let counter = format!("{n}/72");
    let over = n > 72;
    text_input(ui, &app.th, &mut sum, InputOpts::new(id_sum, "Commit özeti").width(w).trailing(counter).warn(over));
    let mut body = ctx_buf(ui, "commit-body-buf", &r.body);
    text_input(ui, &app.th, &mut body, InputOpts::new(Id::new("commit-body"), "Açıklama (isteğe bağlı)").width(w).rows(3));
    set_bufs(ui, sum, body, (&r.summary, &r.body));

    if r.coauthor_open {
        let mut co = ctx_buf(ui, "commit-co-buf", &r.coauthor);
        text_input(ui, &app.th, &mut co, InputOpts::new(Id::new("commit-co"), "Ad Soyad <e-posta>").width(w).mono());
        if co != r.coauthor {
            ui.ctx().data_mut(|d| d.insert_temp(Id::new("commit-co-val"), co));
        }
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let label = if r.coauthor_open { "− Yardımcı yazar" } else { "+ Yardımcı yazar ekle" };
        if button(ui, &app.th, Btn::new(label).ghost().small().height(26.0)).clicked() {
            acts.push(Act::AddCoauthor);
        }
        let amend = button(ui, &app.th, Btn::new("Düzelt").ghost().height(26.0).selected(r.amend));
        if amend.clicked() {
            acts.push(Act::Cmd(Cmd::ToggleAmend));
        }
        let who = if app.git_user.0.is_empty() { String::new() } else { format!("{} · {}", app.git_user.0, app.git_user.1) };
        let rest = ui.available_width();
        if rest > 40.0 {
            let (rr, _) = ui.allocate_exact_size(vec2(rest, 26.0), Sense::hover());
            let tw = text_width(ui, &who, &app.th.f(Ty::Small)).min(rr.width());
            draw_trunc(ui.painter(), rr.right() - tw, rr.center().y, tw + 1.0, &who, app.th.f(Ty::Small), p.text2);
        }
    });

    let (files, hunks) = r.staged_counts;
    let staged = r.entries().iter().filter(|e| staged_flags(e).0).count();
    let can = (staged > 0 || r.amend) && !r.summary.trim().is_empty() && app.busy == 0;
    let text = if r.amend {
        "Commit'i düzelt".to_string()
    } else if staged == 0 {
        "Commit · hazır dosya yok".to_string()
    } else {
        format!("Commit · {} dosya{}", staged.max(files), if hunks > 0 { format!(", {hunks} hunk") } else { String::new() })
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 38.0), Sense::click());
    let hov = resp.hovered() && can;
    let bg = if !can { p.bg3 } else if resp.is_pointer_button_down_on() { p.accent_line } else if hov { p.accent_hi } else { p.accent };
    let fg = if can { p.accent_fg } else { p.text3 };
    let painter = ui.painter();
    painter.rect_filled(rect, cr(6), bg);
    let font = FontId::new(13.0, semibold());
    let kbd = "Ctrl ↵";
    let kf = app.th.f(Ty::MonoXs);
    let tw = text_width(ui, &text, &font);
    let kw = text_width(ui, kbd, &kf) + 10.0;
    let total = tw + 8.0 + kw;
    let x = rect.center().x - total / 2.0;
    draw_left(painter, x, rect.center().y, &text, font, fg);
    let kr = Rect::from_min_size(Pos2::new(x + tw + 8.0, rect.center().y - 8.0), vec2(kw, 16.0));
    painter.rect_stroke(kr, cr(4), Stroke::new(1.0, if can { Color32::from_black_alpha(100) } else { p.line_strong }), StrokeKind::Inside);
    centered(painter, kr, kbd, kf, fg);
    if can {
        pointer(&resp);
    }
    if resp.clicked() {
        acts.push(Act::Cmd(Cmd::Commit));
    }
}

/// Text buffers for the commit fields live in egui memory while editing, and are mirrored to `Repo`
/// at the end of the frame by `sync_commit_fields` (the render pass only borrows `App` immutably).
fn ctx_buf(ui: &Ui, key: &str, current: &str) -> String {
    ui.ctx().data_mut(|d| d.get_temp::<String>(Id::new(key))).unwrap_or_else(|| current.to_string())
}

fn set_bufs(ui: &Ui, sum: String, body: String, cur: (&str, &str)) {
    ui.ctx().data_mut(|d| {
        d.insert_temp(Id::new("commit-sum-buf"), sum.clone());
        d.insert_temp(Id::new("commit-body-buf"), body.clone());
        // only user edits travel back; programmatic changes (amend prefill, clear after commit) must win
        if sum != cur.0 {
            d.insert_temp(Id::new("commit-sum-val"), sum);
        }
        if body != cur.1 {
            d.insert_temp(Id::new("commit-body-val"), body);
        }
    });
}

/// Copy the edited commit fields from egui memory back into the repo state.
pub fn sync_commit_fields(app: &mut App, ctx: &egui::Context) {
    let Some(r) = app.repo.as_mut() else { return };
    let (s, b, c) = ctx.data_mut(|d| (d.remove_temp::<String>(Id::new("commit-sum-val")), d.remove_temp::<String>(Id::new("commit-body-val")), d.remove_temp::<String>(Id::new("commit-co-val"))));
    if let Some(s) = s {
        r.summary = s;
    }
    if let Some(b) = b {
        r.body = b;
    }
    if let Some(c) = c {
        r.coauthor = c;
    }
    // programmatic changes (commit cleared, amend prefill) must reach the buffers too
    ctx.data_mut(|d| {
        d.insert_temp(Id::new("commit-sum-buf"), r.summary.clone());
        d.insert_temp(Id::new("commit-body-buf"), r.body.clone());
        d.insert_temp(Id::new("commit-co-buf"), r.coauthor.clone());
    });
}

fn main_view(app: &App, ui: &mut Ui, rect: Rect, acts: &mut Vec<Act>) {
    let p = &app.th.p;
    let r = app.repo.as_ref().unwrap();
    let head = Rect::from_min_size(rect.min, vec2(rect.width(), 48.0));
    ui.painter().rect_filled(head, 0.0, p.bg1);
    hline(ui.painter(), head.left(), head.right(), head.bottom() - 0.5, p.line_soft);
    let Some(file) = r.sel_file.clone() else {
        let c = rect.center();
        icons::paint_at(ui, c - vec2(0.0, 24.0), 28.0, Icon::Diff, p.text3);
        ui.painter().text_t(c + vec2(0.0, 6.0), egui::Align2::CENTER_CENTER, "Fark görmek için bir dosya seç", FontId::new(14.0, semibold()), p.text1);
        ui.painter().text_t(c + vec2(0.0, 28.0), egui::Align2::CENTER_CENTER, "J / K ile dosyalar arasında gez", app.th.f(Ty::Small), p.text3);
        return;
    };
    let entry = r.entries().iter().find(|e| e.path == file).cloned();
    let painter = ui.painter().clone();
    let cy = head.center().y;
    let mut x = head.left() + 16.0;
    if let Some(e) = &entry {
        badge(&painter, p, Rect::from_center_size(Pos2::new(x + 10.0, cy), vec2(20.0, 20.0)), letter_of(e));
    }
    x += 20.0 + 12.0;
    let (dir, name) = crate::util::split_path(&file);
    let x1 = draw_left(&painter, x, cy, dir, app.th.f(Ty::Mono), p.text3);
    let x1 = draw_left(&painter, x1, cy, name, app.th.f(Ty::Mono), p.text1) + 12.0;
    if let Some(fv) = r.file_view.as_ref() {
        let a = fv.staged.additions + fv.unstaged.additions;
        let d = fv.staged.deletions + fv.unstaged.deletions;
        let x2 = draw_left(&painter, x1, cy, &format!("+{a}"), app.th.f(Ty::MonoSm), p.added) + 8.0;
        draw_left(&painter, x2, cy, &format!("−{d}"), app.th.f(Ty::MonoSm), p.deleted);
    }
    let mut rx = head.right() - 16.0;
    let more = Rect::from_min_size(Pos2::new(rx - 28.0, cy - 14.0), vec2(28.0, 28.0));
    let mr = ui.interact(more, Id::new("diff-more"), Sense::click());
    if mr.hovered() { painter.rect_filled(more, cr(6), p.bg3); }
    icons::paint_at(ui, more.center(), 16.0, Icon::More, if mr.hovered() { p.text1 } else { p.text2 });
    pointer(&mr);
    if mr.clicked() {
        acts.push(Act::More(Pos2::new(more.right(), more.bottom() + 4.0)));
    }
    rx = more.left() - 8.0;
    if let Some(e) = &entry {
        let (st, un) = staged_flags(e);
        let label = if st && !un { "Dosyayı hazırlıktan çıkar" } else { "Tüm dosyayı hazırla" };
        let w = text_width(ui, label, &FontId::new(12.0, semibold())) + 24.0;
        let br = Rect::from_min_size(Pos2::new(rx - w, cy - 14.0), vec2(w, 28.0));
        let mut bui = ui.new_child(UiBuilder::new().max_rect(br).layout(Layout::left_to_right(Align::Center)));
        if button(&mut bui, &app.th, Btn::new(label).small()).clicked() {
            acts.push(Act::StageWhole(e.clone()));
        }
        rx = br.left() - 10.0;
    }
    {
        let label = "Tüm dosya";
        let w = text_width(ui, label, &FontId::new(12.0, semibold())) + 24.0;
        let br = Rect::from_min_size(Pos2::new(rx - w, cy - 14.0), vec2(w, 28.0));
        let mut bui = ui.new_child(UiBuilder::new().max_rect(br).layout(Layout::left_to_right(Align::Center)));
        if button(&mut bui, &app.th, Btn::new(label).small().selected(r.full_ctx)).on_hover_text_t("Dosyanın tamamını bağlam olarak göster").clicked() {
            acts.push(Act::FullCtx);
        }
        rx = br.left() - 10.0;
    }
    {
        let labels = ["Satır içi", "Yan yana"];
        let ws: Vec<f32> = labels.iter().map(|l| text_width(ui, l, &FontId::new(12.0, semibold())) + 24.0).collect();
        let total: f32 = ws.iter().sum();
        let seg = Rect::from_min_size(Pos2::new(rx - total, cy - 14.0), vec2(total, 28.0));
        painter.rect_stroke(seg, cr(6), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        let mut x = seg.left();
        for (i, l) in labels.iter().enumerate() {
            let r_ = Rect::from_min_size(Pos2::new(x, seg.top()), vec2(ws[i], 28.0));
            let sel = app.s.split_diff == (i == 1);
            let resp = ui.interact(r_, Id::new(("seg", i)), Sense::click());
            if sel {
                painter.rect_filled(r_.shrink(1.0), cr(5), Color32::from_rgb(0x3A, 0x37, 0x31).lerp_to_gamma(p.bg3, if p.dark { 0.0 } else { 1.0 }));
            } else if resp.hovered() {
                painter.rect_filled(r_.shrink(1.0), cr(5), p.bg3);
            }
            centered(&painter, r_, l, if sel { FontId::new(12.0, semibold()) } else { app.th.f(Ty::Small) }, if sel { p.text1 } else { p.text2 });
            pointer(&resp);
            if resp.clicked() {
                acts.push(Act::SetSplit(i == 1));
            }
            x += ws[i];
        }
    }

    let body = Rect::from_min_max(Pos2::new(rect.left(), head.bottom()), rect.max);
    let Some(fv) = r.file_view.as_ref() else {
        spinner_row(ui, app, body);
        return;
    };
    if let Some(e) = &fv.error {
        ui.painter().text_t(body.center(), egui::Align2::CENTER_CENTER, e, app.th.f(Ty::Row), p.deleted);
        return;
    }
    let binary = fv.unstaged.binary || fv.staged.binary;
    if binary || fv.blocks.is_empty() {
        let c = body.center();
        let msg = if binary { "İkili dosya — fark gösterilemiyor" } else if fv.entry.kind == "renamed" { "Dosya yeniden adlandırıldı, içerik değişmedi" } else { "Gösterilecek metin değişikliği yok" };
        ui.painter().text_t(c, egui::Align2::CENTER_CENTER, msg, app.th.f(Ty::Row), p.text2);
        return;
    }
    let mut dui = ui.new_child(UiBuilder::new().max_rect(body).layout(Layout::top_down(Align::Min)));
    diff_view(app, &mut dui, fv, acts);
}

fn spinner_row(ui: &Ui, app: &App, body: Rect) {
    let t = ui.input(|i| i.time) as f32;
    let c = body.center();
    for k in 0..3 {
        let a = ((t * 3.0 - k as f32 * 0.6).sin() * 0.5 + 0.5).clamp(0.2, 1.0);
        ui.painter().circle_filled(c + vec2((k as f32 - 1.0) * 14.0, 0.0), 3.5, app.th.p.accent.gamma_multiply(a));
    }
    ui.ctx().request_repaint();
}

fn diff_view(app: &App, ui: &mut Ui, fv: &FileView, acts: &mut Vec<Act>) {
    let p = &app.th.p;
    let r = app.repo.as_ref().unwrap();
    let lang = highlight::lang_of(&fv.path);
    let mono = app.th.f(Ty::Mono);
    let mw = ui.painter().layout_no_wrap("M".into(), mono.clone(), Color32::WHITE).size().x;
    // row table: (y, kind)
    #[derive(Clone, Copy)]
    enum RK {
        Head(usize),
        Line(usize, usize),
        Pair(usize, Option<usize>, Option<usize>),
    }
    let split = app.s.split_diff;
    let mut rows: Vec<(f32, RK)> = Vec::new();
    let mut y = 8.0;
    let mut max_chars = 40usize;
    for (bi, b) in fv.blocks.iter().enumerate() {
        rows.push((y, RK::Head(bi)));
        y += HEAD_H;
        let h = fv.hunk(*b);
        if split {
            let ls = &h.lines;
            let mut i = 0;
            while i < ls.len() {
                if ls[i].kind == LineKind::Ctx {
                    rows.push((y, RK::Pair(bi, Some(i), Some(i))));
                    y += LINE_H;
                    i += 1;
                } else {
                    let ds = i;
                    while i < ls.len() && ls[i].kind == LineKind::Del {
                        i += 1;
                    }
                    let de = i;
                    while i < ls.len() && ls[i].kind == LineKind::Add {
                        i += 1;
                    }
                    let ae = i;
                    for k in 0..(de - ds).max(ae - de) {
                        let l = if ds + k < de { Some(ds + k) } else { None };
                        let rr = if de + k < ae { Some(de + k) } else { None };
                        rows.push((y, RK::Pair(bi, l, rr)));
                        y += LINE_H;
                    }
                }
            }
        } else {
            for (li, l) in h.lines.iter().enumerate() {
                rows.push((y, RK::Line(bi, li)));
                y += LINE_H;
                max_chars = max_chars.max(l.text.chars().count() + l.text.matches('\t').count() * 3);
            }
        }
        y += 8.0;
    }
    let total_h = y + 40.0;
    let gutter = 46.0 * 2.0 + 22.0 + 20.0;
    let content_w = if split { 0.0 } else { gutter + max_chars as f32 * mw + 40.0 };
    let mut sa = egui::ScrollArea::both().auto_shrink([false, false]).id_salt(("diff", &fv.path));
    if let Some(h) = r.focus_hunk {
        if r.scroll_req {
            if let Some((y, _)) = rows.iter().find(|(_, k)| matches!(k, RK::Head(i) if *i == h)) {
                sa = sa.vertical_scroll_offset((*y - 8.0).max(0.0));
            }
        }
    }
    sa.show_viewport(ui, |ui, vp| {
        let w = content_w.max(vp.width());
        ui.set_min_size(vec2(w, total_h));
        let origin = ui.min_rect().min;
        let first = rows.partition_point(|(ry, k)| ry + if matches!(k, RK::Head(_)) { HEAD_H } else { LINE_H } < vp.top());
        let painter = ui.painter().clone();
        let paint_line = |ui: &Ui, acts: &mut Vec<Act>, rr: Rect, bi: usize, li: usize, mode: u8| {
            let painter = ui.painter().clone();
            let b = fv.blocks[bi];
            let h = fv.hunk(b);
            let l = &h.lines[li];
                                let bg = match l.kind {
                LineKind::Add => p.diff_add,
                LineKind::Del => p.diff_del,
                LineKind::Ctx => Color32::TRANSPARENT,
            };
            if bg != Color32::TRANSPARENT {
                painter.rect_filled(rr, 0.0, bg);
            }
            let cy = rr.center().y;
            let gcol = p.code_gutter;
            let old_c = if l.kind == LineKind::Del { gcol.lerp_to_gamma(p.deleted, 0.45) } else { gcol };
            let new_c = if l.kind == LineKind::Add { gcol.lerp_to_gamma(p.added, 0.45) } else { gcol };
            if mode != 2 {
                if let Some(n) = l.old_no {
                    draw_right(&painter, rr.left() + 46.0 - 10.0, cy, &n.to_string(), mono.clone(), old_c);
                }
            }
            if mode != 1 {
                if let Some(n) = l.new_no {
                    draw_right(&painter, rr.left() + if mode == 0 { 92.0 } else { 46.0 } - 10.0, cy, &n.to_string(), mono.clone(), new_c);
                }
            }
            let base = if mode == 0 { 92.0 } else { 46.0 };
            if l.kind != LineKind::Ctx {
                let cbr = Rect::from_center_size(Pos2::new(rr.left() + base + 11.0, cy), vec2(12.0, 12.0));
                let on = b.staged || r.line_sel.contains(&(b.staged, b.hunk, li));
                let hit = ui.interact(cbr.expand(5.0), Id::new(("lcb", bi, li)), Sense::click());
                if on {
                    painter.rect_filled(cbr, cr(3), p.accent);
                    icons::paint_at(ui, cbr.center(), 9.0, Icon::Check, p.accent_fg);
                } else {
                    painter.rect_stroke(cbr, cr(3), Stroke::new(1.5, if hit.hovered() { p.text2 } else { Color32::from_rgb(0x6A, 0x65, 0x5B) }), StrokeKind::Inside);
                }
                pointer(&hit);
                if hit.clicked() {
                    let shift = ui.input(|i| i.modifiers.shift);
                    acts.push(Act::LineToggle(b, li, shift));
                }
                let (m, mc) = if l.kind == LineKind::Add { ("+", p.added) } else { ("−", p.deleted) };
                painter.text_t(Pos2::new(rr.left() + base + 22.0 + 10.0, cy), egui::Align2::CENTER_CENTER, m, mono.clone(), mc);
            }
            let cx = rr.left() + base + 42.0;
            if let Some((s, e)) = fv.intra.get(&(bi, li)).filter(|_| mode == 0) {
                let rect = Rect::from_min_max(Pos2::new(cx + *s as f32 * mw, rr.top() + 2.0), Pos2::new(cx + *e as f32 * mw, rr.bottom() - 2.0));
                painter.rect_filled(rect, cr(2), if l.kind == LineKind::Add { p.diff_add_word } else { p.tint_a(p.deleted, 0x55) });
            }
            let text = l.text.replace('\t', "    ");
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = f32::INFINITY;
            for (t, k) in highlight::tokenize(&text, lang) {
                job.append(&t, 0.0, egui::TextFormat { font_id: mono.clone(), color: highlight::color(p, k), ..Default::default() });
            }
            if text.is_empty() {
                job.append(" ", 0.0, egui::TextFormat { font_id: mono.clone(), color: p.text1, ..Default::default() });
            }
            let g = painter.layout_job(job);
            painter.galley(Pos2::new(cx, cy - g.size().y / 2.0), g, p.text1);
            if l.no_newline {
                draw_left(&painter, cx + (text.chars().count() as f32 + 2.0) * mw, cy, "⏎ satır sonu yok", app.th.f(Ty::MonoXs), p.text3);
            }

        };
        for (ry, k) in rows.iter().skip(first) {
            if *ry > vp.bottom() {
                break;
            }
            match *k {
                RK::Head(bi) => {
                    let b = fv.blocks[bi];
                    let h = fv.hunk(b);
                    // sticky horizontally: keep the header at the viewport's left edge
                    let rr = Rect::from_min_size(Pos2::new(origin.x + vp.left(), origin.y + ry), vec2(vp.width().max(300.0), HEAD_H));
                    painter.rect_filled(rr, 0.0, p.diff_head);
                    hline(&painter, rr.left(), rr.right(), rr.top() + 0.5, p.diff_head_line);
                    hline(&painter, rr.left(), rr.right(), rr.bottom() - 0.5, p.diff_head_line);
                    if r.focus_hunk == Some(bi) {
                        painter.rect_filled(Rect::from_min_size(rr.min, vec2(3.0, HEAD_H)), 0.0, p.accent);
                    }
                    let cy = rr.center().y;
                    let mut x = rr.left() + 16.0;
                    let head = format!("@@ −{},{} +{},{} @@", h.old_start, h.old_lines, h.new_start, h.new_lines);
                    x = draw_left(&painter, x, cy, &head, app.th.f(Ty::MonoSm), p.text2) + 8.0;
                    if !h.section.is_empty() {
                        x = draw_trunc(&painter, x, cy, (rr.width() * 0.35).max(60.0), &h.section, app.th.f(Ty::MonoSm), p.sx_ty) + 12.0;
                    }
                    if b.staged {
                        let t = "Hazırlandı";
                        let tw = text_width(ui, t, &FontId::new(11.0, semibold()));
                        let cr_ = Rect::from_min_size(Pos2::new(x, cy - 10.0), vec2(tw + 14.0 + 16.0, 20.0));
                        painter.rect_filled(cr_, cr(255), p.accent_soft);
                        icons::paint_at(ui, Pos2::new(cr_.left() + 13.0, cy), 11.0, Icon::Check, p.accent_hi);
                        draw_left(&painter, cr_.left() + 22.0, cy, t, FontId::new(11.0, semibold()), p.accent_hi);
                    }
                    let mut rx = rr.right() - 16.0;
                    let n_changed = h.changed();
                    let sel_n = r.line_sel.iter().filter(|(s, hh, _)| *s == b.staged && *hh == b.hunk).count();
                    let btn = |ui: &mut Ui, id: &str, label: &str, danger: bool, rx: &mut f32| -> bool {
                        let w = text_width(ui, label, &FontId::new(12.0, semibold())) + 22.0;
                        let br = Rect::from_min_size(Pos2::new(*rx - w, cy - 12.0), vec2(w, 24.0));
                        *rx -= w + 8.0;
                        let resp = ui.interact(br, Id::new((id, bi)), Sense::click());
                        let hov = resp.hovered();
                        let (bg, bd, fg) = if danger { (if hov { p.tint_a(p.deleted, 0x1E) } else { Color32::TRANSPARENT }, p.danger_line, p.deleted) } else { (if hov { p.bg3 } else if b.staged { Color32::TRANSPARENT } else { Color32::from_rgb(0x2A, 0x28, 0x24) }, p.line_strong, p.text1) };
                        let bg = if !danger && !p.dark { if hov { p.bg3 } else { p.bg3 } } else { bg };
                        painter.rect_filled(br, cr(6), bg);
                        painter.rect_stroke(br, cr(6), Stroke::new(1.0, bd), StrokeKind::Inside);
                        centered(&painter, br, label, FontId::new(12.0, egui::FontFamily::Proportional), fg);
                        pointer(&resp);
                        resp.clicked()
                    };
                    if b.staged {
                        if btn(ui, "hunk-un", "Hazırlığı kaldır", false, &mut rx) {
                            acts.push(Act::StageHunk(b));
                        }
                    } else {
                        if btn(ui, "hunk-disc", "Hunk'ı at", true, &mut rx) {
                            acts.push(Act::DiscardHunk(b));
                        }
                        if sel_n > 0 {
                            if btn(ui, "hunk-sel", &format!("Seçilenleri hazırla ({sel_n})"), false, &mut rx) {
                                acts.push(Act::StageSel(b));
                            }
                        } else if btn(ui, "hunk-st", "Hunk'ı hazırla", false, &mut rx) {
                            acts.push(Act::StageHunk(b));
                        }
                    }
                    let info = if b.staged { format!("{n_changed} / {n_changed} satır hazır") } else { format!("{sel_n} / {n_changed} satır seçili") };
                    draw_right(&painter, rx - 4.0, cy, &info, app.th.f(Ty::Small), p.text2);
                }
                RK::Line(bi, li) => {
                    let rr = Rect::from_min_size(Pos2::new(origin.x, origin.y + ry), vec2(w, LINE_H));
                    paint_line(ui, acts, rr, bi, li, 0);
                }
                RK::Pair(bi, l, rg) => {
                    let half = (w / 2.0).floor();
                    let lr = Rect::from_min_size(Pos2::new(origin.x, origin.y + ry), vec2(half, LINE_H));
                    let rr = Rect::from_min_size(Pos2::new(origin.x + half, origin.y + ry), vec2(w - half, LINE_H));
                    match l {
                        Some(li) => paint_line(ui, acts, lr, bi, li, 1),
                        None => {
                            painter.rect_filled(lr, 0.0, p.bg1);
                        }
                    }
                    match rg {
                        Some(li) => paint_line(ui, acts, rr, bi, li, 2),
                        None => {
                            painter.rect_filled(rr, 0.0, p.bg1);
                        }
                    }
                    vline(&painter, origin.x + half, lr.top(), lr.bottom(), p.line_soft);
                }
            }
        }
    });
}
