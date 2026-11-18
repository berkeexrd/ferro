//! Index (staging) operations: the real git index is the single source of truth for "hazır".

use super::git;

fn paths_input(paths: &[String]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in paths {
        v.extend_from_slice(p.as_bytes());
        v.push(0);
    }
    v
}

fn has_head(cwd: &str) -> bool {
    git::run(cwd, &["rev-parse", "--verify", "-q", "HEAD"]).map(|o| o.ok).unwrap_or(false)
}

/// Worktree vs index (unstaged changes). Untracked files are diffed against /dev/null.
pub fn diff_worktree(path: &str, file: &str, untracked: bool, ctx: u32) -> Result<String, String> {
    let u = format!("-U{ctx}");
    if untracked {
        let o = git::run(path, &["diff", "--no-color", "--no-ext-diff", &u, "--no-index", "--", "/dev/null", file])?;
        if o.code > 1 {
            return Err(git::clean_err(&o));
        }
        return Ok(o.text());
    }
    git::ok(path, &["diff", "--no-color", "--no-ext-diff", &u, "--", file])
}

/// Index vs HEAD (staged changes).
pub fn diff_index(path: &str, file: &str, orig: Option<&str>, ctx: u32) -> Result<String, String> {
    let u = format!("-U{ctx}");
    let mut args: Vec<&str> = vec!["diff", "--cached", "--no-color", "--no-ext-diff", "-M", &u, "--"];
    if let Some(o) = orig {
        args.push(o);
    }
    args.push(file);
    git::ok(path, &args)
}

pub fn stage_paths(path: &str, paths: &[String]) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }
    let data = paths_input(paths);
    let o = git::run_full(Some(path), &["add", "-A", "--pathspec-from-file=-", "--pathspec-file-nul"], &[], Some(&data), false)?;
    if o.ok { Ok(()) } else { Err(git::clean_err(&o)) }
}

pub fn unstage_paths(path: &str, paths: &[String]) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }
    let data = paths_input(paths);
    let args: Vec<&str> = if has_head(path) {
        vec!["restore", "--staged", "--pathspec-from-file=-", "--pathspec-file-nul"]
    } else {
        vec!["rm", "--cached", "-r", "-q", "--pathspec-from-file=-", "--pathspec-file-nul"]
    };
    let o = git::run_full(Some(path), &args, &[], Some(&data), false)?;
    if o.ok { Ok(()) } else { Err(git::clean_err(&o)) }
}

/// Apply a patch to the index. `reverse` removes the patch from the index (unstage hunk/lines).
pub fn apply_cached(path: &str, patch: &str, reverse: bool) -> Result<(), String> {
    let mut args: Vec<&str> = vec!["apply", "--cached", "--recount", "--whitespace=nowarn"];
    if reverse {
        args.push("-R");
    }
    args.push("-");
    let o = git::run_full(Some(path), &args, &[], Some(patch.as_bytes()), false)?;
    if o.ok { Ok(()) } else { Err(git::clean_err(&o)) }
}

/// Reverse-apply a patch to the working tree (discard a hunk).
pub fn discard_patch(path: &str, patch: &str) -> Result<(), String> {
    let o = git::run_full(Some(path), &["apply", "-R", "--recount", "--whitespace=nowarn", "-"], &[], Some(patch.as_bytes()), false)?;
    if o.ok { Ok(()) } else { Err(git::clean_err(&o)) }
}

pub fn commit_staged(path: &str, message: &str, amend: bool) -> Result<String, String> {
    let mut args: Vec<&str> = vec!["commit", "-F", "-"];
    if amend {
        args.push("--amend");
    }
    let o = git::run_full(Some(path), &args, &[], Some(message.as_bytes()), true)?;
    if !o.ok {
        return Err(git::clean_err(&o));
    }
    Ok(git::ok(path, &["rev-parse", "HEAD"])?.trim().to_string())
}

pub fn index_tree(path: &str) -> Result<String, String> {
    Ok(git::ok(path, &["write-tree"])?.trim().to_string())
}

pub fn restore_index(path: &str, tree: &str) -> Result<(), String> {
    git::ok(path, &["read-tree", tree]).map(|_| ())
}

pub fn head_sha(path: &str) -> Option<String> {
    git::run(path, &["rev-parse", "--verify", "-q", "HEAD"]).ok().filter(|o| o.ok).map(|o| o.text().trim().to_string())
}

/// (staged files, staged hunks)
pub fn staged_counts(path: &str) -> (usize, usize) {
    let Ok(o) = git::run(path, &["diff", "--cached", "--no-color", "--no-ext-diff", "-U0"]) else { return (0, 0) };
    let t = o.text();
    let files = t.lines().filter(|l| l.starts_with("diff --git ")).count();
    let hunks = t.lines().filter(|l| l.starts_with("@@ ")).count();
    (files, hunks)
}

/// Commits of `rev` filtered by a free-text query (message, hash or author), newest first.
pub fn search_commits(path: &str, query: &str, limit: u32) -> Result<Vec<super::repo::Commit>, String> {
    super::repo::log(path.to_string(), None, 0, limit, Some(query.to_string()), None)
}

/// Rename detection helper: the git command preview shown in the status bar.
pub fn last_log() -> Option<(String, u128, bool)> {
    const READ_ONLY: &[&str] = &["rev-parse", "config", "status", "diff", "log", "for-each-ref", "rev-list", "remote", "show", "cat-file", "ls-files", "symbolic-ref", "write-tree", "var", "--version", "stash list", "merge-base"];
    let log = git::activity_log();
    let is_ro = |args: &str| {
        let mut it = args.split_whitespace().skip_while(|w| *w == "-c" || w.starts_with("core.") || w.starts_with("--"));
        let first = it.next().unwrap_or("");
        READ_ONLY.contains(&first) || (first == "stash" && it.next() == Some("list"))
    };
    log.iter().rev().find(|e| !is_ro(&e.args)).or(log.last()).map(|e| (e.args.clone(), e.ms, e.ok))
}

/// Per-file +/- counts against HEAD (index + worktree combined).
pub fn numstat(path: &str) -> std::collections::HashMap<String, (i64, i64)> {
    let mut m = std::collections::HashMap::new();
    let has_head = git::run(path, &["rev-parse", "--verify", "-q", "HEAD"]).map(|o| o.ok).unwrap_or(false);
    let args: Vec<&str> = if has_head { vec!["-c", "core.quotepath=false", "diff", "HEAD", "--numstat"] } else { vec!["-c", "core.quotepath=false", "diff", "--cached", "--numstat"] };
    if let Ok(out) = git::ok(path, &args) {
        for l in out.lines() {
            let mut it = l.splitn(3, '\t');
            let (a, d, f) = (it.next().unwrap_or("-"), it.next().unwrap_or("-"), it.next().unwrap_or(""));
            let f = match f.rsplit_once(" => ") {
                Some((_, to)) => to.trim_end_matches('}').to_string(),
                None => f.to_string(),
            };
            m.insert(f, (a.parse().unwrap_or(0), d.parse().unwrap_or(0)));
        }
    }
    m
}
