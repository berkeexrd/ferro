#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Ferro Setup: a self-contained installer / uninstaller drawn with egui, in Ferro's own design language.

use eframe::egui::{self, vec2, Align2, Color32, CornerRadius, FontFamily, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind};
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::sync::mpsc::{channel, Receiver};
use std::time::Instant;

const PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.gz"));
const VERSION: &str = "0.1.0";
const W: f32 = 880.0;
const H: f32 = 540.0;
const HERO_W: f32 = 380.0;
const REG: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Ferro";

// ---- palette (dark Ferro)
const BG0: Color32 = Color32::from_rgb(0x15, 0x14, 0x12);
const BG1: Color32 = Color32::from_rgb(0x1B, 0x1A, 0x17);
const BG2: Color32 = Color32::from_rgb(0x20, 0x1F, 0x1B);
const BG3: Color32 = Color32::from_rgb(0x2A, 0x28, 0x24);
const LINE: Color32 = Color32::from_rgb(0x36, 0x33, 0x2D);
const LINE2: Color32 = Color32::from_rgb(0x4A, 0x46, 0x3E);
const TEXT1: Color32 = Color32::from_rgb(0xEC, 0xE8, 0xE1);
const TEXT2: Color32 = Color32::from_rgb(0xB1, 0xAA, 0xA0);
const TEXT3: Color32 = Color32::from_rgb(0x9A, 0x93, 0x88);
const ACCENT: Color32 = Color32::from_rgb(0xE5, 0x8A, 0x55);
const ACCENT_HI: Color32 = Color32::from_rgb(0xF0, 0xA1, 0x73);
const ACCENT_FG: Color32 = Color32::from_rgb(0x1A, 0x11, 0x09);
const GREEN: Color32 = Color32::from_rgb(0x7F, 0xD3, 0xA5);
const RED: Color32 = Color32::from_rgb(0xF2, 0x9B, 0x9B);
const LANES: [Color32; 4] = [ACCENT, Color32::from_rgb(0x7F, 0xB5, 0xF2), Color32::from_rgb(0x86, 0xCB, 0xC6), Color32::from_rgb(0xB7, 0x9C, 0xF5)];

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Welcome,
    Installing,
    Done,
    Failed,
    UConfirm,
    UWorking,
    UDone,
}

enum Msg {
    Step(usize, String, f32),
    Done,
    Fail(String),
}

struct Setup {
    en: bool,
    page: Page,
    page_at: Instant,
    dir: String,
    desktop: bool,
    startmenu: bool,
    launch: bool,
    wipe: bool,
    custom: bool,
    rx: Option<Receiver<Msg>>,
    step: usize,
    steps: Vec<String>,
    target: f32,
    shown: f32,
    log: Vec<String>,
    err: String,
    t0: Instant,
    updating: bool,
    done_flag: bool,
    logo: egui::TextureHandle,
}

fn detect_en() -> bool {
    extern "system" {
        fn GetUserDefaultLocaleName(buf: *mut u16, len: i32) -> i32;
    }
    let mut b = [0u16; 85];
    let n = unsafe { GetUserDefaultLocaleName(b.as_mut_ptr(), 85) };
    if n > 0 {
        let s = String::from_utf16_lossy(&b[..(n as usize).saturating_sub(1)]).to_lowercase();
        return !s.starts_with("tr");
    }
    true
}

fn default_dir() -> String {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".into());
    format!("{base}\\Programs\\Ferro")
}

fn main() -> eframe::Result {
    let uninstall = std::env::args().any(|a| a == "--uninstall");
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([W, H])
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_title("Ferro Setup")
            .with_icon(egui::IconData { rgba: include_bytes!("../../assets/icon64.rgba").to_vec(), width: 64, height: 64 }),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "Ferro Setup",
        opts,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            let mut visuals = egui::Visuals::dark();
            visuals.override_text_color = Some(TEXT1);
            cc.egui_ctx.set_visuals(visuals);
            let dir = default_dir();
            let updating = std::path::Path::new(&dir).join("ferro.exe").exists();
            Ok(Box::new(Setup {
                en: detect_en(),
                page: if uninstall { Page::UConfirm } else { Page::Welcome },
                page_at: Instant::now(),
                dir,
                desktop: true,
                startmenu: true,
                launch: true,
                wipe: false,
                custom: false,
                rx: None,
                step: 0,
                steps: vec![],
                target: 0.0,
                shown: 0.0,
                log: vec![],
                err: String::new(),
                t0: Instant::now(),
                updating,
                done_flag: false,
                logo: cc.egui_ctx.load_texture("logo", egui::ColorImage::from_rgba_unmultiplied([256, 256], include_bytes!("../../assets/icon256.rgba")), egui::TextureOptions::LINEAR),
            }))
        }),
    )
}

