//! Command palette (Ctrl K) and branch switcher (Ctrl B).

use crate::app::*;
use crate::icons::{self, Icon};
use crate::theme::{semibold, Ty};
use crate::ui::*;
use eframe::egui::{self, vec2, Align, Color32, FontId, Id, Key, Layout, Modifiers, Order, Pos2, Rect, Sense, Stroke, StrokeKind, UiBuilder};

#[derive(Default)]
pub struct PaletteState {
    pub query: String,
    pub sel: usize,
}

#[derive(Default)]
pub struct SwitcherState {
    pub query: String,
    pub sel: usize,
    pub pending: Option<String>,
}

struct Item {
    title: String,
    sub: String,
    icon: Icon,
    cmd: Cmd,
    kbd: Option<&'static str>,
    git: Option<String>,
    group: &'static str,
}

fn matches(hay: &str, q: &str) -> bool {
    let h = hay.to_lowercase();
    q.to_lowercase().split_whitespace().all(|w| h.contains(w))
}

fn palette_items(app: &App, q: &str) -> Vec<Item> {
    let mut v: Vec<Item> = vec![];
    let has_repo = app.repo.is_some();
    let cmd_icon = |c: &Cmd| match c {
        Cmd::Commit | Cmd::CommitPush => Icon::Commit,
        Cmd::Push | Cmd::ForcePush | Cmd::PublishRepo => Icon::Up,
        Cmd::Pull => Icon::Down,
        Cmd::Fetch | Cmd::Refresh => Icon::Sync,
        Cmd::Undo => Icon::Undo,
        Cmd::NewBranch | Cmd::SwitchTo(_) | Cmd::Switcher => Icon::Branch,
        Cmd::Goto(Screen::Changes) => Icon::Diff,
        Cmd::Goto(Screen::History) => Icon::History,
        Cmd::Goto(Screen::Branches) => Icon::Branch,
        Cmd::Goto(Screen::Pr) => Icon::Pr,
        Cmd::Goto(Screen::Stash) | Cmd::StashAll | Cmd::StashPop(_) => Icon::Stash,
        Cmd::Goto(_) => Icon::Undo,
        Cmd::OpenFolder | Cmd::Reveal | Cmd::OpenRecent(_) => Icon::Folder,
        Cmd::CloneRepo | Cmd::NewRepo => Icon::Clone,
        Cmd::Settings => Icon::Gear,
        Cmd::OpenTerminal | Cmd::CopyCommand => Icon::Terminal,
        Cmd::OpenEditor => Icon::Edit,
        _ => Icon::Spark,
    };
    let mut cmds: Vec<Cmd> = vec![];
    if has_repo {
        cmds.extend([Cmd::Commit, Cmd::CommitPush, Cmd::Push, Cmd::PublishRepo, Cmd::Pull, Cmd::Fetch, Cmd::Undo, Cmd::Redo, Cmd::NewBranch, Cmd::Switcher, Cmd::StageAll, Cmd::UnstageAll, Cmd::StashAll, Cmd::ToggleAmend, Cmd::Goto(Screen::Changes), Cmd::Goto(Screen::History), Cmd::Goto(Screen::Branches), Cmd::Goto(Screen::Pr), Cmd::Goto(Screen::Stash), Cmd::Goto(Screen::Log), Cmd::OpenEditor, Cmd::OpenTerminal, Cmd::Reveal, Cmd::Refresh, Cmd::ForcePush, Cmd::CopyCommand, Cmd::CloseRepo]);
    }
    cmds.extend([Cmd::OpenFolder, Cmd::CloneRepo, Cmd::NewRepo, Cmd::Settings, Cmd::ToggleTheme]);
    for c in cmds {
        let t = crate::i18n::t(&c.title()).into_owned();
        if q.is_empty() || matches(&t, q) {
            v.push(Item { icon: cmd_icon(&c), sub: String::new(), kbd: c.shortcut(), git: c.git_preview(), title: t, cmd: c, group: "KOMUTLAR" });
        }
    }
    if let Some(r) = app.repo.as_ref() {
        if !q.is_empty() {
            for b in r.branches.iter().filter(|b| !b.is_remote && !b.current).filter(|b| matches(&b.name, q)).take(6) {
                let c = Cmd::SwitchTo(b.name.clone());
                v.push(Item { title: format!("Dala geç: {}", b.name), sub: b.subject.clone(), icon: Icon::Branch, kbd: None, git: c.git_preview(), cmd: c, group: "DALLAR" });
            }
            for e in r.entries().iter().filter(|e| matches(&e.path, q)).take(6) {
                v.push(Item { title: e.path.clone(), sub: "değişen dosya".into(), icon: Icon::Diff, kbd: None, git: None, cmd: Cmd::SelectFile(e.path.clone()), group: "DOSYALAR" });
            }
        }
    }
    for rp in app.s.recents.iter().filter(|x| q.is_empty() || matches(&x.name, q) || matches(&x.path, q)).take(5) {
        if app.repo.as_ref().map(|r| r.path == rp.path).unwrap_or(false) {
            continue;
        }
        v.push(Item { title: format!("Depoyu aç: {}", rp.name), sub: rp.path.clone(), icon: Icon::Folder, kbd: None, git: None, cmd: Cmd::OpenRecent(rp.path.clone()), group: "DEPOLAR" });
    }
    let order = |g: &str| match g { "KOMUTLAR" => 0, "DALLAR" => 1, "DOSYALAR" => 2, _ => 3 };
    v.sort_by_key(|i| order(i.group));
    v
}

