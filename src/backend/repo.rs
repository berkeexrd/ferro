//! Repository level commands: status, diff, commit, discard, branches, conflicts, stash, tags.

use super::git;
use serde::{Deserialize, Serialize};

const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

fn norm(p: &str) -> String {
    #[cfg(windows)]
    {
        p.replace('/', "\\")
    }
    #[cfg(not(windows))]
    {
        p.to_string()
    }
}

fn nul_join(paths: &[String]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in paths {
        v.extend_from_slice(p.as_bytes());
        v.push(0);
    }
    v
}

fn with_paths(cwd: &str, args: &[&str], paths: &[String]) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut a: Vec<&str> = args.to_vec();
    a.push("--pathspec-from-file=-");
    a.push("--pathspec-file-nul");
    let data = nul_join(paths);
    let o = git::run_full(Some(cwd), &a, &[], Some(&data), false)?;
    if o.ok {
        Ok(())
    } else {
        Err(git::clean_err(&o))
    }
}

fn has_head(cwd: &str) -> bool {
    git::run(cwd, &["rev-parse", "--verify", "-q", "HEAD"]).map(|o| o.ok).unwrap_or(false)
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    pub root: String,
    pub name: String,
}

pub fn repo_info(path: String) -> Result<RepoInfo, String> {
        if !std::path::Path::new(&path).exists() {
        return Err("Klasör bulunamadı".into());
    }
    let o = git::run(&path, &["rev-parse", "--show-toplevel"])?;
    if !o.ok {
        return Err("Bu klasör bir Git deposu değil".into());
    }
    let root = norm(o.text().trim());
    let name = std::path::Path::new(&root)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| root.clone());
    Ok(RepoInfo { root, name })
}

pub fn init_repo(path: String, branch: String) -> Result<(), String> {
        std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    let b = if branch.is_empty() { "main".to_string() } else { branch };
    git::ok(&path, &["init", "-b", &b]).map(|_| ())
}

pub fn write_file(path: String, rel: String, content: String) -> Result<(), String> {
        let p = std::path::Path::new(&path).join(&rel);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, content).map_err(|e| e.to_string())
}

pub fn read_file(path: String, rel: String) -> Result<String, String> {
        let p = std::path::Path::new(&path).join(&rel);
    std::fs::read_to_string(&p).map_err(|e| e.to_string())
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Blob {
    pub mime: String,
    pub data: String,
    pub size: usize,
}

pub fn read_blob(path: String, rev: Option<String>, file: String) -> Result<Blob, String> {
        use base64::Engine;
    let bytes = match rev {
        None => std::fs::read(std::path::Path::new(&path).join(&file)).map_err(|e| e.to_string())?,
        Some(r) => {
            let spec = format!("{r}:{file}");
            let o = git::run(&path, &["show", &spec])?;
            if !o.ok {
                return Err(git::clean_err(&o));
            }
            o.stdout
        }
    };
    if bytes.len() > 12 * 1024 * 1024 {
        return Err("Dosya önizleme için çok büyük".into());
    }
    let ext = file.rsplit('.').next().unwrap_or("").to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    };
    Ok(Blob {
        mime: mime.into(),
        size: bytes.len(),
        data: base64::engine::general_purpose::STANDARD.encode(&bytes),
    })
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub path: String,
    pub orig_path: Option<String>,
    pub kind: String,
    pub xy: String,
    pub conflicted: bool,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub branch: Option<String>,
    pub detached: bool,
    pub oid: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub entries: Vec<Entry>,
    pub state: String,
    pub state_info: String,
    pub has_commits: bool,
}

fn kind_of(x: char, y: char) -> &'static str {
    if x == 'R' || y == 'R' {
        "renamed"
    } else if x == 'C' || y == 'C' {
        "copied"
    } else if x == 'A' || y == 'A' {
        "added"
    } else if x == 'D' || y == 'D' {
        "deleted"
    } else {
        "modified"
    }
}

