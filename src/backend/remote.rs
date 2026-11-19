//! Network operations (fetch / pull / push / clone) with live progress streaming.

use super::git;
use super::repo::{op_result, OpResult};
use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Remote {
    pub name: String,
    pub url: String,
}

pub fn remotes(path: String) -> Result<Vec<Remote>, String> {
        let o = git::ok(&path, &["remote", "-v"])?;
    let mut out: Vec<Remote> = vec![];
    for l in o.lines() {
        let mut it = l.split_whitespace();
        if let (Some(n), Some(u)) = (it.next(), it.next()) {
            if !out.iter().any(|r| r.name == n) {
                out.push(Remote { name: n.into(), url: u.into() });
            }
        }
    }
    Ok(out)
}

pub fn set_remote(path: String, name: String, url: String) -> Result<(), String> {
        let exists = git::run(&path, &["remote", "get-url", &name]).map(|o| o.ok).unwrap_or(false);
    let args: Vec<&str> = if exists {
        vec!["remote", "set-url", &name, &url]
    } else {
        vec!["remote", "add", &name, &url]
    };
    git::ok(&path, &args).map(|_| ())
}

pub fn remove_remote(path: String, name: String) -> Result<(), String> {
    git::ok(&path, &["remote", "remove", &name]).map(|_| ())
}

pub fn fetch(progress: &git::Progress, path: String, remote: Option<String>, account: Option<String>) -> Result<(), String> {
        let env = git::auth_env(&path, remote.as_deref(), account.as_deref());
    let mut args: Vec<&str> = vec!["fetch", "--prune", "--progress"];
    match remote.as_deref() {
        Some(r) => args.push(r),
        None => args.push("--all"),
    }
    let o = git::run_stream(progress, Some(&path), &args, &env)?;
    if o.ok {
        Ok(())
    } else {
        Err(git::clean_err(&o))
    }
}

pub fn pull(progress: &git::Progress, path: String, mode: String, account: Option<String>) -> Result<OpResult, String> {
        let mut env = git::auth_env(&path, None, account.as_deref());
    env.push(("GIT_EDITOR".into(), "true".into()));
    let flag = match mode.as_str() {
        "rebase" => "--rebase",
        "ff-only" => "--ff-only",
        _ => "--no-rebase",
    };
    let o = git::run_stream(progress, Some(&path), &["pull", "--progress", flag], &env)?;
    op_result(&path, o)
}

pub fn push(
    progress: &git::Progress,
    path: String,
    remote: Option<String>,
    branch: Option<String>,
    force: bool,
    set_upstream: bool,
    account: Option<String>,
) -> Result<(), String> {
        let env = git::auth_env(&path, remote.as_deref(), account.as_deref());
    let mut args: Vec<&str> = vec!["push", "--progress"];
    if force {
        args.push("--force-with-lease");
    }
    if set_upstream {
        args.push("-u");
    }
    if let Some(r) = remote.as_deref() {
        args.push(r);
    }
    if let Some(b) = branch.as_deref() {
        args.push(b);
    }
    let o = git::run_stream(progress, Some(&path), &args, &env)?;
    if o.ok {
        Ok(())
    } else {
        Err(git::clean_err(&o))
    }
}

pub fn push_tag(progress: &git::Progress, path: String, name: String, account: Option<String>) -> Result<(), String> {
        let env = git::auth_env(&path, None, account.as_deref());
    let o = git::run_stream(progress, Some(&path), &["push", "--progress", "origin", &name], &env)?;
    if o.ok {
        Ok(())
    } else {
        Err(git::clean_err(&o))
    }
}

pub fn clone_repo(
    progress: &git::Progress,
    url: String,
    dest: String,
    branch: Option<String>,
    account: Option<String>,
) -> Result<String, String> {
        let env = git::auth_env_acc(account.as_deref(), &url);
    let mut args: Vec<&str> = vec!["clone", "--progress"];
    if let Some(b) = branch.as_deref().filter(|b| !b.is_empty()) {
        args.push("--branch");
        args.push(b);
    }
    args.push(&url);
    args.push(&dest);
    let o = git::run_stream(progress, None, &args, &env)?;
    if o.ok {
        Ok(dest)
    } else {
        Err(git::clean_err(&o))
    }
}
