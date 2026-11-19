//! Core git process runner: logging, progress streaming and credential injection.

use serde::Serialize;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Instant;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub struct Out {
    pub ok: bool,
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Out {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).to_string()
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: u64,
    pub args: String,
    pub cwd: String,
    pub ok: bool,
    pub code: i32,
    pub ms: u128,
    pub error: String,
    pub at: u64,
}

static LOG: Mutex<Vec<LogEntry>> = Mutex::new(Vec::new());
static COUNTER: Mutex<u64> = Mutex::new(0);

fn push_log(args: &[String], cwd: &str, ok: bool, code: i32, ms: u128, err: &str) {
    let mut c = COUNTER.lock().unwrap();
    *c += 1;
    let id = *c;
    drop(c);
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut l = LOG.lock().unwrap();
    l.push(LogEntry {
        id,
        args: redact(&args.join(" ")),
        cwd: cwd.to_string(),
        ok,
        code,
        ms,
        error: err.chars().take(600).collect(),
        at,
    });
    let len = l.len();
    if len > 400 {
        l.drain(0..len - 400);
    }
}

fn redact(s: &str) -> String {
    // Never leak credentials embedded in URLs into the activity log.
    let mut out = String::new();
    for part in s.split(' ') {
        let p = if let (Some(a), Some(b)) = (part.find("://"), part.find('@')) {
            if b > a + 3 && !part[a + 3..b].contains('/') {
                format!("{}://***@{}", &part[..a], &part[b + 1..])
            } else {
                part.to_string()
            }
        } else {
            part.to_string()
        };
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&p);
    }
    out
}

pub fn activity_log() -> Vec<LogEntry> {
    LOG.lock().unwrap().clone()
}

pub fn base_cmd(cwd: Option<&str>) -> Command {
    let mut c = Command::new("git");
    #[cfg(windows)]
    c.creation_flags(0x08000000);
    c.args([
        "-c", "core.quotepath=false", "-c", "color.ui=false", "-c", "core.safecrlf=false",
        "-c", "http.lowSpeedLimit=1000", "-c", "http.lowSpeedTime=45",
    ]);
    c.env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("LC_MESSAGES", "C")
        .env("LANGUAGE", "C")
        .env("GIT_LITERAL_PATHSPECS", "1")
        .env("GIT_OPTIONAL_LOCKS", "0");
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    c
}

pub fn run_full(
    cwd: Option<&str>,
    args: &[&str],
    env: &[(String, String)],
    stdin: Option<&[u8]>,
    optional_locks: bool,
) -> Result<Out, String> {
    let start = Instant::now();
    let mut c = base_cmd(cwd);
    if optional_locks {
        c.env_remove("GIT_OPTIONAL_LOCKS");
    }
    c.args(args);
    for (k, v) in env {
        c.env(k, v);
    }
    c.stdout(Stdio::piped()).stderr(Stdio::piped());
    c.stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() });
    let mut child = c
        .spawn()
        .map_err(|e| format!("Git çalıştırılamadı (kurulu mu?): {e}"))?;
    if let Some(data) = stdin {
        use std::io::Write;
        if let Some(mut si) = child.stdin.take() {
            let data = data.to_vec();
            std::thread::spawn(move || {
                let _ = si.write_all(&data);
            });
        }
    }
    let o = child.wait_with_output().map_err(|e| e.to_string())?;
    let out = Out {
        ok: o.status.success(),
        code: o.status.code().unwrap_or(-1),
        stdout: o.stdout,
        stderr: String::from_utf8_lossy(&o.stderr).to_string(),
    };
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    push_log(&argv, cwd.unwrap_or(""), out.ok, out.code, start.elapsed().as_millis(), &out.stderr);
    Ok(out)
}

pub fn run(cwd: &str, args: &[&str]) -> Result<Out, String> {
    run_full(Some(cwd), args, &[], None, false)
}

/// Runs git and returns stdout, or stderr as an error.
pub fn ok(cwd: &str, args: &[&str]) -> Result<String, String> {
    let o = run(cwd, args)?;
    if o.ok {
        Ok(o.text())
    } else {
        Err(clean_err(&o))
    }
}