pub fn status(path: String) -> Result<Status, String> {
        let o = git::run(&path, &["status", "--porcelain=v2", "-z", "--branch", "--untracked-files=all"])?;
    if !o.ok {
        return Err(git::clean_err(&o));
    }
    let text = o.text();
    let mut st = Status {
        branch: None,
        detached: false,
        oid: None,
        upstream: None,
        ahead: 0,
        behind: 0,
        entries: vec![],
        state: "none".into(),
        state_info: String::new(),
        has_commits: true,
    };
    let parts: Vec<&str> = text.split('\0').collect();
    let mut i = 0;
    while i < parts.len() {
        let l = parts[i];
        i += 1;
        if l.is_empty() {
            continue;
        }
        if let Some(h) = l.strip_prefix("# ") {
            if let Some(v) = h.strip_prefix("branch.oid ") {
                if v == "(initial)" {
                    st.has_commits = false;
                } else {
                    st.oid = Some(v.to_string());
                }
            } else if let Some(v) = h.strip_prefix("branch.head ") {
                if v == "(detached)" {
                    st.detached = true;
                } else {
                    st.branch = Some(v.to_string());
                }
            } else if let Some(v) = h.strip_prefix("branch.upstream ") {
                st.upstream = Some(v.to_string());
            } else if let Some(v) = h.strip_prefix("branch.ab ") {
                for t in v.split(' ') {
                    if let Some(a) = t.strip_prefix('+') {
                        st.ahead = a.parse().unwrap_or(0);
                    } else if let Some(b) = t.strip_prefix('-') {
                        st.behind = b.parse().unwrap_or(0);
                    }
                }
            }
            continue;
        }
        let kind = l.chars().next().unwrap_or(' ');
        match kind {
            '1' => {
                let f: Vec<&str> = l.splitn(9, ' ').collect();
                if f.len() == 9 {
                    let xy: Vec<char> = f[1].chars().collect();
                    st.entries.push(Entry {
                        path: f[8].to_string(),
                        orig_path: None,
                        kind: kind_of(xy[0], xy[1]).into(),
                        xy: f[1].to_string(),
                        conflicted: false,
                    });
                }
            }
            '2' => {
                let f: Vec<&str> = l.splitn(10, ' ').collect();
                let orig = parts.get(i).map(|s| s.to_string());
                i += 1;
                if f.len() == 10 {
                    let xy: Vec<char> = f[1].chars().collect();
                    st.entries.push(Entry {
                        path: f[9].to_string(),
                        orig_path: orig,
                        kind: kind_of(xy[0], xy[1]).into(),
                        xy: f[1].to_string(),
                        conflicted: false,
                    });
                }
            }
            'u' => {
                let f: Vec<&str> = l.splitn(11, ' ').collect();
                if f.len() == 11 {
                    st.entries.push(Entry {
                        path: f[10].to_string(),
                        orig_path: None,
                        kind: "conflicted".into(),
                        xy: f[1].to_string(),
                        conflicted: true,
                    });
                }
            }
            '?' => {
                if l.len() > 2 {
                    st.entries.push(Entry {
                        path: l[2..].to_string(),
                        orig_path: None,
                        kind: "untracked".into(),
                        xy: "??".into(),
                        conflicted: false,
                    });
                }
            }
            _ => {}
        }
    }
    st.entries.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));

    if let Ok(gd) = git::ok(&path, &["rev-parse", "--absolute-git-dir"]) {
        let gd = std::path::PathBuf::from(gd.trim());
        let read1 = |p: std::path::PathBuf| {
            std::fs::read_to_string(p)
                .ok()
                .and_then(|s| s.lines().next().map(|x| x.to_string()))
                .unwrap_or_default()
        };
        if gd.join("rebase-merge").exists() || gd.join("rebase-apply").exists() {
            st.state = "rebase".into();
            let hn = read1(gd.join("rebase-merge").join("head-name"));
            st.state_info = hn.trim_start_matches("refs/heads/").to_string();
        } else if gd.join("MERGE_HEAD").exists() {
            st.state = "merge".into();
            st.state_info = read1(gd.join("MERGE_MSG"));
        } else if gd.join("CHERRY_PICK_HEAD").exists() {
            st.state = "cherry-pick".into();
        } else if gd.join("REVERT_HEAD").exists() {
            st.state = "revert".into();
        }
    }
    Ok(st)
}

