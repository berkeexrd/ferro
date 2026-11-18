//! Design tokens: dark and light palettes, type scale, fonts and egui visuals.
#![allow(dead_code)]

use eframe::egui::{self, Color32, FontFamily, FontId};

const fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// CSS-style alpha: premultiply in gamma space (egui's own constructor premultiplies in linear
/// space, which makes translucent tints several times stronger than the design).
pub fn ca(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    let m = |c: u8| ((c as u32 * a as u32 + 127) / 255) as u8;
    Color32::from_rgba_premultiplied(m(r), m(g), m(b), a)
}

fn rgba(v: u32, a: u8) -> Color32 {
    ca((v >> 16) as u8, (v >> 8) as u8, v as u8, a)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Mode {
    Dark,
    Light,
    System,
}

#[derive(Clone)]
pub struct Palette {
    pub dark: bool,
    pub bg0: Color32,
    pub bg1: Color32,
    pub bg2: Color32,
    pub bg3: Color32,
    pub bg_sel: Color32, // row selected tint
    pub line: Color32,
    pub line_soft: Color32,
    pub line_strong: Color32,
    pub text1: Color32,
    pub text2: Color32,
    pub text3: Color32,
    pub code_gutter: Color32,
    pub accent: Color32,
    pub accent_hi: Color32,
    pub accent_dim: Color32,
    pub accent_soft: Color32,
    pub accent_fg: Color32, // text on accent
    pub accent_line: Color32,
    pub added: Color32,
    pub modified: Color32,
    pub deleted: Color32,
    pub renamed: Color32,
    pub untracked: Color32,
    pub conflict: Color32,
    pub merged: Color32,
    pub danger_line: Color32,
    pub diff_add: Color32,
    pub diff_del: Color32,
    pub diff_add_word: Color32,
    pub diff_head: Color32,
    pub diff_head_line: Color32,
    pub diff_ctx: Color32,
    pub sx_kw: Color32,
    pub sx_ty: Color32,
    pub sx_fn: Color32,
    pub sx_com: Color32,
    pub sx_mac: Color32,
    pub sx_num: Color32,
    pub sx_str: Color32,
    pub shadow: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        Self {
            dark: true,
            bg0: rgb(0x151412),
            bg1: rgb(0x1B1A17),
            bg2: rgb(0x201F1B),
            bg3: rgb(0x2A2824),
            bg_sel: rgb(0x3A2B22),
            line: rgb(0x36332D),
            line_soft: rgb(0x2A2824),
            line_strong: rgb(0x4A463E),
            text1: rgb(0xECE8E1),
            text2: rgb(0xB1AAA0),
            text3: rgb(0x9A9388),
            code_gutter: rgb(0x8F887D),
            accent: rgb(0xE58A55),
            accent_hi: rgb(0xF0A173),
            accent_dim: rgb(0x8A5A3C),
            accent_soft: rgba(0xE58A55, 0x26),
            accent_fg: rgb(0x1A1109),
            accent_line: rgb(0xB9683A),
            added: rgb(0x7FD3A5),
            modified: rgb(0xE8BE55),
            deleted: rgb(0xF29B9B),
            renamed: rgb(0x7FB5F2),
            untracked: rgb(0x86CBC6),
            conflict: rgb(0xEE8FBF),
            merged: rgb(0xB79CF5),
            danger_line: rgb(0x6B3B3B),
            diff_add: rgba(0x5FBF8A, 0x21),
            diff_del: rgba(0xF07A7A, 0x21),
            diff_add_word: rgba(0x5FBF8A, 0x61),
            diff_head: rgb(0x26241F),
            diff_head_line: rgb(0x2F2D28),
            diff_ctx: rgb(0x201F1B),
            sx_kw: rgb(0xE39D6D),
            sx_ty: rgb(0x7FB5F2),
            sx_fn: rgb(0xE8D8A0),
            sx_com: rgb(0x8F887D),
            sx_mac: rgb(0x86CBC6),
            sx_num: rgb(0xE8BE55),
            sx_str: rgb(0x9CCFA6),
            shadow: Color32::from_black_alpha(150),
        }
    }

    pub fn light() -> Self {
        Self {
            dark: false,
            bg0: rgb(0xECE8E0),
            bg1: rgb(0xF3F0EA),
            bg2: rgb(0xFBFAF7),
            bg3: rgb(0xFFFFFF),
            bg_sel: rgb(0xF4E3D8),
            line: rgb(0xD9D4CA),
            line_soft: rgb(0xE4DFD5),
            line_strong: rgb(0xCFC9BE),
            text1: rgb(0x1D1B17),
            text2: rgb(0x5B564C),
            text3: rgb(0x6E685D),
            code_gutter: rgb(0x8A8478),
            accent: rgb(0xB4501E),
            accent_hi: rgb(0x9A4318),
            accent_dim: rgb(0xD9A58A),
            accent_soft: rgba(0xB4501E, 0x1F),
            accent_fg: rgb(0xFFFFFF),
            accent_line: rgb(0x8F3E15),
            added: rgb(0x1B6B42),
            modified: rgb(0x7A5600),
            deleted: rgb(0xA8231B),
            renamed: rgb(0x1A56A8),
            untracked: rgb(0x1F6A66),
            conflict: rgb(0x9A2B63),
            merged: rgb(0x5F3DB5),
            danger_line: rgb(0xD9A5A0),
            diff_add: rgba(0x1B6B42, 0x1C),
            diff_del: rgba(0xA8231B, 0x18),
            diff_add_word: rgba(0x1B6B42, 0x40),
            diff_head: rgb(0xF3F0EA),
            diff_head_line: rgb(0xE4DFD5),
            diff_ctx: rgb(0xFBFAF7),
            sx_kw: rgb(0xA8481A),
            sx_ty: rgb(0x1A56A8),
            sx_fn: rgb(0x7A5600),
            sx_com: rgb(0x8A8478),
            sx_mac: rgb(0x1F6A66),
            sx_num: rgb(0x8A5E00),
            sx_str: rgb(0x2D7A4E),
            shadow: Color32::from_black_alpha(40),
        }
    }

    /// Status tint (background chip) for a status color.
    pub fn tint(&self, c: Color32) -> Color32 {
        let a = if self.dark { 0x33 } else { 0x22 };
        ca(c.r(), c.g(), c.b(), a)
    }

    pub fn tint_a(&self, c: Color32, a: u8) -> Color32 {
        ca(c.r(), c.g(), c.b(), a)
    }
}

