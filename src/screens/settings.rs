//! Settings dialog: appearance, git identity, accounts.

use crate::app::*;
use crate::theme::{semibold, Mode, Ty};
use crate::ui::*;
use eframe::egui::{self, vec2, Align, FontId, Id, Layout, Pos2, Rect, Sense};

enum Do {
    Tab(usize),
    SaveIdentity(String, String),
    Login,
    Logout(String),
    Activate(String),
    Close,
}

pub fn show(app: &mut App, ctx: &egui::Context, m: &mut Modal) -> bool {
    let Modal::Settings(tab) = m else { return false };
    let th = app.th.clone();
    let p = th.p.clone();
    let mut s = app.s.clone();
    let mut todo: Vec<Do> = vec![];
    let cur_tab = *tab;
    let (name0, email0) = app.git_user.clone();
    let editors = app.editors.clone();
    let accounts = app.s.accounts.clone();
    let active = app.s.account.as_ref().map(|a| a.key.clone());
    let (_, close) = crate::overlay::dialog(app, ctx, "settings", 700.0, |ui| {
        ui.set_min_height(430.0);
        let top = ui.cursor().min;
        let full = Rect::from_min_size(top, vec2(700.0, 430.0));
        ui.horizontal(|ui| {
            ui.label(crate::i18n::rt("Ayarlar").font(FontId::new(18.0, semibold())).color(p.text1));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button(ui, &th, Btn::new("").icon(crate::icons::Icon::X).ghost().small().min_w(28.0)).clicked() {
                    todo.push(Do::Close);
                }
            });
        });
        ui.add_space(8.0);
        let body_top = ui.cursor().min.y;
        let side = Rect::from_min_size(Pos2::new(full.left(), body_top), vec2(150.0, 360.0));
        for (i, t) in ["Görünüm", "Git", "Hesaplar"].iter().enumerate() {
            let rr = Rect::from_min_size(side.min + vec2(0.0, i as f32 * 34.0), vec2(150.0, 32.0));
            let resp = ui.interact(rr, Id::new(("set-tab", i)), Sense::click());
            if cur_tab == i {
                ui.painter().rect_filled(rr, cr(6), p.bg3);
            } else if resp.hovered() {
                ui.painter().rect_filled(rr, cr(6), p.bg1);
            }
            draw_left(ui.painter(), rr.left() + 12.0, rr.center().y, t, if cur_tab == i { FontId::new(13.0, semibold()) } else { th.f(Ty::Row) }, if cur_tab == i { p.text1 } else { p.text2 });
            pointer(&resp);
            if resp.clicked() {
                todo.push(Do::Tab(i));
            }
        }
        vline(ui.painter(), side.right() + 8.0, body_top, body_top + 360.0, p.line_soft);
        let content = Rect::from_min_max(Pos2::new(side.right() + 24.0, body_top), Pos2::new(full.right(), body_top + 360.0));
        let mut cui = ui.new_child(egui::UiBuilder::new().max_rect(content).layout(Layout::top_down(Align::Min)));
        cui.spacing_mut().item_spacing.y = 10.0;
        let row_label = |ui: &mut egui::Ui, t: &str| {
            ui.label(crate::i18n::rt(t).font(FontId::new(12.0, semibold())).color(p.text2));
        };
        match cur_tab {
            0 => {
                row_label(&mut cui, "Tema");
                cui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for (label, mode) in [("Koyu", Mode::Dark), ("Açık", Mode::Light), ("Sistem", Mode::System)] {
                        if pill(ui, &th, label, s.mode == mode).clicked() {
                            s.mode = mode;
                        }
                    }
                });
                row_label(&mut cui, "Arayüz ölçeği");
                cui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for v in [0.9f32, 1.0, 1.1, 1.25, 1.5] {
                        if pill(ui, &th, &if crate::i18n::is_en() { format!("{}%", (v * 100.0) as u32) } else { format!("%{}", (v * 100.0) as u32) }, (s.scale - v).abs() < 0.01).clicked() {
                            s.scale = v;
                        }
                    }
                });
                row_label(&mut cui, "Dil");
                cui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for (label, l) in [("Otomatik", crate::i18n::Lang::Auto), ("Türkçe", crate::i18n::Lang::Tr), ("English", crate::i18n::Lang::En)] {
                        if pill(ui, &th, label, s.lang == l).clicked() {
                            s.lang = l;
                        }
                    }
                });
                row_label(&mut cui, "Liste yoğunluğu");
                cui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if pill(ui, &th, "Rahat", !s.compact).clicked() {
                        s.compact = false;
                    }
                    if pill(ui, &th, "Sıkışık", s.compact).clicked() {
                        s.compact = true;
                    }
                });
                row_label(&mut cui, "Diff bağlam satırı");
                cui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for v in [1u32, 3, 5, 10] {
                        if pill(ui, &th, &v.to_string(), s.context == v).clicked() {
                            s.context = v;
                        }
                    }
                });
            }
            1 => {
                row_label(&mut cui, "Git kimliği (genel)");
                let mut name: String = ctx.data_mut(|d| d.get_temp(Id::new("set-name"))).unwrap_or(name0.clone());
                let mut email: String = ctx.data_mut(|d| d.get_temp(Id::new("set-email"))).unwrap_or(email0.clone());
                text_input(&mut cui, &th, &mut name, InputOpts::new(Id::new("set-name-in"), "Ad Soyad").width(380.0));
                text_input(&mut cui, &th, &mut email, InputOpts::new(Id::new("set-email-in"), "e-posta").width(380.0));
                ctx.data_mut(|d| {
                    d.insert_temp(Id::new("set-name"), name.clone());
                    d.insert_temp(Id::new("set-email"), email.clone());
                });
                let dirty = name != name0 || email != email0;
                if button(&mut cui, &th, Btn::new("Kimliği kaydet").primary().enabled(dirty)).clicked() {
                    todo.push(Do::SaveIdentity(name, email));
                }
                row_label(&mut cui, "Varsayılan editör");
                cui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    if editors.is_empty() {
                        ui.label(crate::i18n::rt("Desteklenen editör bulunamadı").font(th.f(Ty::Row)).color(p.text3));
                    }
                    for e in &editors {
                        if pill(ui, &th, &e.name, s.editor == e.id).clicked() {
                            s.editor = e.id.clone();
                        }
                    }
                });
                row_label(&mut cui, "Güvenlik");
                cui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let st = if s.confirm_force { Cb::On } else { Cb::Off };
                    if checkbox(ui, &th, st).clicked() {
                        s.confirm_force = !s.confirm_force;
                    }
                    ui.label(crate::i18n::rt("Force push öncesi onay iste").font(th.f(Ty::Row)).color(p.text1));
                });
            }
            _ => {
                row_label(&mut cui, "GitHub hesapları");
                if accounts.is_empty() {
                    cui.label(crate::i18n::rt("Bağlı hesap yok. Token girmen gerekmez: tarayıcıda onaylarsın.").font(th.f(Ty::Row)).color(p.text3));
                }
                for a in &accounts {
                    let (rr, _) = cui.allocate_exact_size(vec2(content.width(), 52.0), Sense::hover());
                    cui.painter().rect_filled(rr, cr(8), p.bg1);
                    cui.painter().rect_stroke(rr, cr(8), egui::Stroke::new(1.0, if active.as_deref() == Some(a.key.as_str()) { p.accent_dim } else { p.line }), egui::StrokeKind::Inside);
                    avatar(cui.painter(), &p, Pos2::new(rr.left() + 26.0, rr.center().y), 28.0, &initials_of(if a.name.is_empty() { &a.login } else { &a.name }));
                    draw_left(cui.painter(), rr.left() + 50.0, rr.center().y - 8.0, if a.name.is_empty() { &a.login } else { &a.name }, FontId::new(13.0, semibold()), p.text1);
                    draw_left(cui.painter(), rr.left() + 50.0, rr.center().y + 9.0, &format!("@{} · {}", a.login, a.host), th.f(Ty::Small), p.text3);
                    let mut rx = rr.right() - 10.0;
                    let lw = 66.0;
                    let lr = Rect::from_min_size(Pos2::new(rx - lw, rr.center().y - 12.0), vec2(lw, 24.0));
                    rx = lr.left() - 6.0;
                    let mut lui = cui.new_child(egui::UiBuilder::new().max_rect(lr).layout(Layout::left_to_right(Align::Center)));
                    if button(&mut lui, &th, Btn::new("Çıkış").danger().tiny().min_w(lw)).clicked() {
                        todo.push(Do::Logout(a.key.clone()));
                    }
                    if active.as_deref() != Some(a.key.as_str()) {
                        let ar = Rect::from_min_size(Pos2::new(rx - 86.0, rr.center().y - 12.0), vec2(86.0, 24.0));
                        let mut aui = cui.new_child(egui::UiBuilder::new().max_rect(ar).layout(Layout::left_to_right(Align::Center)));
                        if button(&mut aui, &th, Btn::new("Etkin yap").tiny().min_w(86.0)).clicked() {
                            todo.push(Do::Activate(a.key.clone()));
                        }
                    } else {
                        draw_right(cui.painter(), rx, rr.center().y, "etkin", FontId::new(11.0, semibold()), p.accent_hi);
                    }
                }
                if button(&mut cui, &th, Btn::new("Hesap ekle").icon(crate::icons::Icon::Plus).primary()).clicked() {
                    todo.push(Do::Login);
                }
            }
        }
    });
    let lang_changed = s.lang != app.s.lang;
    if lang_changed {
        crate::i18n::set(s.lang);
    }
    let changed = lang_changed || s.mode != app.s.mode || (s.scale - app.s.scale).abs() > 0.001 || s.compact != app.s.compact || s.context != app.s.context || s.editor != app.s.editor || s.confirm_force != app.s.confirm_force;
    if changed {
        let theme_changed = s.mode != app.s.mode || (s.scale - app.s.scale).abs() > 0.001 || s.compact != app.s.compact;
        let ctx_changed = s.context != app.s.context;
        app.s = s;
        if theme_changed {
            app.apply_theme();
        } else {
            app.save_settings();
        }
        if ctx_changed {
            app.load_file_view();
        }
    }
    let mut keep = !close;
    for t in todo {
        match t {
            Do::Tab(i) => *tab = i,
            Do::Close => keep = false,
            Do::SaveIdentity(n, e) => {
                app.spawn_busy(
                    move || {
                        crate::backend::repo::config_set(None, "user.name".into(), n.clone(), true)?;
                        crate::backend::repo::config_set(None, "user.email".into(), e.clone(), true)?;
                        Ok::<_, String>((n, e))
                    },
                    |app, r| match r {
                        Ok(v) => {
                            app.git_user = v;
                            app.toast(ToastKind::Ok, "Git kimliği kaydedildi");
                        }
                        Err(e) => app.toast(ToastKind::Err, e),
                    },
                );
            }
            Do::Login => {
                crate::screens::welcome::login_start(app);
                return true;
            }
            Do::Logout(k) => app.remove_account(&k),
            Do::Activate(k) => {
                app.s.account = app.s.accounts.iter().find(|a| a.key == k).cloned();
                app.save_settings();
            }
        }
    }
    keep
}
