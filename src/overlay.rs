//! Floating layers: context menus, toasts, modal dialogs.

use crate::app::*;
use crate::icons::{self, Icon};
use crate::theme::{semibold, Ty};
use crate::ui::*;
use eframe::egui::{self, vec2, Align2, Color32, FontId, Id, Order, Pos2, Rect, Sense, Stroke, StrokeKind};

pub fn menu_frame(app: &App) -> egui::Frame {
    egui::Frame::new()
        .fill(app.th.p.bg3)
        .stroke(Stroke::new(1.0, app.th.p.line_strong))
        .corner_radius(cr(8))
        .inner_margin(egui::Margin::same(4))
        .shadow(egui::epaint::Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(120) })
}

pub fn show_menu(app: &mut App, ctx: &egui::Context) {
    let Some(menu) = app.menu.as_ref() else {
        ctx.data_mut(|d| d.remove_temp::<bool>(Id::new("menu-seen")));
        return;
    };
    // the click that opened the menu must not close it in the same frame
    let seen = ctx.data_mut(|d| d.get_temp::<bool>(Id::new("menu-seen"))).unwrap_or(false);
    ctx.data_mut(|d| d.insert_temp(Id::new("menu-seen"), true));
    let p = app.th.p.clone();
    let w = 252.0;
    let mut pos = menu.pos;
    if menu.right_align {
        pos.x -= w;
    }
    let mut picked: Option<Cmd> = None;
    let items: Vec<(String, Option<Icon>, Cmd, Option<&'static str>, bool, bool)> = menu.items.iter().map(|i| (i.label.clone(), i.icon, i.cmd.clone(), i.kbd, i.danger, i.enabled)).collect();
    let area = egui::Area::new(Id::new("ctx-menu")).order(Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
        menu_frame(app).show(ui, |ui| {
            ui.set_width(w - 8.0);
            ui.spacing_mut().item_spacing.y = 0.0;
            for (label, icon, cmd, kbd, danger, enabled) in items {
                let (rect, resp) = ui.allocate_exact_size(vec2(w - 8.0, 30.0), Sense::click());
                if resp.hovered() && enabled {
                    ui.painter().rect_filled(rect, cr(5), p.bg_sel);
                }
                let fg = if !enabled { p.text3 } else if danger { p.deleted } else { p.text1 };
                if let Some(i) = icon {
                    icons::paint_at(ui, Pos2::new(rect.left() + 16.0, rect.center().y), 14.0, i, if danger { p.deleted } else { p.text2 });
                }
                draw_trunc(ui.painter(), rect.left() + 32.0, rect.center().y, rect.width() - 40.0 - kbd.map(|_| 70.0).unwrap_or(0.0), &label, app.th.f(Ty::Row), fg);
                if let Some(k) = kbd {
                    draw_right(ui.painter(), rect.right() - 10.0, rect.center().y, k, app.th.f(Ty::MonoXs), p.text3);
                }
                pointer(&resp);
                if resp.clicked() && enabled {
                    picked = Some(cmd);
                }
            }
        });
    });
    let outside = seen && ctx.input(|i| i.pointer.any_pressed()) && !area.response.rect.contains(ctx.input(|i| i.pointer.interact_pos()).unwrap_or(Pos2::ZERO));
    let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    if let Some(c) = picked {
        app.menu = None;
        app.run(c);
    } else if outside || esc {
        app.menu = None;
    }
}