/// Dim + centered-top card. Returns whether it should close.
fn card(app: &App, ctx: &egui::Context, width: f32, id: &str, add: impl FnOnce(&mut egui::Ui)) -> bool {
    let p = &app.th.p;
    let screen = ctx.screen_rect();
    let mut close = false;
    egui::Area::new(Id::new((id, "dim"))).order(Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
        let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
        ui.painter().rect_filled(rect, 0.0, Color32::from_black_alpha(if p.dark { 130 } else { 70 }));
        if resp.clicked() {
            close = true;
        }
    });
    egui::Area::new(Id::new((id, "card"))).order(Order::Foreground).fixed_pos(Pos2::new(screen.center().x - width / 2.0, screen.top() + 96.0)).show(ctx, |ui| {
        egui::Frame::new()
            .fill(p.bg3)
            .stroke(Stroke::new(1.0, p.line_strong))
            .corner_radius(cr(12))
            .shadow(egui::epaint::Shadow { offset: [0, 16], blur: 48, spread: 0, color: Color32::from_black_alpha(160) })
            .show(ui, |ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                add(ui);
            });
    });
    close
}

fn search_row(app: &App, ui: &mut egui::Ui, text: &mut String, hint: &str, id: &str) -> egui::Response {
    let p = &app.th.p;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::hover());
    icons::paint_at(ui, Pos2::new(rect.left() + 26.0, rect.center().y), 16.0, Icon::Search, p.text3);
    let te = Rect::from_min_max(rect.min + vec2(46.0, 0.0), rect.max - vec2(70.0, 0.0));
    let mut c = ui.new_child(UiBuilder::new().max_rect(te).layout(Layout::left_to_right(Align::Center)));
    let r = c.add(egui::TextEdit::singleline(text).id(Id::new(id)).hint_text(crate::i18n::rt(hint).color(p.text3).font(FontId::new(15.0, egui::FontFamily::Proportional))).font(FontId::new(15.0, egui::FontFamily::Proportional)).frame(false).desired_width(te.width()).text_color(p.text1).vertical_align(Align::Center));
    r.request_focus();
    let kr = Rect::from_min_size(Pos2::new(rect.right() - 54.0, rect.center().y - 9.0), vec2(38.0, 18.0));
    ui.painter().rect_stroke(kr, cr(4), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
    centered(ui.painter(), kr, "Esc", app.th.f(Ty::MonoXs), p.text2);
    hline(ui.painter(), rect.left(), rect.right(), rect.bottom(), p.line_soft);
    r
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if app.palette.is_some() {
        show_palette(app, ctx);
    } else if app.switcher.is_some() {
        show_switcher(app, ctx);
    }
}

