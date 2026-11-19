//! Welcome screen, clone panel and the GitHub device-flow sign-in dialog.

use crate::i18n::{PainterT};
use crate::app::*;
use crate::backend::{github, remote};
use crate::icons::{self, Icon};
use crate::theme::{semibold, Ty};
use crate::ui::*;
use eframe::egui::{self, vec2, Align, Color32, FontId, Id, Layout, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct WelcomeState {
    pub clone_open: bool,
    pub url: String,
    pub dest: String,
    pub dest_dirty: bool,
    pub tab: usize,
    pub cloning: bool,
    pub clone_err: Option<String>,
    pub gh_repos: Vec<(String, String, String)>, // full name, clone url, description
    pub gh_loading: bool,
    pub gh_loaded: bool,
    pub gh_filter: String,
    pub found: Vec<crate::backend::sys::Found>,
    pub scanning: bool,
    pub scanned: bool,
}

#[derive(Default)]
pub struct LoginState {
    pub phase: u8, // 0 idle, 1 starting, 2 waiting for the device code, 3 waiting for the browser, 4 error
    pub user_code: String,
    pub uri: String,
    pub error: String,
    pub cancel: Option<Arc<AtomicBool>>,
    pub pid: Option<Arc<std::sync::Mutex<Option<u32>>>>,
}

fn repo_name_from(url: &str) -> String {
    let u = url.trim().trim_end_matches('/').trim_end_matches(".git");
    u.rsplit(['/', ':']).next().unwrap_or("").to_string()
}

fn default_root() -> String {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_default();
    format!("{home}\\Documents\\GitHub")
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let th = app.th.clone();
    let p = th.p.clone();
    let full = ui.max_rect();
    let dropped = ui.ctx().input(|i| i.raw.dropped_files.first().and_then(|f| f.path.clone()));
    if let Some(d) = dropped {
        let dir = if d.is_dir() { d } else { d.parent().map(|x| x.to_path_buf()).unwrap_or(d) };
        app.welcome.clone_open = false;
        app.open_repo(dir.to_string_lossy().to_string());
        return;
    }
    let pad_x = (full.width() * 0.05).clamp(32.0, 72.0);
    let left_w = 520.0f32.min(full.width() * 0.42).max(380.0);
    let left = Rect::from_min_size(Pos2::new(full.left() + pad_x, full.top() + 56.0), vec2(left_w, full.height() - 90.0));
    let right = Rect::from_min_max(Pos2::new(left.right() + 56.0, full.top() + 56.0), Pos2::new(full.right() - pad_x, full.bottom() - 40.0));
    let mut cmds: Vec<Cmd> = vec![];
    let mut open: Option<String> = None;

    let mut y = left.top();
    let painter = ui.painter().clone();
    painter.text_t(Pos2::new(left.left(), y), egui::Align2::LEFT_TOP, "Ferro'ya hoş geldin", FontId::new(40.0, semibold()), p.text1);
    y += 58.0;
    let g = painter.layout(crate::i18n::t("Bir depo aç ve çalışmaya başla. Her işlem geri alınabilir, her işlemin git karşılığı görünür.").into_owned(), FontId::new(16.0, egui::FontFamily::Proportional), p.text2, left.width());
    let gh = g.size().y;
    painter.galley(Pos2::new(left.left(), y), g, p.text2);
    y += gh + 20.0;
    let ban = Rect::from_min_size(Pos2::new(left.left(), y), vec2(left.width(), 40.0));
    let ok_id = !app.git_user.0.is_empty();
    painter.rect_filled(ban, cr(8), p.tint_a(if ok_id { p.added } else { p.modified }, 0x22));
    icons::paint_at(ui, Pos2::new(ban.left() + 22.0, ban.center().y), 15.0, if ok_id { Icon::Check } else { Icon::Alert }, if ok_id { p.added } else { p.modified });
    let msg = if ok_id { format!("Git kimliği ~/.gitconfig dosyasından okundu: {} · {}", app.git_user.0, app.git_user.1) } else { "Git kimliği ayarlı değil — Ayarlar > Git bölümünden ekle".to_string() };
    draw_trunc(&painter, ban.left() + 40.0, ban.center().y, ban.width() - 52.0, &msg, th.f(Ty::Row), p.text1);
    y += 40.0 + 20.0;
    let big = |ui: &mut Ui, rect: Rect, icon: Icon, title: &str, sub: &str, kbd: &str, primary: bool, id: &str| -> bool {
        let resp = ui.interact(rect, Id::new(id), Sense::click());
        let (bg, bd, fg, sfg) = if primary { (if resp.hovered() { p.accent_hi } else { p.accent }, Color32::TRANSPARENT, p.accent_fg, p.accent_fg) } else { (if resp.hovered() { p.bg2 } else { p.bg3 }, p.line_strong, p.text1, p.text2) };
        ui.painter().rect_filled(rect, cr(10), bg);
        if bd != Color32::TRANSPARENT {
            ui.painter().rect_stroke(rect, cr(10), Stroke::new(1.0, bd), StrokeKind::Inside);
        }
        icons::paint_at(ui, Pos2::new(rect.left() + 28.0, rect.center().y), 20.0, icon, fg);
        draw_left(ui.painter(), rect.left() + 52.0, rect.center().y - 9.0, title, FontId::new(14.0, semibold()), fg);
        draw_left(ui.painter(), rect.left() + 52.0, rect.center().y + 10.0, sub, th.f(Ty::Small), sfg);
        let kw = text_width(ui, kbd, &th.f(Ty::MonoXs)) + 12.0;
        let kr = Rect::from_min_size(Pos2::new(rect.right() - kw - 16.0, rect.center().y - 9.0), vec2(kw, 18.0));
        ui.painter().rect_stroke(kr, cr(4), Stroke::new(1.0, if primary { Color32::from_black_alpha(100) } else { p.line_strong }), StrokeKind::Inside);
        centered(ui.painter(), kr, kbd, th.f(Ty::MonoXs), if primary { fg } else { p.text2 });
        pointer(&resp);
        resp.clicked()
    };
    if big(ui, Rect::from_min_size(Pos2::new(left.left(), y), vec2(left.width(), 60.0)), Icon::Clone, "Depo klonla", "URL ile ya da hesabından seç", "Ctrl Shift O", true, "w-clone") {
        app.welcome.clone_open = true;
    }
    y += 70.0;
    if big(ui, Rect::from_min_size(Pos2::new(left.left(), y), vec2(left.width(), 60.0)), Icon::Folder, "Yerel depo ekle", "Bilgisayarındaki bir klasörü seç", "Ctrl O", false, "w-open") {
        cmds.push(Cmd::OpenFolder);
    }
    y += 70.0;
    if big(ui, Rect::from_min_size(Pos2::new(left.left(), y), vec2(left.width(), 60.0)), Icon::Plus, "Yeni depo oluştur", "Boş klasörde git init", "Ctrl N", false, "w-new") {
        if let Some(d) = rfd::FileDialog::new().set_title("Yeni deponun klasörünü seç").pick_folder() {
            let ds = d.to_string_lossy().to_string();
            let d2 = ds.clone();
            app.spawn_busy(move || repo_init(d2), move |app, r| match r {
                Ok(()) => app.open_repo(ds),
                Err(e) => app.toast(ToastKind::Err, e),
            });
        }
    }
    y += 70.0;
    let dz = Rect::from_min_size(Pos2::new(left.left(), y), vec2(left.width(), 60.0));
    let hovering = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
    dashed_rect(&painter, dz, if hovering { p.accent } else { p.line_strong });
    centered(&painter, dz, "Ya da bir klasörü pencereye sürükle", th.f(Ty::Row), if hovering { p.accent } else { p.text2 });
    y += 60.0 + 20.0;
    // recents (or repositories found on disk when there is no history yet)
    {
        let using_found = app.s.recents.is_empty();
        let rows: Vec<(String, String, String)> = if using_found {
            app.welcome.found.iter().map(|f| (f.name.clone(), f.path.clone(), crate::util::rel(f.mtime))).collect()
        } else {
            app.s.recents.iter().map(|r| (r.name.clone(), r.path.clone(), crate::util::rel(r.opened))).collect()
        };
        let title = if using_found { "BU BİLGİSAYARDA BULUNANLAR" } else { "SON DEPOLAR" };
        if !rows.is_empty() || app.welcome.scanning {
            draw_left(&painter, left.left() + 12.0, y + 8.0, title, FontId::new(11.0, semibold()), p.text3);
            if app.welcome.scanning {
                draw_right(&painter, left.right() - 12.0, y + 8.0, "Taranıyor…", th.f(Ty::Small), p.text3);
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
            }
            y += 22.0;
        } else if app.welcome.scanned && using_found {
            draw_left(&painter, left.left() + 12.0, y + 8.0, "Depo bulunamadı; elle ekleyebilirsin", th.f(Ty::Small), p.text3);
            y += 22.0;
        }
        let max_rows = ((left.bottom() - y) / 54.0).max(0.0) as usize;
        for (name, path, when) in rows.iter().take(max_rows) {
            let rr = Rect::from_min_size(Pos2::new(left.left(), y), vec2(left.width(), 52.0));
            let resp = ui.interact(rr, Id::new(("recent", path)), Sense::click());
            if resp.hovered() {
                painter.rect_filled(rr, cr(8), p.bg3);
            }
            icons::paint_at(ui, Pos2::new(rr.left() + 24.0, rr.center().y), 18.0, Icon::Folder, p.text2);
            draw_trunc(&painter, rr.left() + 48.0, rr.center().y - 8.0, rr.width() - 160.0, name, FontId::new(13.0, semibold()), p.text1);
            draw_trunc(&painter, rr.left() + 48.0, rr.center().y + 9.0, rr.width() - 160.0, path, th.f(Ty::MonoXs), p.text3);
            draw_right(&painter, rr.right() - 14.0, rr.center().y, when, th.f(Ty::Small), p.text3);
            pointer(&resp);
            if resp.clicked() {
                open = Some(path.clone());
            }
            y += 54.0;
        }
        if using_found && app.welcome.scanned && !app.welcome.scanning {
            let br = Rect::from_min_size(Pos2::new(left.left(), y + 4.0), vec2(220.0, 28.0));
            if br.bottom() < left.bottom() + 20.0 {
                let mut bui = ui.new_child(UiBuilder::new().max_rect(br).layout(Layout::left_to_right(Align::Center)));
                if button(&mut bui, &th, Btn::new("Klasör tara…").ghost().small()).clicked() {
                    if let Some(d) = rfd::FileDialog::new().set_title("Taranacak klasörü seç").pick_folder() {
                        app.welcome.scanned = false;
                        app.scan_repos(vec![d.to_string_lossy().to_string()]);
                    }
                }
            }
        }
    }

    if app.welcome.clone_open || (app.s.recents.is_empty() && app.welcome.found.is_empty() && !app.welcome.scanning) {
        clone_panel(app, ui, right, &mut cmds);
    } else {
        tips_panel(app, ui, right);
    }
    if let Some(path) = open {
        app.welcome.clone_open = false;
        app.open_repo(path);
    }
    for c in cmds {
        if matches!(c, Cmd::CloneRepo) {
            continue;
        }
        app.run(c);
    }
}

fn repo_init(path: String) -> Result<(), String> {
    crate::backend::repo::init_repo(path, "main".into())
}

fn dashed_rect(painter: &egui::Painter, r: Rect, c: Color32) {
    let st = Stroke::new(1.5, c);
    let (dash, gap) = (6.0, 5.0);
    let seg = |a: Pos2, b: Pos2| {
        let len = (b - a).length();
        let dir = (b - a) / len;
        let mut t = 0.0;
        while t < len {
            let e = (t + dash).min(len);
            painter.line_segment([a + dir * t, a + dir * e], st);
            t += dash + gap;
        }
    };
    seg(r.left_top(), r.right_top());
    seg(r.right_top(), r.right_bottom());
    seg(r.right_bottom(), r.left_bottom());
    seg(r.left_bottom(), r.left_top());
}

fn clone_panel(app: &mut App, ui: &mut Ui, rect: Rect, _cmds: &mut Vec<Cmd>) {
    let th = app.th.clone();
    let p = th.p.clone();
    if rect.width() < 360.0 {
        return;
    }
    let rect = Rect::from_min_size(rect.min, vec2(rect.width(), rect.height().min(600.0)));
    let painter = ui.painter().clone();
    painter.rect_filled(rect.translate(vec2(0.0, 10.0)).expand(2.0), cr(14), Color32::from_black_alpha(if p.dark { 70 } else { 22 }));
    painter.rect_filled(rect, cr(12), p.bg2);
    painter.rect_stroke(rect, cr(12), Stroke::new(1.0, p.line), StrokeKind::Inside);
    let x0 = rect.left() + 24.0;
    let x1 = rect.right() - 24.0;
    draw_left(&painter, x0, rect.top() + 34.0, "Depo klonla", FontId::new(20.0, semibold()), p.text1);
    let ty = rect.top() + 56.0;
    let mut tx = x0;
    for (i, t) in ["URL", "GitHub"].iter().enumerate() {
        let w = text_width(ui, t, &FontId::new(13.0, semibold())) + 4.0;
        let tr = Rect::from_min_size(Pos2::new(tx, ty), vec2(w, 36.0));
        let resp = ui.interact(tr, Id::new(("clone-tab", i)), Sense::click());
        let sel = app.welcome.tab == i;
        draw_left(&painter, tx + 2.0, tr.center().y, t, if sel { FontId::new(13.0, semibold()) } else { th.f(Ty::Row) }, if sel { p.text1 } else { p.text2 });
        if sel {
            painter.rect_filled(Rect::from_min_size(Pos2::new(tx, tr.bottom() - 2.0), vec2(w, 2.0)), 0.0, p.accent);
        }
        pointer(&resp);
        if resp.clicked() {
            app.welcome.tab = i;
            if i == 1 && !app.welcome.gh_loaded {
                load_gh_repos(app);
            }
        }
        tx += w + 20.0;
    }
    hline(&painter, rect.left() + 1.0, rect.right() - 1.0, ty + 36.0, p.line_soft);
    let body_top = ty + 36.0 + 20.0;
    let mut y = body_top;
    let w = &mut app.welcome;
    let cloning = w.cloning;
    if w.tab == 0 {
        draw_left(&painter, x0, y + 8.0, "Depo adresi", FontId::new(12.0, semibold()), p.text2);
        y += 22.0;
        let r1 = Rect::from_min_size(Pos2::new(x0, y), vec2(x1 - x0, 34.0));
        let mut cui = ui.new_child(UiBuilder::new().max_rect(r1).layout(Layout::left_to_right(Align::Center)));
        let old = w.url.clone();
        text_input(&mut cui, &th, &mut w.url, InputOpts::new(Id::new("clone-url"), "https://github.com/kullanici/depo.git").width(x1 - x0).mono());
        if w.url != old && !w.dest_dirty {
            let n = repo_name_from(&w.url);
            w.dest = if n.is_empty() { String::new() } else { format!("{}\\{}", default_root(), n) };
        }
        y += 34.0 + 16.0;
    } else {
        draw_left(&painter, x0, y + 8.0, "Depolarının listesi", FontId::new(12.0, semibold()), p.text2);
        y += 22.0;
        let r1 = Rect::from_min_size(Pos2::new(x0, y), vec2(x1 - x0, 34.0));
        let mut cui = ui.new_child(UiBuilder::new().max_rect(r1).layout(Layout::left_to_right(Align::Center)));
        text_input(&mut cui, &th, &mut w.gh_filter, InputOpts::new(Id::new("clone-gh-filter"), "Depo ara").width(x1 - x0));
        y += 34.0 + 8.0;
        let list_rect = Rect::from_min_size(Pos2::new(x0, y), vec2(x1 - x0, 170.0));
        painter.rect_filled(list_rect, cr(6), p.bg1);
        painter.rect_stroke(list_rect, cr(6), Stroke::new(1.0, p.line), StrokeKind::Inside);
        let mut lui = ui.new_child(UiBuilder::new().max_rect(list_rect.shrink(2.0)).layout(Layout::top_down(Align::Min)));
        if w.gh_loading {
            painter.text_t(list_rect.center(), egui::Align2::CENTER_CENTER, "Yükleniyor…", th.f(Ty::Row), p.text3);
        } else if app.s.account.is_none() {
            painter.text_t(list_rect.center(), egui::Align2::CENTER_CENTER, "Önce GitHub hesabı bağla (Ayarlar > Hesaplar)", th.f(Ty::Row), p.text3);
        }
        let filter = w.gh_filter.to_lowercase();
        let mut pick: Option<(String, String)> = None;
        egui::ScrollArea::vertical().id_salt("gh-repos").auto_shrink([false, false]).show(&mut lui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (name, url, desc) in w.gh_repos.iter().filter(|r| filter.is_empty() || r.0.to_lowercase().contains(&filter)) {
                let (rr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
                let sel = &w.url == url;
                if sel {
                    ui.painter().rect_filled(rr, cr(4), p.bg_sel);
                } else if resp.hovered() {
                    ui.painter().rect_filled(rr, cr(4), p.bg3);
                }
                draw_trunc(ui.painter(), rr.left() + 8.0, rr.center().y - 7.0, rr.width() - 16.0, name, FontId::new(13.0, semibold()), p.text1);
                if !desc.is_empty() {
                    draw_trunc(ui.painter(), rr.left() + 8.0, rr.center().y + 8.0, rr.width() - 16.0, desc, th.f(Ty::Small), p.text3);
                }
                pointer(&resp);
                if resp.clicked() {
                    pick = Some((name.clone(), url.clone()));
                }
            }
        });
        if let Some((name, url)) = pick {
            w.url = url;
            if !w.dest_dirty {
                w.dest = format!("{}\\{}", default_root(), name.rsplit('/').next().unwrap_or(&name));
            }
        }
        y += 170.0 + 16.0;
    }
    draw_left(&painter, x0, y + 8.0, "Yerel klasör", FontId::new(12.0, semibold()), p.text2);
    y += 22.0;
    let r2 = Rect::from_min_size(Pos2::new(x0, y), vec2(x1 - x0, 34.0));
    let mut dui = ui.new_child(UiBuilder::new().max_rect(r2).layout(Layout::left_to_right(Align::Center)));
    dui.spacing_mut().item_spacing.x = 8.0;
    let old = w.dest.clone();
    text_input(&mut dui, &th, &mut w.dest, InputOpts::new(Id::new("clone-dest"), "C:\\Users\\sen\\Documents\\GitHub\\depo").width(x1 - x0 - 88.0).mono());
    if w.dest != old {
        w.dest_dirty = true;
    }
    if button(&mut dui, &th, Btn::new("Gözat").height(34.0)).clicked() {
        if let Some(d) = rfd::FileDialog::new().set_title("Klonlanacak klasörü seç").pick_folder() {
            let n = repo_name_from(&w.url);
            w.dest = if n.is_empty() { d.to_string_lossy().to_string() } else { d.join(n).to_string_lossy().to_string() };
            w.dest_dirty = true;
        }
    }
    y += 34.0 + 18.0;
    if cloning {
        let pct = parse_percent(&app.sync_text);
        draw_left(&painter, x0, y + 8.0, &format!("Klonlanıyor · {}", repo_name_from(&app.welcome.url)), FontId::new(14.0, semibold()), p.text1);
        draw_right(&painter, x1, y + 9.0, &format!("{}%", (pct * 100.0) as u32), th.f(Ty::MonoSm), p.text2);
        y += 26.0;
        let br = Rect::from_min_size(Pos2::new(x0, y), vec2(x1 - x0, 8.0));
        painter.rect_filled(br, cr(4), p.line_soft);
        painter.rect_filled(Rect::from_min_size(br.min, vec2(br.width() * pct, 8.0)), cr(4), p.accent);
        y += 8.0 + 12.0;
        let line = Rect::from_min_size(Pos2::new(x0, y), vec2(x1 - x0, 30.0));
        painter.rect_filled(line, cr(6), p.bg1);
        icons::paint_at(ui, Pos2::new(line.left() + 16.0, line.center().y), 12.0, Icon::Terminal, p.text3);
        draw_trunc(&painter, line.left() + 30.0, line.center().y, line.width() - 40.0, if app.sync_text.is_empty() { "bağlanıyor…" } else { &app.sync_text }, th.f(Ty::MonoXs), p.text2);
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
    } else if let Some(e) = app.welcome.clone_err.clone() {
        let g = painter.layout(crate::i18n::t(&e).into_owned(), th.f(Ty::Row), p.deleted, x1 - x0);
        painter.galley(Pos2::new(x0, y), g, p.deleted);
    } else {
        let cmd = format!("git clone {}", if app.welcome.url.is_empty() { "‹adres›" } else { &app.welcome.url });
        let line = Rect::from_min_size(Pos2::new(x0, y), vec2(x1 - x0, 30.0));
        painter.rect_filled(line, cr(6), p.bg1);
        icons::paint_at(ui, Pos2::new(line.left() + 16.0, line.center().y), 12.0, Icon::Terminal, p.text3);
        draw_trunc(&painter, line.left() + 30.0, line.center().y, line.width() - 40.0, &cmd, th.f(Ty::MonoXs), p.text2);
    }
    let fr = Rect::from_min_max(Pos2::new(rect.left() + 1.0, rect.bottom() - 62.0), rect.max - vec2(1.0, 1.0));
    painter.rect_filled(fr, bottom_corners(11), p.bg1);
    hline(&painter, fr.left(), fr.right(), fr.top(), p.line_soft);
    let acc = app.s.account.as_ref().map(|a| format!("GitHub · {} bağlı", a.login)).unwrap_or_else(|| "GitHub hesabı bağlı değil".into());
    icons::paint_at(ui, Pos2::new(fr.left() + 36.0, fr.center().y), 13.0, Icon::Link, p.text2);
    draw_left(&painter, fr.left() + 50.0, fr.center().y, &acc, th.f(Ty::Small), p.text2);
    let can = !app.welcome.url.trim().is_empty() && !app.welcome.dest.trim().is_empty() && !app.welcome.cloning;
    let mut bui = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(Pos2::new(fr.left() + 200.0, fr.top()), Pos2::new(fr.right() - 24.0, fr.bottom()))).layout(Layout::right_to_left(Align::Center)));
    bui.spacing_mut().item_spacing.x = 10.0;
    if button(&mut bui, &th, Btn::new(if cloning { "Klonlanıyor…" } else { "Klonla" }).primary().height(34.0).min_w(96.0).enabled(can)).clicked() {
        start_clone(app);
    }
    if button(&mut bui, &th, Btn::new("İptal").height(34.0)).clicked() && !app.welcome.cloning {
        app.welcome.clone_open = false;
        app.welcome.clone_err = None;
    }
}