/// Type styles (size / line height / weight in the trailing comments).
#[derive(Clone, Copy)]
pub enum Ty {
    Title,     // 24/32 600
    Section,   // 16/24 600
    Body,      // 14/20 400
    Row,       // 13/18
    RowStrong, // 13/18 600
    Label,     // 11/14 500
    Small,     // 12
    Mono,      // 13
    MonoSm,    // 12
    MonoXs,    // 11
    Heading20,
}

pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

pub fn monobold() -> FontFamily {
    FontFamily::Name("monobold".into())
}

pub fn font(t: Ty, scale: f32) -> FontId {
    let s = |v: f32| v * scale;
    match t {
        Ty::Title => FontId::new(s(24.0), semibold()),
        Ty::Heading20 => FontId::new(s(20.0), semibold()),
        Ty::Section => FontId::new(s(16.0), semibold()),
        Ty::Body => FontId::new(s(14.0), FontFamily::Proportional),
        Ty::Row => FontId::new(s(13.0), FontFamily::Proportional),
        Ty::RowStrong => FontId::new(s(13.0), semibold()),
        Ty::Label => FontId::new(s(11.0), FontFamily::Proportional),
        Ty::Small => FontId::new(s(12.0), FontFamily::Proportional),
        Ty::Mono => FontId::new(s(13.0), FontFamily::Monospace),
        Ty::MonoSm => FontId::new(s(12.0), FontFamily::Monospace),
        Ty::MonoXs => FontId::new(s(11.0), FontFamily::Monospace),
    }
}

#[derive(Clone)]
pub struct Theme {
    pub p: Palette,
    pub scale: f32,
    pub compact: bool,
}

