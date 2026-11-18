//! Pull requests: open PR list and details from the GitHub REST API.

use crate::i18n::{PainterT, RespT};
use crate::app::*;
use crate::backend::github;
use crate::icons::{self, Icon};
use crate::theme::{semibold, Ty};
use crate::ui::*;
use eframe::egui::{self, vec2, Align, FontId, Layout, Pos2, Rect, Sense, Ui, UiBuilder};

#[derive(Clone, Debug, Default)]
pub struct Pr {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub author: String,
    pub head: String,
    pub base: String,
    pub draft: bool,
    pub updated: i64,
    pub url: String,
}

#[derive(Default)]
pub struct PrState {
    pub prs: Vec<Pr>,
    pub loading: bool,
    pub loaded_for: Option<String>,
    pub error: Option<String>,
    pub sel: Option<usize>,
}

fn parse_iso(s: &str) -> i64 {
    // 2026-11-18T08:41:00Z
    let b: Vec<i64> = s.split(|c: char| !c.is_ascii_digit()).filter(|x| !x.is_empty()).filter_map(|x| x.parse().ok()).collect();
    if b.len() < 6 {
        return 0;
    }
    let (y, m, d) = (b[0], b[1], b[2]);
    let (y2, m2) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * m2 + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    days * 86400 + b[3] * 3600 + b[4] * 60 + b[5]
}

