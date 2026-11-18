//! Component library: buttons, badges, checkboxes, pills and text inputs.
//! Everything is painted by hand so it follows the design tokens exactly.

use crate::i18n::{PainterT};
use crate::icons::{self, Icon};
use crate::theme::{semibold, Palette, Theme, Ty};
use eframe::egui::{
    self, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Layout, Painter, Pos2, Rect, Response, Sense,
    Stroke, StrokeKind, Ui, UiBuilder,
};

pub fn cr(r: u8) -> CornerRadius {
    CornerRadius::same(r)
}

pub fn text_width(ui: &Ui, text: &str, font: &FontId) -> f32 {
    let text = crate::i18n::t(text);
    let text: &str = &text;
    ui.painter().layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE).size().x
}

/// Draw text starting at `x` vertically centered on `cy`; returns the right edge.
pub fn draw_left(painter: &Painter, x: f32, cy: f32, text: &str, font: FontId, color: Color32) -> f32 {
    let text = crate::i18n::t(text);
    let text: &str = &text;
    let g = painter.layout_no_wrap(text.to_owned(), font, color);
    let w = g.size().x;
    let h = g.size().y;
    painter.galley(Pos2::new(x, cy - h / 2.0), g, color);
    x + w
}

/// Draw text right-aligned to `x_right`; returns the left edge.
pub fn draw_right(painter: &Painter, x_right: f32, cy: f32, text: &str, font: FontId, color: Color32) -> f32 {
    let text = crate::i18n::t(text);
    let text: &str = &text;
    let g = painter.layout_no_wrap(text.to_owned(), font, color);
    let w = g.size().x;
    let h = g.size().y;
    painter.galley(Pos2::new(x_right - w, cy - h / 2.0), g, color);
    x_right - w
}

/// Single-line text clipped with an ellipsis at `max_w`.
pub fn draw_trunc(painter: &Painter, x: f32, cy: f32, max_w: f32, text: &str, font: FontId, color: Color32) -> f32 {
    let text = crate::i18n::t(text);
    let text: &str = &text;
    if max_w <= 4.0 {
        return x;
    }
    let mut job = egui::text::LayoutJob::single_section(
        text.to_owned(),
        egui::TextFormat { font_id: font, color, ..Default::default() },
    );
    job.wrap.max_width = max_w;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let g = painter.layout_job(job);
    let h = g.size().y;
    let w = g.size().x;
    painter.galley(Pos2::new(x, cy - h / 2.0), g, color);
    x + w
}

pub fn centered(painter: &Painter, rect: Rect, text: &str, font: FontId, color: Color32) {
    painter.text_t(rect.center(), Align2::CENTER_CENTER, text, font, color);
}

pub fn hline(painter: &Painter, x0: f32, x1: f32, y: f32, color: Color32) {
    painter.line_segment([Pos2::new(x0, y), Pos2::new(x1, y)], Stroke::new(1.0, color));
}

pub fn vline(painter: &Painter, x: f32, y0: f32, y1: f32, color: Color32) {
    painter.line_segment([Pos2::new(x, y0), Pos2::new(x, y1)], Stroke::new(1.0, color));
}

