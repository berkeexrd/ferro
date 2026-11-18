//! 16 px grid, 1.5 px stroke, round caps. Drawn white and tinted at paint time (= `currentColor`).
#![allow(dead_code)]

use eframe::egui::{self, Color32, Rect, Ui};

macro_rules! svg {
    ($body:expr) => {
        concat!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="#ffffff" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">"##,
            $body,
            "</svg>"
        )
    };
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Icon {
    Branch,
    Search,
    Check,
    X,
    Up,
    Down,
    Commit,
    Chev,
    ChevRight,
    Terminal,
    Plus,
    Edit,
    Trash,
    Pr,
    Diff,
    Stash,
    Gear,
    History,
    Undo,
    Redo,
    Folder,
    Sync,
    Tag,
    Copy,
    Pick,
    More,
    Comment,
    Alert,
    Link,
    Clone,
    Spark,
    Minus,
    Eye,
}

fn svg_of(i: Icon) -> &'static str {
    match i {
        Icon::Branch => svg!(r#"<circle cx="4" cy="3.5" r="1.5"/><circle cx="4" cy="12.5" r="1.5"/><circle cx="12" cy="6" r="1.5"/><path d="M4 5v6M12 7.5c0 3-3.5 3.5-6.2 4.6"/>"#),
        Icon::Search => svg!(r#"<circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5L14 14"/>"#),
        Icon::Check => svg!(r#"<path d="M3 8.5l3.2 3.2L13 4.8"/>"#),
        Icon::X => svg!(r#"<path d="M4 4l8 8M12 4l-8 8"/>"#),
        Icon::Up => svg!(r#"<path d="M8 13V3M3.8 7.2L8 3l4.2 4.2"/>"#),
        Icon::Down => svg!(r#"<path d="M8 3v10M3.8 8.8L8 13l4.2-4.2"/>"#),
        Icon::Commit => svg!(r#"<circle cx="8" cy="8" r="2.5"/><path d="M1.5 8h4M10.5 8h4"/>"#),
        Icon::Chev => svg!(r#"<path d="M4 6l4 4 4-4"/>"#),
        Icon::ChevRight => svg!(r#"<path d="M6 4l4 4-4 4"/>"#),
        Icon::Terminal => svg!(r#"<rect x="1.5" y="2.5" width="13" height="11" rx="2"/><path d="M4.5 6l2 2-2 2M8 10.5h3.5"/>"#),
        Icon::Plus => svg!(r#"<path d="M8 3v10M3 8h10"/>"#),
        Icon::Minus => svg!(r#"<path d="M3 8h10"/>"#),
        Icon::Edit => svg!(r#"<path d="M2.5 13.5l.6-3L11 2.6a1.4 1.4 0 0 1 2 0l.4.4a1.4 1.4 0 0 1 0 2L5.5 12.9z"/>"#),
        Icon::Trash => svg!(r#"<path d="M2.5 4.5h11M6 4.5V3h4v1.5M4 4.5l.6 8.5h6.8l.6-8.5"/>"#),
        Icon::Pr => svg!(r#"<circle cx="4" cy="3.5" r="1.5"/><circle cx="4" cy="12.5" r="1.5"/><circle cx="12" cy="12.5" r="1.5"/><path d="M4 5v6M12 11V6.5a2 2 0 0 0-2-2H8.5M10 2.5l-1.5 2 1.5 2"/>"#),
        Icon::Diff => svg!(r#"<rect x="2.5" y="2.5" width="11" height="11" rx="2"/><path d="M5.5 6.5h5M8 4v5M5.5 11h5"/>"#),
        Icon::Stash => svg!(r#"<path d="M2.5 6L8 3l5.5 3L8 9zM2.5 9.5l5.5 3 5.5-3"/>"#),
        Icon::Gear => svg!(r#"<circle cx="8" cy="8" r="2"/><path d="M8 1.8v1.7M8 12.5v1.7M1.8 8h1.7M12.5 8h1.7M3.6 3.6l1.2 1.2M11.2 11.2l1.2 1.2M12.4 3.6l-1.2 1.2M4.8 11.2l-1.2 1.2"/>"#),
        Icon::History => svg!(r#"<path d="M2.5 8a5.5 5.5 0 1 0 1.8-4.1M2.5 2.5v3h3M8 5v3.2l2 1.3"/>"#),
        Icon::Undo => svg!(r#"<path d="M5 3.5L2.5 6 5 8.5M2.5 6H9a4 4 0 0 1 0 8H6"/>"#),
        Icon::Redo => svg!(r#"<path d="M11 3.5L13.5 6 11 8.5M13.5 6H7a4 4 0 0 0 0 8h3"/>"#),
        Icon::Folder => svg!(r#"<path d="M1.5 4.5a1 1 0 0 1 1-1h3.2l1.5 1.7h6.3a1 1 0 0 1 1 1v6.3a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z"/>"#),
        Icon::Sync => svg!(r#"<path d="M13.5 6A5.5 5.5 0 0 0 3.2 4.8M2.5 2.5v2.8h2.8M2.5 10a5.5 5.5 0 0 0 10.3 1.2M13.5 13.5v-2.8h-2.8"/>"#),
        Icon::Tag => svg!(r#"<path d="M2 8.2V2.5h5.7l6.3 6.3-5.2 5.2z"/><circle cx="5.2" cy="5.2" r=".8"/>"#),
        Icon::Copy => svg!(r#"<rect x="5.5" y="5.5" width="8" height="8" rx="1.5"/><path d="M10.5 5.5V4a1.5 1.5 0 0 0-1.5-1.5H4A1.5 1.5 0 0 0 2.5 4v5A1.5 1.5 0 0 0 4 10.5h1.5"/>"#),
        Icon::Pick => svg!(r#"<circle cx="8" cy="8" r="2.5"/><path d="M8 1.5v4M8 10.5v4M3 3l2 2M11 11l2 2"/>"#),
        Icon::More => svg!(r##"<circle cx="3.5" cy="8" r=".9" fill="#ffffff"/><circle cx="8" cy="8" r=".9" fill="#ffffff"/><circle cx="12.5" cy="8" r=".9" fill="#ffffff"/>"##),
        Icon::Comment => svg!(r#"<path d="M2.5 3.5h11v7h-6l-3 2.5v-2.5h-2z"/>"#),
        Icon::Alert => svg!(r#"<path d="M8 2.5l6 10.5H2zM8 7v3M8 11.6v.1"/>"#),
        Icon::Link => svg!(r#"<path d="M6.5 9.5l3-3M5.5 7L4 8.5a2.5 2.5 0 0 0 3.5 3.5L9 10.5M10.5 9L12 7.5A2.5 2.5 0 0 0 8.5 4L7 5.5"/>"#),
        Icon::Clone => svg!(r#"<path d="M8 2v8M4.8 7L8 10.2 11.2 7M2.5 11.5v1.5h11v-1.5"/>"#),
        Icon::Spark => svg!(r#"<path d="M8 2l1.5 4.5L14 8l-4.5 1.5L8 14l-1.5-4.5L2 8l4.5-1.5z"/>"#),
        Icon::Eye => svg!(r#"<path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z"/><circle cx="8" cy="8" r="2"/>"#),
    }
}

fn source(i: Icon) -> egui::ImageSource<'static> {
    let uri: &'static str = match i {
        Icon::Branch => "bytes://ferro/branch.svg",
        Icon::Search => "bytes://ferro/search.svg",
        Icon::Check => "bytes://ferro/check.svg",
        Icon::X => "bytes://ferro/x.svg",
        Icon::Up => "bytes://ferro/up.svg",
        Icon::Down => "bytes://ferro/down.svg",
        Icon::Commit => "bytes://ferro/commit.svg",
        Icon::Chev => "bytes://ferro/chev.svg",
        Icon::ChevRight => "bytes://ferro/chevr.svg",
        Icon::Terminal => "bytes://ferro/terminal.svg",
        Icon::Plus => "bytes://ferro/plus.svg",
        Icon::Minus => "bytes://ferro/minus.svg",
        Icon::Edit => "bytes://ferro/edit.svg",
        Icon::Trash => "bytes://ferro/trash.svg",
        Icon::Pr => "bytes://ferro/pr.svg",
        Icon::Diff => "bytes://ferro/diff.svg",
        Icon::Stash => "bytes://ferro/stash.svg",
        Icon::Gear => "bytes://ferro/gear.svg",
        Icon::History => "bytes://ferro/history.svg",
        Icon::Undo => "bytes://ferro/undo.svg",
        Icon::Redo => "bytes://ferro/redo.svg",
        Icon::Folder => "bytes://ferro/folder.svg",
        Icon::Sync => "bytes://ferro/sync.svg",
        Icon::Tag => "bytes://ferro/tag.svg",
        Icon::Copy => "bytes://ferro/copy.svg",
        Icon::Pick => "bytes://ferro/pick.svg",
        Icon::More => "bytes://ferro/more.svg",
        Icon::Comment => "bytes://ferro/comment.svg",
        Icon::Alert => "bytes://ferro/alert.svg",
        Icon::Link => "bytes://ferro/link.svg",
        Icon::Clone => "bytes://ferro/clone.svg",
        Icon::Spark => "bytes://ferro/spark.svg",
        Icon::Eye => "bytes://ferro/eye.svg",
    };
    egui::ImageSource::Bytes { uri: uri.into(), bytes: egui::load::Bytes::Static(svg_of(i).as_bytes()) }
}

/// Paint `icon` into `rect`, tinted with `color`.
pub fn paint(ui: &Ui, rect: Rect, icon: Icon, color: Color32) {
    egui::Image::new(source(icon)).fit_to_exact_size(rect.size()).tint(color).paint_at(ui, rect);
}

/// Paint centered in `center` with edge length `size`.
pub fn paint_at(ui: &Ui, center: egui::Pos2, size: f32, icon: Icon, color: Color32) {
    paint(ui, Rect::from_center_size(center, egui::vec2(size, size)), icon, color);
}
