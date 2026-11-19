//! History: virtual commit list with the graph on the left, commit details with files and diff on the right.

use crate::i18n::{PainterT};
use crate::app::*;
use crate::diff::{FileDiff, LineKind};
use crate::highlight;
use crate::icons::{self, Icon};
use crate::theme::{semibold, Palette, Ty};
use crate::ui::*;
use eframe::egui::{self, vec2, Align, Color32, FontId, Id, Layout, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder};

enum Act {
    Select(usize),
    Query(String),
    More,
    Cmd(Cmd),
    File(String, Option<String>),
    Reset(Pos2),
}

pub fn lane_color(p: &Palette, i: usize) -> Color32 {
    [p.accent, p.renamed, p.untracked, p.merged, p.modified, p.added][i % 6]
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let full = ui.max_rect();
    let p = app.th.p.clone();
    let lw = 720.0f32.min(full.width() * 0.56);
    let left = Rect::from_min_max(full.min, Pos2::new(full.left() + lw, full.bottom()));
    let right = Rect::from_min_max(Pos2::new(left.right(), full.top()), full.max);
    ui.painter().rect_filled(left, 0.0, p.bg1);
    vline(ui.painter(), left.right() - 0.5, left.top(), left.bottom(), p.line_soft);
    let mut acts: Vec<Act> = vec![];
    {
        let mut lui = ui.new_child(UiBuilder::new().max_rect(left).layout(Layout::top_down(Align::Min)));
        list(app, &mut lui, left, &mut acts);
        let mut rui = ui.new_child(UiBuilder::new().max_rect(right).layout(Layout::top_down(Align::Min)));
        detail(app, &mut rui, right, &mut acts);
    }
    for a in acts {
        match a {
            Act::Select(i) => app.select_commit(i),
            Act::Query(q) => {
                if let Some(r) = app.repo.as_mut() {
                    r.hist_query = q;
                }
                app.load_commits(true);
            }
            Act::More => app.load_commits(false),
            Act::Cmd(c) => app.run(c),
            Act::File(f, o) => app.select_commit_file(f, o),
            Act::Reset(pos) => {
                let sha = app.repo.as_ref().and_then(|r| r.sel_commit.and_then(|i| r.commits.get(i)).map(|c| c.c.sha.clone())).unwrap_or_default();
                let mk = |l: &str, k: &str, d: bool| MenuItem { label: l.into(), icon: None, kbd: None, cmd: Cmd::CommitOp(k.into(), sha.clone()), danger: d, enabled: true };
                app.menu = Some(Menu { pos, right_align: false, items: vec![mk("Soft — değişiklikleri hazırda tut", "reset-soft", false), mk("Mixed — değişiklikleri çalışma alanında tut", "reset-mixed", false), mk("Hard — tüm değişiklikleri sil", "reset-hard", true)] });
            }
        }
    }
}