fn show_palette(app: &mut App, ctx: &egui::Context) {
    let mut st = app.palette.take().unwrap();
    let items = palette_items(app, &st.query);
    let n = items.len();
    let (mut down, mut up, mut enter, mut esc) = (false, false, false, false);
    ctx.input_mut(|i| {
        down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
        up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
        enter = i.consume_key(Modifiers::NONE, Key::Enter);
        esc = i.consume_key(Modifiers::NONE, Key::Escape);
    });
    if down && n > 0 {
        st.sel = (st.sel + 1).min(n - 1);
    }
    if up {
        st.sel = st.sel.saturating_sub(1);
    }
    let prev_q = st.query.clone();
    let mut picked: Option<usize> = if enter && n > 0 { Some(st.sel.min(n - 1)) } else { None };
    let p = app.th.p.clone();
    let close = card(app, ctx, 640.0, "palette", |ui| {
        search_row(app, ui, &mut st.query, "Komut, dosya, dal veya depo ara", "palette-input");
        let list_h = 360.0f32.min(n as f32 * 40.0 + 40.0);
        egui::ScrollArea::vertical().max_height(list_h).auto_shrink([false, true]).show(ui, |ui| {
            ui.add_space(6.0);
            let mut last_group = "";
            for (i, it) in items.iter().enumerate() {
                if it.group != last_group {
                    last_group = it.group;
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
                    draw_left(ui.painter(), r.left() + 18.0, r.center().y + 2.0, it.group, FontId::new(11.0, semibold()), p.text3);
                }
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
                let r = r.shrink2(vec2(6.0, 0.0));
                let sel = i == st.sel.min(n.saturating_sub(1));
                if sel {
                    ui.painter().rect_filled(r, cr(6), p.bg_sel);
                    ui.painter().rect_stroke(r, cr(6), Stroke::new(1.0, p.accent_dim), StrokeKind::Inside);
                    ui.scroll_to_rect(r, None);
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, cr(6), p.bg2);
                }
                icons::paint_at(ui, Pos2::new(r.left() + 20.0, r.center().y), 16.0, it.icon, if sel { p.accent } else { p.text2 });
                let mut rx = r.right() - 12.0;
                if let Some(k) = it.kbd {
                    rx = draw_right(ui.painter(), rx, r.center().y, k, app.th.f(Ty::MonoXs), p.text3) - 12.0;
                }
                let tx = r.left() + 40.0;
                let tw = draw_trunc(ui.painter(), tx, r.center().y, rx - tx - 20.0, &it.title, app.th.f(Ty::Row), p.text1);
                if !it.sub.is_empty() && rx - tw > 80.0 {
                    draw_trunc(ui.painter(), tw + 10.0, r.center().y, rx - tw - 14.0, &it.sub, app.th.f(Ty::Small), p.text3);
                }
                pointer(&resp);
                if resp.hovered() && ui.input(|i| i.pointer.delta().length() > 0.5) {
                    st.sel = i;
                }
                if resp.clicked() {
                    picked = Some(i);
                }
            }
            if items.is_empty() {
                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 60.0), Sense::hover());
                centered(ui.painter(), r, "Sonuç yok", app.th.f(Ty::Row), p.text3);
            }
            ui.add_space(6.0);
        });
        // footer: git preview
        let (fr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
        hline(ui.painter(), fr.left(), fr.right(), fr.top(), p.line_soft);
        let sel_item = items.get(st.sel.min(n.saturating_sub(1)));
        if let Some(it) = sel_item {
            if let Some(g) = &it.git {
                icons::paint_at(ui, Pos2::new(fr.left() + 22.0, fr.center().y), 12.0, Icon::Terminal, p.text3);
                draw_trunc(ui.painter(), fr.left() + 36.0, fr.center().y, fr.width() - 230.0, g, app.th.f(Ty::MonoXs), p.text2);
            }
        }
        draw_right(ui.painter(), fr.right() - 16.0, fr.center().y, "↑↓ gezin   ↵ çalıştır", app.th.f(Ty::Small), p.text3);
    });
    if st.query != prev_q {
        st.sel = 0;
    }
    if esc || close {
        return;
    }
    if let Some(i) = picked {
        let c = items[i].cmd.clone();
        app.run(c);
        return;
    }
    app.palette = Some(st);
}

