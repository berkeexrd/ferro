//! eframe entry point: pumps jobs, handles shortcuts, draws chrome + current screen + overlays.

use crate::app::*;
use eframe::egui;
use std::time::Duration;

pub struct AppShell {
    pub app: App,
}

impl AppShell {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self { app: App::new(cc) }
    }
}

fn shortcuts(app: &mut App, ctx: &egui::Context) {
    use egui::{Key, Modifiers};
    let typing = ctx.wants_keyboard_input();
    let modal = app.modal.is_some() || app.palette.is_some() || app.switcher.is_some();
    let mut cmds: Vec<Cmd> = vec![];
    ctx.input_mut(|i| {
        let ctrl = Modifiers::COMMAND;
        let cs = Modifiers::COMMAND | Modifiers::SHIFT;
        if i.consume_key(cs, Key::Enter) {
            cmds.push(Cmd::CommitPush);
        }
        if i.consume_key(ctrl, Key::Enter) {
            cmds.push(Cmd::Commit);
        }
        if i.consume_key(cs, Key::P) {
            cmds.push(Cmd::Push);
        }
        if i.consume_key(cs, Key::N) {
            cmds.push(Cmd::NewBranch);
        }
        if i.consume_key(cs, Key::O) {
            cmds.push(Cmd::CloneRepo);
        }
        if i.consume_key(ctrl, Key::K) {
            cmds.push(Cmd::Palette);
        }
        if i.consume_key(ctrl, Key::B) {
            cmds.push(Cmd::Switcher);
        }
        if i.consume_key(ctrl, Key::N) {
            cmds.push(Cmd::NewRepo);
        }
        if i.consume_key(ctrl, Key::O) {
            cmds.push(Cmd::OpenFolder);
        }
        if i.consume_key(ctrl, Key::Comma) {
            cmds.push(Cmd::Settings);
        }
        if i.consume_key(Modifiers::NONE, Key::F5) {
            cmds.push(Cmd::Refresh);
        }
        for (k, s) in [(Key::Num1, Screen::Changes), (Key::Num2, Screen::History), (Key::Num3, Screen::Branches), (Key::Num4, Screen::Pr)] {
            if i.consume_key(ctrl, k) {
                cmds.push(Cmd::Goto(s));
            }
        }
        if !typing && !modal {
            if i.consume_key(cs, Key::Z) {
                cmds.push(Cmd::Redo);
            }
            if i.consume_key(ctrl, Key::Z) {
                cmds.push(Cmd::Undo);
            }
            if i.consume_key(Modifiers::NONE, Key::Space) {
                cmds.push(Cmd::ToggleStageSelected);
            }
            if i.consume_key(Modifiers::NONE, Key::J) {
                cmds.push(Cmd::NextFile);
            }
            if i.consume_key(Modifiers::NONE, Key::K) {
                cmds.push(Cmd::PrevFile);
            }
            if i.consume_key(Modifiers::NONE, Key::CloseBracket) {
                cmds.push(Cmd::NextHunk);
            }
            if i.consume_key(Modifiers::NONE, Key::OpenBracket) {
                cmds.push(Cmd::PrevHunk);
            }
        }
    });
    for c in cmds {
        if app.repo.is_none() && !matches!(c, Cmd::Palette | Cmd::OpenFolder | Cmd::CloneRepo | Cmd::NewRepo | Cmd::Settings) {
            continue;
        }
        if app.screen != Screen::Changes && matches!(c, Cmd::ToggleStageSelected | Cmd::NextFile | Cmd::PrevFile | Cmd::NextHunk | Cmd::PrevHunk) {
            continue;
        }
        app.run(c);
    }
}

/// Frameless window: resize from the edges.
fn resize_edges(ctx: &egui::Context) {
    if ctx.input(|i| i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)) {
        return;
    }
    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else { return };
    let r = ctx.screen_rect();
    let m = 5.0;
    let (l, rt, t, b) = (pos.x < r.left() + m, pos.x > r.right() - m, pos.y < r.top() + m, pos.y > r.bottom() - m);
    use egui::viewport::ResizeDirection as D;
    let dir = match (l, rt, t, b) {
        (true, _, true, _) => Some(D::NorthWest),
        (_, true, true, _) => Some(D::NorthEast),
        (true, _, _, true) => Some(D::SouthWest),
        (_, true, _, true) => Some(D::SouthEast),
        (true, ..) => Some(D::West),
        (_, true, ..) => Some(D::East),
        (_, _, true, _) => Some(D::North),
        (_, _, _, true) => Some(D::South),
        _ => None,
    };
    if let Some(d) = dir {
        use egui::CursorIcon as C;
        ctx.set_cursor_icon(match d {
            D::North | D::South => C::ResizeVertical,
            D::East | D::West => C::ResizeHorizontal,
            D::NorthWest | D::SouthEast => C::ResizeNwSe,
            D::NorthEast | D::SouthWest => C::ResizeNeSw,
        });
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(d));
        }
    }
}

impl eframe::App for AppShell {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Ok(j) = serde_json::to_string(&self.app.s) {
            let _ = crate::backend::sys::save_state(&j);
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let app = &mut self.app;
        while let Ok(job) = app.rx.try_recv() {
            job(app);
        }
        app.save_if_due();
        if app.save_due.is_some() {
            ctx.request_repaint_after(Duration::from_millis(450));
        }
        if app.repo.is_some() {
            let focused = ctx.input(|i| i.focused);
            if focused && app.last_poll.elapsed() > Duration::from_millis(2500) {
                app.refresh_status(false);
            }
            if focused && app.last_meta.elapsed() > Duration::from_secs(8) {
                app.refresh_meta();
            }
            ctx.request_repaint_after(Duration::from_millis(2600));
        }
        if app.s.mode == crate::theme::Mode::System {
            let sd = ctx.input(|i| i.raw.system_theme.map(|t| t == egui::Theme::Dark)).unwrap_or(app.system_dark);
            if sd != app.system_dark {
                app.system_dark = sd;
                app.apply_theme();
            }
        }
        shortcuts(app, ctx);
        resize_edges(ctx);

        crate::chrome::title_bar(app, ctx);
        crate::chrome::status_bar(app, ctx);
        if app.repo.is_some() {
            crate::chrome::nav_rail(app, ctx);
        }
        let fill = if app.repo.is_some() { app.th.p.bg2 } else { app.th.p.bg0 };
        egui::CentralPanel::default().frame(egui::Frame::new().fill(fill)).show(ctx, |ui| {
            if app.repo.is_none() || app.welcome.clone_open {
                crate::screens::welcome::show(app, ui);
                return;
            }
            match app.screen {
                Screen::Changes => crate::screens::changes::show(app, ui),
                Screen::History => crate::screens::history::show(app, ui),
                Screen::Conflict => crate::screens::conflict::show(app, ui),
                Screen::Branches | Screen::Stash | Screen::Log => crate::screens::lists::show(app, ui),
                Screen::Pr => crate::screens::pr::show(app, ui),
            }
        });
        crate::screens::changes::sync_commit_fields(app, ctx);
        if let Some(r) = app.repo.as_mut() {
            r.scroll_req = false;
        }
        crate::screens::palette::show(app, ctx);
        crate::overlay::show_menu(app, ctx);
        crate::overlay::show_modal(app, ctx);
        crate::overlay::show_toasts(app, ctx);
    }
}