pub fn pointer(resp: &Response) {
    if resp.hovered() {
        resp.ctx.set_cursor_icon(CursorIcon::PointingHand);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

pub struct Btn {
    label: String,
    kind: Kind,
    icon: Option<Icon>,
    trail_icon: Option<Icon>,
    h: f32,
    min_w: f32,
    enabled: bool,
    selected: bool,
    pad: f32,
}

impl Btn {
    pub fn new(label: impl Into<String>) -> Self {
        let label: String = label.into();
        let label = crate::i18n::t(&label).into_owned();
        Self { label, kind: Kind::Secondary, icon: None, trail_icon: None, h: 32.0, min_w: 0.0, enabled: true, selected: false, pad: 12.0 }
    }
    pub fn kind(mut self, k: Kind) -> Self {
        self.kind = k;
        self
    }
    pub fn primary(self) -> Self {
        self.kind(Kind::Primary)
    }
    pub fn ghost(self) -> Self {
        self.kind(Kind::Ghost)
    }
    pub fn danger(self) -> Self {
        self.kind(Kind::Danger)
    }
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn trail(mut self, i: Icon) -> Self {
        self.trail_icon = Some(i);
        self
    }
    pub fn height(mut self, h: f32) -> Self {
        self.h = h;
        self
    }
    pub fn small(mut self) -> Self {
        self.h = 28.0;
        self.pad = 10.0;
        self
    }
    pub fn tiny(mut self) -> Self {
        self.h = 24.0;
        self.pad = 10.0;
        self
    }
    pub fn min_w(mut self, w: f32) -> Self {
        self.min_w = w;
        self
    }
    pub fn enabled(mut self, e: bool) -> Self {
        self.enabled = e;
        self
    }
    pub fn selected(mut self, s: bool) -> Self {
        self.selected = s;
        self
    }
}

pub fn button(ui: &mut Ui, th: &Theme, b: Btn) -> Response {
    let p = &th.p;
    let font = if b.kind == Kind::Secondary && !b.selected && b.h <= 28.0 { th.f(Ty::Small) } else if b.h <= 26.0 { th.f(Ty::Small) } else { th.f(Ty::Row) };
    let font = if b.kind == Kind::Primary || b.selected { FontId::new(font.size, semibold()) } else { font };
    let icon_sz = if b.h >= 32.0 { 14.0 } else { 13.0 };
    let gap = 6.0;
    let mut w = b.pad * 2.0;
    if b.icon.is_some() {
        w += icon_sz + gap;
    }
    if !b.label.is_empty() {
        w += text_width(ui, &b.label, &font);
    }
    if b.trail_icon.is_some() {
        w += gap + 11.0;
    }
    let natural = w;
    let w = w.max(b.min_w);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, b.h), if b.enabled { Sense::click() } else { Sense::hover() });
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let hov = resp.hovered() && b.enabled;
    let down = resp.is_pointer_button_down_on() && b.enabled;
    let (bg, border, fg) = match b.kind {
        Kind::Primary => (
            if !b.enabled { p.bg3 } else if down { p.accent_line } else if hov { p.accent_hi } else { p.accent },
            Color32::TRANSPARENT,
            if b.enabled { p.accent_fg } else { p.text3 },
        ),
        Kind::Secondary => (
            if b.selected { p.bg_sel } else if down { p.bg1 } else if hov { if p.dark { Color32::from_rgb(0x33, 0x30, 0x2B) } else { Color32::from_rgb(0xF4, 0xF1, 0xEB) } } else { p.bg3 },
            if b.selected { p.accent } else { p.line_strong },
            if b.enabled { p.text1 } else { p.text3 },
        ),
        Kind::Ghost => (
            if down || hov { p.bg3 } else { Color32::TRANSPARENT },
            Color32::TRANSPARENT,
            if hov { p.text1 } else { p.text2 },
        ),
        Kind::Danger => (
            if hov { p.tint_a(p.deleted, 0x1E) } else { Color32::TRANSPARENT },
            p.danger_line,
            p.deleted,
        ),
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, cr(6), bg);
    if border != Color32::TRANSPARENT {
        painter.rect_stroke(rect, cr(6), Stroke::new(1.0, border), StrokeKind::Inside);
    }
    let cy = rect.center().y;
    let mut x = rect.left() + b.pad + (rect.width() - natural) / 2.0;
    if let Some(i) = b.icon {
        icons::paint_at(ui, Pos2::new(x + icon_sz / 2.0, cy), icon_sz, i, fg);
        x += icon_sz + gap;
    }
    if !b.label.is_empty() {
        x = draw_left(&painter, x, cy, &b.label, font.clone(), fg);
    }
    if let Some(t) = b.trail_icon {
        icons::paint_at(ui, Pos2::new(x + gap + 5.5, cy), 11.0, t, fg);
    }
    pointer(&resp);
    resp
}

/// 40x40 style rail / toolbar icon button.
pub fn icon_button(ui: &mut Ui, th: &Theme, icon: Icon, size: f32, active: bool) -> Response {
    let p = &th.p;
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let hov = resp.hovered();
    let bg = if active { p.bg3 } else if hov { p.bg3 } else { Color32::TRANSPARENT };
    ui.painter().rect_filled(rect, cr(if size >= 36.0 { 8 } else { 6 }), bg);
    let fg = if active { p.accent } else if hov { p.text1 } else { p.text2 };
    icons::paint_at(ui, rect.center(), if size >= 36.0 { 18.0 } else { 16.0 }, icon, fg);
    pointer(&resp);
    resp
}

