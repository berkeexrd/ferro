//! Application state, background jobs, commands, undo log and the window chrome.

use crate::backend::{git, github, remote, repo, stage, sys};
use crate::diff::{self, FileDiff};
use crate::graph;
use crate::theme::{self, Mode, Theme};
use crate::util;
use eframe::egui::{self, Pos2};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

pub type Job = Box<dyn FnOnce(&mut App) + Send>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Changes,
    History,
    Branches,
    Pr,
    Stash,
    Log,
    Conflict,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Acc {
    pub key: String,
    pub host: String,
    pub login: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepoRef {
    pub path: String,
    pub name: String,
    pub opened: i64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub mode: Mode,
    pub scale: f32,
    pub compact: bool,
    pub editor: String,
    pub split_diff: bool,
    pub context: u32,
    pub confirm_force: bool,
    pub recents: Vec<RepoRef>,
    pub last_repo: Option<String>,
    pub account: Option<Acc>,
    pub accounts: Vec<Acc>,
    pub signed_out: Vec<String>,
    #[serde(skip)]
    pub client_id: String,
    pub lang: crate::i18n::Lang,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::Dark,
            scale: 1.0,
            compact: false,
            editor: String::new(),
            split_diff: false,
            context: 3,
            confirm_force: true,
            recents: vec![],
            last_repo: None,
            account: None,
            accounts: vec![],
            signed_out: vec![],
            client_id: std::env::var("FERRO_GITHUB_CLIENT_ID").unwrap_or_default(),
            lang: crate::i18n::Lang::Auto,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Ok,
    Err,
}

pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub at: Instant,
    pub action: Option<(String, Cmd)>,
}

/// Every user-visible action. Menus, the command palette and shortcuts all dispatch these.
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    Commit,
    CommitPush,
    Push,
    ForcePush,
    Pull,
    Fetch,
    Undo,
    Refresh,
    Goto(Screen),
    Palette,
    Switcher,
    NewBranch,
    SwitchTo(String),
    OpenFolder,
    CloneRepo,
    NewRepo,
    Settings,
    StageAll,
    UnstageAll,
    ToggleStageSelected,
    NextFile,
    PrevFile,
    NextHunk,
    PrevHunk,
    ToggleAmend,
    StashAll,
    StashPop(String),
    OpenEditor,
    OpenTerminal,
    Reveal,
    CloseRepo,
    ToggleTheme,
    DiscardFile(String),
    CopyCommand,
    OpenRecent(String),
    StageFile(String),
    CopyPath(String),
    CommitOp(String, String),
    SelectFile(String),
    MergeInto(String),
    DeleteBranch(String),
    Redo,
    UndoTo(usize),
    PublishRepo,
    CopyText(String),
}

impl Cmd {
    pub fn title(&self) -> String {
        match self {
            Cmd::Commit => "Commit".into(),
            Cmd::CommitPush => "Commit ve push".into(),
            Cmd::Push => "Push".into(),
            Cmd::ForcePush => "Force push (güvenli)".into(),
            Cmd::Pull => "Pull".into(),
            Cmd::Fetch => "Fetch".into(),
            Cmd::Undo => "Son işlemi geri al".into(),
            Cmd::Refresh => "Yenile".into(),
            Cmd::Goto(Screen::Changes) => "Değişiklikler sekmesine git".into(),
            Cmd::Goto(Screen::History) => "Geçmiş sekmesine git".into(),
            Cmd::Goto(Screen::Branches) => "Dallar sekmesine git".into(),
            Cmd::Goto(Screen::Pr) => "Pull request'lere git".into(),
            Cmd::Goto(Screen::Stash) => "Stash'lere git".into(),
            Cmd::Goto(Screen::Log) => "İşlem günlüğünü aç".into(),
            Cmd::Goto(Screen::Conflict) => "Çatışma çözümüne git".into(),
            Cmd::Palette => "Komut paleti".into(),
            Cmd::Switcher => "Dal değiştir…".into(),
            Cmd::NewBranch => "Yeni dal oluştur…".into(),
            Cmd::SwitchTo(b) => format!("Dala geç: {b}"),
            Cmd::OpenFolder => "Yerel depo aç…".into(),
            Cmd::CloneRepo => "Depo klonla…".into(),
            Cmd::NewRepo => "Yeni depo oluştur…".into(),
            Cmd::Settings => "Ayarlar".into(),
            Cmd::StageAll => "Tümünü hazırla".into(),
            Cmd::UnstageAll => "Hazırlananları geri çek".into(),
            Cmd::ToggleStageSelected => "Seçili dosyayı hazırla / çıkar".into(),
            Cmd::NextFile => "Sonraki dosya".into(),
            Cmd::PrevFile => "Önceki dosya".into(),
            Cmd::NextHunk => "Sonraki hunk".into(),
            Cmd::PrevHunk => "Önceki hunk".into(),
            Cmd::ToggleAmend => "Son commit'i düzelt (amend)".into(),
            Cmd::StashAll => "Değişiklikleri stash'le".into(),
            Cmd::StashPop(n) => format!("Stash'i geri yükle: {n}"),
            Cmd::OpenEditor => "Editörde aç".into(),
            Cmd::OpenTerminal => "Terminalde aç".into(),
            Cmd::Reveal => "Dosya Gezgini'nde göster".into(),
            Cmd::CloseRepo => "Depoyu kapat".into(),
            Cmd::ToggleTheme => "Açık / koyu temayı değiştir".into(),
            Cmd::DiscardFile(f) => format!("Değişiklikleri at: {f}"),
            Cmd::CopyCommand => "Son git komutunu kopyala".into(),
            Cmd::OpenRecent(p) => format!("Depoyu aç: {p}"),
            Cmd::StageFile(f) => format!("Hazırla: {f}"),
            Cmd::CommitOp(k, _) => format!("Commit işlemi: {k}"),
            Cmd::SelectFile(f) => format!("Dosyayı göster: {f}"),
            Cmd::MergeInto(b) => format!("Birleştir: {b}"),
            Cmd::DeleteBranch(b) => format!("Dalı sil: {b}"),
            Cmd::Redo => "Yinele".into(),
            Cmd::PublishRepo => "Depoyu GitHub'da yayınla".into(),
            Cmd::CopyText(_) => "Kopyala".into(),
            Cmd::UndoTo(_) => "Bu noktaya dön".into(),
            Cmd::CopyPath(f) => format!("Yolu kopyala: {f}"),
        }
    }