impl Theme {
    pub fn new(mode: Mode, scale: f32, compact: bool, system_dark: bool) -> Self {
        let dark = match mode {
            Mode::Dark => true,
            Mode::Light => false,
            Mode::System => system_dark,
        };
        Self { p: if dark { Palette::dark() } else { Palette::light() }, scale, compact }
    }

    pub fn f(&self, t: Ty) -> FontId {
        font(t, 1.0)
    }

    /// Row height of lists (design: 32 normal, 28 compact).
    pub fn row_h(&self) -> f32 {
        if self.compact { 28.0 } else { 32.0 }
    }

    pub fn commit_row_h(&self) -> f32 {
        if self.compact { 38.0 } else { 44.0 }
    }
}

/// Install fonts: Inter (UI), JetBrains Mono (code). System Segoe fonts only as glyph fallback.
pub fn install_fonts(ctx: &egui::Context) {
    install_fonts_ex(ctx, vec![]);
}

/// `extra` are CJK / other-script fallback fonts loaded in the background after start-up.
pub fn install_fonts_ex(ctx: &egui::Context, extra: Vec<(String, Vec<u8>)>) {
    use std::sync::Arc;
    let mut fonts = egui::FontDefinitions::empty();
    let mut add = |name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(name.to_owned(), Arc::new(egui::FontData::from_static(bytes)));
    };
    add("inter", include_bytes!("../assets/fonts/Inter-Regular.otf"));
    add("inter-sb", include_bytes!("../assets/fonts/Inter-SemiBold.otf"));
    add("jb", include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"));
    add("jb-bold", include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf"));

    let mut fallbacks: Vec<String> = vec![];
    for (name, file) in [("segoeui", "segoeui.ttf"), ("seguisym", "seguisym.ttf"), ("seguiemj", "seguiemj.ttf")] {
        let p = std::path::Path::new("C:\\Windows\\Fonts").join(file);
        if let Ok(bytes) = std::fs::read(&p) {
            fonts.font_data.insert(name.to_owned(), Arc::new(egui::FontData::from_owned(bytes)));
            fallbacks.push(name.to_owned());
        }
    }
    for (name, bytes) in extra {
        fonts.font_data.insert(name.clone(), Arc::new(egui::FontData::from_owned(bytes)));
        fallbacks.push(name);
    }
    let with_fb = |first: &[&str]| -> Vec<String> {
        let mut v: Vec<String> = first.iter().map(|s| s.to_string()).collect();
        v.extend(fallbacks.iter().cloned());
        v
    };
    fonts.families.insert(FontFamily::Proportional, with_fb(&["inter"]));
    fonts.families.insert(FontFamily::Monospace, with_fb(&["jb", "inter"]));
    fonts.families.insert(semibold(), with_fb(&["inter-sb", "inter"]));
    fonts.families.insert(monobold(), with_fb(&["jb-bold", "jb"]));
    ctx.set_fonts(fonts);
}

/// Neutralise egui's own widget look: Ferro draws everything itself.
pub fn apply_visuals(ctx: &egui::Context, th: &Theme) {
    let p = &th.p;
    let mut v = if p.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = p.bg2;
    v.window_fill = p.bg3;
    v.extreme_bg_color = p.bg0;
    v.faint_bg_color = p.bg1;
    v.override_text_color = Some(p.text1);
    v.selection.bg_fill = p.accent_soft;
    v.selection.stroke = egui::Stroke::new(1.0, p.accent);
    v.hyperlink_color = p.accent_hi;
    v.window_stroke = egui::Stroke::new(1.0, p.line_strong);
    v.window_corner_radius = egui::CornerRadius::same(10);
    v.menu_corner_radius = egui::CornerRadius::same(6);
    v.popup_shadow = egui::epaint::Shadow { offset: [0, 4], blur: 14, spread: 0, color: Color32::from_black_alpha(115) };
    v.window_shadow = egui::epaint::Shadow { offset: [0, 12], blur: 40, spread: 0, color: p.shadow };
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = egui::CornerRadius::same(6);
    }
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.line_soft);
    ctx.set_visuals(v);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.scroll = egui::style::ScrollStyle::thin();
    style.spacing.scroll.bar_width = 6.0;
    style.interaction.tooltip_delay = 0.35;
    ctx.set_style(style);
}