fn list(app: &App, ui: &mut Ui, rect: Rect, acts: &mut Vec<Act>) {
    let p = &app.th.p;
    let r = app.repo.as_ref().unwrap();
    let painter = ui.painter().clone();
    let x0 = rect.left() + 16.0;
    let x1 = rect.right() - 16.0;
    let mut y = rect.top() + 14.0;
    draw_left(&painter, x0, y + 10.0, "Geçmiş", FontId::new(15.0, semibold()), p.text1);
    let count = if r.commits_done { format!("{} commit", r.commits.len()) } else { format!("{}+ commit · sanal liste", r.commits.len()) };
    draw_right(&painter, x1, y + 11.0, &count, app.th.f(Ty::Small), p.text2);
    y += 20.0 + 10.0;
    let srect = Rect::from_min_size(Pos2::new(x0, y), vec2(x1 - x0, 30.0));
    let mut sui = ui.new_child(UiBuilder::new().max_rect(srect).layout(Layout::left_to_right(Align::Center)));
    let mut q = r.hist_query.clone();
    let inner = Rect::from_min_size(srect.min, vec2(srect.width(), 30.0));
    painter.rect_filled(inner, cr(6), p.bg0);
    icons::paint_at(ui, Pos2::new(inner.left() + 20.0, inner.center().y), 14.0, Icon::Search, p.text3);
    let te_rect = Rect::from_min_max(inner.min + vec2(34.0, 0.0), inner.max - vec2(10.0, 0.0));
    let mut tui = sui.new_child(UiBuilder::new().max_rect(te_rect).layout(Layout::left_to_right(Align::Center)));
    let resp = tui.add(egui::TextEdit::singleline(&mut q).id(Id::new("hist-search")).hint_text(crate::i18n::rt("Mesaj, hash veya yazar ara").color(p.text3)).font(app.th.f(Ty::Row)).frame(false).desired_width(te_rect.width()).vertical_align(Align::Center).text_color(p.text1));
    painter.rect_stroke(inner, cr(6), Stroke::new(1.0, if resp.has_focus() { p.accent } else { p.line }), StrokeKind::Inside);
    if q != r.hist_query {
        acts.push(Act::Query(q));
    }
    y += 30.0 + 10.0;
    let n_un = r.unpushed.len();
    if n_un > 0 {
        let br = Rect::from_min_size(Pos2::new(x0, y), vec2(x1 - x0, 36.0));
        painter.rect_filled(br, cr(8), p.accent_soft);
        icons::paint_at(ui, Pos2::new(br.left() + 20.0, br.center().y), 14.0, Icon::Up, p.accent_hi);
        draw_trunc(&painter, br.left() + 36.0, br.center().y, br.width() - 130.0, &format!("{n_un} commit henüz origin'e gönderilmedi (içi boş daireler)"), app.th.f(Ty::Small), p.text1);
        let pr = Rect::from_min_size(Pos2::new(br.right() - 62.0, br.center().y - 12.0), vec2(52.0, 24.0));
        let mut pui = ui.new_child(UiBuilder::new().max_rect(pr).layout(Layout::left_to_right(Align::Center)));
        if button(&mut pui, &app.th, Btn::new("Push").primary().tiny().min_w(52.0)).clicked() {
            acts.push(Act::Cmd(Cmd::Push));
        }
        y += 36.0 + 10.0;
    }
    y += 4.0;
    hline(&painter, rect.left(), rect.right(), y, p.line_soft);
    let area = Rect::from_min_max(Pos2::new(rect.left(), y + 1.0), rect.max);
    if r.commits.is_empty() {
        let msg = if r.commits_loading { "Yükleniyor…" } else if r.hist_query.is_empty() { "Henüz commit yok" } else { "Eşleşen commit yok" };
        painter.text_t(area.center(), egui::Align2::CENTER_CENTER, msg, app.th.f(Ty::Row), p.text3);
        return;
    }
    let rh = app.th.commit_row_h();
    let mut lui = ui.new_child(UiBuilder::new().max_rect(area).layout(Layout::top_down(Align::Min)));
    let n = r.commits.len();
    let lanes = r.lanes.max(1);
    let spacing = (80.0 / lanes as f32).min(16.0).max(8.0);
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("hist-list").show_rows(&mut lui, rh, n, |ui, range| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if range.end + 30 >= n && !r.commits_done && !r.commits_loading {
            acts.push(Act::More);
        }
        for i in range {
            commit_row(app, ui, i, rh, spacing, acts);
        }
        if r.commits_done && n > 0 {
        }
    });
}

fn chip_for(p: &Palette, r: &str) -> Option<(String, Color32, Color32, bool)> {
    let r = r.trim();
    if let Some(b) = r.strip_prefix("HEAD -> ") {
        return Some((format!("HEAD · {b}"), p.tint_a(p.renamed, 0x26), if p.dark { Color32::from_rgb(0x9C, 0xC6, 0xF5) } else { p.renamed }, false));
    }
    if r == "HEAD" {
        return Some(("HEAD".into(), p.tint_a(p.renamed, 0x26), if p.dark { Color32::from_rgb(0x9C, 0xC6, 0xF5) } else { p.renamed }, false));
    }
    if let Some(t) = r.strip_prefix("tag: ") {
        return Some((t.to_string(), p.tint_a(p.modified, 0x26), p.modified, true));
    }
    if r.contains('/') {
        return Some((r.to_string(), p.bg_sel_alt(), p.text2, false));
    }
    if r == "main" || r == "master" {
        return Some((r.to_string(), p.accent_soft, p.accent_hi, false));
    }
    Some((r.to_string(), p.tint_a(p.merged, 0x26), if p.dark { Color32::from_rgb(0xC9, 0xB6, 0xF7) } else { p.merged }, false))
}