pub fn ok_env(cwd: &str, args: &[&str], env: &[(String, String)]) -> Result<String, String> {
    let o = run_full(Some(cwd), args, env, None, false)?;
    if o.ok {
        Ok(o.text())
    } else {
        Err(clean_err(&o))
    }
}

pub fn clean_err(o: &Out) -> String {
    let e = o.stderr.trim();
    if e.is_empty() {
        let t = o.text();
        if t.trim().is_empty() {
            format!("git {} koduyla sonlandı", o.code)
        } else {
            t.trim().to_string()
        }
    } else {
        e.to_string()
    }
}

/// Progress sink: receives raw stderr progress lines of long running git commands.
pub type Progress = std::sync::Arc<dyn Fn(String) + Send + Sync>;

pub fn no_progress() -> Progress {
    std::sync::Arc::new(|_| {})
}

/// Streams stderr progress lines (fetch/push/clone) to `progress`.
pub fn run_stream(
    progress: &Progress,
    cwd: Option<&str>,
    args: &[&str],
    env: &[(String, String)],
) -> Result<Out, String> {
    let start = Instant::now();
    let mut c = base_cmd(cwd);
    c.args(args);
    for (k, v) in env {
        c.env(k, v);
    }
    c.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
    let mut child = c
        .spawn()
        .map_err(|e| format!("Git çalıştırılamadı (kurulu mu?): {e}"))?;
    let mut so = child.stdout.take().unwrap();
    let th = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = so.read_to_end(&mut v);
        v
    });
    let mut se = child.stderr.take().unwrap();
    let mut buf = [0u8; 4096];
    let mut line: Vec<u8> = Vec::new();
    let mut all = String::new();
    loop {
        let n = match se.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        for &b in &buf[..n] {
            if b == b'\r' || b == b'\n' {
                if !line.is_empty() {
                    let s = String::from_utf8_lossy(&line).to_string();
                    progress(s.clone());
                    all.push_str(&s);
                    all.push('\n');
                    line.clear();
                }
            } else {
                line.push(b);
            }
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let stdout = th.join().unwrap_or_default();
    let out = Out { ok: status.success(), code: status.code().unwrap_or(-1), stdout, stderr: all };
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    push_log(&argv, cwd.unwrap_or(""), out.ok, out.code, start.elapsed().as_millis(), &out.stderr);
    Ok(out)
}

const KEYRING_SERVICE: &str = "ferro-git";

/// Accounts are identified by "host|login". Tokens come from Git Credential Manager (the same
/// store `git` itself uses), or from the OS vault for accounts signed in with a custom OAuth app.
pub fn token_for(key: &str) -> Option<String> {
    if let Some(t) = keyring::Entry::new(KEYRING_SERVICE, key).ok().and_then(|e| e.get_password().ok()) {
        return Some(t);
    }
    let (host, login) = key.split_once('|')?;
    gcm_token(host, login)
}

pub fn set_token(key: &str, t: &str) -> Result<(), String> {
    keyring::Entry::new(KEYRING_SERVICE, key)
        .map_err(|e| e.to_string())?
        .set_password(t)
        .map_err(|e| e.to_string())
}

pub fn clear_token(key: &str) {
    if let Ok(e) = keyring::Entry::new(KEYRING_SERVICE, key) {
        let _ = e.delete_credential();
    }
    forget_cached(key);
}

type TokenCache = std::sync::Mutex<Vec<(String, std::time::Instant, String)>>;
static TOKEN_CACHE: TokenCache = std::sync::Mutex::new(Vec::new());

pub fn forget_cached(key: &str) {
    if let Ok(mut c) = TOKEN_CACHE.lock() {
        c.retain(|(k, _, _)| k != key);
    }
}

fn plain_git(args: &[&str]) -> Command {
    let mut c = Command::new("git");
    #[cfg(windows)]
    c.creation_flags(0x08000000);
    c.args(args).env("GIT_TERMINAL_PROMPT", "0").env("GCM_INTERACTIVE", "never");
    c
}

/// Asks the credential helper (Git Credential Manager) for the stored token of an account, silently.
fn gcm_token(host: &str, login: &str) -> Option<String> {
    use std::io::Write;
    use std::process::Stdio;
    let key = format!("{host}|{login}");
    if let Ok(c) = TOKEN_CACHE.lock() {
        if let Some((_, at, t)) = c.iter().find(|(k, _, _)| *k == key) {
            if at.elapsed().as_secs() < 600 {
                return Some(t.clone());
            }
        }
    }
    let mut child = plain_git(&["credential", "fill"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    child.stdin.take()?.write_all(format!("protocol=https\nhost={host}\nusername={login}\n\n").as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let token = text.lines().find_map(|l| l.strip_prefix("password=")).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())?;
    if let Ok(mut c) = TOKEN_CACHE.lock() {
        c.retain(|(k, _, _)| *k != key);
        c.push((key, std::time::Instant::now(), token.clone()));
    }
    Some(token)
}

fn gcm_url_args(host: &str) -> Vec<String> {
    if host == "github.com" { vec![] } else { vec!["--url".into(), format!("https://{host}")] }
}

/// GitHub accounts known to Git Credential Manager (usernames).
pub fn gcm_accounts(host: &str) -> Vec<String> {
    let mut args: Vec<String> = vec!["credential-manager".into(), "github".into(), "list".into()];
    args.extend(gcm_url_args(host));
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match plain_git(&refs).output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect(),
        _ => vec![],
    }
}

/// Browser sign-in through Git Credential Manager. Blocks until the user finishes (or `pid_slot` is killed).
pub fn gcm_login(host: &str, pid_slot: &std::sync::Mutex<Option<u32>>) -> Result<(), String> {
    use std::process::Stdio;
    let mut args: Vec<String> = vec!["credential-manager".into(), "github".into(), "login".into(), "--browser".into(), "--force".into()];
    args.extend(gcm_url_args(host));
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let mut cmd = plain_git(&refs);
    cmd.env_remove("GCM_INTERACTIVE");
    let child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "Git Credential Manager bulunamadı. Git for Windows'u “Git Credential Manager” seçeneğiyle kur.".to_string())?;
    if let Ok(mut s) = pid_slot.lock() {
        *s = Some(child.id());
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        let msg = String::from_utf8_lossy(&out.stderr).lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
        Err(if msg.is_empty() { "Giriş tamamlanamadı".to_string() } else { msg })
    }
}

pub fn host_of(key: &str) -> &str {
    key.split('|').next().unwrap_or("github.com")
}

/// Environment that makes git authenticate against the account's host using its token,
/// without ever putting the secret on a command line.
pub fn auth_env_acc(account: Option<&str>, url: &str) -> Vec<(String, String)> {
    use base64::Engine;
    let Some(key) = account else { return vec![] };
    let host = host_of(key);
    if !url.contains(host) {
        return vec![];
    }
    match token_for(key) {
        Some(t) => {
            let b = base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{t}"));
            vec![
                ("GIT_CONFIG_COUNT".into(), "1".into()),
                ("GIT_CONFIG_KEY_0".into(), format!("http.https://{host}/.extraheader")),
                ("GIT_CONFIG_VALUE_0".into(), format!("AUTHORIZATION: basic {b}")),
            ]
        }
        None => vec![],
    }
}

pub fn remote_url(cwd: &str, remote: Option<&str>) -> String {
    if let Some(r) = remote {
        let key = format!("remote.{r}.url");
        if let Ok(o) = run(cwd, &["config", "--get", &key]) {
            if o.ok {
                return o.text().trim().to_string();
            }
        }
    }
    for r in ["origin", ""] {
        if r.is_empty() {
            if let Ok(o) = run(cwd, &["remote"]) {
                if let Some(first) = o.text().lines().next() {
                    let key = format!("remote.{first}.url");
                    if let Ok(o2) = run(cwd, &["config", "--get", &key]) {
                        return o2.text().trim().to_string();
                    }
                }
            }
        } else if let Ok(o) = run(cwd, &["config", "--get", "remote.origin.url"]) {
            if o.ok {
                return o.text().trim().to_string();
            }
        }
    }
    String::new()
}

pub fn auth_env(cwd: &str, remote: Option<&str>, account: Option<&str>) -> Vec<(String, String)> {
    auth_env_acc(account, &remote_url(cwd, remote))
}