pub fn load_prs(app: &mut App) {
    let acc_key = app.account_key();
    let Some(r) = app.repo.as_mut() else { return };
    let Some((o, n)) = r.gh.clone() else {
        r.pr.error = Some("Bu deponun GitHub uzak adresi yok.".into());
        return;
    };
    let key = format!("{o}/{n}");
    if r.pr.loading || r.pr.loaded_for.as_deref() == Some(key.as_str()) {
        return;
    }
    let Some(acc) = acc_key else {
        r.pr.error = Some("GitHub hesabı bağlı değil (Ayarlar > Hesaplar).".into());
        return;
    };
    r.pr.loading = true;
    r.pr.error = None;
    let path = r.path.clone();
    app.spawn(
        move || github::gh_api(acc, "GET".into(), format!("/repos/{o}/{n}/pulls?state=open&per_page=50"), None, Some(1)),
        move |app, res| {
            if !app.repo_is(&path) {
                return;
            }
            let Some(r) = app.repo.as_mut() else { return };
            r.pr.loading = false;
            match res {
                Ok(v) => {
                    r.pr.loaded_for = Some(key);
                    r.pr.prs = v
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .map(|x| Pr {
                                    number: x["number"].as_u64().unwrap_or(0),
                                    title: x["title"].as_str().unwrap_or("").into(),
                                    body: x["body"].as_str().unwrap_or("").into(),
                                    author: x["user"]["login"].as_str().unwrap_or("").into(),
                                    head: x["head"]["ref"].as_str().unwrap_or("").into(),
                                    base: x["base"]["ref"].as_str().unwrap_or("").into(),
                                    draft: x["draft"].as_bool().unwrap_or(false),
                                    updated: parse_iso(x["updated_at"].as_str().unwrap_or("")),
                                    url: x["html_url"].as_str().unwrap_or("").into(),
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    let sample: String = r.pr.prs.iter().map(|x| format!("{}{}", x.title, x.body)).collect();
                    app.ensure_cjk(&sample);
                    let Some(r) = app.repo.as_mut() else { return };
                    if r.pr.sel.is_none() && !r.pr.prs.is_empty() {
                        r.pr.sel = Some(0);
                    }
                }
                Err(e) => r.pr.error = Some(e),
            }
        },
    );
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let th = app.th.clone();
    let p = th.p.clone();
    let full = ui.max_rect();
    let r = app.repo.as_ref().unwrap();
    let left = Rect::from_min_max(full.min, Pos2::new(full.left() + 460.0, full.bottom()));
    let right = Rect::from_min_max(Pos2::new(left.right(), full.top()), full.max);
    let painter = ui.painter().clone();
    painter.rect_filled(left, 0.0, p.bg1);
    vline(&painter, left.right() - 0.5, left.top(), left.bottom(), p.line_soft);
    draw_left(&painter, left.left() + 16.0, left.top() + 28.0, "Pull request'ler", FontId::new(15.0, semibold()), p.text1);
    draw_right(&painter, left.right() - 16.0, left.top() + 29.0, &format!("{} açık", r.pr.prs.len()), th.f(Ty::Small), p.text2);
    let mut open_url: Option<String> = None;
    let mut checkout: Option<String> = None;
    let mut sel_new: Option<usize> = None;
    let mut refresh = false;
    // create PR for the current branch
    let cur = r.branch();
    let can_create = r.gh.is_some() && r.status.as_ref().map(|s| s.upstream.is_some()).unwrap_or(false) && cur != r.default_branch;
    let cb = Rect::from_min_size(Pos2::new(left.left() + 14.0, left.top() + 48.0), vec2(left.width() - 28.0, 32.0));
    let mut bui = ui.new_child(UiBuilder::new().max_rect(cb).layout(Layout::left_to_right(Align::Center)));
    if button(&mut bui, &th, Btn::new(format!("“{cur}” için PR oluştur")).icon(Icon::Pr).primary().min_w(left.width() - 28.0).enabled(can_create)).clicked() {
        if let Some((o, n)) = &r.gh {
            open_url = Some(format!("https://github.com/{o}/{n}/compare/{cur}?expand=1"));
        }
    }
    let area = Rect::from_min_max(Pos2::new(left.left() + 6.0, left.top() + 92.0), left.max - vec2(7.0, 0.0));
    if r.pr.loading {
        painter.text_t(area.center(), egui::Align2::CENTER_CENTER, "Yükleniyor…", th.f(Ty::Row), p.text3);
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
    } else if let Some(e) = &r.pr.error {
        let g = painter.layout(crate::i18n::t(e).into_owned(), th.f(Ty::Row), p.text2, area.width() - 40.0);
        let sz = g.size();
        painter.galley(area.center() - sz / 2.0, g, p.text2);
    } else if r.pr.prs.is_empty() {
        painter.text_t(area.center(), egui::Align2::CENTER_CENTER, "Açık pull request yok", th.f(Ty::Row), p.text3);
    }
    let mut lui = ui.new_child(UiBuilder::new().max_rect(area).layout(Layout::top_down(Align::Min)));
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut lui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for (i, pr) in r.pr.prs.iter().enumerate() {
            let (rr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 56.0), Sense::click());
            let sel = r.pr.sel == Some(i);
            if sel {
                ui.painter().rect_filled(rr, cr(8), p.bg_sel);
            } else if resp.hovered() {
                ui.painter().rect_filled(rr, cr(8), p.bg3);
            }
            icons::paint_at(ui, Pos2::new(rr.left() + 22.0, rr.center().y), 16.0, Icon::Pr, if pr.draft { p.text3 } else { p.added });
            draw_trunc(ui.painter(), rr.left() + 44.0, rr.center().y - 9.0, rr.width() - 56.0, &pr.title, th.f(Ty::RowStrong), p.text1);
            draw_trunc(ui.painter(), rr.left() + 44.0, rr.center().y + 10.0, rr.width() - 56.0, &format!("#{} · {} · {}", pr.number, pr.author, crate::util::rel(pr.updated)), th.f(Ty::Small), p.text3);
            pointer(&resp);
            if resp.clicked() {
                sel_new = Some(i);
            }
        }
    });
    if let Some(pr) = r.pr.sel.and_then(|i| r.pr.prs.get(i)) {
        let x0 = right.left() + 24.0;
        let w = right.width() - 48.0;
        let mut y = right.top() + 28.0;
        let g = painter.layout(pr.title.clone(), FontId::new(20.0, semibold()), p.text1, w);
        let gh = g.size().y;
        painter.galley(Pos2::new(x0, y), g, p.text1);
        y += gh + 10.0;
        let mut x = x0;
        for (t, bg, fg) in [(format!("#{}", pr.number), p.bg3, p.text2), (pr.head.clone(), p.tint_a(p.merged, 0x26), p.merged), (format!("→ {}", pr.base), p.accent_soft, p.accent_hi)] {
            let f = th.f(Ty::MonoXs);
            let cw = text_width(ui, &t, &f) + 16.0;
            let rr = Rect::from_min_size(Pos2::new(x, y), vec2(cw, 22.0));
            painter.rect_filled(rr, cr(255), bg);
            centered(&painter, rr, &t, f, fg);
            x += cw + 8.0;
        }
        if pr.draft {
            let rr = Rect::from_min_size(Pos2::new(x, y), vec2(54.0, 22.0));
            painter.rect_filled(rr, cr(255), p.bg3);
            centered(&painter, rr, "Taslak", FontId::new(11.0, semibold()), p.text2);
        }
        y += 22.0 + 12.0;
        avatar(&painter, &p, Pos2::new(x0 + 11.0, y + 11.0), 22.0, &initials_of(&pr.author));
        draw_left(&painter, x0 + 30.0, y + 11.0, &format!("{} · {} güncelledi", pr.author, crate::util::rel(pr.updated)), th.f(Ty::Small), p.text2);
        y += 22.0 + 14.0;
        let ar = Rect::from_min_size(Pos2::new(x0, y), vec2(w, 28.0));
        let mut aui = ui.new_child(UiBuilder::new().max_rect(ar).layout(Layout::left_to_right(Align::Center)));
        aui.spacing_mut().item_spacing.x = 8.0;
        if button(&mut aui, &th, Btn::new("Tarayıcıda aç").icon(Icon::Link).small()).clicked() {
            open_url = Some(pr.url.clone());
        }
        if button(&mut aui, &th, Btn::new("Dalı al").icon(Icon::Branch).small()).clicked() {
            checkout = Some(pr.head.clone());
        }
        if button(&mut aui, &th, Btn::new("").icon(Icon::Sync).small().min_w(28.0)).on_hover_text_t("Yenile").clicked() {
            refresh = true;
        }
        y += 28.0 + 14.0;
        hline(&painter, right.left(), right.right(), y, p.line_soft);
        y += 14.0;
        let body = if pr.body.trim().is_empty() { "Açıklama yok.".to_string() } else { pr.body.clone() };
        let g = painter.layout(crate::i18n::t(&body).into_owned(), th.f(Ty::Row), p.text2, w);
        let clip = painter.with_clip_rect(Rect::from_min_max(Pos2::new(x0, y), right.max - vec2(24.0, 16.0)));
        clip.galley(Pos2::new(x0, y), g, p.text2);
    } else if !r.pr.loading && r.pr.error.is_none() {
        painter.text_t(right.center(), egui::Align2::CENTER_CENTER, "Ayrıntı için bir PR seç", FontId::new(14.0, semibold()), p.text2);
    }
    if let Some(i) = sel_new {
        if let Some(r) = app.repo.as_mut() {
            r.pr.sel = Some(i);
        }
    }
    if let Some(u) = open_url {
        let _ = open::that(u);
    }
    if let Some(b) = checkout {
        app.run(Cmd::SwitchTo(b));
    }
    if refresh {
        if let Some(r) = app.repo.as_mut() {
            r.pr.loaded_for = None;
        }
        load_prs(app);
    }
}