trait PalExt {
    fn bg_sel_alt(&self) -> Color32;
}
impl PalExt for Palette {
    fn bg_sel_alt(&self) -> Color32 {
        if self.dark { Color32::from_rgb(0x3A, 0x37, 0x31) } else { Color32::from_rgb(0xE4, 0xDF, 0xD5) }
    }
}

fn commit_row(app: &App, ui: &mut Ui, i: usize, rh: f32, spacing: f32, acts: &mut Vec<Act>) {
    let p = &app.th.p;
    let r = app.repo.as_ref().unwrap();
    let c = &r.commits[i];
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), rh), Sense::click());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter_at(rect);
    let sel = r.sel_commit == Some(i);
    if sel {
        painter.rect_filled(rect, 0.0, p.bg3);
    } else if resp.hovered() {
        painter.rect_filled(rect, 0.0, p.bg2);
    }
    let cy = rect.center().y;
    let gx = |lane: usize| rect.left() + 20.0 + lane as f32 * spacing;
    for e in &c.g.edges {
        let col = lane_color(p, e.color);
        let st = Stroke::new(1.6, col);
        let (a, b) = match e.half {
            crate::graph::Half::Upper => (Pos2::new(gx(e.from), rect.top()), Pos2::new(gx(e.to), cy)),
            crate::graph::Half::Lower => (Pos2::new(gx(e.from), cy), Pos2::new(gx(e.to), rect.bottom())),
        };
        if (a.x - b.x).abs() < 0.5 {
            painter.line_segment([a, b], st);
        } else {
            let mid = (a.y + b.y) / 2.0;
            painter.add(egui::epaint::CubicBezierShape::from_points_stroke([a, Pos2::new(a.x, mid), Pos2::new(b.x, mid), b], false, Color32::TRANSPARENT, st));
        }
    }
    let nc = Pos2::new(gx(c.g.lane), cy);
    let col = lane_color(p, c.g.color);
    let unpushed = r.unpushed.contains(&c.c.sha);
    if unpushed {
        painter.circle_filled(nc, 4.5, p.bg1);
        painter.circle_stroke(nc, 4.5, Stroke::new(1.8, col));
    } else {
        painter.circle_filled(nc, 4.5, col);
        painter.circle_stroke(nc, 4.5, Stroke::new(1.5, if sel { p.bg3 } else { p.bg1 }));
    }
    let text_x = rect.left() + 104.0;
    let right = rect.right() - 16.0;
    let hash_x = right - 56.0;
    let time_right = hash_x - 12.0;
    let time_x = time_right - 60.0;
    let au_x = time_x - 12.0 - 64.0;
    let msg_right = au_x - 12.0;
    draw_trunc(&painter, hash_x, cy, 56.0, crate::util::short(&c.c.sha), app.th.f(Ty::MonoXs), p.text2);
    draw_right(&painter, time_right, cy, &crate::util::rel_short(c.ts), app.th.f(Ty::Small), p.text2);
    let author = c.c.author.split_whitespace().next().map(|f| {
        let rest = c.c.author.split_whitespace().nth(1).and_then(|s| s.chars().next());
        match rest { Some(ch) => format!("{f} {ch}."), None => f.to_string() }
    }).unwrap_or_default();
    draw_trunc(&painter, au_x, cy, 64.0, &author, app.th.f(Ty::Small), p.text2);
    let font = if i == 0 { FontId::new(13.0, semibold()) } else { app.th.f(Ty::Row) };
    let chips: Vec<_> = c.c.refs.iter().filter(|x| !x.starts_with("origin/HEAD")).filter_map(|x| chip_for(p, x)).collect();
    let mut chips_w = 0.0;
    for (t, ..) in chips.iter().take(3) {
        chips_w += text_width(ui, t, &FontId::new(11.0, semibold())) + 14.0 + 6.0;
    }
    let msg_w = (msg_right - text_x - chips_w).max(80.0);
    let mut x = draw_trunc(&painter, text_x, cy, msg_w, &c.c.subject, font, p.text1) + 8.0;
    for (t, bg, fg, tag) in chips.iter().take(3) {
        let f = FontId::new(11.0, semibold());
        let w = text_width(ui, t, &f) + 14.0 + if *tag { 12.0 } else { 0.0 };
        if x + w > msg_right {
            break;
        }
        let cr_ = Rect::from_min_size(Pos2::new(x, cy - 10.0), vec2(w, 20.0));
        painter.rect_filled(cr_, cr(255), *bg);
        let mut tx = cr_.left() + 7.0;
        if *tag {
            icons::paint_at(ui, Pos2::new(tx + 5.0, cy), 10.0, Icon::Tag, *fg);
            tx += 12.0;
        }
        draw_left(&painter, tx, cy, t, f, *fg);
        x += w + 6.0;
    }
    pointer(&resp);
    if resp.clicked() {
        acts.push(Act::Select(i));
    }
}