pub fn show_toasts(app: &mut App, ctx: &egui::Context) {
    app.toasts.retain(|t| t.at.elapsed().as_secs_f32() < if t.kind == ToastKind::Err { 8.0 } else if t.action.is_some() { 6.0 } else { 3.5 });
    if app.toasts.is_empty() {
        return;
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(250));
    let p = app.th.p.clone();
    let screen = ctx.screen_rect();
    let mut y = screen.bottom() - 28.0 - 14.0;
    let mut dismiss: Option<usize> = None;
    let mut run: Option<Cmd> = None;
    for (i, t) in app.toasts.iter().enumerate().rev() {
        let w = 360.0;
        let has_action = t.action.is_some();
        let g = ctx.fonts(|f| f.layout(crate::i18n::t(&t.text).into_owned(), app.th.f(Ty::Row), p.text1, w - 70.0 - if has_action { 70.0 } else { 0.0 }));
        let h = (g.size().y + 24.0).max(44.0);
        y -= h;
        let pos = Pos2::new(screen.right() - w - 16.0, y);
        egui::Area::new(Id::new(("toast", i))).order(Order::Tooltip).fixed_pos(pos).show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
            let painter = ui.painter();
            painter.rect_filled(rect.translate(vec2(0.0, 4.0)), cr(8), Color32::from_black_alpha(60));
            painter.rect_filled(rect, cr(8), p.bg3);
            painter.rect_stroke(rect, cr(8), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
            let (col, ic) = match t.kind {
                ToastKind::Ok => (p.added, Icon::Check),
                ToastKind::Err => (p.deleted, Icon::Alert),
                ToastKind::Info => (p.renamed, Icon::Spark),
            };
            painter.rect_filled(Rect::from_min_size(rect.min + vec2(0.0, 6.0), vec2(3.0, h - 12.0)), cr(2), col);
            icons::paint_at(ui, Pos2::new(rect.left() + 24.0, rect.center().y), 15.0, ic, col);
            painter.galley(Pos2::new(rect.left() + 42.0, rect.center().y - g.size().y / 2.0), g, p.text1);
            if let Some((label, cmd)) = &t.action {
                let bw = text_width(ui, label, &FontId::new(12.0, semibold())) + 16.0;
                let br = Rect::from_min_size(Pos2::new(rect.right() - bw - 10.0, rect.center().y - 12.0), vec2(bw, 24.0));
                let r = ui.interact(br, Id::new(("toast-act", i)), Sense::click());
                if r.hovered() {
                    painter.rect_filled(br, cr(5), p.bg_sel);
                }
                centered(painter, br, label, FontId::new(12.0, semibold()), p.accent_hi);
                pointer(&r);
                if r.clicked() {
                    run = Some(cmd.clone());
                    dismiss = Some(i);
                }
            }
            if resp.clicked() && dismiss.is_none() {
                dismiss = Some(i);
            }
        });
        y -= 8.0;
    }
    if let Some(i) = dismiss {
        if i < app.toasts.len() {
            app.toasts.remove(i);
        }
    }
    if let Some(c) = run {
        app.run(c);
    }
}

/// Dim layer + centered card. Returns the card's response rect; closes on Escape / backdrop click.
pub fn dialog<R>(app: &App, ctx: &egui::Context, key: &str, width: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> (Option<R>, bool) {
    let p = &app.th.p;
    let screen = ctx.screen_rect();
    let mut close = false;
    egui::Area::new(Id::new("modal-dim")).order(Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
        let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
        ui.painter().rect_filled(rect, 0.0, Color32::from_black_alpha(if p.dark { 150 } else { 90 }));
        if resp.clicked() {
            close = true;
        }
    });
    let mut out = None;
    egui::Area::new(Id::new(("modal-card", key))).order(Order::Tooltip).anchor(Align2::CENTER_CENTER, vec2(0.0, -20.0)).show(ctx, |ui| {
        egui::Frame::new()
            .fill(p.bg2)
            .stroke(Stroke::new(1.0, p.line_strong))
            .corner_radius(cr(12))
            .inner_margin(egui::Margin::same(20))
            .shadow(egui::epaint::Shadow { offset: [0, 16], blur: 48, spread: 0, color: Color32::from_black_alpha(150) })
            .show(ui, |ui| {
                ui.set_width(width);
                out = Some(add(ui));
            });
    });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close = true;
    }
    (out, close)
}