fn install_fonts(ctx: &egui::Context) {
    use std::sync::Arc;
    let mut f = egui::FontDefinitions::empty();
    f.font_data.insert("inter".into(), Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/Inter-Regular.otf"))));
    f.font_data.insert("inter-sb".into(), Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/Inter-SemiBold.otf"))));
    f.font_data.insert("jb".into(), Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf"))));
    f.font_data.insert("jb-b".into(), Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf"))));
    let mut fb: Vec<String> = vec![];
    if let Ok(b) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        f.font_data.insert("segoe".into(), Arc::new(egui::FontData::from_owned(b)));
        fb.push("segoe".into());
    }
    let with = |first: &[&str]| -> Vec<String> {
        let mut v: Vec<String> = first.iter().map(|s| s.to_string()).collect();
        v.extend(fb.iter().cloned());
        v
    };
    f.families.insert(FontFamily::Proportional, with(&["inter"]));
    f.families.insert(FontFamily::Monospace, with(&["jb"]));
    f.families.insert(FontFamily::Name("sb".into()), with(&["inter-sb"]));
    f.families.insert(FontFamily::Name("mb".into()), with(&["jb-b"]));
    ctx.set_fonts(f);
}

fn sb(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("sb".into()))
}
fn rg(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}
fn monob(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mb".into()))
}
fn cr(r: u8) -> CornerRadius {
    CornerRadius::same(r)
}
/// CSS-style alpha (egui's own constructor premultiplies in linear space).
fn ca(c: Color32, a: f32) -> Color32 {
    let a8 = (a.clamp(0.0, 1.0) * 255.0) as u32;
    let m = |v: u8| ((v as u32 * a8 + 127) / 255) as u8;
    Color32::from_rgba_premultiplied(m(c.r()), m(c.g()), m(c.b()), a8 as u8)
}

/// Smooth radial gradient (centre colour → transparent) as a triangle fan.
fn glow(p: &egui::Painter, c: Pos2, r: f32, col: Color32, a: f32) {
    let mut mesh = egui::Mesh::default();
    let centre = mesh.vertices.len() as u32;
    mesh.colored_vertex(c, ca(col, a));
    let n = 64;
    for i in 0..=n {
        let ang = i as f32 / n as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(c + vec2(ang.cos(), ang.sin()) * r, Color32::TRANSPARENT);
    }
    for i in 0..n {
        mesh.add_triangle(centre, centre + 1 + i, centre + 2 + i);
    }
    p.add(egui::Shape::mesh(mesh));
}

fn hash(a: i64, b: i64) -> u32 {
    let mut h = (a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B)) as u32;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^ (h >> 15)
}

// ------------------------------------------------------------------ hero artwork

fn hero(p: &egui::Painter, r: Rect, t: f32, logo: &egui::TextureHandle) {
    // base
    p.rect_filled(r, CornerRadius { nw: 16, sw: 16, ne: 0, se: 0 }, BG0);
    // soft colour pools
    glow(p, Pos2::new(r.left() + 30.0, r.bottom() - 10.0), 420.0, ACCENT, 0.30);
    glow(p, Pos2::new(r.right() - 10.0, r.top() + 20.0), 300.0, LANES[1], 0.18);
    // flowing commit graph
    let clip = p.with_clip_rect(r.shrink2(vec2(0.0, 0.0)));
    let spacing = 74.0;
    let lane_x = |i: usize| r.left() + 56.0 + i as f32 * 78.0;
    let scroll = t * 26.0;
    let shift = (scroll / spacing).floor() as i64;
    let off = scroll % spacing;
    for (i, col) in LANES.iter().enumerate() {
        let x = lane_x(i);
        clip.line_segment([Pos2::new(x, r.top()), Pos2::new(x, r.bottom())], Stroke::new(2.0, ca(*col, 0.16)));
    }
    for k in -1..9 {
        let y = r.top() + k as f32 * spacing + off;
        let idx = k as i64 - shift;
        for (i, col) in LANES.iter().enumerate() {
            let h = hash(idx, i as i64 + 3);
            // lane 0 is the trunk: every row; other lanes branch occasionally
            let present = i == 0 || h % 5 < 2;
            if !present {
                continue;
            }
            let x = lane_x(i);
            let fade = (((y - (r.top() + 176.0)).abs() - 40.0) / 150.0).clamp(0.22, 1.0);
            // merge curve back to trunk
            if i > 0 {
                let a = Pos2::new(x, y);
                let b = Pos2::new(lane_x(0), y + spacing);
                clip.add(egui::epaint::CubicBezierShape::from_points_stroke([a, Pos2::new(x, y + spacing * 0.6), Pos2::new(b.x, y + spacing * 0.4), b], false, Color32::TRANSPARENT, Stroke::new(2.0, ca(*col, 0.45 * fade))));
            }
            let pulse = 0.5 + 0.5 * (t * 2.0 + h as f32 * 0.001).sin();
            clip.circle_filled(Pos2::new(x, y), 9.0 + 3.0 * pulse, ca(*col, 0.07 * fade));
            clip.circle_filled(Pos2::new(x, y), 5.0, ca(*col, fade));
            clip.circle_stroke(Pos2::new(x, y), 5.0, Stroke::new(2.0, ca(BG0, fade)));
        }
    }
    // calm the lanes behind the logo
    glow(p, Pos2::new(r.center().x, r.top() + 190.0), 190.0, BG0, 0.92);
    // logo
    let lc = Pos2::new(r.center().x, r.top() + 176.0);
    let breath = 0.5 + 0.5 * (t * 1.6).sin();
    glow(p, lc, 150.0 + 20.0 * breath, ACCENT, 0.22 + 0.06 * breath);
    let lr = Rect::from_center_size(lc, vec2(112.0, 112.0));
    p.rect_filled(lr.translate(vec2(0.0, 10.0)).expand(2.0), cr(30), ca(Color32::BLACK, 0.40));
    p.image(logo.id(), lr, Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)), Color32::WHITE);
    // wordmark
    p.text(Pos2::new(r.center().x, lc.y + 96.0), Align2::CENTER_CENTER, "Ferro", sb(38.0), TEXT1);
    p.text(Pos2::new(r.left() + 34.0, r.bottom() - 28.0), Align2::LEFT_CENTER, format!("v{VERSION}"), mono(11.0), TEXT3);
    // drifting branch chips
    let chips = [("main", 0usize, 0.0f32), ("feature/fast-status", 1, 1.7), ("v0.1.0", 3, 3.1)];
    for (i, (label, lane, ph)) in chips.iter().enumerate() {
        let wob = (t * 0.7 + ph).sin();
        let cx = r.left() + 34.0 + (i as f32) * 62.0 + wob * 6.0;
        let cy = r.bottom() - 78.0 - (i as f32) * 30.0 + (t * 0.9 + ph).cos() * 4.0;
        let w = p.layout_no_wrap(label.to_string(), monob(11.0), TEXT1).size().x + 28.0;
        let cr_ = Rect::from_min_size(Pos2::new(cx, cy), vec2(w, 22.0));
        p.rect_filled(cr_, cr(255), ca(LANES[*lane], 0.16));
        p.rect_stroke(cr_, cr(255), Stroke::new(1.0, ca(LANES[*lane], 0.5)), StrokeKind::Inside);
        p.circle_filled(Pos2::new(cr_.left() + 11.0, cr_.center().y), 3.0, LANES[*lane]);
        p.text(Pos2::new(cr_.left() + 21.0, cr_.center().y), Align2::LEFT_CENTER, label, monob(11.0), TEXT1);
    }
}