fn detail(app: &App, ui: &mut Ui, rect: Rect, acts: &mut Vec<Act>) {
    let p = &app.th.p;
    let r = app.repo.as_ref().unwrap();
    let painter = ui.painter().clone();
    let Some(c) = r.sel_commit.and_then(|i| r.commits.get(i)) else {
        icons::paint_at(ui, rect.center() - vec2(0.0, 24.0), 28.0, Icon::History, p.text3);
        painter.text_t(rect.center() + vec2(0.0, 6.0), egui::Align2::CENTER_CENTER, "Ayrıntı için bir commit seç", FontId::new(14.0, semibold()), p.text1);
        return;
    };
    let x0 = rect.left() + 20.0;
    let x1 = rect.right() - 20.0;
    let mut y = rect.top() + 18.0;
    let sha7 = crate::util::short(&c.c.sha).to_string();
    let f = app.th.f(Ty::MonoSm);
    let tw = text_width(ui, &sha7, &f);
    let chip_r = Rect::from_min_size(Pos2::new(x0, y), vec2(tw + 16.0 + 18.0, 22.0));
    let cresp = ui.interact(chip_r, Id::new("sha-chip"), Sense::click());
    painter.rect_filled(chip_r, cr(4), if cresp.hovered() { p.bg_sel_alt() } else { p.bg3 });
    draw_left(&painter, chip_r.left() + 8.0, chip_r.center().y, &sha7, f, p.text2);
    icons::paint_at(ui, Pos2::new(chip_r.right() - 12.0, chip_r.center().y), 11.0, Icon::Copy, p.text3);
    pointer(&cresp);
    if cresp.clicked() {
        ui.ctx().copy_text(c.c.sha.clone());
    }
    let mut x = chip_r.right() + 8.0;
    if r.sel_commit == Some(0) {
        let w = text_width(ui, "HEAD", &FontId::new(11.0, semibold())) + 14.0;
        let hr = Rect::from_min_size(Pos2::new(x, y + 1.0), vec2(w, 20.0));
        painter.rect_filled(hr, cr(255), p.tint_a(p.renamed, 0x26));
        draw_left(&painter, hr.left() + 7.0, hr.center().y, "HEAD", FontId::new(11.0, semibold()), if p.dark { Color32::from_rgb(0x9C, 0xC6, 0xF5) } else { p.renamed });
        x += w + 8.0;
    }
    if let Some(par) = c.c.parents.first() {
        let xe = draw_left(&painter, x, y + 11.0, "ebeveyn ", app.th.f(Ty::Small), p.text2);
        draw_left(&painter, xe, y + 11.0, crate::util::short(par), app.th.f(Ty::MonoSm), p.text2);
    }
    y += 22.0 + 10.0;
    let mut job = egui::text::LayoutJob::single_section(c.c.subject.clone(), egui::TextFormat { font_id: FontId::new(20.0, semibold()), color: p.text1, ..Default::default() });
    job.wrap.max_width = x1 - x0;
    job.wrap.max_rows = 2;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let g = painter.layout_job(job);
    let gh = g.size().y;
    painter.galley(Pos2::new(x0, y), g, p.text1);
    y += gh + 10.0;
    if !c.c.body.trim().is_empty() {
        let g = painter.layout(c.c.body.trim().to_string(), app.th.f(Ty::Row), p.text2, x1 - x0);
        let gh = g.size().y.min(80.0);
        painter.with_clip_rect(Rect::from_min_size(Pos2::new(x0, y), vec2(x1 - x0, gh))).galley(Pos2::new(x0, y), g, p.text2);
        y += gh + 10.0;
    }
    avatar(&painter, p, Pos2::new(x0 + 11.0, y + 11.0), 22.0, &initials_of(&c.c.author));
    let when = format!("{} · {}{}", c.c.author, crate::util::date_tr(c.ts), if r.unpushed.contains(&c.c.sha) { " · henüz gönderilmedi" } else { "" });
    draw_trunc(&painter, x0 + 30.0, y + 11.0, x1 - x0 - 30.0, &when, app.th.f(Ty::Small), p.text2);
    y += 22.0 + 14.0;
    hline(&painter, rect.left(), rect.right(), y, p.line_soft);
    y += 12.0;
    let arect = Rect::from_min_size(Pos2::new(x0, y), vec2(x1 - x0, 28.0));
    let mut aui = ui.new_child(UiBuilder::new().max_rect(arect).layout(Layout::left_to_right(Align::Center)));
    aui.spacing_mut().item_spacing.x = 8.0;
    let sha = c.c.sha.clone();
    if button(&mut aui, &app.th, Btn::new("Cherry-pick").icon(Icon::Pick).small()).clicked() {
        acts.push(Act::Cmd(Cmd::CommitOp("pick".into(), sha.clone())));
    }
    if button(&mut aui, &app.th, Btn::new("Geri al (revert)").icon(Icon::Undo).small()).clicked() {
        acts.push(Act::Cmd(Cmd::CommitOp("revert".into(), sha.clone())));
    }
    if button(&mut aui, &app.th, Btn::new("Buradan dal aç").icon(Icon::Branch).small()).clicked() {
        acts.push(Act::Cmd(Cmd::CommitOp("branch".into(), sha.clone())));
    }
    if button(&mut aui, &app.th, Btn::new("Etiketle").icon(Icon::Tag).small()).clicked() {
        acts.push(Act::Cmd(Cmd::CommitOp("tag".into(), sha.clone())));
    }
    let rr = button(&mut aui, &app.th, Btn::new("Sıfırla").trail(Icon::Chev).danger().small());
    if rr.clicked() {
        acts.push(Act::Reset(Pos2::new(rr.rect.left(), rr.rect.bottom() + 4.0)));
    }
    y += 28.0 + 12.0;
    hline(&painter, rect.left(), rect.right(), y, p.line_soft);
    y += 12.0;
    let adds: i64 = r.commit_files.iter().map(|f| f.added.max(0)).sum();
    let dels: i64 = r.commit_files.iter().map(|f| f.deleted.max(0)).sum();
    draw_left(&painter, x0, y + 10.0, &format!("{} dosya değişti", r.commit_files.len()), FontId::new(13.0, semibold()), p.text1);
    let xr = draw_right(&painter, x1, y + 10.0, &format!("−{dels}"), app.th.f(Ty::MonoSm), p.deleted);
    draw_right(&painter, xr - 8.0, y + 10.0, &format!("+{adds}"), app.th.f(Ty::MonoSm), p.added);
    y += 20.0 + 8.0;
    // file rows (max 6 visible then scroll)
    let nf = r.commit_files.len();
    let fh = (nf.min(6) as f32) * 34.0;
    let frect = Rect::from_min_size(Pos2::new(rect.left() + 14.0, y), vec2(rect.width() - 28.0, fh));
    let mut fui = ui.new_child(UiBuilder::new().max_rect(frect).layout(Layout::top_down(Align::Min)));
    egui::ScrollArea::vertical().id_salt("hist-files").auto_shrink([false, false]).show(&mut fui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for f in &r.commit_files {
            let (rr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
            let painter = ui.painter_at(rr);
            let sel = r.sel_commit_file.as_deref() == Some(f.path.as_str());
            if sel {
                painter.rect_filled(rr, cr(6), p.bg3);
            } else if resp.hovered() {
                painter.rect_filled(rr, cr(6), p.bg1);
            }
            let letter = f.status.chars().next().unwrap_or('M');
            badge(&painter, p, Rect::from_center_size(Pos2::new(rr.left() + 6.0 + 10.0, rr.center().y), vec2(20.0, 20.0)), letter);
            let mut right = rr.right() - 6.0;
            if f.deleted > 0 {
                right = draw_right(&painter, right, rr.center().y, &format!("−{}", f.deleted), app.th.f(Ty::MonoXs), p.deleted) - 6.0;
            }
            if f.added > 0 {
                right = draw_right(&painter, right, rr.center().y, &format!("+{}", f.added), app.th.f(Ty::MonoXs), p.added) - 8.0;
            }
            let total = (f.added + f.deleted).max(1) as f32;
            let blocks = 4usize;
            let add_b = ((f.added as f32 / total) * blocks as f32).round() as usize;
            let nb = if f.added + f.deleted == 0 { 0 } else { blocks };
            for k in 0..nb {
                let bx = right - (nb - k) as f32 * 5.0 + 1.0;
                painter.rect_filled(Rect::from_min_size(Pos2::new(bx, rr.center().y - 5.0), vec2(4.0, 10.0)), cr(1), if k < add_b { p.added } else { p.deleted });
            }
            right -= nb as f32 * 5.0 + 8.0;
            let (dir, name) = crate::util::split_path(&f.path);
            let x = rr.left() + 6.0 + 20.0 + 8.0;
            let nw = text_width(ui, name, &app.th.f(Ty::Row)).min(right - x);
            let x2 = if dir.is_empty() { x } else { draw_trunc(&painter, x, rr.center().y, (right - x - nw).max(0.0), dir, app.th.f(Ty::Row), p.text3) };
            draw_trunc(&painter, x2, rr.center().y, right - x2, name, app.th.f(Ty::Row), p.text1);
            pointer(&resp);
            if resp.clicked() {
                acts.push(Act::File(f.path.clone(), f.orig_path.clone()));
            }
        }
    });
    y += fh + 12.0;
    let box_rect = Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, rect.bottom() - 16.0));
    if box_rect.height() > 60.0 {
        painter.rect_filled(box_rect, cr(8), p.bg2);
        painter.rect_stroke(box_rect, cr(8), Stroke::new(1.0, p.diff_head_line), StrokeKind::Inside);
        if let (Some(fd), Some(file)) = (r.commit_diff.as_ref(), r.sel_commit_file.as_ref()) {
            let mut dui = ui.new_child(UiBuilder::new().max_rect(box_rect.shrink(1.0)).layout(Layout::top_down(Align::Min)));
            dui.set_clip_rect(box_rect.shrink(1.0));
            readonly_diff(app, &mut dui, box_rect.shrink(1.0), fd, file);
        } else {
            painter.text_t(box_rect.center(), egui::Align2::CENTER_CENTER, if r.sel_commit_file.is_some() { "Yükleniyor…" } else { "Bir dosya seç" }, app.th.f(Ty::Row), p.text3);
        }
    }
}