fn bottom_corners(r: u8) -> egui::CornerRadius {
    egui::CornerRadius { nw: 0, ne: 0, sw: r, se: r }
}

fn parse_percent(s: &str) -> f32 {
    if let Some(i) = s.find('%') {
        let digits: String = s[..i].chars().rev().take_while(|c| c.is_ascii_digit()).collect::<Vec<_>>().into_iter().rev().collect();
        if let Ok(n) = digits.parse::<f32>() {
            return (n / 100.0).clamp(0.0, 1.0);
        }
    }
    0.0
}

fn load_gh_repos(app: &mut App) {
    let Some(acc) = app.account_key() else { return };
    app.welcome.gh_loading = true;
    app.spawn(
        move || github::gh_api(acc, "GET".into(), "/user/repos?per_page=100&sort=updated&affiliation=owner,collaborator,organization_member".into(), None, Some(3)),
        |app, r| {
            app.welcome.gh_loading = false;
            app.welcome.gh_loaded = true;
            match r {
                Ok(v) => {
                    app.welcome.gh_repos = v
                        .as_array()
                        .map(|a| a.iter().map(|x| (x["full_name"].as_str().unwrap_or("").to_string(), x["clone_url"].as_str().unwrap_or("").to_string(), x["description"].as_str().unwrap_or("").to_string())).collect())
                        .unwrap_or_default();
                }
                Err(e) => app.toast(ToastKind::Err, e),
            }
        },
    );
}