pub fn show_modal(app: &mut App, ctx: &egui::Context) {
    let Some(mut modal) = app.modal.take() else { return };
    let mut keep = true;
    match &mut modal {
        Modal::Confirm { title, body, ok, danger, action } => {
            let (title, body, ok, danger) = (title.clone(), body.clone(), ok.clone(), *danger);
            let mut confirmed = false;
            let (_, close) = dialog(app, ctx, "confirm", 420.0, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                ui.label(crate::i18n::rt(&title).font(FontId::new(16.0, semibold())).color(app.th.p.text1));
                ui.label(crate::i18n::rt(&body).font(app.th.f(Ty::Row)).color(app.th.p.text2));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let b = Btn::new(ok.clone());
                        let b = if danger { b.danger() } else { b.primary() };
                        if button(ui, &app.th, b).clicked() {
                            confirmed = true;
                        }
                        if button(ui, &app.th, Btn::new("Vazgeç").ghost()).clicked() {
                            ui.ctx().memory_mut(|m| m.data.insert_temp(Id::new("modal-cancel"), true));
                        }
                    });
                });
            });
            let cancel = ctx.memory_mut(|m| m.data.remove_temp::<bool>(Id::new("modal-cancel"))).unwrap_or(false);
            if confirmed {
                keep = false;
                if let Some(a) = action.take() {
                    a(app);
                }
            } else if close || cancel {
                keep = false;
            }
        }
        Modal::NewBranch(name) => {
            let mut create = false;
            let base = app.repo.as_ref().map(|r| r.branch()).unwrap_or_default();
            let (_, close) = dialog(app, ctx, "newbranch", 420.0, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                ui.label(crate::i18n::rt("Yeni dal").font(FontId::new(16.0, semibold())).color(app.th.p.text1));
                ui.label(crate::i18n::rt(format!("{base} dalından oluşturulur ve geçilir.")).font(app.th.f(Ty::Row)).color(app.th.p.text2));
                let r = text_input(ui, &app.th, name, InputOpts::new(Id::new("newbranch-input"), "feature/ad").mono());
                r.request_focus();
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    create = true;
                }
                ui.add_space(6.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui, &app.th, Btn::new("Oluştur").primary().enabled(!name.trim().is_empty())).clicked() {
                        create = true;
                    }
                    if button(ui, &app.th, Btn::new("Vazgeç").ghost()).clicked() {
                        ui.ctx().memory_mut(|m| m.data.insert_temp(Id::new("modal-cancel"), true));
                    }
                });
            });
            let cancel = ctx.memory_mut(|m| m.data.remove_temp::<bool>(Id::new("modal-cancel"))).unwrap_or(false);
            if create && !name.trim().is_empty() {
                let n = name.trim().replace(' ', "-");
                keep = false;
                app.create_branch(n);
            } else if close || cancel {
                keep = false;
            }
        }
        Modal::FromCommit { sha, name, tag } => {
            let mut create = false;
            let is_tag = *tag;
            let short = crate::util::short(sha).to_string();
            let (_, close) = dialog(app, ctx, "fromcommit", 420.0, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                ui.label(crate::i18n::rt(if is_tag { "Etiket oluştur" } else { "Buradan dal aç" }).font(FontId::new(16.0, semibold())).color(app.th.p.text1));
                ui.label(crate::i18n::rt(format!("{short} commit'inden")).font(app.th.f(Ty::Row)).color(app.th.p.text2));
                let r = text_input(ui, &app.th, name, InputOpts::new(Id::new("fromcommit-input"), if is_tag { "v1.0.0" } else { "feature/ad" }).mono());
                r.request_focus();
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    create = true;
                }
                ui.add_space(6.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui, &app.th, Btn::new("Oluştur").primary().enabled(!name.trim().is_empty())).clicked() {
                        create = true;
                    }
                    if button(ui, &app.th, Btn::new("Vazgeç").ghost()).clicked() {
                        ui.ctx().memory_mut(|m| m.data.insert_temp(Id::new("modal-cancel"), true));
                    }
                });
            });
            let cancel = ctx.memory_mut(|m| m.data.remove_temp::<bool>(Id::new("modal-cancel"))).unwrap_or(false);
            if create && !name.trim().is_empty() {
                keep = false;
                app.create_at(sha.clone(), name.trim().replace(' ', "-"), is_tag);
            } else if close || cancel {
                keep = false;
            }
        }
        Modal::Settings(_) => {
            keep = crate::screens::settings::show(app, ctx, &mut modal);
        }
        Modal::Publish { name, desc, private, acc } => {
            let mut go = false;
            let (_, close) = dialog(app, ctx, "publish", 460.0, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                ui.label(crate::i18n::rt("Depoyu GitHub'da yayınla").font(FontId::new(16.0, semibold())).color(app.th.p.text1));
                ui.label(crate::i18n::rt("Depo GitHub hesabında oluşturulur ve geçerli dal gönderilir.").font(app.th.f(Ty::Row)).color(app.th.p.text2));
                ui.add_space(4.0);
                let nr = text_input(ui, &app.th, name, InputOpts::new(Id::new("pub-name"), "depo-adi").mono());
                if ui.memory(|m| m.focused().is_none()) {
                    nr.request_focus();
                }
                if nr.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    go = true;
                }
                text_input(ui, &app.th, desc, InputOpts::new(Id::new("pub-desc"), "Açıklama (isteğe bağlı)"));
                if app.s.accounts.len() > 1 {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.label(crate::i18n::rt("Hesap").font(app.th.f(Ty::Row)).color(app.th.p.text2));
                        for a in &app.s.accounts {
                            if pill(ui, &app.th, &format!("@{}", a.login), *acc == a.key).clicked() {
                                *acc = a.key.clone();
                            }
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if checkbox(ui, &app.th, if *private { Cb::On } else { Cb::Off }).clicked() {
                        *private = !*private;
                    }
                    ui.label(crate::i18n::rt("Özel depo").font(app.th.f(Ty::Row)).color(app.th.p.text1));
                });
                ui.add_space(6.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui, &app.th, Btn::new("Yayınla").primary().enabled(!name.trim().is_empty())).clicked() {
                        go = true;
                    }
                    if button(ui, &app.th, Btn::new("Vazgeç").ghost()).clicked() {
                        ui.ctx().memory_mut(|m| m.data.insert_temp(Id::new("modal-cancel"), true));
                    }
                });
            });
            let cancel = ctx.memory_mut(|m| m.data.remove_temp::<bool>(Id::new("modal-cancel"))).unwrap_or(false);
            if go && !name.trim().is_empty() {
                keep = false;
                app.publish_repo(name.trim().replace(' ', "-"), desc.trim().to_string(), *private, acc.clone());
            } else if close || cancel {
                keep = false;
            }
        }
        Modal::Login => {
            keep = crate::screens::welcome::login_modal(app, ctx);
        }
    }
    if keep && app.modal.is_none() {
        app.modal = Some(modal);
    }
}