/// Read-only unified diff (also used by the stash page).
pub fn readonly_diff(app: &App, ui: &mut Ui, area: Rect, fd: &FileDiff, file: &str) {
    let p = &app.th.p;
    let lang = highlight::lang_of(file);
    let mono = app.th.f(Ty::Mono);
    let mw = ui.painter().layout_no_wrap("M".into(), mono.clone(), Color32::WHITE).size().x;
    if fd.binary {
        ui.painter().text_t(area.center(), egui::Align2::CENTER_CENTER, "İkili dosya", app.th.f(Ty::Row), p.text2);
        return;
    }
    enum RK {
        Head(usize),
        Line(usize, usize),
    }
    let mut rows: Vec<(f32, RK)> = vec![];
    let mut y = 0.0;
    let mut maxc = 40;
    for (hi, h) in fd.hunks.iter().enumerate() {
        rows.push((y, RK::Head(hi)));
        y += 28.0;
        for (li, l) in h.lines.iter().enumerate() {
            rows.push((y, RK::Line(hi, li)));
            y += 22.0;
            maxc = maxc.max(l.text.chars().count() + 3);
        }
    }
    let gutter = 46.0 * 2.0 + 20.0;
    let cw = gutter + maxc as f32 * mw + 30.0;
    egui::ScrollArea::both().id_salt(("rodiff", file)).auto_shrink([false, false]).show_viewport(ui, |ui, vp| {
        ui.set_min_size(vec2(cw.max(vp.width()), y + 8.0));
        let o = ui.min_rect().min;
        let w = cw.max(vp.width());
        let first = rows.partition_point(|(ry, _)| ry + 28.0 < vp.top());
        let painter = ui.painter().clone();
        for (ry, k) in rows.iter().skip(first) {
            if *ry > vp.bottom() {
                break;
            }
            match k {
                RK::Head(hi) => {
                    let h = &fd.hunks[*hi];
                    let rr = Rect::from_min_size(Pos2::new(o.x, o.y + ry), vec2(w, 28.0));
                    painter.rect_filled(rr, 0.0, p.diff_head);
                    let t = format!("{} · @@ −{},{} +{},{} @@ {}", file, h.old_start, h.old_lines, h.new_start, h.new_lines, h.section);
                    draw_left(&painter, o.x + vp.left() + 12.0, rr.center().y, &t, app.th.f(Ty::MonoXs), p.text2);
                }
                RK::Line(hi, li) => {
                    let l = &fd.hunks[*hi].lines[*li];
                    let rr = Rect::from_min_size(Pos2::new(o.x, o.y + ry), vec2(w, 22.0));
                    let row = ui.interact(rr, Id::new(("rl", *hi, *li)), Sense::click());
                    if row.hovered() {
                        crate::overlay::hover_copy(ui.ctx(), l.text.clone());
                    }
                    if row.secondary_clicked() {
                        crate::overlay::request_copy_menu(ui.ctx(), crate::overlay::CopyReq { pos: row.interact_pointer_pos().unwrap_or(rr.center()), line: l.text.clone(), hunk: crate::screens::changes::hunk_text(&fd.hunks[*hi]), file: file.to_string(), sel: String::new() });
                    }
                    match l.kind {
                        LineKind::Add => {
                            painter.rect_filled(rr, 0.0, p.diff_add);
                        }
                        LineKind::Del => {
                            painter.rect_filled(rr, 0.0, p.diff_del);
                        }
                        LineKind::Ctx => {}
                    }
                    let cy = rr.center().y;
                    if let Some(n) = l.old_no {
                        draw_right(&painter, rr.left() + 36.0, cy, &n.to_string(), mono.clone(), if l.kind == LineKind::Del { p.code_gutter.lerp_to_gamma(p.deleted, 0.45) } else { p.code_gutter });
                    }
                    if let Some(n) = l.new_no {
                        draw_right(&painter, rr.left() + 82.0, cy, &n.to_string(), mono.clone(), if l.kind == LineKind::Add { p.code_gutter.lerp_to_gamma(p.added, 0.45) } else { p.code_gutter });
                    }
                    let (m, mc) = match l.kind {
                        LineKind::Add => ("+", p.added),
                        LineKind::Del => ("−", p.deleted),
                        LineKind::Ctx => ("", p.text3),
                    };
                    painter.text_t(Pos2::new(rr.left() + 92.0 + 10.0, cy), egui::Align2::CENTER_CENTER, m, mono.clone(), mc);
                    let text = l.text.replace('\t', "    ");
                    let mut job = egui::text::LayoutJob::default();
                    job.wrap.max_width = f32::INFINITY;
                    for (t, kk) in highlight::tokenize(&text, lang) {
                        job.append(&t, 0.0, egui::TextFormat { font_id: mono.clone(), color: highlight::color(p, kk), ..Default::default() });
                    }
                    if text.is_empty() {
                        job.append(" ", 0.0, egui::TextFormat { font_id: mono.clone(), color: p.text1, ..Default::default() });
                    }
                    let g = painter.layout_job(job);
                    painter.galley(Pos2::new(rr.left() + gutter, cy - g.size().y / 2.0), g, p.text1);
                }
            }
        }
    });
}