// ------------------------------------------------------------------ small widgets

struct Ui2<'a> {
    ui: &'a mut egui::Ui,
    fade: f32,
}

impl Ui2<'_> {
    fn text(&self, pos: Pos2, s: &str, font: FontId, col: Color32, anchor: Align2) -> Rect {
        self.ui.painter().text(pos, anchor, s, font, ca(col, self.fade))
    }

    fn wrapped(&self, pos: Pos2, s: &str, font: FontId, col: Color32, width: f32) -> f32 {
        let g = self.ui.painter().layout(s.to_string(), font, ca(col, self.fade), width);
        let h = g.size().y;
        self.ui.painter().galley(pos, g, ca(col, self.fade));
        h
    }

    fn button(&mut self, rect: Rect, id: &str, label: &str, primary: bool) -> bool {
        let resp = self.ui.interact(rect, Id::new(id), Sense::click());
        let hov = resp.hovered();
        let down = resp.is_pointer_button_down_on();
        let p = self.ui.painter();
        let f = self.fade;
        if primary {
            if hov {
                p.rect_filled(rect.expand(3.0), cr(12), ca(ACCENT, 0.18));
            }
            p.rect_filled(rect, cr(9), ca(if down { Color32::from_rgb(0xB9, 0x68, 0x3A) } else if hov { ACCENT_HI } else { ACCENT }, f));
            p.text(rect.center(), Align2::CENTER_CENTER, label, sb(15.0), ca(ACCENT_FG, f));
        } else {
            p.rect_filled(rect, cr(9), ca(if hov { BG3 } else { BG2 }, f));
            p.rect_stroke(rect, cr(9), Stroke::new(1.0, ca(LINE2, f)), StrokeKind::Inside);
            p.text(rect.center(), Align2::CENTER_CENTER, label, rg(14.0), ca(TEXT1, f));
        }
        if hov {
            self.ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        resp.clicked()
    }

    fn checkbox(&mut self, pos: Pos2, id: &str, label: &str, on: &mut bool, width: f32) {
        let rect = Rect::from_min_size(pos, vec2(width, 28.0));
        let resp = self.ui.interact(rect, Id::new(id), Sense::click());
        if resp.clicked() {
            *on = !*on;
        }
        let bx = Rect::from_min_size(Pos2::new(pos.x, pos.y + 6.0), vec2(16.0, 16.0));
        let p = self.ui.painter();
        let f = self.fade;
        if *on {
            p.rect_filled(bx, cr(4), ca(ACCENT, f));
            let c = bx.center();
            let st = Stroke::new(2.0, ca(ACCENT_FG, f));
            p.line_segment([Pos2::new(c.x - 4.0, c.y), Pos2::new(c.x - 1.0, c.y + 3.0)], st);
            p.line_segment([Pos2::new(c.x - 1.0, c.y + 3.0), Pos2::new(c.x + 4.5, c.y - 3.5)], st);
        } else {
            p.rect_stroke(bx, cr(4), Stroke::new(1.5, ca(if resp.hovered() { TEXT2 } else { Color32::from_rgb(0x6A, 0x65, 0x5B) }, f)), StrokeKind::Inside);
        }
        p.text(Pos2::new(pos.x + 26.0, pos.y + 14.0), Align2::LEFT_CENTER, label, rg(14.0), ca(TEXT1, f));
        if resp.hovered() {
            self.ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }
}

// ------------------------------------------------------------------ app

impl Setup {
    fn t<'a>(&self, tr: &'a str, en: &'a str) -> &'a str {
        if self.en {
            en
        } else {
            tr
        }
    }

    fn goto(&mut self, p: Page) {
        self.page = p;
        self.page_at = Instant::now();
    }

    fn start_install(&mut self, ctx: &egui::Context) {
        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.steps = vec![
            self.t("Klasör hazırlanıyor", "Preparing folder").into(),
            self.t("Dosyalar çıkarılıyor", "Extracting files").into(),
            self.t("Kaldırıcı yazılıyor", "Writing uninstaller").into(),
            self.t("Kısayollar oluşturuluyor", "Creating shortcuts").into(),
            self.t("Windows'a kaydediliyor", "Registering with Windows").into(),
        ];
        self.step = 0;
        self.target = 0.0;
        self.shown = 0.0;
        self.log.clear();
        let (dir, desktop, startmenu) = (self.dir.clone(), self.desktop, self.startmenu);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let send = |m: Msg| {
                let _ = tx.send(m);
                ctx.request_repaint();
            };
            let pace = || std::thread::sleep(std::time::Duration::from_millis(420));
            let r: Result<(), String> = (|| {
                let shown = match std::env::var("LOCALAPPDATA") {
                    Ok(l) if dir.starts_with(&l) => dir.replacen(&l, "%LOCALAPPDATA%", 1),
                    _ => dir.clone(),
                };
                send(Msg::Step(0, format!("mkdir {shown}"), 0.06));
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                pace();
                send(Msg::Step(1, "gunzip payload → ferro.exe".into(), 0.28));
                let mut raw = Vec::new();
                flate2::read::GzDecoder::new(PAYLOAD).read_to_end(&mut raw).map_err(|e| e.to_string())?;
                let exe = std::path::Path::new(&dir).join("ferro.exe");
                std::fs::write(&exe, &raw).map_err(|e| format!("ferro.exe: {e} (Ferro açıksa kapat / close Ferro first)"))?;
                let _ = std::fs::write(std::path::Path::new(&dir).join("icon.ico"), include_bytes!("../../assets/icon.ico"));
                pace();
                send(Msg::Step(2, "copy setup → uninstall.exe".into(), 0.55));
                let me = std::env::current_exe().map_err(|e| e.to_string())?;
                std::fs::copy(&me, std::path::Path::new(&dir).join("uninstall.exe")).map_err(|e| e.to_string())?;
                pace();
                send(Msg::Step(3, "create .lnk shortcuts".into(), 0.74));
                shortcuts(&dir, desktop, startmenu)?;
                pace();
                send(Msg::Step(4, format!("reg add {REG}"), 0.92));
                register(&dir, raw.len() as u64 / 1024)?;
                pace();
                Ok(())
            })();
            match r {
                Ok(()) => send(Msg::Done),
                Err(e) => send(Msg::Fail(e)),
            }
        });
        self.goto(Page::Installing);
    }

    fn start_uninstall(&mut self, ctx: &egui::Context) {
        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.steps = vec![
            self.t("Dosyalar siliniyor", "Removing files").into(),
            self.t("Kısayollar kaldırılıyor", "Removing shortcuts").into(),
            self.t("Kayıt temizleniyor", "Cleaning the registry").into(),
        ];
        self.step = 0;
        self.target = 0.0;
        self.shown = 0.0;
        self.log.clear();
        let wipe = self.wipe;
        let ctx = ctx.clone();
        let dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_string_lossy().to_string())).unwrap_or_else(default_dir);
        std::thread::spawn(move || {
            let send = |m: Msg| {
                let _ = tx.send(m);
                ctx.request_repaint();
            };
            let pace = || std::thread::sleep(std::time::Duration::from_millis(450));
            send(Msg::Step(0, "del ferro.exe icon.ico".into(), 0.2));
            let exe = std::path::Path::new(&dir).join("ferro.exe");
            if exe.exists() && std::fs::remove_file(&exe).is_err() {
                send(Msg::Fail("ferro.exe: Ferro açık / Ferro is running".into()));
                return;
            }
            let _ = std::fs::remove_file(std::path::Path::new(&dir).join("icon.ico"));
            if wipe {
                if let Some(a) = std::env::var_os("APPDATA") {
                    let _ = std::fs::remove_dir_all(std::path::Path::new(&a).join("Ferro"));
                }
            }
            pace();
            send(Msg::Step(1, "del *.lnk".into(), 0.55));
            remove_shortcuts();
            pace();
            send(Msg::Step(2, format!("reg delete {REG}"), 0.85));
            let _ = hidden("reg").args(["delete", REG, "/f"]).status();
            pace();
            send(Msg::Done);
        });
        self.goto(Page::UWorking);
    }
}

