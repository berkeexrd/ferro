//! GitHub (and GitHub Enterprise) access with multiple accounts.
//! Accounts are keyed "host|login"; tokens live only in the OS credential vault.

use super::git;
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::time::Duration;

/// One shared agent: keeps TLS connections alive (no handshake per request) and never hangs forever.
fn agent() -> &'static ureq::Agent {
    static A: OnceLock<ureq::Agent> = OnceLock::new();
    A.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout(Duration::from_secs(40))
            .user_agent("Ferro")
            .build()
    })
}

fn api_base(host: &str) -> String {
    if host == "github.com" {
        "https://api.github.com".to_string()
    } else {
        format!("https://{host}/api/v3")
    }
}

fn graphql_url(host: &str) -> String {
    if host == "github.com" {
        "https://api.github.com/graphql".to_string()
    } else {
        format!("https://{host}/api/graphql")
    }
}

fn send(method: &str, url: &str, token: &str, body: Option<Value>) -> Result<(Value, Option<String>), String> {
    let req = agent().request(method, url)
        .set("Authorization", &format!("Bearer {token}"))
        .set("User-Agent", "Ferro")
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28");
    let res = match body {
        Some(b) => req.send_json(b),
        None => req.call(),
    };
    match res {
        Ok(r) => {
            let link = r.header("link").map(|s| s.to_string());
            let v = if r.status() == 204 { Value::Null } else { r.into_json::<Value>().unwrap_or(Value::Null) };
            Ok((v, link))
        }
        Err(ureq::Error::Status(code, r)) => {
            let v: Value = r.into_json().unwrap_or(Value::Null);
            let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("istek başarısız");
            let detail = v
                .get("errors")
                .and_then(|e| e.as_array())
                .and_then(|a| a.first())
                .and_then(|e| e.get("message").and_then(|m| m.as_str()))
                .unwrap_or("");
            Err(if code == 401 {
                "401: Kimlik doğrulama başarısız. Erişim anahtarı geçersiz ya da süresi dolmuş.".to_string()
            } else if detail.is_empty() {
                format!("GitHub {code}: {msg}")
            } else {
                format!("GitHub {code}: {msg} ({detail})")
            })
        }
        Err(e) => Err(format!("Ağ hatası: {e}")),
    }
}

fn token(account: &str) -> Result<String, String> {
    git::token_for(account).ok_or_else(|| "Bu hesabın erişim anahtarı bulunamadı. Yeniden giriş yapın.".to_string())
}

fn user_json(host: &str, v: &Value) -> Value {
    let login = v["login"].as_str().unwrap_or("");
    json!({
        "key": format!("{host}|{login}"),
        "host": host,
        "id": v["id"],
        "login": login,
        "name": v["name"],
        "email": v["email"],
        "avatarUrl": v["avatar_url"],
        "htmlUrl": v["html_url"],
    })
}

/// Validates a token, stores it in the vault and returns the account description.
pub fn gh_login(host: String, token: String) -> Result<Value, String> {
        let t = token.trim().to_string();
    let host = host.trim().trim_start_matches("https://").trim_end_matches('/').to_string();
    let (u, _) = send("GET", &format!("{}/user", api_base(&host)), &t, None)?;
    let acc = user_json(&host, &u);
    git::set_token(acc["key"].as_str().unwrap_or(""), &t)?;
    Ok(acc)
}

/// Profile for a token that is stored elsewhere (Git Credential Manager); nothing is saved.
pub fn gh_whoami(host: String, token: String) -> Result<Value, String> {
    let (u, _) = send("GET", &format!("{}/user", api_base(&host)), token.trim(), None)?;
    Ok(user_json(&host, &u))
}

/// Re-reads the profile of a stored account (also verifies that the token still works).
pub fn gh_user(account: String) -> Result<Value, String> {
        let t = token(&account)?;
    let host = git::host_of(&account).to_string();
    let (u, _) = send("GET", &format!("{}/user", api_base(&host)), &t, None)?;
    Ok(user_json(&host, &u))
}

pub fn gh_logout(account: String) -> Result<(), String> {
    git::clear_token(&account);
    Ok(())
}