fn start_clone(app: &mut App) {
    let url = app.welcome.url.trim().to_string();
    let dest = app.welcome.dest.trim().to_string();
    app.welcome.cloning = true;
    app.welcome.clone_err = None;
    app.sync_text.clear();
    let progress = app.progress();
    let acc = app.account_key();
    let d2 = dest.clone();
    app.spawn(
        move || remote::clone_repo(&progress, url, d2, None, acc),
        move |app, r| {
            app.welcome.cloning = false;
            match r {
                Ok(_) => {
                    app.welcome.clone_open = false;
                    app.welcome.url.clear();
                    app.welcome.dest.clear();
                    app.welcome.dest_dirty = false;
                    app.toast(ToastKind::Ok, "Klonlama tamamlandı");
                    app.open_repo(dest);
                }
                Err(e) => app.welcome.clone_err = Some(e),
            }
        },
    );
}

/// Sign in with Git Credential Manager unless a custom OAuth client id is configured.
pub fn login_start(app: &mut App) {
    if app.s.client_id.trim().is_empty() {
        login_start_gcm(app);
    } else {
        login_start_device(app);
    }
}

fn login_start_gcm(app: &mut App) {
    if let Some(c) = &app.login.cancel {
        c.store(true, Ordering::Relaxed);
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let pid = Arc::new(std::sync::Mutex::new(None));
    app.login = LoginState { phase: 3, cancel: Some(cancel.clone()), pid: Some(pid.clone()), ..Default::default() };
    app.modal = Some(Modal::Login);
    let host = "github.com".to_string();
    app.spawn(
        move || -> Result<Vec<Acc>, String> {
            let snapshot = |host: &str| -> Vec<(String, Option<String>)> {
                crate::backend::git::gcm_accounts(host)
                    .into_iter()
                    .map(|login| {
                        let key = format!("{host}|{login}");
                        crate::backend::git::forget_cached(&key);
                        let t = crate::backend::git::token_for(&key);
                        (login, t)
                    })
                    .collect()
            };
            let before = snapshot(&host);
            crate::backend::git::gcm_login(&host, &pid)?;
            if cancel.load(Ordering::Relaxed) {
                return Err(String::new());
            }
            let after = snapshot(&host);
            // the account that was just signed in is the new one, or the one whose token was renewed
            let fresh: Vec<String> = after
                .iter()
                .filter(|(login, token)| match before.iter().find(|(l, _)| l == login) {
                    None => true,
                    Some((_, old)) => old != token,
                })
                .map(|(l, _)| l.clone())
                .collect();
            if fresh.is_empty() {
                return Err("Tarayıcıda zaten bağlı olan bir hesapla giriş yapıldı; yeni hesap eklenmedi. Başka bir hesap için önce GitHub'da o hesaba geç (ya da gizli pencerede giriş yap) ve tekrar dene.".into());
            }
            let mut out = vec![];
            for login in fresh.into_iter().take(1) {
                let key = format!("{host}|{login}");
                let token = crate::backend::git::token_for(&key).ok_or_else(|| "Hesap bulundu ama erişim anahtarı okunamadı".to_string())?;
                let v = github::gh_whoami(host.clone(), token)?;
                out.push(Acc { key, host: host.clone(), login: v["login"].as_str().unwrap_or(&login).into(), name: v["name"].as_str().unwrap_or("").into() });
            }
            Ok(out)
        },
        |app, r| match r {
            Ok(list) => {
                for a in list {
                    app.add_account(a);
                }
            }
            Err(e) => {
                if app.login.phase == 3 && !e.is_empty() {
                    app.login.phase = 4;
                    app.login.error = e;
                }
            }
        },
    );
}

fn login_start_device(app: &mut App) {
    if let Some(c) = &app.login.cancel {
        c.store(true, Ordering::Relaxed);
    }
    let cancel = Arc::new(AtomicBool::new(false));
    app.login = LoginState { phase: 1, cancel: Some(cancel.clone()), ..Default::default() };
    app.modal = Some(Modal::Login);
    let client = app.s.client_id.clone();
    let host = "github.com".to_string();
    let (c2, h2) = (client.clone(), host.clone());
    app.spawn(
        move || github::gh_device_start(h2, c2),
        move |app, r| match r {
            Ok(v) => {
                let code = v["deviceCode"].as_str().unwrap_or("").to_string();
                let interval = v["interval"].as_u64().unwrap_or(5);
                app.login.user_code = v["userCode"].as_str().unwrap_or("").to_string();
                app.login.uri = v["verificationUri"].as_str().unwrap_or("https://github.com/login/device").to_string();
                app.login.phase = 2;
                app.ctx.copy_text(app.login.user_code.clone());
                let _ = open::that(&app.login.uri);
                let tx = app.tx.clone();
                let ctx = app.ctx.clone();
                std::thread::spawn(move || {
                    let mut wait = interval.max(2);
                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(wait));
                        if cancel.load(Ordering::Relaxed) {
                            return;
                        }
                        let res = github::gh_device_poll(host.clone(), client.clone(), code.clone());
                        let (done, job): (bool, Job) = match res {
                            Ok(v) => match v["status"].as_str().unwrap_or("") {
                                "ok" => {
                                    let acc = Acc {
                                        key: v["account"]["key"].as_str().unwrap_or("").into(),
                                        host: v["account"]["host"].as_str().unwrap_or("github.com").into(),
                                        login: v["account"]["login"].as_str().unwrap_or("").into(),
                                        name: v["account"]["name"].as_str().unwrap_or("").into(),
                                    };
                                    (true, Box::new(move |app: &mut App| app.add_account(acc)))
                                }
                                "slow_down" => {
                                    wait += 5;
                                    (false, Box::new(|_: &mut App| {}))
                                }
                                "expired" => (true, Box::new(|app: &mut App| { app.login.phase = 4; app.login.error = "Kodun süresi doldu; yeniden dene.".into(); })),
                                "denied" => (true, Box::new(|app: &mut App| { app.login.phase = 4; app.login.error = "Giriş reddedildi.".into(); })),
                                _ => (false, Box::new(|_: &mut App| {})),
                            },
                            Err(e) => (true, Box::new(move |app: &mut App| { app.login.phase = 4; app.login.error = e; })),
                        };
                        let _ = tx.send(job);
                        ctx.request_repaint();
                        if done {
                            return;
                        }
                    }
                });
            }
            Err(e) => {
                app.login.phase = 4;
                app.login.error = e;
            }
        },
    );
}