/// A right-click on a diff row: what can be copied from it.
#[derive(Clone, Default)]
pub struct CopyReq {
    pub pos: eframe::egui::Pos2,
    pub line: String,
    pub hunk: String,
    pub file: String,
    pub sel: String,
}

pub fn request_copy_menu(ctx: &egui::Context, req: CopyReq) {
    ctx.data_mut(|d| d.insert_temp(Id::new("copy-req"), req));
}

/// Text under the pointer in the current frame, used by Ctrl+C when nothing is selected.
pub fn hover_copy(ctx: &egui::Context, text: String) {
    ctx.data_mut(|d| d.insert_temp(Id::new("hover-copy"), text));
}

pub fn clear_hover(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove_temp::<String>(Id::new("hover-copy")));
}

/// Opens the copy menu requested by a diff row and handles Ctrl+C over the diff.
pub fn frame_copy(app: &mut App, ctx: &egui::Context) {
    if let Some(req) = ctx.data_mut(|d| d.remove_temp::<CopyReq>(Id::new("copy-req"))) {
        let mk = |label: &str, icon: Icon, cmd: Cmd| MenuItem { label: label.into(), icon: Some(icon), kbd: None, cmd, danger: false, enabled: true };
        let mut items = vec![];
        if !req.sel.is_empty() {
            items.push(mk("Seçili satırları kopyala", Icon::Copy, Cmd::CopyText(req.sel.clone())));
        }
        items.push(mk("Satırı kopyala", Icon::Copy, Cmd::CopyText(req.line)));
        items.push(mk("Hunk'ı kopyala", Icon::Diff, Cmd::CopyText(req.hunk)));
        items.push(mk("Yolu kopyala", Icon::Link, Cmd::CopyPath(req.file)));
        app.menu = Some(Menu { pos: req.pos, right_align: false, items });
    }
    let copy = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
    if copy && !ctx.wants_keyboard_input() {
        let sel = if app.screen == Screen::Changes { crate::screens::changes::selected_text(app) } else { String::new() };
        let text = if sel.is_empty() { ctx.data_mut(|d| d.get_temp::<String>(Id::new("hover-copy"))).unwrap_or_default() } else { sel };
        if !text.is_empty() {
            ctx.copy_text(text);
            app.toast(ToastKind::Info, "Kopyalandı");
        }
    }
}