/// Generic REST call. `pages` > 1 follows `Link: rel="next"` and concatenates array results.
pub fn gh_api(account: String, method: String, path: String, body: Option<Value>, pages: Option<u32>) -> Result<Value, String> {
        let t = token(&account)?;
    let host = git::host_of(&account).to_string();
    let base = api_base(&host);
    let mut url = if path.starts_with("http") { path.clone() } else { format!("{base}{path}") };
    let max = pages.unwrap_or(1).max(1);
    let (first, mut link) = send(&method, &url, &t, body)?;
    if max == 1 {
        return Ok(first);
    }
    // search results wrap arrays in {items: []}
    let mut out: Vec<Value> = match &first {
        Value::Array(a) => a.clone(),
        v if v.get("items").is_some() => v["items"].as_array().cloned().unwrap_or_default(),
        other => return Ok(other.clone()),
    };
    let wrapped = !first.is_array();
    let total = first.get("total_count").cloned();
    for _ in 1..max {
        let Some(l) = link.clone() else { break };
        let next = l.split(',').find(|p| p.contains("rel=\"next\"")).and_then(|p| {
            let s = p.find('<')?;
            let e = p.find('>')?;
            Some(p[s + 1..e].to_string())
        });
        let Some(n) = next else { break };
        url = n;
        let (v, l2) = send("GET", &url, &t, None)?;
        link = l2;
        match v {
            Value::Array(a) => out.extend(a),
            v2 if v2.get("items").is_some() => out.extend(v2["items"].as_array().cloned().unwrap_or_default()),
            _ => break,
        }
    }
    if wrapped {
        Ok(json!({ "items": out, "total_count": total }))
    } else {
        Ok(Value::Array(out))
    }
}

pub fn gh_graphql(account: String, query: String) -> Result<Value, String> {
        let t = token(&account)?;
    let host = git::host_of(&account).to_string();
    let (v, _) = send("POST", &graphql_url(&host), &t, Some(json!({ "query": query })))?;
    if let Some(errs) = v.get("errors").and_then(|e| e.as_array()) {
        if v.get("data").map(|d| d.is_null()).unwrap_or(true) {
            let m = errs.first().and_then(|e| e["message"].as_str()).unwrap_or("GraphQL hatası");
            return Err(m.to_string());
        }
    }
    Ok(v["data"].clone())
}

// No client secret and no redirect URI are needed: the user approves in the browser.

/// `repo` covers private clone / push / pull requests; `workflow` is needed to push workflow files.
const SCOPES: &str = "repo workflow";

fn norm_host(host: &str) -> String {
    host.trim().trim_start_matches("https://").trim_end_matches('/').to_string()
}

fn post_form(url: &str, form: &[(&str, &str)]) -> Result<Value, String> {
    match agent()
        .post(url)
        .set("Accept", "application/json")
        .send_form(form)
    {
        Ok(r) => r.into_json::<Value>().map_err(|e| e.to_string()),
        Err(ureq::Error::Status(code, r)) => {
            let v: Value = r.into_json().unwrap_or(Value::Null);
            let m = v["error_description"].as_str().or(v["message"].as_str()).unwrap_or("istek başarısız");
            Err(format!("GitHub {code}: {m}"))
        }
        Err(e) => Err(format!("Ağ hatası: {e}")),
    }
}

pub fn gh_device_start(host: String, client_id: String) -> Result<Value, String> {
        let host = norm_host(&host);
    let v = post_form(
        &format!("https://{host}/login/device/code"),
        &[("client_id", client_id.trim()), ("scope", SCOPES)],
    )?;
    if let Some(e) = v["error"].as_str() {
        let hint = if e == "device_flow_disabled" {
            "Bu OAuth uygulamasında “Enable Device Flow” seçeneği kapalı."
        } else {
            v["error_description"].as_str().unwrap_or(e)
        };
        return Err(hint.to_string());
    }
    Ok(json!({
        "deviceCode": v["device_code"],
        "userCode": v["user_code"],
        "verificationUri": v["verification_uri"],
        "interval": v["interval"].as_u64().unwrap_or(5),
        "expiresIn": v["expires_in"].as_u64().unwrap_or(900),
    }))
}

/// Returns {status: pending|slow_down|expired|denied|ok, account?}. On "ok" the token is already in the vault.
pub fn gh_device_poll(host: String, client_id: String, device_code: String) -> Result<Value, String> {
        let host = norm_host(&host);
    let v = post_form(
        &format!("https://{host}/login/oauth/access_token"),
        &[
            ("client_id", client_id.trim()),
            ("device_code", &device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ],
    )?;
    if let Some(t) = v["access_token"].as_str() {
        let (u, _) = send("GET", &format!("{}/user", api_base(&host)), t, None)?;
        let acc = user_json(&host, &u);
        git::set_token(acc["key"].as_str().unwrap_or(""), t)?;
        return Ok(json!({ "status": "ok", "account": acc }));
    }
    let status = match v["error"].as_str().unwrap_or("") {
        "authorization_pending" => "pending",
        "slow_down" => "slow_down",
        "expired_token" => "expired",
        "access_denied" => "denied",
        other => return Err(v["error_description"].as_str().unwrap_or(other).to_string()),
    };
    Ok(json!({ "status": status }))
}