fn show_switcher(app: &mut App, ctx: &egui::Context) {
    let mut st = app.switcher.take().unwrap();
    let Some(r) = app.repo.as_ref() else { return };
    let p = app.th.p.clone();
    let current = r.branch();
    let dirty = r.dirty();
    let n_changes = r.entries().len();
    let mut esc = false;
    let mut enter = false;
    let (mut down, mut up) = (false, false);
    ctx.input_mut(|i| {
        esc = i.consume_key(Modifiers::NONE, Key::Escape);
        enter = i.consume_key(Modifiers::NONE, Key::Enter);
        down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
        up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
    });
    if let Some(target) = st.pending.clone() {
        let mut act: Option<u8> = None;
        let close = card(app, ctx, 520.0, "switch-dirty", |ui| {
            ui.add_space(18.0);
            let (r0, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
            icons::paint_at(ui, Pos2::new(r0.left() + 30.0, r0.center().y), 16.0, Icon::Alert, p.modified);
            draw_left(ui.painter(), r0.left() + 48.0, r0.center().y, &format!("{n_changes} kaydedilmemiş değişiklik var"), FontId::new(15.0, semibold()), p.text1);
            let (r1, _) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::hover());
            let g = ui.painter().layout(crate::i18n::t(&format!("“{target}” dalına geçmeden önce ne yapılsın?")).into_owned(), app.th.f(Ty::Row), p.text2, r1.width() - 48.0);
            ui.painter().galley(Pos2::new(r1.left() + 24.0, r1.top() + 4.0), g, p.text2);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.spacing_mut().item_spacing.x = 8.0;
                if button(ui, &app.th, Btn::new("Stash'le ve geç").primary()).clicked() {
                    act = Some(1);
                }
                if button(ui, &app.th, Btn::new("Değişiklikleri taşı")).clicked() {
                    act = Some(2);
                }
                if button(ui, &app.th, Btn::new("İptal").ghost()).clicked() {
                    act = Some(0);
                }
            });
            ui.add_space(18.0);
        });
        if esc || close || act == Some(0) {
            return;
        }
        match act {
            Some(1) => {
                app.switch_branch(target, true);
                return;
            }
            Some(2) => {
                app.switch_branch(target, false);
                return;
            }
            _ => {}
        }
        app.switcher = Some(st);
        return;
    }

    let q = st.query.trim().to_string();
    let mut list: Vec<(String, String, u32, u32, i64, bool, bool)> = vec![]; // name, subject, ahead, behind, ts, current, remote
    for b in r.branches.iter().filter(|b| !b.name.ends_with("/HEAD")) {
        if !q.is_empty() && !matches(&b.name, &q) {
            continue;
        }
        if b.is_remote && r.branches.iter().any(|l| !l.is_remote && Some(l.name.as_str()) == b.name.split_once('/').map(|x| x.1)) {
            continue;
        }
        list.push((b.name.clone(), b.subject.clone(), b.ahead, b.behind, crate::util::parse_ts(&b.date), b.current, b.is_remote));
    }
    list.sort_by(|a, b| b.5.cmp(&a.5).then(a.6.cmp(&b.6)).then(b.4.cmp(&a.4)));
    let can_create = !q.is_empty() && !list.iter().any(|x| x.0 == q);
    let total = list.len() + if can_create { 1 } else { 0 };
    if down && total > 0 {
        st.sel = (st.sel + 1).min(total - 1);
    }
    if up {
        st.sel = st.sel.saturating_sub(1);
    }
    let mut picked: Option<usize> = if enter && total > 0 { Some(st.sel.min(total - 1)) } else { None };
    let prev_q = st.query.clone();
    let close = card(app, ctx, 560.0, "switcher", |ui| {
        search_row(app, ui, &mut st.query, "Dal ara veya yeni dal adı yaz", "switcher-input");
        let h = (total as f32 * 44.0 + 12.0).min(380.0).max(60.0);
        egui::ScrollArea::vertical().max_height(h).auto_shrink([false, true]).show(ui, |ui| {
            ui.add_space(6.0);
            for (i, (name, subject, ahead, behind, ts, cur, remote)) in list.iter().enumerate() {
                let (rr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
                let rr = rr.shrink2(vec2(6.0, 0.0));
                let sel = i == st.sel.min(total.saturating_sub(1));
                if sel {
                    ui.painter().rect_filled(rr, cr(6), p.bg_sel);
                    ui.scroll_to_rect(rr, None);
                } else if resp.hovered() {
                    ui.painter().rect_filled(rr, cr(6), p.bg2);
                }
                icons::paint_at(ui, Pos2::new(rr.left() + 20.0, rr.center().y), 16.0, Icon::Branch, if *cur { p.accent } else { p.text2 });
                let mut rx = rr.right() - 12.0;
                if *cur {
                    rx = draw_right(ui.painter(), rx, rr.center().y, "mevcut", FontId::new(11.0, semibold()), p.accent_hi) - 10.0;
                } else {
                    rx = draw_right(ui.painter(), rx, rr.center().y, &crate::util::rel(*ts), app.th.f(Ty::Small), p.text3) - 10.0;
                }
                if *behind > 0 {
                    rx = draw_right(ui.painter(), rx, rr.center().y, &format!("↓{behind}"), app.th.f(Ty::MonoXs), p.renamed) - 6.0;
                }
                if *ahead > 0 {
                    rx = draw_right(ui.painter(), rx, rr.center().y, &format!("↑{ahead}"), app.th.f(Ty::MonoXs), p.added) - 6.0;
                }
                let tx = rr.left() + 40.0;
                draw_trunc(ui.painter(), tx, rr.center().y - 8.0, rx - tx, name, FontId::new(13.0, crate::theme::monobold()), p.text1);
                let sub = if *remote { format!("uzak · {subject}") } else { subject.clone() };
                draw_trunc(ui.painter(), tx, rr.center().y + 9.0, rx - tx, &sub, app.th.f(Ty::Small), p.text3);
                pointer(&resp);
                if resp.clicked() {
                    picked = Some(i);
                }
            }
            if can_create {
                let i = list.len();
                let (rr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
                let rr = rr.shrink2(vec2(6.0, 0.0));
                if i == st.sel.min(total - 1) {
                    ui.painter().rect_filled(rr, cr(6), p.bg_sel);
                } else if resp.hovered() {
                    ui.painter().rect_filled(rr, cr(6), p.bg2);
                }
                icons::paint_at(ui, Pos2::new(rr.left() + 20.0, rr.center().y), 16.0, Icon::Plus, p.accent);
                draw_trunc(ui.painter(), rr.left() + 40.0, rr.center().y, rr.width() - 60.0, &format!("Yeni dal oluştur: {q}"), app.th.f(Ty::Row), p.text1);
                pointer(&resp);
                if resp.clicked() {
                    picked = Some(i);
                }
            }
            if total == 0 {
                let (r2, _) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::hover());
                centered(ui.painter(), r2, "Dal bulunamadı", app.th.f(Ty::Row), p.text3);
            }
            ui.add_space(6.0);
        });
        let (fr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
        hline(ui.painter(), fr.left(), fr.right(), fr.top(), p.line_soft);
        let note = if dirty { format!("{n_changes} değişiklik var; geçişte seçenek sorulur") } else { "Değişiklik yok; doğrudan geçilir".to_string() };
        draw_left(ui.painter(), fr.left() + 18.0, fr.center().y, &format!("{current}  ·  {note}"), app.th.f(Ty::Small), p.text3);
        draw_right(ui.painter(), fr.right() - 16.0, fr.center().y, "↑↓ gezin   ↵ geç", app.th.f(Ty::Small), p.text3);
    });
    if st.query != prev_q {
        st.sel = 0;
    }
    if esc || close {
        return;
    }
    if let Some(i) = picked {
        if i < list.len() {
            let (name, remote) = (list[i].0.clone(), list[i].6);
            if list[i].5 {
                return;
            }
            let target = if remote { name.split_once('/').map(|x| x.1.to_string()).unwrap_or(name) } else { name };
            if dirty {
                st.pending = Some(target);
                app.switcher = Some(st);
            } else {
                app.switch_branch(target, false);
            }
        } else {
            app.create_branch(q.replace(' ', "-"));
        }
        return;
    }
    app.switcher = Some(st);
}