pub fn login_modal(app: &mut App, ctx: &egui::Context) -> bool {
    let th = app.th.clone();
    let p = th.p.clone();
    let mut cancel = false;
    let mut retry = false;
    let (phase, code, uri, err) = (app.login.phase, app.login.user_code.clone(), app.login.uri.clone(), app.login.error.clone());
    let (_, close) = crate::overlay::dialog(app, ctx, "login", 440.0, |ui| {
        ui.spacing_mut().item_spacing.y = 10.0;
        ui.label(crate::i18n::rt("GitHub ile giriş yap").font(FontId::new(18.0, semibold())).color(p.text1));
        match phase {
            1 => {
                ui.label(crate::i18n::rt("Kod isteniyor…").font(th.f(Ty::Row)).color(p.text2));
            }
            2 => {
                ui.label(crate::i18n::rt("Tarayıcıda açılan sayfaya aşağıdaki kodu gir. Token kopyalamana gerek yok.").font(th.f(Ty::Row)).color(p.text2));
                let (rr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 64.0), Sense::hover());
                ui.painter().rect_filled(rr, cr(8), p.bg0);
                ui.painter().rect_stroke(rr, cr(8), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
                centered(ui.painter(), rr, &code, FontId::new(30.0, crate::theme::monobold()), p.accent_hi);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if button(ui, &th, Btn::new("Kodu kopyala").icon(Icon::Copy)).clicked() {
                        ui.ctx().copy_text(code.clone());
                    }
                    if button(ui, &th, Btn::new("Tarayıcıyı aç").icon(Icon::Link)).clicked() {
                        let _ = open::that(&uri);
                    }
                });
                ui.label(crate::i18n::rt("Onaylayınca bu pencere kendiliğinden kapanır.").font(th.f(Ty::Small)).color(p.text3));
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
            }
            3 => {
                ui.label(crate::i18n::rt("Tarayıcıda GitHub giriş sayfası açıldı. Orada onayla; bu pencere kendiliğinden kapanır.").font(th.f(Ty::Row)).color(p.text2));
                let (rr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 56.0), Sense::hover());
                let t = ui.input(|i| i.time) as f32;
                for k in 0..3 {
                    let a = ((t * 3.0 - k as f32 * 0.6).sin() * 0.5 + 0.5).clamp(0.2, 1.0);
                    ui.painter().circle_filled(rr.center() + vec2((k as f32 - 1.0) * 16.0, 0.0), 4.0, p.accent.gamma_multiply(a));
                }
                ui.label(crate::i18n::rt("Giriş, git ile aynı yerde saklanan Git Credential Manager hesabıyla yapılır. Başka bir hesap için önce tarayıcıda o hesaba geç.").font(th.f(Ty::Small)).color(p.text3));
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            _ => {
                ui.label(crate::i18n::rt(&err).font(th.f(Ty::Row)).color(p.deleted));
                if button(ui, &th, Btn::new("Yeniden dene").primary()).clicked() {
                    retry = true;
                }
            }
        }
        ui.add_space(4.0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if button(ui, &th, Btn::new("Kapat").ghost()).clicked() {
                cancel = true;
            }
        });
    });
    if retry {
        login_start(app);
        return true;
    }
    if cancel || close {
        if let Some(c) = &app.login.cancel {
            c.store(true, Ordering::Relaxed);
        }
        if let Some(pid) = app.login.pid.as_ref().and_then(|p| p.lock().ok().and_then(|g| *g)) {
            let mut kill = std::process::Command::new("taskkill");
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                kill.creation_flags(0x0800_0000);
            }
            let _ = kill.args(["/PID", &pid.to_string(), "/T", "/F"]).status();
        }
        return false;
    }
    // completed: add_account clears the modal
    app.login.phase != 9
}