/// Split button: primary action + chevron half. Returns (main, chevron).
pub fn split_button(ui: &mut Ui, th: &Theme, label: &str, icon: Icon, count: Option<String>, enabled: bool) -> (Response, Response) {
    let p = &th.p;
    let font = FontId::new(13.0, semibold());
    let kbd_font = th.f(Ty::MonoXs);
    let mut w = 12.0 * 2.0 + 14.0 + 6.0 + text_width(ui, label, &font);
    if let Some(c) = &count {
        w += 6.0 + text_width(ui, c, &kbd_font) + 10.0;
    }
    let h = 32.0;
    let (r1, resp1) = ui.allocate_exact_size(vec2(w, h), Sense::click());
    let (r2, resp2) = ui.allocate_exact_size(vec2(28.0, h), Sense::click());
    ui.add_space(-ui.spacing().item_spacing.x);
    let all = r1.union(r2);
    let painter = ui.painter_at(all);
    let bg = |hov: bool, down: bool| if !enabled { p.bg3 } else if down { p.accent_line } else if hov { p.accent_hi } else { p.accent };
    let fg = if enabled { p.accent_fg } else { p.text3 };
    painter.rect_filled(r1, CornerRadius { nw: 6, sw: 6, ne: 0, se: 0 }, bg(resp1.hovered(), resp1.is_pointer_button_down_on()));
    painter.rect_filled(r2, CornerRadius { nw: 0, sw: 0, ne: 6, se: 6 }, bg(resp2.hovered(), resp2.is_pointer_button_down_on()));
    vline(&painter, r2.left(), r2.top(), r2.bottom(), if enabled { p.accent_line } else { p.line_strong });
    let cy = r1.center().y;
    let mut x = r1.left() + 12.0;
    icons::paint_at(ui, Pos2::new(x + 7.0, cy), 14.0, icon, fg);
    x += 20.0;
    x = draw_left(&painter, x, cy, label, font, fg);
    if let Some(c) = count {
        x += 6.0;
        let tw = text_width(ui, &c, &kbd_font);
        let r = Rect::from_min_size(Pos2::new(x, cy - 8.0), vec2(tw + 10.0, 16.0));
        painter.rect_filled(r, cr(4), Color32::from_black_alpha(38));
        centered(&painter, r, &c, kbd_font, fg);
    }
    icons::paint_at(ui, r2.center(), 12.0, Icon::Chev, fg);
    pointer(&resp1);
    pointer(&resp2);
    (resp1, resp2)
}

pub fn status_color(p: &Palette, letter: char) -> Color32 {
    match letter {
        'A' => p.added,
        'M' | 'T' => p.modified,
        'D' => p.deleted,
        'R' | 'C' => p.renamed,
        '?' => p.untracked,
        '!' => p.conflict,
        'B' => p.merged,
        _ => p.text2,
    }
}

/// 20x20 status badge (letter + color, never color alone).
pub fn badge(painter: &Painter, p: &Palette, rect: Rect, letter: char) {
    let c = status_color(p, letter);
    painter.rect_filled(rect, cr(4), p.tint(c));
    painter.text_t(rect.center(), Align2::CENTER_CENTER, letter.to_string(), FontId::new(11.0, egui::FontFamily::Monospace), c);
}

pub fn avatar(painter: &Painter, p: &Palette, center: Pos2, size: f32, initials: &str) {
    painter.circle_filled(center, size / 2.0, if p.dark { Color32::from_rgb(0x3A, 0x37, 0x31) } else { Color32::from_rgb(0xD9, 0xD4, 0xCA) });
    painter.text_t(center, Align2::CENTER_CENTER, initials, FontId::new((size * 0.38).max(9.0), semibold()), p.text1);
}

pub fn initials_of(name: &str) -> String {
    let mut it = name.split(|c: char| !c.is_alphanumeric()).filter(|s| !s.is_empty());
    let a = it.next().and_then(|s| s.chars().next());
    let b = it.next().and_then(|s| s.chars().next());
    match (a, b) {
        (Some(a), Some(b)) => format!("{}{}", a.to_uppercase(), b.to_uppercase()),
        (Some(a), None) => a.to_uppercase().to_string(),
        _ => "?".into(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Cb {
    Off,
    On,
    Partial,
}

pub fn checkbox(ui: &mut Ui, th: &Theme, state: Cb) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(15.0, 15.0), Sense::click());
    paint_checkbox(ui, th, rect, state, resp.hovered());
    pointer(&resp);
    resp
}

pub fn paint_checkbox(ui: &Ui, th: &Theme, rect: Rect, state: Cb, hov: bool) {
    let p = &th.p;
    let painter = ui.painter();
    match state {
        Cb::On => {
            painter.rect_filled(rect, cr(3), p.accent);
            icons::paint_at(ui, rect.center(), 11.0, Icon::Check, p.accent_fg);
        }
        Cb::Partial => {
            painter.rect_filled(rect, cr(3), p.accent_soft);
            painter.rect_stroke(rect, cr(3), Stroke::new(1.5, p.accent), StrokeKind::Inside);
            let c = rect.center();
            painter.line_segment([Pos2::new(c.x - 3.5, c.y), Pos2::new(c.x + 3.5, c.y)], Stroke::new(1.5, p.accent));
        }
        Cb::Off => {
            painter.rect_stroke(rect, cr(3), Stroke::new(1.5, if hov { p.text2 } else { Color32::from_rgb(0x6A, 0x65, 0x5B) }), StrokeKind::Inside);
        }
    }
}

pub fn pill(ui: &mut Ui, th: &Theme, text: &str, selected: bool) -> Response {
    let text = crate::i18n::t(text);
    let text: &str = &text;
    let p = &th.p;
    let font = if selected { FontId::new(12.0, semibold()) } else { th.f(Ty::Small) };
    let tw = text_width(ui, text, &font);
    let (rect, resp) = ui.allocate_exact_size(vec2(tw + 18.0, 24.0), Sense::click());
    let painter = ui.painter_at(rect);
    if selected {
        painter.rect_filled(rect, cr(255), if p.dark { Color32::from_rgb(0x3A, 0x37, 0x31) } else { p.bg3 });
        if !p.dark {
            painter.rect_stroke(rect, cr(255), Stroke::new(1.0, p.line_strong), StrokeKind::Inside);
        }
    } else {
        painter.rect_stroke(rect, cr(255), Stroke::new(1.0, if resp.hovered() { p.line_strong } else { p.line }), StrokeKind::Inside);
    }
    centered(&painter, rect, text, font, if selected { p.text1 } else { p.text2 });
    pointer(&resp);
    resp
}

pub struct InputOpts<'a> {
    pub hint: &'a str,
    pub mono: bool,
    pub width: f32,
    pub height: f32,
    pub multiline: Option<usize>,
    pub id: Id,
    pub trailing: Option<String>,
    pub warn: bool,
}

