//! Desktop integration: persisted state, external editors/terminals, file explorer.

use serde::Serialize;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub fn data_dir() -> std::path::PathBuf {
    let base = std::env::var_os("APPDATA").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("."));
    let d = base.join("Ferro");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn load_state() -> Option<String> {
    std::fs::read_to_string(data_dir().join("state.json")).ok()
}

pub fn save_state(json: &str) -> Result<(), String> {
    let f = data_dir().join("state.json");
    let tmp = f.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &f).map_err(|e| e.to_string())
}

fn hidden(mut c: Command) -> Command {
    #[cfg(windows)]
    c.creation_flags(0x08000000);
    let _ = &mut c;
    c
}

pub fn exists_on_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else { return false };
    let exts: Vec<String> = std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
        .split(';')
        .map(|e| e.to_lowercase())
        .collect();
    for dir in std::env::split_paths(&path) {
        for e in &exts {
            if dir.join(format!("{name}{e}")).is_file() {
                return true;
            }
        }
    }
    false
}

#[derive(Serialize, Clone, Debug)]
pub struct Tool {
    pub id: String,
    pub name: String,
}

pub fn detect_editors() -> Vec<Tool> {
    let editors = [
        ("code", "Visual Studio Code"),
        ("cursor", "Cursor"),
        ("subl", "Sublime Text"),
        ("idea64", "IntelliJ IDEA"),
        ("notepad++", "Notepad++"),
        ("zed", "Zed"),
    ];
    let mut e: Vec<Tool> = editors
        .iter()
        .filter(|(id, _)| exists_on_path(id))
        .map(|(id, n)| Tool { id: id.to_string(), name: n.to_string() })
        .collect();
    e.push(Tool { id: "notepad".into(), name: "Not Defteri".into() });
    e
}

pub fn open_in_editor(path: &str, editor: &str) -> Result<(), String> {
    let mut c = hidden(Command::new("cmd"));
    c.args(["/C", editor, path]);
    c.spawn().map(|_| ()).map_err(|e| format!("Editör açılamadı: {e}"))
}

pub fn open_in_terminal(path: &str) -> Result<(), String> {
    let mut c = if exists_on_path("wt") {
        let mut c = hidden(Command::new("wt"));
        c.args(["-d", path]);
        c
    } else {
        let mut c = Command::new("cmd");
        #[cfg(windows)]
        c.raw_arg(format!("/C start \"\" /D \"{path}\" powershell.exe -NoExit"));
        c
    };
    c.spawn().map(|_| ()).map_err(|e| format!("Terminal açılamadı: {e}"))
}

pub fn reveal(path: &str) -> Result<(), String> {
    Command::new("explorer").arg(path).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// A git repository found on disk.
#[derive(Clone, Debug)]
pub struct Found {
    pub path: String,
    pub name: String,
    pub mtime: i64,
}

fn skip_dir(name: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with('.') || matches!(n.as_str(), "node_modules" | "target" | "venv" | "appdata" | "windows" | "program files" | "program files (x86)" | "$recycle.bin" | "system volume information" | "bin" | "obj" | "dist" | "build" | "packages" | "programdata" | "vendor" | "__pycache__")
}

fn walk(dir: &std::path::Path, depth: u32, out: &mut Vec<Found>, deadline: std::time::Instant) {
    if std::time::Instant::now() > deadline || out.len() >= 80 {
        return;
    }
    let git = dir.join(".git");
    if git.exists() {
        let mt = std::fs::metadata(&git).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0);
        let name = dir.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        out.push(Found { path: dir.to_string_lossy().replace('\\', "/"), name, mtime: mt });
        return;
    }
    if depth == 0 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if skip_dir(&name) {
            continue;
        }
        walk(&e.path(), depth - 1, out, deadline);
    }
}

/// Look for repositories in the usual places (Documents\GitHub, source\repos, dev folders on every drive).
/// `extra` roots are scanned a little deeper.
pub fn scan_repos(extra: &[String]) -> Vec<Found> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    let mut out: Vec<Found> = vec![];
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).map(std::path::PathBuf::from).unwrap_or_default();
    for sub in ["Documents\\GitHub", "GitHub", "source\\repos", "repos", "projects", "Projects", "dev", "code", "Documents", "Desktop"] {
        let p = home.join(sub);
        if p.is_dir() {
            walk(&p, 3, &mut out, deadline);
        }
    }
    for x in extra {
        walk(std::path::Path::new(x), 5, &mut out, deadline);
    }
    #[cfg(windows)]
    for letter in b'C'..=b'Z' {
        let root = std::path::PathBuf::from(format!("{}:\\", letter as char));
        let Ok(rd) = std::fs::read_dir(&root) else { continue };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if !ft.is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            let l = name.to_lowercase();
            if skip_dir(&name) {
                continue;
            }
            if ["git", "repo", "proj", "dev", "code", "source", "work", "src"].iter().any(|k| l.contains(k)) {
                walk(&e.path(), 3, &mut out, deadline);
            }
        }
    }
    out.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
    out.dedup_by(|a, b| a.path.eq_ignore_ascii_case(&b.path));
    out.sort_by_key(|f| std::cmp::Reverse(f.mtime));
    out
}
