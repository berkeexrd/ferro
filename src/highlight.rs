//! Tiny single-line syntax highlighter (Rust first, C-like and Python good enough). No dependencies.

use crate::theme::Palette;
use eframe::egui::Color32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tok {
    Plain,
    Kw,
    Ty,
    Fn,
    Com,
    Mac,
    Num,
    Str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust,
    CLike,
    Py,
    Json,
    Plain,
}

pub fn lang_of(path: &str) -> Lang {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => Lang::Rust,
        "c" | "h" | "cpp" | "hpp" | "cc" | "cs" | "java" | "js" | "jsx" | "ts" | "tsx" | "go" | "kt" | "swift" | "php" | "dart" | "mjs" | "scala" | "css" | "scss" => Lang::CLike,
        "py" | "rb" | "sh" | "ps1" | "toml" | "yml" | "yaml" | "ini" | "cfg" => Lang::Py,
        "json" => Lang::Json,
        _ => Lang::Plain,
    }
}

const KW: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while", "class", "namespace", "using", "public", "private", "protected", "virtual", "override", "void", "int", "long", "char", "float", "double", "bool", "new", "delete", "this", "null", "nullptr", "function", "var", "export", "import", "from", "default", "switch", "case", "try", "catch", "throw", "finally", "typeof", "def", "lambda", "pass", "raise", "with", "None", "True", "False", "and", "or", "not", "elif", "is", "template", "typename", "typedef", "inline", "extends", "implements", "interface", "package", "final", "abstract",
];
const TYPES: &[&str] = &["usize", "isize", "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128", "f32", "f64", "str", "bool", "char", "String", "Vec", "Option", "Result", "Box", "Self"];

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}
fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn tokenize(line: &str, lang: Lang) -> Vec<(String, Tok)> {
    let cs: Vec<char> = line.chars().collect();
    if lang == Lang::Plain || cs.len() > 600 {
        return vec![(line.to_string(), Tok::Plain)];
    }
    let mut out: Vec<(String, Tok)> = vec![];
    let mut i = 0;
    let mut plain = String::new();
    macro_rules! flush {
        () => {
            if !plain.is_empty() {
                out.push((std::mem::take(&mut plain), Tok::Plain));
            }
        };
    }
    while i < cs.len() {
        let c = cs[i];
        let line_comment = match lang {
            Lang::Py => c == '#',
            Lang::Json => false,
            _ => c == '/' && cs.get(i + 1) == Some(&'/'),
        };
        if line_comment {
            flush!();
            out.push((cs[i..].iter().collect(), Tok::Com));
            return out;
        }
        if c == '/' && cs.get(i + 1) == Some(&'*') {
            flush!();
            let mut j = i + 2;
            while j + 1 < cs.len() && !(cs[j] == '*' && cs[j + 1] == '/') {
                j += 1;
            }
            let end = (j + 2).min(cs.len());
            out.push((cs[i..end].iter().collect(), Tok::Com));
            i = end;
            continue;
        }
        if c == '"' || (c == '\'' && (lang != Lang::Rust || is_rust_char(&cs, i))) || (c == '`' && lang == Lang::CLike) {
            flush!();
            let q = c;
            let mut j = i + 1;
            while j < cs.len() {
                if cs[j] == '\\' {
                    j += 2;
                    continue;
                }
                if cs[j] == q {
                    j += 1;
                    break;
                }
                j += 1;
            }
            let end = j.min(cs.len());
            out.push((cs[i..end].iter().collect(), Tok::Str));
            i = end;
            continue;
        }
        if c.is_ascii_digit() {
            flush!();
            let mut j = i;
            while j < cs.len() && (cs[j].is_ascii_alphanumeric() || cs[j] == '.' || cs[j] == '_') {
                j += 1;
            }
            out.push((cs[i..j].iter().collect(), Tok::Num));
            i = j;
            continue;
        }
        if is_ident_start(c) {
            let mut j = i;
            while j < cs.len() && is_ident(cs[j]) {
                j += 1;
            }
            let w: String = cs[i..j].iter().collect();
            let next = cs.get(j).copied();
            let kind = if lang == Lang::Json {
                if w == "true" || w == "false" || w == "null" { Tok::Kw } else { Tok::Plain }
            } else if next == Some('!') && cs.get(j + 1) != Some(&'=') && lang == Lang::Rust {
                Tok::Mac
            } else if KW.contains(&w.as_str()) {
                Tok::Kw
            } else if TYPES.contains(&w.as_str()) || (w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && w.len() > 1 && !w.chars().all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())) {
                Tok::Ty
            } else if next == Some('(') {
                Tok::Fn
            } else {
                Tok::Plain
            };
            if kind == Tok::Plain {
                plain.push_str(&w);
            } else {
                flush!();
                out.push((w, kind));
            }
            i = j;
            continue;
        }
        plain.push(c);
        i += 1;
    }
    flush!();
    out
}

fn is_rust_char(cs: &[char], i: usize) -> bool {
    // 'a' or '\n' style char literal; otherwise a lifetime
    if cs.get(i + 1) == Some(&'\\') {
        return true;
    }
    cs.get(i + 2) == Some(&'\'')
}

pub fn color(p: &Palette, t: Tok) -> Color32 {
    match t {
        Tok::Plain => p.text1,
        Tok::Kw => p.sx_kw,
        Tok::Ty => p.sx_ty,
        Tok::Fn => p.sx_fn,
        Tok::Com => p.sx_com,
        Tok::Mac => p.sx_mac,
        Tok::Num => p.sx_num,
        Tok::Str => p.sx_str,
    }
}