impl<'a> InputOpts<'a> {
    pub fn new(id: Id, hint: &'a str) -> Self {
        Self { hint, mono: false, width: f32::INFINITY, height: 34.0, multiline: None, id, trailing: None, warn: false }
    }
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }
    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self
    }
    pub fn rows(mut self, r: usize) -> Self {
        self.multiline = Some(r);
        self
    }
    pub fn warn(mut self, w: bool) -> Self {
        self.warn = w;
        self
    }
    pub fn trailing(mut self, t: impl Into<String>) -> Self {
        self.trailing = Some(t.into());
        self
    }
}

/// Bordered text field with an accent focus ring. Returns the TextEdit response.
pub fn text_input(ui: &mut Ui, th: &Theme, text: &mut String, o: InputOpts) -> Response {
    let p = &th.p;
    let w = o.width.min(ui.available_width());
    let rows = o.multiline;
    let h = if let Some(r) = rows { (r as f32) * 18.0 + 16.0 } else { o.height };
    let outer = ui.allocate_exact_size(vec2(w, h), Sense::hover()).0;
    let font = if o.mono { th.f(Ty::MonoSm) } else { th.f(Ty::Row) };
    let trail_w = o.trailing.as_ref().map(|t| text_width(ui, t, &th.f(Ty::MonoXs)) + 6.0).unwrap_or(0.0);
    let inner = Rect::from_min_max(outer.min + vec2(10.0, if rows.is_some() { 8.0 } else { 0.0 }), outer.max - vec2(10.0 + trail_w, if rows.is_some() { 8.0 } else { 0.0 }));
    let bg_painter = ui.painter().clone();
    bg_painter.rect_filled(outer, cr(6), p.bg0);
    let mut child = ui.new_child(UiBuilder::new().max_rect(inner).layout(Layout::left_to_right(Align::Center)));
    let resp = if rows.is_some() {
        child.add(
            egui::TextEdit::multiline(text)
                .id(o.id)
                .hint_text(crate::i18n::rt(o.hint).color(p.text3).font(font.clone()))
                .font(font)
                .frame(false)
                .desired_width(inner.width())
                .desired_rows(rows.unwrap())
                .text_color(p.text1),
        )
    } else {
        child.add(
            egui::TextEdit::singleline(text)
                .id(o.id)
                .hint_text(crate::i18n::rt(o.hint).color(p.text3).font(font.clone()))
                .font(font)
                .frame(false)
                .vertical_align(Align::Center)
                .desired_width(inner.width())
                .text_color(p.text1),
        )
    };
    let focused = resp.has_focus();
    bg_painter.rect_stroke(outer, cr(6), Stroke::new(1.0, if focused { p.accent } else if resp.hovered() { p.line_strong } else { p.line }), StrokeKind::Inside);
    if focused {
        bg_painter.rect_stroke(outer.expand(2.0), cr(8), Stroke::new(2.0, p.tint_a(p.accent, 0x33)), StrokeKind::Outside);
    }
    if let Some(t) = &o.trailing {
        draw_right(&bg_painter, outer.right() - 10.0, outer.center().y, t, th.f(Ty::MonoXs), if o.warn { p.deleted } else { p.text2 });
    }
    resp
}