fn tips_panel(app: &App, ui: &mut Ui, rect: Rect) {
    let th = app.th.clone();
    let p = th.p.clone();
    if rect.width() < 300.0 {
        return;
    }
    let rect = Rect::from_min_size(rect.min, vec2(rect.width(), 360.0f32.min(rect.height())));
    let painter = ui.painter().clone();
    painter.rect_filled(rect, cr(12), p.bg1);
    painter.rect_stroke(rect, cr(12), Stroke::new(1.0, p.line), StrokeKind::Inside);
    draw_left(&painter, rect.left() + 28.0, rect.top() + 38.0, "Kısayollar", FontId::new(18.0, semibold()), p.text1);
    let rows = [("Komut paleti", "Ctrl K"), ("Dal değiştir…", "Ctrl B"), ("Depo klonla…", "Ctrl Shift O"), ("Yerel depo aç…", "Ctrl O"), ("Yeni depo oluştur…", "Ctrl N"), ("Ayarlar", "Ctrl ,")];
    let mut y = rect.top() + 72.0;
    for (label, k) in rows {
        draw_left(&painter, rect.left() + 28.0, y + 14.0, label, th.f(Ty::Row), p.text2);
        let kw = text_width(ui, k, &th.f(Ty::MonoXs)) + 14.0;
        let kr = Rect::from_min_size(Pos2::new(rect.right() - 28.0 - kw, y + 4.0), vec2(kw, 20.0));
        painter.rect_stroke(kr, cr(4), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        centered(&painter, kr, k, th.f(Ty::MonoXs), p.text2);
        hline(&painter, rect.left() + 28.0, rect.right() - 28.0, y + 32.0, p.line_soft);
        y += 40.0;
    }
}