/// The running uninstaller can't delete itself: a detached cmd does it a moment after we exit.
fn self_delete() {
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_string_lossy().to_string())) {
        let _ = hidden("cmd").raw_arg(format!("/C ping -n 3 127.0.0.1 >nul & del /f /q \"{dir}\\uninstall.exe\" & rmdir \"{dir}\"")).spawn();
    }
}

fn hidden(program: &str) -> std::process::Command {
    let mut c = std::process::Command::new(program);
    c.creation_flags(0x0800_0000);
    c
}

fn ps_quote(s: &str) -> String {
    s.replace('\'', "''")
}

fn shortcuts(dir: &str, desktop: bool, startmenu: bool) -> Result<(), String> {
    let exe = format!("{dir}\\ferro.exe");
    let ico = format!("{dir}\\icon.ico");
    let mut script = String::from("$w = New-Object -ComObject WScript.Shell\n");
    let mk = |target: &str| {
        format!(
            "$s = $w.CreateShortcut({t}); $s.TargetPath = '{e}'; $s.WorkingDirectory = '{d}'; $s.IconLocation = '{i}'; $s.Description = 'Ferro'; $s.Save()\n",
            t = target,
            e = ps_quote(&exe),
            d = ps_quote(dir),
            i = ps_quote(&ico)
        )
    };
    if desktop {
        script.push_str(&mk("([Environment]::GetFolderPath('Desktop') + '\\Ferro.lnk')"));
    }
    if startmenu {
        script.push_str("$sm = Join-Path $env:APPDATA 'Microsoft\\Windows\\Start Menu\\Programs'\n");
        script.push_str(&mk("(Join-Path $sm 'Ferro.lnk')"));
    }
    let st = hidden("powershell").args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script]).status().map_err(|e| e.to_string())?;
    if st.success() {
        Ok(())
    } else {
        Err("shortcut creation failed".into())
    }
}