    pub fn shortcut(&self) -> Option<&'static str> {
        Some(match self {
            Cmd::Commit => "Ctrl ↵",
            Cmd::CommitPush => "Ctrl Shift ↵",
            Cmd::Push => "Ctrl Shift P",
            Cmd::Undo => "Ctrl Z",
            Cmd::Redo => "Ctrl Shift Z",
            Cmd::Palette => "Ctrl K",
            Cmd::Switcher => "Ctrl B",
            Cmd::NewBranch => "Ctrl Shift N",
            Cmd::OpenFolder => "Ctrl O",
            Cmd::NewRepo => "Ctrl N",
            Cmd::CloneRepo => "Ctrl Shift O",
            Cmd::Settings => "Ctrl ,",
            Cmd::Goto(Screen::Changes) => "Ctrl 1",
            Cmd::Goto(Screen::History) => "Ctrl 2",
            Cmd::Goto(Screen::Branches) => "Ctrl 3",
            Cmd::Goto(Screen::Pr) => "Ctrl 4",
            Cmd::ToggleStageSelected => "Space",
            Cmd::NextFile => "J",
            Cmd::PrevFile => "K",
            Cmd::NextHunk => "]",
            Cmd::PrevHunk => "[",
            Cmd::Refresh => "F5",
            _ => return None,
        })
    }

    pub fn git_preview(&self) -> Option<String> {
        Some(match self {
            Cmd::Commit => "git commit -F -".into(),
            Cmd::CommitPush => "git commit -F - && git push".into(),
            Cmd::Push => "git push --progress".into(),
            Cmd::ForcePush => "git push --force-with-lease".into(),
            Cmd::Pull => "git pull --progress".into(),
            Cmd::Fetch => "git fetch --prune --progress".into(),
            Cmd::NewBranch => "git switch -c ‹ad› main".into(),
            Cmd::SwitchTo(b) => format!("git switch {b}"),
            Cmd::StageAll => "git add -A".into(),
            Cmd::UnstageAll => "git restore --staged .".into(),
            Cmd::StashAll => "git stash push -u".into(),
            Cmd::StashPop(n) => format!("git stash pop {n}"),
            Cmd::DiscardFile(f) => format!("git checkout HEAD -- {f}"),
            Cmd::Undo => "git reset --soft ‹önceki›".into(),
            Cmd::PublishRepo => "POST /user/repos · git push -u origin".into(),
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CommitRow {
    pub c: repo::Commit,
    pub ts: i64,
    pub g: graph::GraphRow,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Block {
    pub staged: bool,
    pub hunk: usize,
}

pub struct FileView {
    pub path: String,
    pub entry: repo::Entry,
    pub unstaged: FileDiff,
    pub staged: FileDiff,
    pub blocks: Vec<Block>,
    pub error: Option<String>,
    pub intra: std::collections::HashMap<(usize, usize), (usize, usize)>,
}

impl FileView {
    pub fn hunk(&self, b: Block) -> &diff::Hunk {
        if b.staged { &self.staged.hunks[b.hunk] } else { &self.unstaged.hunks[b.hunk] }
    }
    pub fn fdiff(&self, b: Block) -> &FileDiff {
        if b.staged { &self.staged } else { &self.unstaged }
    }
}

pub struct Repo {
    pub path: String,
    pub name: String,
    pub status: Option<repo::Status>,
    pub status_sig: String,
    pub staged_counts: (usize, usize),
    pub branches: Vec<repo::Branch>,
    pub recent_branches: Vec<String>,
    pub default_branch: String,
    pub remotes: Vec<remote::Remote>,
    pub stashes: Vec<repo::StashEntry>,
    pub tags: Vec<repo::Tag>,
    pub meta: Option<repo::RepoMeta>,
    pub remotes_loaded: bool,
    pub unpushed: HashSet<String>,
    pub gh: Option<(String, String)>,
    pub sel_file: Option<String>,
    pub file_view: Option<FileView>,
    pub file_seq: u64,
    pub filter: usize, // 0 all, 1 staged, 2 untracked
    pub summary: String,
    pub body: String,
    pub amend: bool,
    pub coauthor: String,
    pub focus_hunk: Option<usize>,
    pub line_anchor: Option<(Block, usize)>,
    pub line_sel: HashSet<(bool, usize, usize)>,
    pub numstat: std::collections::HashMap<String, (i64, i64)>,
    pub coauthor_open: bool,
    pub scroll_req: bool,
    pub full_ctx: bool,
    pub commits: Vec<CommitRow>,
    pub commits_done: bool,
    pub commits_loading: bool,
    pub lanes: usize,
    pub sel_commit: Option<usize>,
    pub commit_files: Vec<repo::FileChange>,
    pub sel_commit_file: Option<String>,
    pub commit_diff: Option<FileDiff>,
    pub hist_query: String,
    pub hist_seq: u64,
    pub sel_stash: Option<usize>,
    pub stash_files: Vec<repo::FileChange>,
    pub stash_diff: Option<(String, FileDiff)>,
    pub conflict_file: Option<String>,
    pub conflict_segs: Vec<diff::Seg>,
    pub conflict_idx: usize,
    pub conflict_edit: bool,
    pub pr: crate::screens::pr::PrState,
}

impl Repo {
    pub fn new(path: String, name: String) -> Self {
        Self {
            path,
            name,
            status: None,
            status_sig: String::new(),
            staged_counts: (0, 0),
            branches: vec![],
            recent_branches: vec![],
            default_branch: String::new(),
            remotes: vec![],
            stashes: vec![],
            tags: vec![],
            meta: None,
            remotes_loaded: false,
            unpushed: HashSet::new(),
            gh: None,
            sel_file: None,
            file_view: None,
            file_seq: 0,
            filter: 0,
            summary: String::new(),
            body: String::new(),
            amend: false,
            coauthor: String::new(),
            focus_hunk: None,
            line_anchor: None,
            line_sel: HashSet::new(),
            numstat: Default::default(),
            coauthor_open: false,
            scroll_req: false,
            full_ctx: false,
            commits: vec![],
            commits_done: false,
            commits_loading: false,
            lanes: 1,
            sel_commit: None,
            commit_files: vec![],
            sel_commit_file: None,
            commit_diff: None,
            hist_query: String::new(),
            hist_seq: 0,
            sel_stash: None,
            stash_files: vec![],
            stash_diff: None,
            conflict_file: None,
            conflict_segs: vec![],
            conflict_idx: 0,
            conflict_edit: false,
            pr: Default::default(),
        }
    }

    pub fn entries(&self) -> &[repo::Entry] {
        self.status.as_ref().map(|s| s.entries.as_slice()).unwrap_or(&[])
    }

    pub fn branch(&self) -> String {
        match &self.status {
            Some(s) if s.detached => format!("HEAD · {}", s.oid.as_deref().map(util::short).unwrap_or("")),
            Some(s) => s.branch.clone().unwrap_or_else(|| "main".into()),
            None => "…".into(),
        }
    }

    pub fn dirty(&self) -> bool {
        self.entries().iter().any(|e| !e.conflicted)
    }
}

pub fn staged_flags(e: &repo::Entry) -> (bool, bool) {
    let mut ch = e.xy.chars();
    let x = ch.next().unwrap_or('.');
    let y = ch.next().unwrap_or('.');
    if e.kind == "untracked" {
        return (false, true);
    }
    (x != '.' && x != '?', y != '.' && y != '?')
}

pub fn letter_of(e: &repo::Entry) -> char {
    match e.kind.as_str() {
        "added" => 'A',
        "deleted" => 'D',
        "renamed" | "copied" => 'R',
        "untracked" => '?',
        "conflicted" => '!',
        _ => 'M',
    }
}

#[derive(Clone, Debug)]
pub enum Undo {
    None(&'static str),
    IndexTree(String),
    ResetSoft(Option<String>),
    ResetHard(String),
    Switch(String),
    Discard { backup: Option<String>, files: Vec<String> },
}

#[derive(Clone, Debug)]
pub struct OpEntry {
    pub at: i64,
    pub title: String,
    pub cmd: String,
    pub undo: Undo,
}

/// State captured right before an undo so that the undo itself can be redone.
#[derive(Clone, Debug)]
pub struct RedoSnap {
    pub head: Option<String>,
    pub tree: Option<String>,
    pub branch: String,
}

impl RedoSnap {
    pub fn capture(path: &str) -> Self {
        Self {
            head: stage::head_sha(path),
            tree: stage::index_tree(path).ok(),
            branch: git::ok(path, &["symbolic-ref", "--short", "-q", "HEAD"]).unwrap_or_default().trim().to_string(),
        }
    }

    pub fn apply(&self, path: &str) -> Result<(), String> {
        let cur = git::ok(path, &["symbolic-ref", "--short", "-q", "HEAD"]).unwrap_or_default().trim().to_string();
        if !self.branch.is_empty() && cur != self.branch {
            git::ok(path, &["switch", &self.branch])?;
        }
        if let Some(h) = &self.head {
            git::ok(path, &["reset", "--soft", h])?;
        }
        if let Some(t) = &self.tree {
            stage::restore_index(path, t)?;
        }
        Ok(())
    }
}

pub fn run_undo(path: &str, u: Undo) -> Result<(), String> {
    match u {
        Undo::IndexTree(t) => stage::restore_index(path, &t),
        Undo::ResetSoft(Some(h)) => git::ok(path, &["reset", "--soft", &h]).map(|_| ()),
        Undo::ResetSoft(None) => git::ok(path, &["update-ref", "-d", "HEAD"]).map(|_| ()),
        Undo::ResetHard(h) => git::ok(path, &["reset", "--hard", &h]).map(|_| ()),
        Undo::Switch(b) => git::ok(path, &["switch", &b]).map(|_| ()),
        Undo::Discard { backup, files } => match backup {
            Some(sha) => repo::restore_discarded(path.to_string(), sha, files),
            None => Err("Yedek bulunamadı".into()),
        },
        Undo::None(_) => Ok(()),
    }
}

pub enum Modal {
    Settings(usize),
    Confirm { title: String, body: String, ok: String, danger: bool, action: Option<Box<dyn FnOnce(&mut App) + Send>> },
    NewBranch(String),
    Login,
    FromCommit { sha: String, name: String, tag: bool },
    Publish { name: String, desc: String, private: bool, acc: String },
}

pub struct MenuItem {
    pub label: String,
    pub icon: Option<crate::icons::Icon>,
    pub cmd: Cmd,
    pub kbd: Option<&'static str>,
    pub danger: bool,
    pub enabled: bool,
}

pub struct Menu {
    pub pos: Pos2,
    pub items: Vec<MenuItem>,
    pub right_align: bool,
}

pub struct App {
    pub ctx: egui::Context,
    pub tx: Sender<Job>,
    pub rx: Receiver<Job>,
    pub th: Theme,
    pub s: Settings,
    pub repo: Option<Repo>,
    pub screen: Screen,
    pub modal: Option<Modal>,
    pub menu: Option<Menu>,
    pub toasts: Vec<Toast>,
    pub oplog: Vec<OpEntry>,
    pub redo: Vec<(String, RedoSnap)>,
    pub sync_text: String,
    pub sync_label: Option<&'static str>,
    pub busy: u32,
    pub git_user: (String, String),
    pub git_version: String,
    pub editors: Vec<sys::Tool>,
    pub last_poll: Instant,
    pub last_meta: Instant,
    pub status_inflight: bool,
    pub palette: Option<crate::screens::palette::PaletteState>,
    pub switcher: Option<crate::screens::palette::SwitcherState>,
    pub welcome: crate::screens::welcome::WelcomeState,
    pub login: crate::screens::welcome::LoginState,
    pub save_due: Option<Instant>,
    pub cjk_loaded: bool,
    pub system_dark: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();
        egui_extras::install_image_loaders(&ctx);
        theme::install_fonts(&ctx);
        let (tx, rx) = channel();
        let mut s: Settings = sys::load_state().and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default();
        if let Some(a) = s.account.clone() {
            if !s.accounts.iter().any(|x| x.key == a.key) {
                s.accounts.push(a);
            }
        }
        crate::i18n::set(s.lang);
        let system_dark = cc.egui_ctx.input(|i| i.raw.system_theme.map(|t| t == egui::Theme::Dark).unwrap_or(true));
        let th = Theme::new(s.mode, s.scale, s.compact, system_dark);
        theme::apply_visuals(&ctx, &th);
        ctx.set_zoom_factor(s.scale);
        let mut app = Self {
            ctx,
            tx,
            rx,
            th,
            s,
            repo: None,
            screen: Screen::Changes,
            modal: None,
            menu: None,
            toasts: vec![],
            oplog: vec![],
            redo: vec![],
            sync_text: String::new(),
            sync_label: None,
            busy: 0,
            git_user: (String::new(), String::new()),
            git_version: String::new(),
            editors: sys::detect_editors(),
            last_poll: Instant::now(),
            last_meta: Instant::now(),
            status_inflight: false,
            palette: None,
            switcher: None,
            welcome: Default::default(),
            login: Default::default(),
            save_due: None,
            cjk_loaded: false,
            system_dark,
        };
        app.spawn(
            || {
                (
                    repo::git_version().unwrap_or_default(),
                    repo::config_get(None, "user.name".into(), true).unwrap_or_default(),
                    repo::config_get(None, "user.email".into(), true).unwrap_or_default(),
                )
            },
            |app, (v, n, e)| {
                app.git_version = v.replace("git version ", "git ");
                app.git_user = (n, e);
            },
        );
        app.discover_accounts();
        if app.s.recents.is_empty() {
            app.scan_repos(vec![]);
        }
        if let Some(last) = app.s.last_repo.clone() {
            if std::path::Path::new(&last).exists() {
                app.open_repo(last);
            }
        }
        app
    }

    pub fn spawn<T: Send + 'static>(&self, work: impl FnOnce() -> T + Send + 'static, done: impl FnOnce(&mut App, T) + Send + 'static) {
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let r = work();
            let _ = tx.send(Box::new(move |app: &mut App| done(app, r)));
            ctx.request_repaint();
        });
    }

    /// Like `spawn` but shows the busy indicator while it runs.
    pub fn spawn_busy<T: Send + 'static>(&mut self, work: impl FnOnce() -> T + Send + 'static, done: impl FnOnce(&mut App, T) + Send + 'static) {
        self.busy += 1;
        self.spawn(work, move |app, r| {
            app.busy = app.busy.saturating_sub(1);
            done(app, r);
        });
    }

    pub fn progress(&self) -> git::Progress {
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::sync::Arc::new(move |line: String| {
            let _ = tx.send(Box::new(move |app: &mut App| app.sync_text = line));
            ctx.request_repaint();
        })
    }

    pub fn toast(&mut self, kind: ToastKind, text: impl Into<String>) {
        self.toasts.push(Toast { text: text.into(), kind, at: Instant::now(), action: None });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    pub fn toast_undo(&mut self, text: impl Into<String>) {
        self.toasts.push(Toast { text: text.into(), kind: ToastKind::Ok, at: Instant::now(), action: Some(("Geri al".into(), Cmd::Undo)) });
    }

    /// CJK fallback fonts are ~40 MB of RAM, so they are only loaded once such text actually shows up.
    pub fn ensure_cjk(&mut self, sample: &str) {
        if self.cjk_loaded || !sample.chars().any(|c| (c as u32) >= 0x2E80) {
            return;
        }
        self.cjk_loaded = true;
        self.spawn(
            || {
                let mut v: Vec<(String, Vec<u8>)> = vec![];
                for (n, f) in [("cjk-sc", "msyh.ttc"), ("cjk-kr", "malgun.ttf"), ("cjk-jp", "YuGothR.ttc")] {
                    if let Ok(b) = std::fs::read(std::path::Path::new(r"C:\Windows\Fonts").join(f)) {
                        v.push((n.to_string(), b));
                    }
                }
                v
            },
            |app, v| {
                if !v.is_empty() {
                    theme::install_fonts_ex(&app.ctx, v);
                }
            },
        );
    }

    /// Picks up GitHub accounts that Git Credential Manager already knows, so signing in to `git`
    /// once is enough.
    pub fn discover_accounts(&mut self) {
        let have: Vec<String> = self.s.accounts.iter().map(|a| a.key.clone()).collect();
        let skip = self.s.signed_out.clone();
        self.spawn(
            move || {
                let mut found = vec![];
                for login in git::gcm_accounts("github.com") {
                    let key = format!("github.com|{login}");
                    if have.contains(&key) || skip.contains(&key) {
                        continue;
                    }
                    let name = git::token_for(&key)
                        .and_then(|t| github::gh_whoami("github.com".into(), t).ok())
                        .and_then(|v| v["name"].as_str().map(String::from))
                        .unwrap_or_default();
                    found.push(Acc { key, host: "github.com".into(), login, name });
                }
                found
            },
            |app, list| {
                if list.is_empty() {
                    return;
                }
                for a in list {
                    app.s.accounts.push(a.clone());
                    if app.s.account.is_none() {
                        app.s.account = Some(a);
                    }
                }
                app.save_settings();
                app.welcome.gh_loaded = false;
            },
        );
    }

    pub fn scan_repos(&mut self, extra: Vec<String>) {
        if self.welcome.scanning {
            return;
        }
        self.welcome.scanning = true;
        self.spawn(
            move || sys::scan_repos(&extra),
            |app, found| {
                app.welcome.scanning = false;
                app.welcome.scanned = true;
                app.welcome.found = found;
            },
        );
    }

    pub fn path(&self) -> Option<String> {
        self.repo.as_ref().map(|r| r.path.clone())
    }

    pub fn account_key(&self) -> Option<String> {
        self.s.account.as_ref().map(|a| a.key.clone())
    }

    pub fn add_account(&mut self, acc: Acc) {
        self.s.accounts.retain(|a| a.key != acc.key);
        self.s.signed_out.retain(|k| *k != acc.key);
        self.s.accounts.push(acc.clone());
        self.s.account = Some(acc.clone());
        self.save_settings();
        self.login.phase = 0;
        if matches!(self.modal, Some(Modal::Login)) {
            self.modal = None;
        }
        self.toast(ToastKind::Ok, format!("{} hesabı bağlandı", acc.login));
        self.welcome.gh_loaded = false;
    }

    pub fn remove_account(&mut self, key: &str) {
        let _ = github::gh_logout(key.to_string());
        if !self.s.signed_out.iter().any(|k| k == key) {
            self.s.signed_out.push(key.to_string());
        }
        self.s.accounts.retain(|a| a.key != key);
        if self.s.account.as_ref().map(|a| a.key == key).unwrap_or(false) {
            self.s.account = self.s.accounts.first().cloned();
        }
        self.save_settings();
    }

    pub fn save_settings(&mut self) {
        self.save_due = Some(Instant::now() + Duration::from_millis(400));
    }

    pub fn apply_theme(&mut self) {
        self.th = Theme::new(self.s.mode, self.s.scale, self.s.compact, self.system_dark);
        theme::apply_visuals(&self.ctx, &self.th);
        self.ctx.set_zoom_factor(self.s.scale);
        self.save_settings();
    }

    pub fn log_op(&mut self, title: impl Into<String>, cmd: impl Into<String>, undo: Undo) {
        self.redo.clear();
        self.oplog.insert(0, OpEntry { at: util::now(), title: title.into(), cmd: cmd.into(), undo });
        if self.oplog.len() > 200 {
            self.oplog.pop();
        }
    }

    pub fn can_undo(&self) -> bool {
        self.oplog.first().map(|o| !matches!(o.undo, Undo::None(_))).unwrap_or(false)
    }

    pub fn undo_last(&mut self) {
        if !self.can_undo() {
            let why = match self.oplog.first().map(|o| &o.undo) {
                Some(Undo::None(w)) if !w.is_empty() => format!("Son işlem geri alınamaz: {w}"),
                Some(_) => "Son işlem geri alınamaz".to_string(),
                None => "Geri alınacak işlem yok".to_string(),
            };
            self.toast(ToastKind::Info, why);
            return;
        }
        self.undo_n(0);
    }

    /// Undo the top `n + 1` log entries in order (n = 0 is the plain "Geri al").
    pub fn undo_n(&mut self, n: usize) {
        let Some(path) = self.path() else { return };
        let k = n + 1;
        if self.oplog.len() < k || self.oplog[..k].iter().any(|o| matches!(o.undo, Undo::None(_))) {
            self.toast(ToastKind::Info, "Bu noktaya dönülemez: araya geri alınamayan bir işlem girmiş");
            return;
        }
        let entries: Vec<OpEntry> = self.oplog.drain(..k).collect();
        let title = if k == 1 { entries[0].title.clone() } else { format!("{k} işlem") };
        let back = entries.clone();
        self.spawn_busy(
            move || -> Result<RedoSnap, String> {
                let snap = RedoSnap::capture(&path);
                for e in &entries {
                    run_undo(&path, e.undo.clone())?;
                }
                Ok(snap)
            },
            move |app, r| match r {
                Ok(snap) => {
                    app.redo.push((title.clone(), snap));
                    app.toast(ToastKind::Ok, format!("Geri alındı: {title}"));
                    app.refresh_all();
                }
                Err(e) => {
                    for o in back.into_iter().rev() {
                        app.oplog.insert(0, o);
                    }
                    app.toast(ToastKind::Err, format!("Geri alınamadı: {e}"));
                }
            },
        );
    }

    pub fn redo(&mut self) {
        let Some(path) = self.path() else { return };
        let Some((title, snap)) = self.redo.pop() else {
            self.toast(ToastKind::Info, "Yinelenecek işlem yok");
            return;
        };
        let t2 = title.clone();
        self.spawn_busy(
            move || snap.apply(&path),
            move |app, r| match r {
                Ok(()) => {
                    app.toast(ToastKind::Ok, format!("Yinelendi: {t2}"));
                    app.refresh_all();
                }
                Err(e) => app.toast(ToastKind::Err, format!("Yinelenemedi: {e}")),
            },
        );
    }

    pub fn open_repo(&mut self, path: String) {
        let p = path.clone();
        self.spawn_busy(
            move || repo::repo_info(p),
            move |app, r| match r {
                Ok(info) => {
                    let name = info.name.clone();
                    app.s.recents.retain(|x| !x.path.eq_ignore_ascii_case(&info.root));
                    app.s.recents.insert(0, RepoRef { path: info.root.clone(), name: name.clone(), opened: util::now() });
                    app.s.recents.truncate(12);
                    app.s.last_repo = Some(info.root.clone());
                    app.repo = Some(Repo::new(info.root, name));
                    app.screen = Screen::Changes;
                    app.oplog.clear();
                    app.save_settings();
                    app.refresh_all();
                }
                Err(e) => {
                    if e.contains("Git deposu değil") {
                        let dir = path.clone();
                        app.confirm("Bu klasör bir Git deposu değil", "Burada yeni bir depo oluşturulsun mu? (git init)", "Depo oluştur", false, move |app| {
                            let d = dir.clone();
                            app.spawn_busy(
                                move || repo::init_repo(d, "main".into()),
                                move |app, r| match r {
                                    Ok(()) => app.open_repo(dir),
                                    Err(e) => app.toast(ToastKind::Err, e),
                                },
                            );
                        });
                    } else {
                        app.toast(ToastKind::Err, e);
                    }
                }
            },
        );
    }

    pub fn close_repo(&mut self) {
        self.repo = None;
        self.s.last_repo = None;
        self.save_settings();
    }

    pub fn repo_is(&self, path: &str) -> bool {
        self.repo.as_ref().map(|r| r.path == path).unwrap_or(false)
    }

    pub fn refresh_all(&mut self) {
        self.refresh_status(true);
        self.refresh_meta();
        self.load_commits(true);
    }

    /// Branches, remotes, stashes, tags, unpushed set. Cheap; also run on a timer and before pickers open.
    pub fn refresh_meta(&mut self) {
        let Some(path) = self.path() else { return };
        self.last_meta = Instant::now();
        let p = path.clone();
        self.spawn(
            move || (repo::branches(p.clone()), remote::remotes(p.clone()), repo::stash_list(p.clone()), repo::tags(p.clone()), repo::unpushed(p.clone()), repo::repo_meta(p)),
            move |app, (b, rem, st, tags, unp, meta)| {
                if !app.repo_is(&path) {
                    return;
                }
                let acc = app.s.account.clone();
                let r = app.repo.as_mut().unwrap();
                if let Ok(b) = b {
                    r.branches = b.list;
                    r.recent_branches = b.recent;
                    r.default_branch = b.default_branch;
                }
                if let Ok(rem) = rem {
                    r.gh = rem
                        .iter()
                        .find(|x| x.name == "origin")
                        .or(rem.first())
                        .and_then(|x| parse_github(&x.url, acc.as_ref().map(|a| a.host.as_str()).unwrap_or("github.com")));
                    r.remotes = rem;
                    r.remotes_loaded = true;
                }
                if let Ok(st) = st {
                    r.stashes = st;
                }
                if let Ok(t) = tags {
                    r.tags = t;
                }
                if let Ok(u) = unp {
                    r.unpushed = u.into_iter().collect();
                }
                if let Ok(m) = meta {
                    r.meta = Some(m);
                }
            },
        );
    }

    pub fn refresh_status(&mut self, force: bool) {
        let Some(path) = self.path() else { return };
        if self.status_inflight && !force {
            return;
        }
        self.status_inflight = true;
        self.last_poll = Instant::now();
        let p = path.clone();
        self.spawn(
            move || {
                let st = repo::status(p.clone());
                let counts = if st.as_ref().map(|s| s.entries.iter().any(|e| staged_flags(e).0)).unwrap_or(false) { stage::staged_counts(&p) } else { (0, 0) };
                (st, counts)
            },
            move |app, (st, counts)| {
                app.status_inflight = false;
                if !app.repo_is(&path) {
                    return;
                }
                match st {
                    Ok(st) => app.apply_status(st, counts),
                    Err(e) => {
                        if !e.is_empty() {
                            app.toast(ToastKind::Err, e);
                        }
                    }
                }
            },
        );
    }

    fn apply_status(&mut self, st: repo::Status, counts: (usize, usize)) {
        let sig = format!(
            "{}|{:?}|{:?}|{}|{}|{}|{}|{:?}",
            st.branch.clone().unwrap_or_default(),
            st.oid,
            st.upstream,
            st.ahead,
            st.behind,
            st.state,
            st.entries.iter().map(|e| format!("{}{}", e.xy, e.path)).collect::<Vec<_>>().join(";"),
            counts
        );
        let mut reload_file = false;
        let head_moved;
        let goto_conflict;
        {
            let r = self.repo.as_mut().unwrap();
            if r.status_sig == sig {
                return;
            }
            head_moved = r.status.as_ref().map(|o| o.oid != st.oid || o.branch != st.branch).unwrap_or(false);
            goto_conflict = st.entries.iter().any(|e| e.conflicted) && r.status.as_ref().map(|o| !o.entries.iter().any(|e| e.conflicted)).unwrap_or(true);
            let exists = r.sel_file.as_ref().map(|f| st.entries.iter().any(|e| &e.path == f)).unwrap_or(false);
            if !exists {
                r.sel_file = st.entries.iter().find(|e| !e.conflicted).map(|e| e.path.clone());
                r.file_view = None;
                reload_file = true;
            } else if let Some(sel) = &r.sel_file {
                let old = r.status.as_ref().and_then(|o| o.entries.iter().find(|e| &e.path == sel).map(|e| e.xy.clone()));
                let new = st.entries.iter().find(|e| &e.path == sel).map(|e| e.xy.clone());
                reload_file = old != new || r.file_view.is_none();
            }
            r.status_sig = sig;
            r.status = Some(st);
            r.staged_counts = counts;
        }
        if let Some(path) = self.path() {
            let p2 = path.clone();
            self.spawn(
                move || crate::backend::stage::numstat(&p2),
                move |app, m| {
                    if app.repo_is(&path) {
                        if let Some(r) = app.repo.as_mut() {
                            r.numstat = m;
                        }
                    }
                },
            );
        }
        if reload_file {
            self.load_file_view();
        }
        if head_moved {
            self.load_commits(true);
            let path = self.path().unwrap();
            let p2 = path.clone();
            self.spawn(
                move || (repo::branches(path.clone()), repo::unpushed(path)),
                move |app, (b, u)| {
                    if !app.repo_is(&p2) {
                        return;
                    }
                    let r = app.repo.as_mut().unwrap();
                    if let Ok(b) = b {
                        r.branches = b.list;
                        r.recent_branches = b.recent;
                        r.default_branch = b.default_branch;
                    }
                    if let Ok(u) = u {
                        r.unpushed = u.into_iter().collect();
                    }
                },
            );
        }
        if goto_conflict {
            self.screen = Screen::Conflict;
            self.load_conflicts();
        }
    }

    pub fn select_file(&mut self, path: String) {
        if let Some(r) = self.repo.as_mut() {
            if r.sel_file.as_deref() == Some(path.as_str()) {
                return;
            }
            r.sel_file = Some(path);
            r.file_view = None;
            r.focus_hunk = Some(0);
        }
        self.load_file_view();
    }

    pub fn load_file_view(&mut self) {
        let Some(r) = self.repo.as_mut() else { return };
        let Some(file) = r.sel_file.clone() else { return };
        let Some(entry) = r.entries().iter().find(|e| e.path == file).cloned() else { return };
        r.file_seq += 1;
        let seq = r.file_seq;
        let path = r.path.clone();
        let ctx_lines = if r.full_ctx { 100000 } else { self.s.context };
        self.spawn(
            move || {
                let untracked = entry.kind == "untracked";
                let (st, un) = staged_flags(&entry);
                let mut err = None;
                let unstaged = if un || untracked {
                    match crate::backend::stage::diff_worktree(&path, &entry.path, untracked, ctx_lines) {
                        Ok(t) => diff::parse_diff(&t),
                        Err(e) => {
                            err = Some(e);
                            FileDiff::default()
                        }
                    }
                } else {
                    FileDiff::default()
                };
                let staged = if st {
                    match crate::backend::stage::diff_index(&path, &entry.path, entry.orig_path.as_deref(), ctx_lines) {
                        Ok(t) => diff::parse_diff(&t),
                        Err(e) => {
                            err = Some(e);
                            FileDiff::default()
                        }
                    }
                } else {
                    FileDiff::default()
                };
                (entry, unstaged, staged, err)
            },
            move |app, (entry, unstaged, staged, error)| {
                let Some(r) = app.repo.as_mut() else { return };
                if r.file_seq != seq {
                    return;
                }
                let mut blocks: Vec<(u32, Block)> = vec![];
                for (i, h) in unstaged.hunks.iter().enumerate() {
                    blocks.push((h.old_start, Block { staged: false, hunk: i }));
                }
                for (i, h) in staged.hunks.iter().enumerate() {
                    blocks.push((h.new_start, Block { staged: true, hunk: i }));
                }
                blocks.sort_by_key(|(k, b)| (*k, b.staged));
                let blocks: Vec<Block> = blocks.into_iter().map(|x| x.1).collect();
                let mut intra = std::collections::HashMap::new();
                for (bi, b) in blocks.iter().enumerate() {
                    let h = if b.staged { &staged.hunks[b.hunk] } else { &unstaged.hunks[b.hunk] };
                    let mut i = 0;
                    while i < h.lines.len() {
                        if h.lines[i].kind == diff::LineKind::Del {
                            let ds = i;
                            while i < h.lines.len() && h.lines[i].kind == diff::LineKind::Del {
                                i += 1;
                            }
                            let as_ = i;
                            while i < h.lines.len() && h.lines[i].kind == diff::LineKind::Add {
                                i += 1;
                            }
                            let (nd, na) = (as_ - ds, i - as_);
                            if nd == na {
                                for k in 0..nd {
                                    let a = h.lines[ds + k].text.replace('\t', "    ");
                                    let bb = h.lines[as_ + k].text.replace('\t', "    ");
                                    let (ra, rb) = diff::intra_range(&a, &bb);
                                    if let Some(x) = ra {
                                        intra.insert((bi, ds + k), x);
                                    }
                                    if let Some(x) = rb {
                                        intra.insert((bi, as_ + k), x);
                                    }
                                }
                            }
                        } else {
                            i += 1;
                        }
                    }
                }
                r.line_sel.clear();
                r.line_anchor = None;
                r.file_view = Some(FileView { path: entry.path.clone(), entry, unstaged, staged, blocks, error, intra });
            },
        );
    }

    /// Run an index-changing operation with undo snapshot, then refresh.
    pub fn index_op(&mut self, title: &str, cmd: &str, work: impl FnOnce(&str) -> Result<(), String> + Send + 'static) {
        let Some(path) = self.path() else { return };
        let title = title.to_string();
        let cmd = cmd.to_string();
        self.spawn_busy(
            move || {
                let tree = stage::index_tree(&path).ok();
                let r = work(&path);
                (tree, r)
            },
            move |app, (tree, r)| match r {
                Ok(()) => {
                    app.log_op(title, cmd, tree.map(Undo::IndexTree).unwrap_or(Undo::None("Anlık görüntü alınamadı")));
                    app.refresh_status(true);
                    app.load_file_view();
                }
                Err(e) => app.toast(ToastKind::Err, e),
            },
        );
    }

    pub fn stage_files(&mut self, files: Vec<String>) {
        let n = files.len();
        self.index_op(&format!("{n} dosya hazırlandı"), "git add -A", move |p| stage::stage_paths(p, &files));
    }

    pub fn unstage_files(&mut self, files: Vec<String>) {
        let n = files.len();
        self.index_op(&format!("{n} dosya hazırlıktan çıkarıldı"), "git restore --staged", move |p| stage::unstage_paths(p, &files));
    }

    pub fn toggle_file(&mut self, e: &repo::Entry) {
        let (st, un) = staged_flags(e);
        let files = vec![e.path.clone()];
        let mut files2 = files.clone();
        if let Some(o) = &e.orig_path {
            files2.push(o.clone());
        }
        if st && !un {
            self.unstage_files(files2);
        } else {
            self.stage_files(files2);
        }
    }

    pub fn stage_all(&mut self) {
        let files: Vec<String> = self.repo.as_ref().map(|r| r.entries().iter().filter(|e| !e.conflicted).map(|e| e.path.clone()).collect()).unwrap_or_default();
        if !files.is_empty() {
            self.stage_files(files);
        }
    }

    pub fn unstage_all(&mut self) {
        let files: Vec<String> = self.repo.as_ref().map(|r| r.entries().iter().filter(|e| staged_flags(e).0).map(|e| e.path.clone()).collect()).unwrap_or_default();
        if !files.is_empty() {
            self.unstage_files(files);
        }
    }

    pub fn apply_hunk_lines(&mut self, b: Block, lines: Vec<usize>) {
        let Some(r) = self.repo.as_ref() else { return };
        let Some(fv) = r.file_view.as_ref() else { return };
        let mode = if b.staged { diff::PatchMode::Unstage } else { diff::PatchMode::Stage };
        let fd = fv.fdiff(b);
        let Some(patch) = diff::build_patch(fd, b.hunk, &lines, mode) else { return };
        let reverse = b.staged;
        let n = lines.len();
        let title = if reverse { format!("{n} satır hazırlıktan çıkarıldı") } else { format!("{n} satır hazırlandı") };
        self.index_op(&title, "git apply --cached", move |p| stage::apply_cached(p, &patch, reverse));
    }

    pub fn apply_hunk(&mut self, b: Block) {
        let Some(r) = self.repo.as_ref() else { return };
        let Some(fv) = r.file_view.as_ref() else { return };
        let all = diff::all_lines(fv.hunk(b));
        let reverse = b.staged;
        let Some(patch) = diff::build_patch(fv.fdiff(b), b.hunk, &all, if reverse { diff::PatchMode::Unstage } else { diff::PatchMode::Stage }) else { return };
        let title = if reverse { "1 hunk hazırlıktan çıkarıldı" } else { "1 hunk hazırlandı" };
        self.index_op(title, "git apply --cached", move |p| stage::apply_cached(p, &patch, reverse));
    }

    pub fn discard_hunk(&mut self, b: Block) {
        let Some(r) = self.repo.as_ref() else { return };
        let Some(fv) = r.file_view.as_ref() else { return };
        let all = diff::all_lines(fv.hunk(b));
        let Some(patch) = diff::build_patch(fv.fdiff(b), b.hunk, &all, diff::PatchMode::Stage) else { return };
        let path = r.path.clone();
        self.confirm(
            "Hunk atılsın mı?",
            "Bu hunk'taki değişiklikler çalışma alanından kaldırılır. İşlem günlüğünden geri alınamaz.",
            "Hunk'ı at",
            true,
            move |app| {
                app.spawn_busy(
                    move || stage::discard_patch(&path, &patch),
                    |app, r| match r {
                        Ok(()) => {
                            app.toast(ToastKind::Ok, "Hunk atıldı");
                            app.refresh_status(true);
                            app.load_file_view();
                        }
                        Err(e) => app.toast(ToastKind::Err, e),
                    },
                );
            },
        );
    }

    pub fn discard_file(&mut self, e: repo::Entry) {
        let Some(path) = self.path() else { return };
        let name = e.path.clone();
        self.confirm(
            &format!("“{}” değişiklikleri atılsın mı?", util::split_path(&name).1),
            "Takip edilen dosyalar kısa süreliğine geri alınabilir; yeni dosyalar Geri Dönüşüm Kutusu'na taşınır.",
            "Değişiklikleri at",
            true,
            move |app| {
                let item = repo::DiscardItem { path: e.path.clone(), orig_path: e.orig_path.clone(), kind: e.kind.clone() };
                let files = vec![e.path.clone()];
                app.spawn_busy(
                    move || repo::discard(path, vec![item]),
                    move |app, r| match r {
                        Ok(d) => {
                            app.log_op("Değişiklikler atıldı", "git checkout HEAD --", Undo::Discard { backup: d.backup, files });
                            app.toast_undo("Değişiklikler atıldı");
                            app.refresh_status(true);
                        }
                        Err(e) => app.toast(ToastKind::Err, e),
                    },
                );
            },
        );
    }

    pub fn commit(&mut self, then_push: bool) {
        let Some(r) = self.repo.as_ref() else { return };
        let staged = r.entries().iter().filter(|e| staged_flags(e).0).count();
        let summary = r.summary.trim().to_string();
        if summary.is_empty() {
            self.toast(ToastKind::Err, "Bir commit özeti yaz");
            return;
        }
        if staged == 0 && !r.amend {
            self.toast(ToastKind::Err, "Önce commit'e girecek değişiklikleri hazırla");
            return;
        }
        if r.entries().iter().any(|e| e.conflicted) {
            self.toast(ToastKind::Err, "Önce çatışmaları çöz");
            return;
        }
        let mut msg = summary.clone();
        if !r.body.trim().is_empty() {
            msg.push_str("\n\n");
            msg.push_str(r.body.trim());
        }
        if !r.coauthor.trim().is_empty() {
            msg.push_str("\n\nCo-authored-by: ");
            msg.push_str(r.coauthor.trim());
        }
        msg.push('\n');
        let path = r.path.clone();
        let amend = r.amend;
        let prev_head = stage::head_sha(&path);
        self.spawn_busy(
            move || stage::commit_staged(&path, &msg, amend),
            move |app, res| match res {
                Ok(sha) => {
                    if let Some(r) = app.repo.as_mut() {
                        r.summary.clear();
                        r.body.clear();
                        r.amend = false;
                        r.coauthor.clear();
                    }
                    app.log_op(
                        format!("Commit oluşturuldu · {}", util::short(&sha)),
                        if amend { "git commit --amend -F -" } else { "git commit -F -" },
                        if amend { Undo::None("Amend geri alınamaz; reflog'da durur") } else { Undo::ResetSoft(prev_head) },
                    );
                    app.toast_undo(format!("Commit oluşturuldu · {}", util::short(&sha)));
                    app.refresh_all();
                    if then_push {
                        app.push(false);
                    }
                }
                Err(e) => app.toast(ToastKind::Err, e),
            },
        );
    }

    pub fn fetch(&mut self) {
        let Some(path) = self.path() else { return };
        if self.sync_label.is_some() {
            return;
        }
        let progress = self.progress();
        let acc = self.account_key();
        self.sync_label = Some("Fetch");
        self.sync_text.clear();
        self.spawn(
            move || remote::fetch(&progress, path, None, acc),
            |app, r| {
                app.sync_label = None;
                match r {
                    Ok(()) => {
                        app.log_op("Fetch origin", "git fetch origin --prune", Undo::None("uzak durum, geri alınmaz"));
                        app.toast(ToastKind::Ok, "Fetch tamamlandı");
                        app.refresh_all();
                    }
                    Err(e) => app.toast(ToastKind::Err, e),
                }
            },
        );
    }

    pub fn pull(&mut self) {
        let Some(path) = self.path() else { return };
        if self.sync_label.is_some() {
            return;
        }
        let progress = self.progress();
        let acc = self.account_key();
        let prev = stage::head_sha(&path);
        self.sync_label = Some("Pull");
        self.sync_text.clear();
        self.spawn(
            move || remote::pull(&progress, path, "merge".into(), acc),
            move |app, r| {
                app.sync_label = None;
                match r {
                    Ok(res) if res.conflict => {
                        app.toast(ToastKind::Err, "Pull: çatışmalar var");
                        app.refresh_all();
                    }
                    Ok(_) => {
                        app.log_op("Pull origin", "git pull", prev.map(Undo::ResetHard).unwrap_or(Undo::None("")));
                        app.toast_undo("Pull tamamlandı");
                        app.refresh_all();
                    }
                    Err(e) => app.toast(ToastKind::Err, e),
                }
            },
        );
    }

    pub fn push(&mut self, force: bool) {
        let Some(r) = self.repo.as_ref() else { return };
        if self.sync_label.is_some() {
            return;
        }
        if r.remotes.is_empty() {
            self.run(Cmd::PublishRepo);
            return;
        }
        let path = r.path.clone();
        let st = r.status.clone();
        let progress = self.progress();
        let acc = self.account_key();
        self.sync_label = Some(if force { "Force push" } else { "Push" });
        self.sync_text.clear();
        let remote_name = r.remotes.iter().find(|x| x.name == "origin").or(r.remotes.first()).map(|x| x.name.clone()).unwrap_or("origin".into());
        self.spawn(
            move || {
                let publish = st.as_ref().map(|s| s.upstream.is_none() && s.branch.is_some()).unwrap_or(false);
                if publish {
                    remote::push(&progress, path, Some(remote_name), st.and_then(|s| s.branch), false, true, acc)
                } else {
                    remote::push(&progress, path, None, None, force, false, acc)
                }
            },
            move |app, r| {
                app.sync_label = None;
                match r {
                    Ok(()) => {
                        app.log_op(if force { "Force push" } else { "Push origin" }, if force { "git push --force-with-lease" } else { "git push" }, Undo::None("uzağa gönderildi, geri alınmaz"));
                        app.toast(ToastKind::Ok, "Değişiklikler gönderildi");
                        app.refresh_all();
                    }
                    Err(e) => app.toast(ToastKind::Err, e),
                }
            },
        );
    }

    /// Creates the repository on GitHub, adds it as `origin` and pushes the current branch.
    pub fn publish_repo(&mut self, name: String, desc: String, private: bool, acc: String) {
        let Some(r) = self.repo.as_ref() else { return };
        let path = r.path.clone();
        let branch = r.status.as_ref().and_then(|s| s.branch.clone());
        let has_commits = r.status.as_ref().map(|s| s.has_commits).unwrap_or(false);
        let progress = self.progress();
        self.sync_label = Some("Yayın");
        self.sync_text.clear();
        self.spawn(
            move || -> Result<bool, String> {
                let body = serde_json::json!({ "name": name, "description": desc, "private": private });
                let v = github::gh_api(acc.clone(), "POST".into(), "/user/repos".into(), Some(body), None)?;
                let url = v["clone_url"].as_str().ok_or_else(|| "GitHub yanıtında depo adresi yok".to_string())?.to_string();
                remote::set_remote(path.clone(), "origin".into(), url)?;
                if has_commits {
                    remote::push(&progress, path, Some("origin".into()), branch, false, true, Some(acc))?;
                }
                Ok(has_commits)
            },
            |app, r| {
                app.sync_label = None;
                match r {
                    Ok(true) => {
                        app.log_op("Depo yayınlandı", "git push -u origin", Undo::None("uzağa gönderildi, geri alınmaz"));
                        app.toast(ToastKind::Ok, "Depo yayınlandı");
                        app.refresh_all();
                    }
                    Ok(false) => {
                        app.toast(ToastKind::Ok, "Depo oluşturuldu; ilk commit'ten sonra Push'la");
                        app.refresh_all();
                    }
                    Err(e) => app.toast(ToastKind::Err, e),
                }
            },
        );
    }

    pub fn switch_branch(&mut self, name: String, stash_first: bool) {
        let Some(r) = self.repo.as_ref() else { return };
        let path = r.path.clone();
        let prev = r.branch();
        let remote_branch = r.branches.iter().find(|b| b.name == name && b.is_remote).map(|_| true).unwrap_or(false)
            && !r.branches.iter().any(|b| b.name == name && !b.is_remote);
        let msg = format!("ferro-autostash:{prev}");
        let n2 = name.clone();
        self.spawn_busy(
            move || -> Result<(), String> {
                if stash_first {
                    repo::stash_push(path.clone(), msg, true)?;
                }
                repo::checkout(path, n2, remote_branch, false)
            },
            move |app, res| match res {
                Ok(()) => {
                    app.log_op(format!("Dal değiştirildi: {prev} → {name}"), format!("git switch {name}"), Undo::Switch(prev.clone()));
                    app.toast_undo(format!("{name} dalına geçildi"));
                    if let Some(r) = app.repo.as_mut() {
                        r.sel_file = None;
                        r.file_view = None;
                    }
                    app.refresh_all();
                }
                Err(e) => app.toast(ToastKind::Err, e),
            },
        );
    }

    pub fn create_branch(&mut self, name: String) {
        let Some(path) = self.path() else { return };
        let prev = self.repo.as_ref().map(|r| r.branch()).unwrap_or_default();
        let n = name.clone();
        self.spawn_busy(
            move || repo::create_branch(path, n, None, true),
            move |app, r| match r {
                Ok(()) => {
                    app.log_op(format!("Dal oluşturuldu: {name}"), format!("git switch -c {name}"), Undo::Switch(prev));
                    app.toast_undo(format!("{name} oluşturuldu ve geçildi"));
                    app.refresh_all();
                }
                Err(e) => app.toast(ToastKind::Err, e),
            },
        );
    }

    pub fn stash_all(&mut self) {
        let Some(path) = self.path() else { return };
        self.spawn_busy(
            move || repo::stash_push(path, String::new(), true),
            |app, r| match r {
                Ok(()) => {
                    app.log_op("Değişiklikler stash'lendi", "git stash push -u", Undo::None("Stash sayfasından geri yükle"));
                    app.toast(ToastKind::Ok, "Değişiklikler stash'lendi");
                    app.refresh_all();
                }
                Err(e) => app.toast(ToastKind::Err, e),
            },
        );
    }

    pub fn stash_pop(&mut self, name: String) {
        let Some(path) = self.path() else { return };
        self.spawn_busy(
            move || repo::stash_op(path, name, "pop".into()),
            |app, r| match r {
                Ok(_) => {
                    app.toast(ToastKind::Ok, "Stash geri yüklendi");
                    app.refresh_all();
                }
                Err(e) => app.toast(ToastKind::Err, e),
            },
        );
    }

    pub fn load_commits(&mut self, reset: bool) {
        let Some(r) = self.repo.as_mut() else { return };
        if r.commits_loading && !reset {
            return;
        }
        if !reset && r.commits_done {
            return;
        }
        r.commits_loading = true;
        r.hist_seq += 1;
        let seq = r.hist_seq;
        let skip = if reset { 0 } else { r.commits.len() as u32 };
        let path = r.path.clone();
        let q = r.hist_query.trim().to_string();
        self.spawn(
            move || repo::log(path, None, skip, 200, if q.is_empty() { None } else { Some(q) }, None),
            move |app, res| {
                let Some(r) = app.repo.as_mut() else { return };
                if r.hist_seq != seq {
                    return;
                }
                r.commits_loading = false;
                let mut sel0 = false;
                let mut cjk_sample: Option<String> = None;
                match res {
                    Ok(list) => {
                        r.commits_done = list.len() < 200;
                        let old_sha = r.sel_commit.and_then(|i| r.commits.get(i)).map(|c| c.c.sha.clone());
                        let mut all: Vec<repo::Commit> = if reset { vec![] } else { r.commits.iter().map(|c| c.c.clone()).collect() };
                        all.extend(list);
                        let sample: String = all.iter().take(400).map(|c| format!("{}{}", c.subject, c.author)).collect();
                        cjk_sample = Some(sample);
                        let pairs: Vec<(String, Vec<String>)> = all.iter().map(|c| (c.sha.clone(), c.parents.clone())).collect();
                        let (rows, lanes) = graph::compute(&pairs);
                        r.lanes = lanes;
                        r.commits = all.into_iter().zip(rows).map(|(c, g)| CommitRow { ts: util::parse_ts(&c.date), c, g }).collect();
                        if reset {
                            r.sel_commit = old_sha.and_then(|sha| r.commits.iter().position(|c| c.c.sha == sha));
                        }
                        if r.sel_commit.is_none() && !r.commits.is_empty() {
                            sel0 = true;
                        }
                    }
                    Err(e) => {
                        if !e.contains("does not have any commits") {
                            app.toast(ToastKind::Err, e);
                        }
                    }
                }
                if let Some(sm) = cjk_sample {
                    app.ensure_cjk(&sm);
                }
                if sel0 {
                    app.select_commit(0);
                }
            },
        );
    }

    pub fn select_commit(&mut self, idx: usize) {
        let Some(r) = self.repo.as_mut() else { return };
        let Some(c) = r.commits.get(idx) else { return };
        r.sel_commit = Some(idx);
        r.commit_files.clear();
        r.sel_commit_file = None;
        r.commit_diff = None;
        let sha = c.c.sha.clone();
        let path = r.path.clone();
        let s2 = sha.clone();
        self.spawn(
            move || repo::commit_files(path, sha),
            move |app, res| {
                let Some(r) = app.repo.as_mut() else { return };
                if r.sel_commit.and_then(|i| r.commits.get(i)).map(|c| c.c.sha.as_str()) != Some(s2.as_str()) {
                    return;
                }
                if let Ok(f) = res {
                    let first = f.first().map(|x| (x.path.clone(), x.orig_path.clone()));
                    r.commit_files = f;
                    if let Some((p, o)) = first {
                        app.select_commit_file(p, o);
                    }
                }
            },
        );
    }

    pub fn select_commit_file(&mut self, file: String, orig: Option<String>) {
        let Some(r) = self.repo.as_mut() else { return };
        let Some(i) = r.sel_commit else { return };
        let sha = r.commits[i].c.sha.clone();
        r.sel_commit_file = Some(file.clone());
        r.commit_diff = None;
        let path = r.path.clone();
        let f2 = file.clone();
        self.spawn(
            move || repo::commit_diff(path, sha, file, orig),
            move |app, res| {
                let Some(r) = app.repo.as_mut() else { return };
                if r.sel_commit_file.as_deref() != Some(f2.as_str()) {
                    return;
                }
                if let Ok(t) = res {
                    r.commit_diff = Some(diff::parse_diff(&t));
                }
            },
        );
    }

    pub fn commit_op(&mut self, kind: &str, sha: String) {
        let Some(path) = self.path() else { return };
        let short = util::short(&sha).to_string();
        match kind {
            "branch" => self.modal = Some(Modal::FromCommit { sha, name: String::new(), tag: false }),
            "tag" => self.modal = Some(Modal::FromCommit { sha, name: String::new(), tag: true }),
            "pick" | "revert" => {
                let prev = stage::head_sha(&path);
                let pick = kind == "pick";
                self.spawn_busy(
                    move || if pick { repo::cherry_pick(path, sha) } else { repo::revert_commit(path, sha) },
                    move |app, r| match r {
                        Ok(res) if res.conflict => {
                            app.toast(ToastKind::Err, "Çatışma çıktı; çözüp devam et");
                            app.refresh_all();
                        }
                        Ok(_) => {
                            let t = if pick { format!("Cherry-pick: {short}") } else { format!("Geri alındı (revert): {short}") };
                            app.log_op(t.clone(), if pick { "git cherry-pick" } else { "git revert --no-edit" }, prev.map(Undo::ResetHard).unwrap_or(Undo::None("")));
                            app.toast_undo(t);
                            app.refresh_all();
                        }
                        Err(e) => app.toast(ToastKind::Err, e),
                    },
                );
            }
            k if k.starts_with("reset-") => {
                let mode = k.trim_start_matches("reset-").to_string();
                let prev = stage::head_sha(&path);
                let run = move |app: &mut App| {
                    let p2 = path.clone();
                    let m2 = mode.clone();
                    app.spawn_busy(
                        move || repo::reset_to(p2, sha, m2),
                        move |app, r| match r {
                            Ok(()) => {
                                app.log_op(format!("Sıfırlandı ({mode}) → {short}"), format!("git reset --{mode}"), if mode == "hard" { Undo::None("Hard reset: reflog'dan kurtarılabilir") } else { Undo::ResetSoft(prev) });
                                app.toast(ToastKind::Ok, "Sıfırlandı");
                                app.refresh_all();
                            }
                            Err(e) => app.toast(ToastKind::Err, e),
                        },
                    );
                };
                if kind == "reset-hard" {
                    self.confirm("Hard reset yapılsın mı?", "Seçili commit'ten sonraki commit'ler ve tüm yerel değişiklikler silinir.", "Hard reset", true, run);
                } else {
                    run(self);
                }
            }
            _ => {}
        }
    }

    pub fn create_at(&mut self, sha: String, name: String, tag: bool) {
        let Some(path) = self.path() else { return };
        let n = name.clone();
        self.spawn_busy(
            move || if tag { repo::create_tag(path, n, Some(sha), None) } else { repo::create_branch(path, n, Some(sha), false) },
            move |app, r| match r {
                Ok(()) => {
                    app.toast(ToastKind::Ok, if tag { format!("Etiket oluşturuldu: {name}") } else { format!("Dal oluşturuldu: {name}") });
                    app.refresh_all();
                }
                Err(e) => app.toast(ToastKind::Err, e),
            },
        );
    }

    pub fn merge_branch(&mut self, name: String) {
        let Some(path) = self.path() else { return };
        let prev = stage::head_sha(&path);
        let n2 = name.clone();
        self.spawn_busy(
            move || repo::merge(path, n2, false, false),
            move |app, r| match r {
                Ok(res) if res.conflict => {
                    app.toast(ToastKind::Err, "Birleştirme çatışma verdi; çözüp tamamla");
                    app.refresh_all();
                }
                Ok(_) => {
                    app.log_op(format!("Birleştirildi: {name}"), format!("git merge {name}"), prev.map(Undo::ResetHard).unwrap_or(Undo::None("")));
                    app.toast_undo(format!("{name} birleştirildi"));
                    app.refresh_all();
                }
                Err(e) => app.toast(ToastKind::Err, e),
            },
        );
    }

    pub fn select_stash(&mut self, idx: usize) {
        let Some(r) = self.repo.as_mut() else { return };
        let Some(s) = r.stashes.get(idx) else { return };
        r.sel_stash = Some(idx);
        r.stash_files.clear();
        r.stash_diff = None;
        let name = s.name.clone();
        let path = r.path.clone();
        let n2 = name.clone();
        self.spawn(
            move || repo::stash_files(path, name),
            move |app, res| {
                let Some(r) = app.repo.as_mut() else { return };
                if r.sel_stash.and_then(|i| r.stashes.get(i)).map(|s| s.name.as_str()) != Some(n2.as_str()) {
                    return;
                }
                if let Ok(f) = res {
                    let first = f.first().map(|x| x.path.clone());
                    r.stash_files = f;
                    if let Some(p) = first {
                        app.select_stash_file(p);
                    }
                }
            },
        );
    }

    pub fn select_stash_file(&mut self, file: String) {
        let Some(r) = self.repo.as_ref() else { return };
        let Some(s) = r.sel_stash.and_then(|i| r.stashes.get(i)) else { return };
        let (name, path) = (s.name.clone(), r.path.clone());
        let f2 = file.clone();
        self.spawn(
            move || repo::stash_diff(path, name, file),
            move |app, res| {
                if let (Some(r), Ok(t)) = (app.repo.as_mut(), res) {
                    r.stash_diff = Some((f2, diff::parse_diff(&t)));
                }
            },
        );
    }

    pub fn stash_apply(&mut self, name: String, op: &str) {
        let Some(path) = self.path() else { return };
        let op = op.to_string();
        let label = match op.as_str() {
            "pop" => "Stash geri yüklendi",
            "apply" => "Stash uygulandı",
            _ => "Stash silindi",
        };
        self.spawn_busy(
            move || repo::stash_op(path, name, op),
            move |app, r| match r {
                Ok(res) if res.conflict => {
                    app.toast(ToastKind::Err, "Stash çatışma verdi");
                    app.refresh_all();
                }
                Ok(_) => {
                    app.toast(ToastKind::Ok, label);
                    if let Some(r) = app.repo.as_mut() {
                        r.sel_stash = None;
                    }
                    app.refresh_all();
                }
                Err(e) => app.toast(ToastKind::Err, e),
            },
        );
    }

    pub fn load_conflicts(&mut self) {
        let Some(r) = self.repo.as_mut() else { return };
        let Some(file) = r.entries().iter().find(|e| e.conflicted).map(|e| e.path.clone()) else { return };
        if r.conflict_file.as_deref() == Some(file.as_str()) && !r.conflict_segs.is_empty() {
            return;
        }
        self.open_conflict_file(file);
    }

    pub fn open_conflict_file(&mut self, file: String) {
        let Some(path) = self.path() else { return };
        if let Some(r) = self.repo.as_mut() {
            r.conflict_file = Some(file.clone());
            r.conflict_segs.clear();
            r.conflict_idx = 0;
            r.conflict_edit = false;
        }
        let f2 = file.clone();
        self.spawn(
            move || repo::read_file(path, file),
            move |app, res| {
                let Some(r) = app.repo.as_mut() else { return };
                if r.conflict_file.as_deref() != Some(f2.as_str()) {
                    return;
                }
                if let Ok(t) = res {
                    r.conflict_segs = diff::parse_conflicts(&t);
                }
            },
        );
    }

    pub fn save_conflict_file(&mut self) {
        let Some(r) = self.repo.as_ref() else { return };
        let Some(file) = r.conflict_file.clone() else { return };
        let text = diff::compose_conflicts(&r.conflict_segs);
        let path = r.path.clone();
        self.spawn_busy(
            move || {
                repo::write_file(path.clone(), file.clone(), text)?;
                repo::mark_resolved(path, file)
            },
            |app, r| match r {
                Ok(()) => {
                    app.toast(ToastKind::Ok, "Dosya çözüldü olarak işaretlendi");
                    if let Some(r) = app.repo.as_mut() {
                        r.conflict_file = None;
                        r.conflict_segs.clear();
                    }
                    app.refresh_all();
                }
                Err(e) => app.toast(ToastKind::Err, e),
            },
        );
    }

    pub fn confirm(&mut self, title: &str, body: &str, ok: &str, danger: bool, action: impl FnOnce(&mut App) + Send + 'static) {
        self.modal = Some(Modal::Confirm { title: title.into(), body: body.into(), ok: ok.into(), danger, action: Some(Box::new(action)) });
    }

    pub fn goto(&mut self, s: Screen) {
        self.screen = s;
        self.menu = None;
        if matches!(s, Screen::Branches | Screen::Stash) {
            self.refresh_meta();
        }
        if s == Screen::History && self.repo.as_ref().map(|r| r.commits.is_empty() && !r.commits_loading).unwrap_or(false) {
            self.load_commits(true);
        }
        if s == Screen::Pr {
            crate::screens::pr::load_prs(self);
        }
    }

    pub fn run(&mut self, cmd: Cmd) {
        self.menu = None;
        match cmd {
            Cmd::Commit => self.commit(false),
            Cmd::CommitPush => self.commit(true),
            Cmd::Push => self.push(false),
            Cmd::ForcePush => {
                if self.s.confirm_force {
                    self.confirm("Force push yapılsın mı?", "Uzak daldaki geçmiş yerel geçmişinle değiştirilir. --force-with-lease kullanılır: başkası araya commit eklediyse işlem reddedilir.", "Force push", true, |a| a.push(true));
                } else {
                    self.push(true)
                }
            }
            Cmd::Pull => self.pull(),
            Cmd::Fetch => self.fetch(),
            Cmd::Undo => self.undo_last(),
            Cmd::Refresh => self.refresh_all(),
            Cmd::Goto(s) => self.goto(s),
            Cmd::Palette => {
                self.palette = Some(Default::default());
                self.switcher = None;
            }
            Cmd::Switcher => {
                self.refresh_meta();
                self.switcher = Some(Default::default());
                self.palette = None;
            }
            Cmd::NewBranch => self.modal = Some(Modal::NewBranch(String::new())),
            Cmd::SwitchTo(b) => {
                let dirty = self.repo.as_ref().map(|r| r.dirty()).unwrap_or(false);
                if dirty {
                    self.switcher = Some(crate::screens::palette::SwitcherState { pending: Some(b), ..Default::default() });
                } else {
                    self.switch_branch(b, false);
                }
            }
            Cmd::NewRepo => {
                if let Some(d) = rfd::FileDialog::new().set_title("Yeni deponun klasörünü seç").pick_folder() {
                    let ds = d.to_string_lossy().to_string();
                    let d2 = ds.clone();
                    self.spawn_busy(move || repo::init_repo(d2, "main".into()), move |app, r| match r {
                        Ok(()) => app.open_repo(ds),
                        Err(e) => app.toast(ToastKind::Err, e),
                    });
                }
            }
            Cmd::OpenFolder => {
                if let Some(p) = rfd::FileDialog::new().set_title("Depo klasörünü seç").pick_folder() {
                    self.open_repo(p.to_string_lossy().to_string());
                }
            }
            Cmd::CloneRepo => {
                self.welcome.clone_open = true;
                self.close_repo();
            }
            Cmd::Settings => self.modal = Some(Modal::Settings(0)),
            Cmd::StageAll => self.stage_all(),
            Cmd::UnstageAll => self.unstage_all(),
            Cmd::ToggleStageSelected => {
                let e = self.repo.as_ref().and_then(|r| r.sel_file.as_ref().and_then(|f| r.entries().iter().find(|e| &e.path == f).cloned()));
                if let Some(e) = e {
                    self.toggle_file(&e);
                }
            }
            Cmd::NextFile | Cmd::PrevFile => {
                let next = cmd == Cmd::NextFile;
                let pick = self.repo.as_ref().and_then(|r| {
                    let es: Vec<&repo::Entry> = r.entries().iter().filter(|e| !e.conflicted).collect();
                    if es.is_empty() {
                        return None;
                    }
                    let cur = r.sel_file.as_ref().and_then(|f| es.iter().position(|e| &e.path == f)).unwrap_or(0);
                    let i = if next { (cur + 1).min(es.len() - 1) } else { cur.saturating_sub(1) };
                    Some(es[i].path.clone())
                });
                if let Some(p) = pick {
                    self.select_file(p);
                }
            }
            Cmd::NextHunk | Cmd::PrevHunk => {
                if let Some(r) = self.repo.as_mut() {
                    let n = r.file_view.as_ref().map(|f| f.blocks.len()).unwrap_or(0);
                    if n > 0 {
                        let cur = r.focus_hunk.unwrap_or(0);
                        r.focus_hunk = Some(if cmd == Cmd::NextHunk { (cur + 1).min(n - 1) } else { cur.saturating_sub(1) });
                        r.pending_scroll();
                    }
                }
            }
            Cmd::ToggleAmend => {
                let want = self.repo.as_ref().map(|r| !r.amend).unwrap_or(false);
                self.set_amend(want);
            }
            Cmd::StashAll => self.stash_all(),
            Cmd::StashPop(n) => self.stash_pop(n),
            Cmd::OpenEditor => {
                if let Some(p) = self.path() {
                    let ed = if self.s.editor.is_empty() { self.editors.first().map(|e| e.id.clone()).unwrap_or("notepad".into()) } else { self.s.editor.clone() };
                    if let Err(e) = sys::open_in_editor(&p, &ed) {
                        self.toast(ToastKind::Err, e);
                    }
                }
            }
            Cmd::OpenTerminal => {
                if let Some(p) = self.path() {
                    if let Err(e) = sys::open_in_terminal(&p) {
                        self.toast(ToastKind::Err, e);
                    }
                }
            }
            Cmd::Reveal => {
                if let Some(p) = self.path() {
                    let _ = sys::reveal(&p);
                }
            }
            Cmd::CloseRepo => self.close_repo(),
            Cmd::ToggleTheme => {
                self.s.mode = if self.th.p.dark { Mode::Light } else { Mode::Dark };
                self.apply_theme();
            }
            Cmd::DiscardFile(f) => {
                let e = self.repo.as_ref().and_then(|r| r.entries().iter().find(|e| e.path == f).cloned());
                if let Some(e) = e {
                    self.discard_file(e);
                }
            }
            Cmd::OpenRecent(p) => self.open_repo(p),
            Cmd::CommitOp(k, sha) => self.commit_op(&k, sha),
            Cmd::Redo => self.redo(),
            Cmd::PublishRepo => {
                if self.s.account.is_none() {
                    self.toast(ToastKind::Info, "Önce GitHub'a giriş yap");
                    crate::screens::welcome::login_start(self);
                } else if let Some(r) = self.repo.as_ref() {
                    let name: String = r.name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '-' }).collect();
                    let acc = self.account_key().unwrap_or_default();
                    self.modal = Some(Modal::Publish { name, desc: String::new(), private: true, acc });
                }
            }
            Cmd::CopyText(t) => {
                self.ctx.copy_text(t);
                self.toast(ToastKind::Info, "Kopyalandı");
            }
            Cmd::UndoTo(n) => self.undo_n(n),
            Cmd::SelectFile(f) => {
                self.screen = Screen::Changes;
                self.select_file(f);
            }
            Cmd::MergeInto(b) => self.merge_branch(b),
            Cmd::DeleteBranch(b) => {
                let b2 = b.clone();
                self.confirm(&format!("“{b}” dalı silinsin mi?"), "Birleştirilmemiş commit'ler varsa git silmeyi reddeder.", "Dalı sil", true, move |app| {
                    let Some(path) = app.path() else { return };
                    app.spawn_busy(
                        move || repo::delete_branch(path, b2, false),
                        |app, r| match r {
                            Ok(()) => {
                                app.toast(ToastKind::Ok, "Dal silindi");
                                app.refresh_all();
                            }
                            Err(e) => app.toast(ToastKind::Err, e),
                        },
                    );
                });
            }
            Cmd::StageFile(f) => {
                let e = self.repo.as_ref().and_then(|r| r.entries().iter().find(|e| e.path == f).cloned());
                if let Some(e) = e {
                    self.toggle_file(&e);
                }
            }
            Cmd::CopyPath(f) => {
                self.ctx.copy_text(f);
                self.toast(ToastKind::Info, "Yol kopyalandı");
            }
            Cmd::CopyCommand => {
                if let Some((c, _, _)) = stage::last_log() {
                    self.ctx.copy_text(format!("git {c}"));
                    self.toast(ToastKind::Info, "Komut kopyalandı");
                }
            }
        }
    }

    pub fn set_amend(&mut self, on: bool) {
        let Some(r) = self.repo.as_mut() else { return };
        r.amend = on;
        if on && r.summary.is_empty() {
            if let Some(c) = r.commits.first() {
                r.summary = c.c.subject.clone();
                r.body = c.c.body.clone();
            }
        }
    }

    pub fn save_if_due(&mut self) {
        if let Some(t) = self.save_due {
            if Instant::now() >= t {
                self.save_due = None;
                if let Ok(j) = serde_json::to_string(&self.s) {
                    let _ = sys::save_state(&j);
                }
            }
        }
    }
}

impl Repo {
    pub fn pending_scroll(&mut self) {
        self.scroll_req = true;
    }
}

pub fn parse_github(url: &str, host: &str) -> Option<(String, String)> {
    let u = url.trim();
    let rest = u
        .strip_prefix(&format!("https://{host}/"))
        .or_else(|| u.strip_prefix(&format!("git@{host}:")))
        .or_else(|| u.strip_prefix(&format!("ssh://git@{host}/")))
        .or_else(|| u.strip_prefix(&format!("http://{host}/")))?;
    let rest = rest.trim_end_matches('/').trim_end_matches(".git");
    let (o, r) = rest.split_once('/')?;
    Some((o.to_string(), r.to_string()))
}