pub fn diff_file(
    path: String,
    file: String,
    orig: Option<String>,
    untracked: bool,
    ignore_ws: bool,
    context: Option<u32>,
) -> Result<String, String> {
        let ctx = format!("-U{}", context.unwrap_or(3));
    let mut args: Vec<&str> = vec!["diff", "--no-color", "--no-ext-diff", &ctx];
    if ignore_ws {
        args.push("-w");
    }
    if untracked {
        args.push("--no-index");
        args.push("--");
        args.push("/dev/null");
        args.push(&file);
        let o = git::run(&path, &args)?;
        if o.code > 1 {
            return Err(git::clean_err(&o));
        }
        return Ok(o.text());
    }
    let base = if has_head(&path) { "HEAD" } else { EMPTY_TREE };
    args.push("-M");
    args.push(base);
    args.push("--");
    if let Some(o) = orig.as_deref() {
        args.push(o);
    }
    args.push(&file);
    git::ok(&path, &args)
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Pick {
    pub path: String,
    pub orig_path: Option<String>,
    pub patch: Option<String>,
}

pub fn commit(
    path: String,
    message: String,
    picks: Vec<Pick>,
    amend: bool,
    state: String,
) -> Result<String, String> {
        if state == "none" {
        let _ = git::run(&path, &["reset", "-q"]);
    }
    let mut whole: Vec<String> = vec![];
    for p in &picks {
        match &p.patch {
            None => {
                whole.push(p.path.clone());
                if let Some(o) = &p.orig_path {
                    whole.push(o.clone());
                }
            }
            Some(patch) => {
                let o = git::run_full(
                    Some(&path),
                    &["apply", "--cached", "--recount", "--whitespace=nowarn", "-"],
                    &[],
                    Some(patch.as_bytes()),
                    false,
                )?;
                if !o.ok {
                    return Err(format!("Seçili satırlar uygulanamadı: {}", git::clean_err(&o)));
                }
            }
        }
    }
    with_paths(&path, &["add", "-A"], &whole)?;

    let mut args: Vec<&str> = vec!["commit", "-F", "-"];
    if amend {
        args.push("--amend");
    }
    let o = git::run_full(Some(&path), &args, &[], Some(message.as_bytes()), true)?;
    if !o.ok {
        return Err(git::clean_err(&o));
    }
    Ok(git::ok(&path, &["rev-parse", "HEAD"])?.trim().to_string())
}

pub fn amend_message(path: String, message: String) -> Result<(), String> {
        let o = git::run_full(
        Some(&path),
        &["commit", "--amend", "--only", "-F", "-"],
        &[],
        Some(message.as_bytes()),
        true,
    )?;
    if o.ok {
        Ok(())
    } else {
        Err(git::clean_err(&o))
    }
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct UndoInfo {
    pub summary: String,
    pub body: String,
}

pub fn undo_commit(path: String) -> Result<UndoInfo, String> {
        let s = git::ok(&path, &["log", "-1", "--format=%s%x1f%b"])?;
    let mut it = s.splitn(2, '\x1f');
    let summary = it.next().unwrap_or("").trim().to_string();
    let body = it.next().unwrap_or("").trim().to_string();
    let parents = git::ok(&path, &["rev-list", "--parents", "-n", "1", "HEAD"])?;
    if parents.split_whitespace().count() < 2 {
        // root commit: drop it but keep files staged
        git::ok(&path, &["update-ref", "-d", "HEAD"])?;
    } else {
        git::ok(&path, &["reset", "--soft", "HEAD~1"])?;
    }
    git::run(&path, &["reset", "-q"])?;
    Ok(UndoInfo { summary, body })
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiscardItem {
    pub path: String,
    pub orig_path: Option<String>,
    pub kind: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscardResult {
    pub backup: Option<String>,
}

fn remove_empty_parents(root: &str, file: &std::path::Path) {
    let root = std::path::Path::new(root);
    let mut cur = file.parent();
    while let Some(d) = cur {
        if d == root || !d.starts_with(root) {
            break;
        }
        if std::fs::remove_dir(d).is_err() {
            break;
        }
        cur = d.parent();
    }
}

pub fn discard(path: String, items: Vec<DiscardItem>) -> Result<DiscardResult, String> {
        // Save a recoverable snapshot of tracked changes before touching anything.
    let mut backup = None;
    if has_head(&path) {
        if let Ok(o) = git::run(&path, &["stash", "create"]) {
            let sha = o.text().trim().to_string();
            if o.ok && !sha.is_empty() {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let r = format!("refs/ferro/discards/{ts}");
                if git::run(&path, &["update-ref", &r, &sha]).map(|o| o.ok).unwrap_or(false) {
                    backup = Some(sha);
                }
            }
        }
    }
    let mut restore: Vec<String> = vec![];
    let mut remove: Vec<String> = vec![];
    let mut trash: Vec<String> = vec![];
    for it in &items {
        match it.kind.as_str() {
            "untracked" => trash.push(it.path.clone()),
            "added" | "copied" => remove.push(it.path.clone()),
            "renamed" => {
                remove.push(it.path.clone());
                if let Some(o) = &it.orig_path {
                    restore.push(o.clone());
                }
            }
            _ => restore.push(it.path.clone()),
        }
    }
    with_paths(&path, &["rm", "-f", "-q"], &remove)?;
    with_paths(&path, &["checkout", "HEAD"], &restore)?;
    for t in &trash {
        let full = std::path::Path::new(&path).join(t);
        if full.exists() {
            if let Err(e) = trash::delete(&full) {
                // fall back to hard delete only when the recycle bin is unavailable
                if std::fs::remove_file(&full).is_err() {
                    return Err(format!("{t} silinemedi: {e}"));
                }
            }
            remove_empty_parents(&path, &full);
        }
    }
    Ok(DiscardResult { backup })
}

pub fn restore_discarded(path: String, sha: String, files: Vec<String>) -> Result<(), String> {
        for f in &files {
        let ok = git::run(&path, &["checkout", &sha, "--", f]).map(|o| o.ok).unwrap_or(false);
        if !ok {
            let _ = git::run(&path, &["rm", "-f", "-q", "--", f]);
        }
    }
    let _ = git::run(&path, &["reset", "-q"]);
    Ok(())
}

pub fn ignore_pattern(path: String, pattern: String) -> Result<(), String> {
        let f = std::path::Path::new(&path).join(".gitignore");
    let mut cur = std::fs::read_to_string(&f).unwrap_or_default();
    if cur.lines().any(|l| l.trim() == pattern) {
        return Ok(());
    }
    if !cur.is_empty() && !cur.ends_with('\n') {
        cur.push('\n');
    }
    cur.push_str(&pattern);
    cur.push('\n');
    std::fs::write(&f, cur).map_err(|e| e.to_string())
}

pub fn resolve_side(path: String, file: String, side: String) -> Result<(), String> {
        let flag = if side == "ours" { "--ours" } else { "--theirs" };
    let o = git::run(&path, &["checkout", flag, "--", &file])?;
    if o.ok {
        git::ok(&path, &["add", "--", &file]).map(|_| ())
    } else {
        // one side deleted the file: accepting that side means removing it
        git::ok(&path, &["rm", "-f", "-q", "--", &file]).map(|_| ())
    }
}

pub fn mark_resolved(path: String, file: String) -> Result<(), String> {
        if std::path::Path::new(&path).join(&file).exists() {
        git::ok(&path, &["add", "--", &file]).map(|_| ())
    } else {
        git::ok(&path, &["rm", "-f", "-q", "--", &file]).map(|_| ())
    }
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    pub name: String,
    pub full: String,
    pub is_remote: bool,
    pub remote: Option<String>,
    pub sha: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub gone: bool,
    pub date: String,
    pub author: String,
    pub subject: String,
    pub current: bool,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Branches {
    pub list: Vec<Branch>,
    pub recent: Vec<String>,
    pub default_branch: String,
}

pub fn branches(path: String) -> Result<Branches, String> {
        let fmt = "--format=%(HEAD)%00%(refname)%00%(objectname)%00%(upstream:short)%00%(upstream:track)%00%(committerdate:unix)%00%(authorname)%00%(contents:subject)";
    let out = git::ok(&path, &["for-each-ref", fmt, "refs/heads", "refs/remotes"])?;
    let mut list = vec![];
    for l in out.lines() {
        let f: Vec<&str> = l.split('\0').collect();
        if f.len() < 8 {
            continue;
        }
        let full = f[1];
        let (is_remote, name, remote) = if let Some(n) = full.strip_prefix("refs/heads/") {
            (false, n.to_string(), None)
        } else if let Some(n) = full.strip_prefix("refs/remotes/") {
            if n.ends_with("/HEAD") {
                continue;
            }
            (true, n.to_string(), n.split('/').next().map(|s| s.to_string()))
        } else {
            continue;
        };
        let track = f[4];
        let mut ahead = 0;
        let mut behind = 0;
        let gone = track.contains("gone");
        for part in track.trim_matches(|c| c == '[' || c == ']').split(',') {
            let p = part.trim();
            if let Some(a) = p.strip_prefix("ahead ") {
                ahead = a.parse().unwrap_or(0);
            } else if let Some(b) = p.strip_prefix("behind ") {
                behind = b.parse().unwrap_or(0);
            }
        }
        list.push(Branch {
            name,
            full: full.to_string(),
            is_remote,
            remote,
            sha: f[2].to_string(),
            upstream: if f[3].is_empty() { None } else { Some(f[3].to_string()) },
            ahead,
            behind,
            gone,
            date: f[5].to_string(),
            author: f[6].to_string(),
            subject: f[7].to_string(),
            current: f[0] == "*",
        });
    }
    let mut recent: Vec<String> = vec![];
    if let Ok(o) = git::run(&path, &["reflog", "-n", "1500", "--format=%gs"]) {
        for l in o.text().lines() {
            if let Some(rest) = l.strip_prefix("checkout: moving from ") {
                if let Some((_, to)) = rest.rsplit_once(" to ") {
                    let to = to.trim().to_string();
                    let is_sha = to.len() >= 7 && to.chars().all(|c| c.is_ascii_hexdigit());
                    if !is_sha
                        && !recent.contains(&to)
                        && list.iter().any(|b| !b.is_remote && b.name == to)
                    {
                        recent.push(to);
                    }
                }
            }
            if recent.len() >= 8 {
                break;
            }
        }
    }
    let mut default_branch = String::new();
    if let Ok(o) = git::run(&path, &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"]) {
        if o.ok {
            default_branch = o.text().trim().trim_start_matches("origin/").to_string();
        }
    }
    if default_branch.is_empty() {
        for c in ["main", "master", "develop"] {
            if list.iter().any(|b| !b.is_remote && b.name == c) {
                default_branch = c.to_string();
                break;
            }
        }
    }
    Ok(Branches { list, recent, default_branch })
}

pub fn create_branch(path: String, name: String, start: Option<String>, checkout: bool) -> Result<(), String> {
        let mut args: Vec<&str> = if checkout { vec!["switch", "-c", &name] } else { vec!["branch", &name] };
    if let Some(s) = start.as_deref() {
        args.push(s);
    }
    git::ok(&path, &args).map(|_| ())
}

pub fn checkout(path: String, name: String, remote: bool, detach: bool) -> Result<(), String> {
        let args: Vec<&str> = if detach {
        vec!["switch", "--detach", &name]
    } else if remote {
        vec!["switch", "--track", &name]
    } else {
        vec!["switch", &name]
    };
    git::ok(&path, &args).map(|_| ())
}

pub fn rename_branch(path: String, old: String, new: String) -> Result<(), String> {
    git::ok(&path, &["branch", "-m", &old, &new]).map(|_| ())
}

pub fn delete_branch(path: String, name: String, force: bool) -> Result<(), String> {
    git::ok(&path, &["branch", if force { "-D" } else { "-d" }, &name]).map(|_| ())
}

pub fn delete_remote_branch(path: String, remote: String, name: String, account: Option<String>) -> Result<(), String> {
        let env = git::auth_env(&path, Some(&remote), account.as_deref());
    git::ok_env(&path, &["push", &remote, "--delete", &name], &env).map(|_| ())
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpResult {
    pub ok: bool,
    pub conflict: bool,
    pub output: String,
}

pub fn op_result(path: &str, o: git::Out) -> Result<OpResult, String> {
    let output = format!("{}{}", o.text(), o.stderr);
    if o.ok {
        return Ok(OpResult { ok: true, conflict: false, output });
    }
    let c = git::ok(path, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
    if !c.trim().is_empty() {
        return Ok(OpResult { ok: false, conflict: true, output });
    }
    Err(git::clean_err(&o))
}

pub fn merge(path: String, branch: String, squash: bool, no_ff: bool) -> Result<OpResult, String> {
        let mut args: Vec<&str> = vec!["merge"];
    if squash {
        args.push("--squash");
    } else if no_ff {
        args.push("--no-ff");
    }
    args.push("--no-edit");
    args.push(&branch);
    let o = git::run_full(Some(&path), &args, &[("GIT_EDITOR".into(), "true".into())], None, true)?;
    op_result(&path, o)
}

pub fn rebase(path: String, onto: String) -> Result<OpResult, String> {
        let o = git::run_full(Some(&path), &["rebase", &onto], &[("GIT_EDITOR".into(), "true".into())], None, true)?;
    op_result(&path, o)
}

pub fn op_continue(path: String, kind: String) -> Result<OpResult, String> {
        let env = vec![("GIT_EDITOR".to_string(), "true".to_string())];
    let args: Vec<&str> = match kind.as_str() {
        "rebase" => vec!["rebase", "--continue"],
        "cherry-pick" => vec!["cherry-pick", "--continue"],
        "revert" => vec!["revert", "--continue"],
        _ => vec!["commit", "--no-edit"],
    };
    let o = git::run_full(Some(&path), &args, &env, None, true)?;
    op_result(&path, o)
}

pub fn op_abort(path: String, kind: String) -> Result<(), String> {
        let args: Vec<&str> = match kind.as_str() {
        "rebase" => vec!["rebase", "--abort"],
        "cherry-pick" => vec!["cherry-pick", "--abort"],
        "revert" => vec!["revert", "--abort"],
        _ => vec!["merge", "--abort"],
    };
    git::ok(&path, &args).map(|_| ())
}

pub fn op_skip(path: String) -> Result<OpResult, String> {
        let o = git::run_full(Some(&path), &["rebase", "--skip"], &[("GIT_EDITOR".into(), "true".into())], None, true)?;
    op_result(&path, o)
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Compare {
    pub ahead: u32,
    pub behind: u32,
}

pub fn compare(path: String, base: String, head: String) -> Result<Compare, String> {
        let spec = format!("{base}...{head}");
    let o = git::ok(&path, &["rev-list", "--left-right", "--count", &spec])?;
    let mut it = o.split_whitespace();
    let behind = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let ahead = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    Ok(Compare { ahead, behind })
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct StashEntry {
    pub name: String,
    pub sha: String,
    pub date: String,
    pub subject: String,
}

pub fn stash_list(path: String) -> Result<Vec<StashEntry>, String> {
        let o = git::ok(&path, &["stash", "list", "--format=%gd%x1f%H%x1f%at%x1f%gs"])?;
    Ok(o
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\x1f').collect();
            if f.len() < 4 {
                return None;
            }
            Some(StashEntry {
                name: f[0].into(),
                sha: f[1].into(),
                date: f[2].into(),
                subject: f[3].into(),
            })
        })
        .collect())
}

pub fn stash_push(path: String, message: String, include_untracked: bool) -> Result<(), String> {
        let mut args: Vec<&str> = vec!["stash", "push"];
    if include_untracked {
        args.push("-u");
    }
    if !message.is_empty() {
        args.push("-m");
        args.push(&message);
    }
    let o = git::run(&path, &args)?;
    if !o.ok {
        return Err(git::clean_err(&o));
    }
    if o.text().contains("No local changes") {
        return Err("Stash'lenecek değişiklik yok".into());
    }
    Ok(())
}

pub fn stash_op(path: String, name: String, op: String) -> Result<OpResult, String> {
        let o = git::run(&path, &["stash", &op, &name])?;
    op_result(&path, o)
}

pub fn stash_files(path: String, name: String) -> Result<Vec<FileChange>, String> {
        let a = format!("{name}^1");
    let mut files = parse_name_status(&git::ok(&path, &["diff", "--name-status", "-M", "-z", &a, &name])?);
    let third = format!("{name}^3");
    if git::run(&path, &["rev-parse", "--verify", "-q", &third]).map(|o| o.ok).unwrap_or(false) {
        let o = git::ok(&path, &["ls-tree", "-r", "--name-only", "-z", &third])?;
        for p in o.split('\0').filter(|s| !s.is_empty()) {
            files.push(FileChange { path: p.into(), orig_path: None, status: "A".into(), added: 0, deleted: 0 });
        }
    }
    Ok(files)
}

pub fn stash_diff(path: String, name: String, file: String) -> Result<String, String> {
        let a = format!("{name}^1");
    let d = git::ok(&path, &["diff", "--no-color", &a, &name, "--", &file])?;
    if !d.trim().is_empty() {
        return Ok(d);
    }
    let third = format!("{name}^3");
    git::ok(&path, &["diff", "--no-color", EMPTY_TREE, &third, "--", &file])
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub sha: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    pub date: String,
    pub refs: Vec<String>,
    pub subject: String,
    pub body: String,
}

pub fn log(
    path: String,
    rev: Option<String>,
    skip: u32,
    limit: u32,
    query: Option<String>,
    file: Option<String>,
) -> Result<Vec<Commit>, String> {
        if !has_head(&path) {
        return Ok(vec![]);
    }
    let skip_s = format!("--skip={skip}");
    let lim = format!("-n{limit}");
    let mut args: Vec<String> = vec![
        "log".into(),
        "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%D%x1f%s%x1f%b%x1e".into(),
        skip_s,
        lim,
        "--decorate=short".into(),
    ];
    if let Some(q) = query.as_deref().filter(|q| !q.trim().is_empty()) {
        args.push(format!("--grep={q}"));
        args.push("-i".into());
    }
    args.push(rev.unwrap_or_else(|| "HEAD".into()));
    if let Some(f) = file.as_deref().filter(|f| !f.is_empty()) {
        args.push("--follow".into());
        args.push("--".into());
        args.push(f.into());
    }
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let o = git::ok(&path, &refs)?;
    let mut out = vec![];
    for rec in o.split('\x1e') {
        let rec = rec.trim_start_matches('\n');
        if rec.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = rec.splitn(8, '\x1f').collect();
        if f.len() < 8 {
            continue;
        }
        out.push(Commit {
            sha: f[0].into(),
            parents: f[1].split_whitespace().map(|s| s.to_string()).collect(),
            author: f[2].into(),
            email: f[3].into(),
            date: f[4].into(),
            refs: f[5].split(", ").filter(|s| !s.is_empty()).map(|s| s.to_string()).collect(),
            subject: f[6].into(),
            body: f[7].trim().into(),
        });
    }
    Ok(out)
}

pub fn unpushed(path: String) -> Result<Vec<String>, String> {
        let o = git::run(&path, &["rev-list", "@{upstream}..HEAD"])?;
    if !o.ok {
        return Ok(vec![]);
    }
    Ok(o.text().lines().map(|s| s.to_string()).collect())
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub orig_path: Option<String>,
    pub status: String,
    pub added: i64,
    pub deleted: i64,
}

fn parse_name_status(s: &str) -> Vec<FileChange> {
    let mut v = vec![];
    let parts: Vec<&str> = s.split('\0').collect();
    let mut i = 0;
    while i < parts.len() {
        let st = parts[i];
        i += 1;
        if st.is_empty() {
            continue;
        }
        let c = st.chars().next().unwrap();
        if c == 'R' || c == 'C' {
            if i + 1 < parts.len() {
                v.push(FileChange {
                    path: parts[i + 1].into(),
                    orig_path: Some(parts[i].into()),
                    status: c.to_string(),
                    added: 0,
                    deleted: 0,
                });
            }
            i += 2;
        } else {
            if i < parts.len() {
                v.push(FileChange { path: parts[i].into(), orig_path: None, status: c.to_string(), added: 0, deleted: 0 });
            }
            i += 1;
        }
    }
    v
}

fn commit_base(path: &str, sha: &str) -> String {
    let spec = format!("{sha}^1");
    match git::run(path, &["rev-parse", "--verify", "-q", &spec]) {
        Ok(o) if o.ok => spec,
        _ => EMPTY_TREE.to_string(),
    }
}

pub fn commit_files(path: String, sha: String) -> Result<Vec<FileChange>, String> {
        let base = commit_base(&path, &sha);
    let ns = git::ok(&path, &["diff", "--name-status", "-M", "-z", &base, &sha])?;
    let mut files = parse_name_status(&ns);
    if let Ok(num) = git::ok(&path, &["diff", "--numstat", "-M", "-z", &base, &sha]) {
        let parts: Vec<&str> = num.split('\0').collect();
        let mut i = 0;
        while i < parts.len() {
            let rec = parts[i];
            i += 1;
            if rec.is_empty() {
                continue;
            }
            let f: Vec<&str> = rec.splitn(3, '\t').collect();
            if f.len() < 3 {
                continue;
            }
            let a = f[0].parse::<i64>().unwrap_or(0);
            let d = f[1].parse::<i64>().unwrap_or(0);
            let name = if f[2].is_empty() {
                let n = parts.get(i + 1).copied().unwrap_or("");
                i += 2;
                n
            } else {
                f[2]
            };
            if let Some(fc) = files.iter_mut().find(|x| x.path == name) {
                fc.added = a;
                fc.deleted = d;
            }
        }
    }
    Ok(files)
}

pub fn commit_diff(path: String, sha: String, file: String, orig: Option<String>) -> Result<String, String> {
        let base = commit_base(&path, &sha);
    let mut args: Vec<&str> = vec!["diff", "--no-color", "--no-ext-diff", "-M", &base, &sha, "--"];
    if let Some(o) = orig.as_deref() {
        args.push(o);
    }
    args.push(&file);
    git::ok(&path, &args)
}

pub fn cherry_pick(path: String, sha: String) -> Result<OpResult, String> {
        let o = git::run_full(
        Some(&path),
        &["cherry-pick", &sha],
        &[("GIT_EDITOR".into(), "true".into())],
        None,
        true,
    )?;
    op_result(&path, o)
}

pub fn revert_commit(path: String, sha: String) -> Result<OpResult, String> {
        let parents = git::ok(&path, &["rev-list", "--parents", "-n", "1", &sha])?;
    let mut args: Vec<&str> = vec!["revert", "--no-edit"];
    if parents.split_whitespace().count() > 2 {
        args.push("-m");
        args.push("1");
    }
    args.push(&sha);
    let o = git::run_full(Some(&path), &args, &[("GIT_EDITOR".into(), "true".into())], None, true)?;
    op_result(&path, o)
}

pub fn reset_to(path: String, sha: String, mode: String) -> Result<(), String> {
        let m = match mode.as_str() {
        "hard" => "--hard",
        "mixed" => "--mixed",
        _ => "--soft",
    };
    git::ok(&path, &["reset", m, &sha]).map(|_| ())
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    pub action: String,
    pub sha: String,
}

/// Reorder / squash / drop / reword commits by driving a non-interactive rebase.
pub fn rewrite_history(
    path: String,
    base: String,
    todo: Vec<TodoItem>,
    message: Option<String>,
) -> Result<OpResult, String> {
        let dir = std::env::temp_dir().join(format!("ferro-rebase-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let todo_file = dir.join("todo.txt");
    let msg_file = dir.join("msg.txt");
    let mut t = String::new();
    for i in &todo {
        let act = match i.action.as_str() {
            "squash" => "squash",
            "fixup" => "fixup",
            "drop" => "drop",
            _ => "pick",
        };
        t.push_str(&format!("{act} {}\n", i.sha));
    }
    std::fs::write(&todo_file, t).map_err(|e| e.to_string())?;
    let fwd = |p: &std::path::Path| p.to_string_lossy().replace('\\', "/");
    let seq = format!("cp \"{}\"", fwd(&todo_file));
    let mut env = vec![("GIT_SEQUENCE_EDITOR".to_string(), seq)];
    if let Some(m) = &message {
        std::fs::write(&msg_file, m).map_err(|e| e.to_string())?;
        env.push(("GIT_EDITOR".into(), format!("cp \"{}\"", fwd(&msg_file))));
    } else {
        env.push(("GIT_EDITOR".into(), "true".into()));
    }
    let args: Vec<&str> = if base.is_empty() {
        vec!["rebase", "-i", "--root"]
    } else {
        vec!["rebase", "-i", &base]
    };
    let o = git::run_full(Some(&path), &args, &env, None, true)?;
    let _ = std::fs::remove_dir_all(&dir);
    op_result(&path, o)
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub name: String,
    pub sha: String,
    pub date: String,
    pub subject: String,
}

pub fn tags(path: String) -> Result<Vec<Tag>, String> {
        let fmt = "--format=%(refname:short)%00%(objectname)%00%(*objectname)%00%(creatordate:unix)%00%(contents:subject)";
    let o = git::ok(&path, &["for-each-ref", "--sort=-creatordate", fmt, "refs/tags"])?;
    Ok(o
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\0').collect();
            if f.len() < 5 {
                return None;
            }
            Some(Tag {
                name: f[0].into(),
                sha: if f[2].is_empty() { f[1].into() } else { f[2].into() },
                date: f[3].into(),
                subject: f[4].into(),
            })
        })
        .collect())
}

pub fn create_tag(path: String, name: String, sha: Option<String>, message: Option<String>) -> Result<(), String> {
        let mut args: Vec<&str> = vec!["tag"];
    if let Some(m) = message.as_deref().filter(|m| !m.trim().is_empty()) {
        args.push("-a");
        args.push("-m");
        args.push(m);
    }
    args.push(&name);
    if let Some(s) = sha.as_deref() {
        args.push(s);
    }
    git::ok(&path, &args).map(|_| ())
}

pub fn delete_tag(path: String, name: String) -> Result<(), String> {
    git::ok(&path, &["tag", "-d", &name]).map(|_| ())
}

pub fn config_get(path: Option<String>, key: String, global: bool) -> Result<String, String> {
        let mut args: Vec<&str> = vec!["config"];
    if global {
        args.push("--global");
    }
    args.push("--get");
    args.push(&key);
    let o = match &path {
        Some(p) if !global => git::run(p, &args)?,
        _ => git::run_full(None, &args, &[], None, false)?,
    };
    Ok(o.text().trim().to_string())
}

pub fn config_set(path: Option<String>, key: String, value: String, global: bool) -> Result<(), String> {
        let mut args: Vec<&str> = vec!["config"];
    if global {
        args.push("--global");
    }
    args.push(&key);
    args.push(&value);
    let o = match &path {
        Some(p) if !global => git::run(p, &args)?,
        _ => git::run_full(None, &args, &[], None, false)?,
    };
    if o.ok {
        Ok(())
    } else {
        Err(git::clean_err(&o))
    }
}

pub fn git_version() -> Result<String, String> {
        let o = git::run_full(None, &["--version"], &[], None, false)?;
    Ok(o.text().trim().to_string())
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepoMeta {
    pub last_fetch: Option<u64>,
    pub latest_tag: Option<String>,
    pub commit_count: u64,
}

pub fn repo_meta(path: String) -> Result<RepoMeta, String> {
        let gd = git::ok(&path, &["rev-parse", "--absolute-git-dir"]).unwrap_or_default();
    let fetch_head = std::path::Path::new(gd.trim()).join("FETCH_HEAD");
    let last_fetch = std::fs::metadata(&fetch_head)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);
    let latest_tag = git::run(&path, &["describe", "--tags", "--abbrev=0"])
        .ok()
        .filter(|o| o.ok)
        .map(|o| o.text().trim().to_string());
    let commit_count = git::run(&path, &["rev-list", "--count", "HEAD"])
        .ok()
        .filter(|o| o.ok)
        .and_then(|o| o.text().trim().parse().ok())
        .unwrap_or(0);
    Ok(RepoMeta { last_fetch, latest_tag, commit_count })
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Contributor {
    pub name: String,
    pub email: String,
    pub commits: u64,
}

pub fn contributors(path: String) -> Result<Vec<Contributor>, String> {
        let o = git::run(&path, &["shortlog", "-sne", "HEAD"])?;
    if !o.ok {
        return Ok(vec![]);
    }
    Ok(o.text()
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let (n, rest) = l.split_once('\t')?;
            let commits = n.trim().parse().ok()?;
            let (name, email) = rest.rsplit_once(" <")?;
            Some(Contributor { name: name.trim().into(), email: email.trim_end_matches('>').into(), commits })
        })
        .collect())
}