fn remove_shortcuts() {
    let script = "Remove-Item ([Environment]::GetFolderPath('Desktop') + '\\Ferro.lnk') -ErrorAction SilentlyContinue; Remove-Item (Join-Path $env:APPDATA 'Microsoft\\Windows\\Start Menu\\Programs\\Ferro.lnk') -ErrorAction SilentlyContinue";
    let _ = hidden("powershell").args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script]).status();
}

fn register(dir: &str, size_kb: u64) -> Result<(), String> {
    let add = |name: &str, kind: &str, val: &str| -> Result<(), String> {
        let st = hidden("reg").args(["add", REG, "/v", name, "/t", kind, "/d", val, "/f"]).status().map_err(|e| e.to_string())?;
        if st.success() {
            Ok(())
        } else {
            Err(format!("reg add {name} failed"))
        }
    };
    add("DisplayName", "REG_SZ", "Ferro")?;
    add("DisplayVersion", "REG_SZ", VERSION)?;
    add("Publisher", "REG_SZ", "Ferro")?;
    add("DisplayIcon", "REG_SZ", &format!("{dir}\\icon.ico"))?;
    add("InstallLocation", "REG_SZ", dir)?;
    add("UninstallString", "REG_SZ", &format!("\"{dir}\\uninstall.exe\" --uninstall"))?;
    add("NoModify", "REG_DWORD", "1")?;
    add("NoRepair", "REG_DWORD", "1")?;
    add("EstimatedSize", "REG_DWORD", &size_kb.to_string())?;
    Ok(())
}

impl eframe::App for Setup {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        // worker messages
        let mut failed: Option<String> = None;
        if let Some(rx) = &self.rx {
            while let Ok(m) = rx.try_recv() {
                match m {
                    Msg::Step(i, line, p) => {
                        self.step = i;
                        self.target = p;
                        self.log.push(line);
                    }
                    Msg::Done => {
                        self.target = 1.0;
                        self.done_flag = true;
                    }
                    Msg::Fail(e) => failed = Some(e),
                }
            }
        }
        // ease the displayed progress
        let dt = ctx.input(|i| i.stable_dt).min(0.05);
        self.shown += (self.target - self.shown) * (1.0 - (-6.0 * dt).exp());
        if matches!(self.page, Page::Installing | Page::UWorking) {
            ctx.request_repaint();
        }
        if let Some(e) = failed {
            self.err = e;
            self.rx = None;
            self.goto(Page::Failed);
        }
        if self.done_flag && self.shown > 0.985 {
            self.done_flag = false;
            self.rx = None;
            self.shown = 1.0;
            let next = if self.page == Page::UWorking { Page::UDone } else { Page::Done };
            self.goto(next);
        }

        let t = self.t0.elapsed().as_secs_f32();
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
            let full = ui.max_rect();
            let p = ui.painter().clone();
            // window body
            p.rect_filled(full, cr(16), BG1);
            let hero_r = Rect::from_min_size(full.min, vec2(HERO_W, full.height()));
            hero(&p, hero_r, t, &self.logo);
            p.rect_stroke(full, cr(16), Stroke::new(1.0, LINE), StrokeKind::Inside);
            // drag + close
            let drag = Rect::from_min_size(full.min, vec2(full.width() - 56.0, 44.0));
            let dr = ui.interact(drag, Id::new("drag"), Sense::click_and_drag());
            if dr.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            let cl = Rect::from_min_size(Pos2::new(full.right() - 46.0, full.top() + 10.0), vec2(32.0, 28.0));
            let cresp = ui.interact(cl, Id::new("close"), Sense::click());
            if cresp.hovered() {
                p.rect_filled(cl, cr(7), BG3);
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let cc = cl.center();
            let st = Stroke::new(1.4, if cresp.hovered() { TEXT1 } else { TEXT2 });
            p.line_segment([cc + vec2(-4.5, -4.5), cc + vec2(4.5, 4.5)], st);
            p.line_segment([cc + vec2(4.5, -4.5), cc + vec2(-4.5, 4.5)], st);
            if cresp.clicked() && !matches!(self.page, Page::Installing | Page::UWorking) {
                if self.page == Page::UDone {
                    self_delete();
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            // language switch
            let lr = Rect::from_min_size(Pos2::new(full.right() - 112.0, full.top() + 10.0), vec2(56.0, 28.0));
            let lresp = ui.interact(lr, Id::new("lang"), Sense::click());
            p.rect_stroke(lr, cr(7), Stroke::new(1.0, if lresp.hovered() { LINE2 } else { LINE }), StrokeKind::Inside);
            p.text(lr.center(), Align2::CENTER_CENTER, if self.en { "EN · TR" } else { "TR · EN" }, mono(11.0), TEXT2);
            if lresp.clicked() {
                self.en = !self.en;
            }
            if lresp.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            let fade = (self.page_at.elapsed().as_secs_f32() / 0.45).clamp(0.0, 1.0);
            let fade = 1.0 - (1.0 - fade).powi(3);
            let slide = (1.0 - fade) * 18.0;
            let x0 = full.left() + HERO_W + 48.0;
            let content = Rect::from_min_max(Pos2::new(x0, full.top() + 56.0 + slide), Pos2::new(full.right() - 44.0, full.bottom() - 40.0));
            let mut u = Ui2 { ui, fade };
            match self.page {
                Page::Welcome => self.page_welcome(&mut u, content, ctx),
                Page::Installing => self.page_progress(&mut u, content, false),
                Page::UWorking => self.page_progress(&mut u, content, true),
                Page::Done => self.page_done(&mut u, content, false),
                Page::UDone => self.page_done(&mut u, content, true),
                Page::Failed => self.page_failed(&mut u, content),
                Page::UConfirm => self.page_uconfirm(&mut u, content, ctx),
            }
            if fade < 1.0 {
                ctx.request_repaint();
            }
        });
    }
}

impl Setup {
    fn title(&self, u: &Ui2, c: Rect, title: &str, sub: &str) -> f32 {
        u.text(c.left_top(), title, sb(30.0), TEXT1, Align2::LEFT_TOP);
        let h = u.wrapped(Pos2::new(c.left(), c.top() + 44.0), sub, rg(14.0), TEXT2, c.width());
        44.0 + h + 22.0
    }

    fn page_welcome(&mut self, u: &mut Ui2, c: Rect, ctx: &egui::Context) {
        let title = if self.updating { self.t("Ferro güncelleniyor", "Update Ferro") } else { self.t("Ferro'ya hoş geldin", "Welcome to Ferro") };
        let sub = self.t("Hızlı, çok yönlü ve sade bir Git istemcisi. Kurulum bir dakikadan kısa sürer; yönetici izni gerekmez.", "A fast, calm Git client. Setup takes under a minute and needs no administrator rights.");
        let mut y = c.top() + self.title(u, c, title, sub);
        // feature rows
        let feats = [
            (self.t("Hunk ve satır bazlı hazırlama", "Hunk and line staging"), self.t("Commit'i istediğin kadar ince böl.", "Slice commits as finely as you like.")),
            (self.t("Her işlem geri alınabilir", "Every action is undoable"), self.t("İşlem günlüğü + Ctrl Z.", "Operation log + Ctrl Z.")),
            (self.t("Git karşılığı hep görünür", "Git is always visible"), self.t("Hiçbir şey sihir değil.", "Nothing is magic.")),
        ];
        for (i, (a, b)) in feats.iter().enumerate() {
            let col = LANES[i];
            let cy = y + 20.0;
            u.ui.painter().circle_filled(Pos2::new(c.left() + 8.0, cy), 6.0, ca(col, u.fade * 0.25));
            u.ui.painter().circle_filled(Pos2::new(c.left() + 8.0, cy), 3.0, ca(col, u.fade));
            u.text(Pos2::new(c.left() + 28.0, y + 11.0), a, sb(14.0), TEXT1, Align2::LEFT_CENTER);
            u.text(Pos2::new(c.left() + 28.0, y + 31.0), b, rg(12.5), TEXT3, Align2::LEFT_CENTER);
            y += 52.0;
        }
        y += 6.0;
        // location + options
        let loc = Rect::from_min_size(Pos2::new(c.left(), y), vec2(c.width(), 40.0));
        let p = u.ui.painter().clone();
        p.rect_filled(loc, cr(9), ca(BG0, u.fade));
        p.rect_stroke(loc, cr(9), Stroke::new(1.0, ca(LINE, u.fade)), StrokeKind::Inside);
        u.text(Pos2::new(loc.left() + 14.0, loc.center().y), self.t("Konum", "Location"), rg(12.0), TEXT3, Align2::LEFT_CENTER);
        let maxc = ((loc.width() - 190.0) / 7.0) as usize;
        let disp = match std::env::var("LOCALAPPDATA") {
            Ok(l) if self.dir.starts_with(&l) => self.dir.replacen(&l, "%LOCALAPPDATA%", 1),
            _ => self.dir.clone(),
        };
        let shown: String = if disp.chars().count() > maxc { format!("…{}", disp.chars().skip(disp.chars().count() - maxc + 1).collect::<String>()) } else { disp };
        u.text(Pos2::new(loc.left() + 76.0, loc.center().y), &shown, mono(12.0), TEXT1, Align2::LEFT_CENTER);
        let cb = Rect::from_min_size(Pos2::new(loc.right() - 92.0, loc.top() + 6.0), vec2(84.0, 28.0));
        let lbl = if self.custom { self.t("Gizle", "Hide") } else { self.t("Özelleştir", "Customize") };
        if u.button(cb, "customize", lbl, false) {
            self.custom = !self.custom;
        }
        y += 52.0;
        if self.custom {
            let br = Rect::from_min_size(Pos2::new(c.left(), y), vec2(120.0, 28.0));
            if u.button(br, "browse", self.t("Klasör seç…", "Browse…"), false) {
                if let Some(d) = rfd::FileDialog::new().pick_folder() {
                    let mut s = d.to_string_lossy().to_string();
                    if !s.to_lowercase().ends_with("ferro") {
                        s = format!("{}\\Ferro", s.trim_end_matches('\\'));
                    }
                    self.dir = s;
                }
            }
            let mut d = self.desktop;
            let mut m = self.startmenu;
            u.checkbox(Pos2::new(c.left() + 136.0, y), "cb-desk", self.t("Masaüstü kısayolu", "Desktop shortcut"), &mut d, 200.0);
            y += 32.0;
            u.checkbox(Pos2::new(c.left() + 136.0, y), "cb-start", self.t("Başlat menüsü kısayolu", "Start menu shortcut"), &mut m, 240.0);
            self.desktop = d;
            self.startmenu = m;
            y += 30.0;
        }
        // actions
        let by = c.bottom() - 52.0;
        let install = Rect::from_min_size(Pos2::new(c.left(), by), vec2(220.0, 50.0));
        let label = if self.updating { self.t("Güncelle", "Update") } else { self.t("Yükle", "Install") };
        if u.button(install, "install", label, true) {
            self.start_install(ctx);
        }
        let note = self.t("≈ 7 MB · yönetici izni gerekmez", "≈ 7 MB · no admin rights needed");
        u.text(Pos2::new(c.left() + 236.0, by + 25.0), note, rg(12.0), TEXT3, Align2::LEFT_CENTER);
        let _ = y;
    }

    fn page_progress(&mut self, u: &mut Ui2, c: Rect, un: bool) {
        let title = if un { self.t("Kaldırılıyor…", "Uninstalling…") } else { self.t("Kuruluyor…", "Installing…") };
        u.text(c.left_top(), title, sb(30.0), TEXT1, Align2::LEFT_TOP);
        // big percentage
        let pct = (self.shown * 100.0).round() as u32;
        u.text(Pos2::new(c.left(), c.top() + 58.0), &format!("{pct}"), monob(64.0), ACCENT, Align2::LEFT_TOP);
        let pw = u.ui.painter().layout_no_wrap(format!("{pct}"), monob(64.0), ACCENT).size().x;
        u.text(Pos2::new(c.left() + pw + 6.0, c.top() + 98.0), "%", mono(22.0), TEXT3, Align2::LEFT_TOP);
        // bar with shimmer
        let by = c.top() + 146.0;
        let bar = Rect::from_min_size(Pos2::new(c.left(), by), vec2(c.width(), 10.0));
        let p = u.ui.painter().clone();
        p.rect_filled(bar, cr(5), ca(BG3, u.fade));
        let fw = (bar.width() * self.shown).max(0.0);
        if fw > 2.0 {
            let fr = Rect::from_min_size(bar.min, vec2(fw, 10.0));
            p.rect_filled(fr, cr(5), ca(ACCENT, u.fade));
            let t = self.t0.elapsed().as_secs_f32();
            let sx = fr.left() + ((t * 0.9) % 1.0) * (fr.width() + 80.0) - 40.0;
            let sh = Rect::from_min_max(Pos2::new(sx.max(fr.left()), fr.top()), Pos2::new((sx + 40.0).min(fr.right()), fr.bottom()));
            if sh.width() > 0.0 {
                p.rect_filled(sh, cr(5), ca(Color32::WHITE, 0.28 * u.fade));
            }
        }
        // steps
        let mut y = by + 34.0;
        let steps = self.steps.clone();
        for (i, s) in steps.iter().enumerate() {
            let done = i < self.step || (i == self.step && self.shown > 0.97);
            let active = i == self.step && !done;
            let cx = c.left() + 9.0;
            let cy = y + 10.0;
            if done {
                p.circle_filled(Pos2::new(cx, cy), 9.0, ca(GREEN, 0.18 * u.fade));
                let st = Stroke::new(1.8, ca(GREEN, u.fade));
                p.line_segment([Pos2::new(cx - 4.0, cy), Pos2::new(cx - 1.0, cy + 3.0)], st);
                p.line_segment([Pos2::new(cx - 1.0, cy + 3.0), Pos2::new(cx + 4.0, cy - 3.5)], st);
            } else if active {
                let t = self.t0.elapsed().as_secs_f32();
                p.circle_stroke(Pos2::new(cx, cy), 8.0, Stroke::new(1.5, ca(ACCENT, 0.35 * u.fade)));
                let a = t * 6.0;
                p.circle_filled(Pos2::new(cx + a.cos() * 8.0, cy + a.sin() * 8.0), 2.6, ca(ACCENT, u.fade));
            } else {
                p.circle_stroke(Pos2::new(cx, cy), 8.0, Stroke::new(1.5, ca(LINE2, u.fade)));
            }
            u.text(Pos2::new(c.left() + 30.0, cy), s, if active { sb(14.0) } else { rg(14.0) }, if done || active { TEXT1 } else { TEXT3 }, Align2::LEFT_CENTER);
            y += 30.0;
        }
        // faux terminal
        let tr = Rect::from_min_max(Pos2::new(c.left(), c.bottom() - 92.0), Pos2::new(c.right(), c.bottom()));
        p.rect_filled(tr, cr(9), ca(BG0, u.fade));
        p.rect_stroke(tr, cr(9), Stroke::new(1.0, ca(LINE, u.fade)), StrokeKind::Inside);
        let n = self.log.len();
        for (k, line) in self.log.iter().skip(n.saturating_sub(3)).enumerate() {
            let last = k + 1 == n.min(3);
            u.text(Pos2::new(tr.left() + 14.0, tr.top() + 20.0 + k as f32 * 22.0), &format!("$ {line}"), mono(11.5), if last { TEXT1 } else { TEXT3 }, Align2::LEFT_CENTER);
        }
    }

    fn page_done(&mut self, u: &mut Ui2, c: Rect, un: bool) {
        // animated ring + check
        let age = self.page_at.elapsed().as_secs_f32();
        let ring = (age / 0.6).clamp(0.0, 1.0);
        let chk = ((age - 0.35) / 0.4).clamp(0.0, 1.0);
        let cc = Pos2::new(c.left() + 36.0, c.top() + 36.0);
        let p = u.ui.painter().clone();
        p.circle_filled(cc, 36.0, ca(GREEN, 0.10 * u.fade));
        // arc
        let steps = (ring * 64.0) as usize;
        let mut pts = vec![];
        for i in 0..=steps {
            let a = -std::f32::consts::FRAC_PI_2 + (i as f32 / 64.0) * std::f32::consts::TAU;
            pts.push(cc + vec2(a.cos(), a.sin()) * 30.0);
        }
        if pts.len() > 1 {
            p.add(egui::Shape::line(pts, Stroke::new(3.0, ca(GREEN, u.fade))));
        }
        let a = cc + vec2(-11.0, 1.0);
        let b = cc + vec2(-3.0, 9.0);
        let d = cc + vec2(12.0, -9.0);
        let st = Stroke::new(3.5, ca(GREEN, u.fade));
        if chk > 0.0 {
            let m1 = a + (b - a) * (chk * 2.0).min(1.0);
            p.line_segment([a, m1], st);
            if chk > 0.5 {
                let m2 = b + (d - b) * ((chk - 0.5) * 2.0);
                p.line_segment([b, m2], st);
            }
        }
        let title = if un { self.t("Ferro kaldırıldı", "Ferro was removed") } else { self.t("Ferro hazır", "Ferro is ready") };
        u.text(Pos2::new(c.left(), c.top() + 92.0), title, sb(30.0), TEXT1, Align2::LEFT_TOP);
        let sub = if un { self.t("Tekrar görüşmek üzere.", "See you again soon.") } else { self.t("Depolarını aç, bir şeyleri hazırla ve commit et. Komut paleti için Ctrl K.", "Open a repository, stage something, commit. Press Ctrl K for the command palette.") };
        u.wrapped(Pos2::new(c.left(), c.top() + 136.0), sub, rg(14.0), TEXT2, c.width());
        let by = c.bottom() - 50.0;
        if !un {
            let keys = [("Ctrl K", self.t("komut paleti", "command palette")), ("Ctrl B", self.t("dal değiştir", "switch branch")), ("Ctrl O", self.t("depo aç", "open a repository")), ("Space", self.t("dosya hazırla / çıkar", "stage / unstage"))];
            let mut ky = c.top() + 190.0;
            for (k, label) in keys {
                let kw = u.ui.painter().layout_no_wrap(k.to_string(), mono(11.5), TEXT2).size().x + 18.0;
                let kr = Rect::from_min_size(Pos2::new(c.left(), ky), vec2(kw.max(62.0), 22.0));
                let p = u.ui.painter().clone();
                p.rect_filled(kr, cr(5), ca(BG0, u.fade));
                p.rect_stroke(kr, cr(5), Stroke::new(1.0, ca(LINE2, u.fade)), StrokeKind::Inside);
                u.text(kr.center(), k, mono(11.5), TEXT2, Align2::CENTER_CENTER);
                u.text(Pos2::new(kr.right() + 14.0, kr.center().y), label, rg(13.0), TEXT2, Align2::LEFT_CENTER);
                ky += 30.0;
            }
            let mut l = self.launch;
            u.checkbox(Pos2::new(c.left(), by - 44.0), "cb-launch", self.t("Ferro'yu şimdi başlat", "Launch Ferro now"), &mut l, 300.0);
            self.launch = l;
        }
        let btn = Rect::from_min_size(Pos2::new(c.left(), by), vec2(200.0, 50.0));
        if u.button(btn, "finish", self.t("Bitir", "Finish"), true) {
            if !un && self.launch {
                let exe = std::path::Path::new(&self.dir).join("ferro.exe");
                let _ = std::process::Command::new(exe).spawn();
            }
            if un {
                self_delete();
            }
            std::process::exit(0);
        }
    }

    fn page_failed(&mut self, u: &mut Ui2, c: Rect) {
        u.text(c.left_top(), self.t("Bir şeyler ters gitti", "Something went wrong"), sb(30.0), RED, Align2::LEFT_TOP);
        let err = self.err.clone();
        let r = Rect::from_min_size(Pos2::new(c.left(), c.top() + 64.0), vec2(c.width(), 120.0));
        let p = u.ui.painter().clone();
        p.rect_filled(r, cr(9), ca(BG0, u.fade));
        p.rect_stroke(r, cr(9), Stroke::new(1.0, ca(Color32::from_rgb(0x6B, 0x3B, 0x3B), u.fade)), StrokeKind::Inside);
        u.wrapped(r.left_top() + vec2(14.0, 12.0), &err, mono(12.0), TEXT1, r.width() - 28.0);
        let by = c.bottom() - 50.0;
        let retry = Rect::from_min_size(Pos2::new(c.left(), by), vec2(180.0, 50.0));
        if u.button(retry, "retry", self.t("Tekrar dene", "Try again"), true) {
            self.goto(Page::Welcome);
        }
        let close = Rect::from_min_size(Pos2::new(c.left() + 192.0, by), vec2(120.0, 50.0));
        if u.button(close, "closeerr", self.t("Kapat", "Close"), false) {
            std::process::exit(1);
        }
    }

    fn page_uconfirm(&mut self, u: &mut Ui2, c: Rect, ctx: &egui::Context) {
        let y = c.top() + self.title(u, c, self.t("Ferro kaldırılsın mı?", "Remove Ferro?"), self.t("Uygulama, kısayollar ve kayıt girdisi silinir. Depoların ve commit'lerin olduğu gibi kalır.", "The app, its shortcuts and registry entry are removed. Your repositories and commits are untouched."));
        let mut w = self.wipe;
        u.checkbox(Pos2::new(c.left(), y), "cb-wipe", self.t("Ayarları ve son depo listesini de sil", "Also delete settings and the recent-repository list"), &mut w, c.width());
        self.wipe = w;
        let by = c.bottom() - 50.0;
        let go = Rect::from_min_size(Pos2::new(c.left(), by), vec2(200.0, 50.0));
        if u.button(go, "uninstall", self.t("Kaldır", "Remove"), true) {
            self.start_uninstall(ctx);
        }
        let cancel = Rect::from_min_size(Pos2::new(c.left() + 212.0, by), vec2(120.0, 50.0));
        if u.button(cancel, "ucancel", self.t("Vazgeç", "Cancel"), false) {
            std::process::exit(0);
        }
    }
}
