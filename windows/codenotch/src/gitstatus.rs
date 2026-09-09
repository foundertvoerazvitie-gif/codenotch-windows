//! Optional Git / CI pulse: local `git` + `gh` CLI, no stored tokens.
//! Persisted at %APPDATA%/codenotch/gitstatus.json; repo from config or last Cursor workspace.

use crate::config::GitStatusConfig;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

const GIT_TIMEOUT_SECS: u64 = 5;
const BACKOFF_BASE_SECS: u64 = 60;
const BACKOFF_CAP_SECS: u64 = 900;

static REFRESH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn request_refresh() {
    REFRESH.store(true, std::sync::atomic::Ordering::Relaxed);
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitSnapshot {
    /// absent | ok | stale | error
    pub status: String,
    /// green | yellow | red | gray
    pub level: String,
    pub branch: String,
    pub repo_path: String,
    pub dirty: bool,
    pub ahead: i32,
    pub behind: i32,
    /// success | failure | pending | running | none
    #[serde(default)]
    pub ci_state: Option<String>,
    pub line1: String,
    pub line2: String,
    pub line3: String,
    pub fetched_at: u64,
    pub note: String,
    #[serde(default)]
    pub backoff_until: u64,
}

fn store_path() -> PathBuf {
    crate::config::config_path().with_file_name("gitstatus.json")
}

pub fn load_persisted() -> GitSnapshot {
    std::fs::read_to_string(store_path())
        .ok()
        .and_then(|t| serde_json::from_str::<GitSnapshot>(&t).ok())
        .map(|mut s| {
            if s.status == "ok" && !s.repo_path.is_empty() {
                s.status = "stale".into();
            }
            s
        })
        .unwrap_or_default()
}

fn persist(s: &GitSnapshot) {
    if let Ok(t) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(store_path(), t);
    }
}

fn sleep_interruptible(total_secs: u64) {
    for _ in 0..total_secs {
        if REFRESH.swap(false, std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn cfg_snapshot(app: &AppHandle) -> GitStatusConfig {
    let st = app.state::<AppState>();
    let cfg = st.cfg.lock().unwrap().git_status.clone();
    cfg
}

fn resolve_repo(cfg: &GitStatusConfig) -> Option<PathBuf> {
    if let Some(p) = cfg.repo_path.as_ref().filter(|s| !s.is_empty()) {
        let pb = PathBuf::from(p);
        if pb.join(".git").exists() {
            return Some(pb);
        }
    }
    last_cursor_workspace()
}

/// Cursor / VS Code: `history.recentlyOpenedPathsList` in globalStorage state.vscdb
pub fn last_cursor_workspace() -> Option<PathBuf> {
    let path = dirs::config_dir()?.join("Cursor").join("User").join("globalStorage").join("state.vscdb");
    let conn = open_ro(&path)?;
    let raw = item(&conn, "history.recentlyOpenedPathsList")?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let entries = v.get("entries").and_then(|x| x.as_array()).or_else(|| v.as_array())?;
    for e in entries {
        if let Some(uri) = e.get("folderUri").and_then(|x| x.as_str()) {
            if let Some(p) = file_uri_to_path(uri) {
                if p.join(".git").exists() {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn open_ro(path: &Path) -> Option<rusqlite::Connection> {
    use rusqlite::OpenFlags;
    if !path.is_file() {
        return None;
    }
    if let Ok(c) = rusqlite::Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        if c.prepare("SELECT 1 FROM ItemTable LIMIT 1")
            .and_then(|mut s| s.query([]).map(|_| ()))
            .is_ok()
        {
            return Some(c);
        }
    }
    let mut uri = String::from("file:///");
    uri.push_str(
        &path
            .to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches('/')
            .replace('#', "%23")
            .replace('?', "%3F"),
    );
    uri.push_str("?immutable=1");
    rusqlite::Connection::open_with_flags(
        &uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()
}

fn item(conn: &rusqlite::Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM ItemTable WHERE key = ?1", [key], |r| r.get::<_, String>(0))
        .ok()
        .filter(|s| !s.is_empty())
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or(""), 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let path_part = rest.strip_prefix('/').unwrap_or(rest);
    let decoded = pct_decode(path_part);
    let normalized = if decoded.len() >= 2 && decoded.as_bytes()[1] == b':' {
        decoded
    } else if cfg!(windows) && decoded.starts_with('/') {
        decoded.trim_start_matches('/').to_string()
    } else {
        decoded
    };
    Some(PathBuf::from(normalized))
}

#[cfg(windows)]
fn no_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000);
}
#[cfg(not(windows))]
fn no_window(_cmd: &mut Command) {}

struct CmdOut {
    ok: bool,
    stdout: String,
    stderr: String,
}

fn run_with_timeout(program: &str, args: &[&str], cwd: Option<&Path>, timeout_secs: u64) -> Option<String> {
    run_with_timeout_ex(program, args, cwd, timeout_secs).filter(|o| o.ok).map(|o| o.stdout)
}

fn run_with_timeout_ex(program: &str, args: &[&str], cwd: Option<&Path>, timeout_secs: u64) -> Option<CmdOut> {
    let program = program.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let cwd = cwd.map(|p| p.to_path_buf());
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut cmd = Command::new(&program);
        cmd.args(&args).stdout(Stdio::piped()).stderr(Stdio::piped());
        if let Some(c) = &cwd {
            cmd.current_dir(c);
        }
        no_window(&mut cmd);
        let _ = tx.send(cmd.output());
    });
    match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(Ok(out)) => Some(CmdOut {
            ok: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }),
        _ => None,
    }
}

fn git_ok() -> bool {
    run_with_timeout("git", &["--version"], None, 2).is_some()
}

fn parse_porcelain_branch(out: &str) -> (String, i32, i32, bool) {
    let mut branch = String::new();
    let mut ahead = 0i32;
    let mut behind = 0i32;
    let mut dirty = false;
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            let head = rest.split_whitespace().next().unwrap_or("");
            branch = head.split("...").next().unwrap_or(head).to_string();
            if let Some(bracket) = rest.find('[') {
                let inner = &rest[bracket + 1..rest.rfind(']').unwrap_or(rest.len())];
                for part in inner.split(',') {
                    let p = part.trim();
                    if let Some(n) = p.strip_prefix("ahead ") {
                        ahead = n.trim().parse().unwrap_or(0);
                    } else if let Some(n) = p.strip_prefix("behind ") {
                        behind = n.trim().parse().unwrap_or(0);
                    }
                }
            }
            continue;
        }
        if line.len() >= 3 && line.as_bytes()[2] != b' ' {
            dirty = true;
        } else if !line.is_empty() && !line.starts_with("##") {
            dirty = true;
        }
    }
    (branch, ahead, behind, dirty)
}

fn origin_https(repo: &Path) -> Option<String> {
    let url = run_with_timeout("git", &["remote", "get-url", "origin"], Some(repo), GIT_TIMEOUT_SECS)?;
    let u = url.trim();
    if let Some(rest) = u.strip_prefix("git@github.com:") {
        let path = rest.trim_end_matches(".git");
        return Some(format!("https://github.com/{path}"));
    }
    if let Some(rest) = u.strip_prefix("ssh://git@github.com/") {
        let path = rest.trim_end_matches(".git");
        return Some(format!("https://github.com/{path}"));
    }
    if u.starts_with("https://github.com/") {
        return Some(u.trim_end_matches(".git").to_string());
    }
    if u.starts_with("http://github.com/") {
        return Some(format!("https://{}", u.trim_start_matches("http://").trim_end_matches(".git")));
    }
    None
}

/// Click the Git cell: open PR on GitHub if one exists, else the repo page, else Explorer.
pub fn open_action(repo_path: &str, branch: &str) -> Result<String, String> {
    if repo_path.is_empty() {
        return Err("no repo".into());
    }
    let path = Path::new(repo_path);
    if !path.is_dir() {
        return Err("repo folder missing".into());
    }

    // Prefer open PR for current branch
    if !branch.is_empty() && branch != "HEAD" {
        if let Some(out) = run_with_timeout_ex(
            "gh",
            &["pr", "view", "--web"],
            Some(path),
            8,
        ) {
            if out.ok {
                return Ok("Opened PR on GitHub".into());
            }
        }
    }

    // Repo homepage via gh
    if let Some(out) = run_with_timeout_ex("gh", &["browse"], Some(path), 8) {
        if out.ok {
            return Ok("Opened repo on GitHub".into());
        }
    }

    // HTTPS origin without gh
    if let Some(url) = origin_https(path) {
        crate::community::open_url(&url)?;
        return Ok("Opened repo on GitHub".into());
    }

    // Last resort: folder
    let mut cmd = Command::new("explorer");
    cmd.arg(repo_path);
    no_window(&mut cmd);
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("explorer: {e}"))?;
    Ok("Opened repo folder".into())
}

fn origin_is_github(repo: &Path) -> bool {
    let Some(url) = run_with_timeout("git", &["remote", "get-url", "origin"], Some(repo), GIT_TIMEOUT_SECS) else {
        return false;
    };
    let u = url.trim().to_lowercase();
    u.contains("github.com")
}

enum GhOutcome {
    Ok(Option<String>), // ci_state
    RateLimited(u64),
    AuthOrMissing(String),
    Other(String),
}

fn query_gh_ci(repo: &Path, branch: &str) -> GhOutcome {
    if run_with_timeout("gh", &["--version"], None, 2).is_none() {
        return GhOutcome::AuthOrMissing("gh not installed".into());
    }
    let Some(out) = run_with_timeout_ex(
        "gh",
        &[
            "pr",
            "list",
            "--head",
            branch,
            "--state",
            "open",
            "--json",
            "statusCheckRollup",
            "--limit",
            "1",
        ],
        Some(repo),
        GIT_TIMEOUT_SECS,
    ) else {
        return GhOutcome::Other("gh timed out".into());
    };
    if !out.ok {
        let combined = format!("{}{}", out.stderr, out.stdout);
        if out.stderr.contains("401") || out.stderr.contains("403") || combined.to_lowercase().contains("auth") {
            return GhOutcome::AuthOrMissing("gh auth required".into());
        }
        if out.stderr.contains("429") || combined.contains("rate limit") {
            return GhOutcome::RateLimited(BACKOFF_BASE_SECS);
        }
        return GhOutcome::Other(combined.trim().chars().take(120).collect());
    }
    let arr: Vec<serde_json::Value> = match serde_json::from_str(out.stdout.trim()) {
        Ok(x) => x,
        Err(e) => return GhOutcome::Other(format!("gh parse: {e}")),
    };
    if arr.is_empty() {
        return GhOutcome::Ok(None);
    }
    let rollup = arr[0].get("statusCheckRollup").and_then(|x| x.as_array());
    GhOutcome::Ok(rollup_to_ci(rollup))
}

fn rollup_to_ci(rollup: Option<&Vec<serde_json::Value>>) -> Option<String> {
    let arr = rollup?;
    if arr.is_empty() {
        return Some("none".into());
    }
    let mut any_fail = false;
    let mut any_pending = false;
    for c in arr {
        let st = c
            .get("state")
            .or_else(|| c.get("conclusion"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_uppercase();
        match st.as_str() {
            "FAILURE" | "ERROR" | "CANCELLED" | "TIMED_OUT" | "ACTION_REQUIRED" => any_fail = true,
            "PENDING" | "IN_PROGRESS" | "QUEUED" | "WAITING" | "REQUESTED" | "STALE" => any_pending = true,
            _ => {}
        }
    }
    if any_fail {
        Some("failure".into())
    } else if any_pending {
        Some("pending".into())
    } else {
        Some("success".into())
    }
}

fn gh_rate_limit_message(repo: &Path) -> Option<u64> {
    let out = run_with_timeout("gh", &["api", "rate_limit"], Some(repo), GIT_TIMEOUT_SECS)?;
    let v: serde_json::Value = serde_json::from_str(out.trim()).ok()?;
    let core = v.get("resources")?.get("core")?;
    let remaining = core.get("remaining")?.as_u64()?;
    if remaining == 0 {
        let reset = core.get("reset")?.as_u64().unwrap_or(0);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
        return Some(reset.saturating_sub(now).max(BACKOFF_BASE_SECS));
    }
    None
}

fn compute_level(ci: &Option<String>, dirty: bool, ahead: i32, behind: i32, gh_gray: bool) -> String {
    if ci.as_deref() == Some("failure") {
        return "red".into();
    }
    if dirty
        || ahead != 0
        || behind != 0
        || matches!(ci.as_deref(), Some("pending") | Some("running"))
    {
        return "yellow".into();
    }
    if gh_gray {
        return "gray".into();
    }
    "green".into()
}

fn build_lines(
    branch: &str,
    dirty: bool,
    ahead: i32,
    behind: i32,
    ci: &Option<String>,
    gh_note: &str,
    ru: bool,
) -> (String, String, String) {
    let mut git_part = branch.to_string();
    if ru {
        if dirty {
            git_part.push_str(" · есть изменения");
        } else {
            git_part.push_str(" · нет изменений");
        }
        if ahead != 0 || behind != 0 {
            git_part.push_str(&format!(" · +{ahead}/−{behind}"));
        }
    } else {
        if dirty {
            git_part.push_str(" · dirty");
        } else {
            git_part.push_str(" · clean");
        }
        if ahead != 0 || behind != 0 {
            git_part.push_str(&format!(" · +{ahead}/−{behind}"));
        }
    }
    let line2 = match ci.as_deref() {
        Some("success") if ru => "CI: success".into(),
        Some("success") => "CI: success".into(),
        Some("failure") if ru => "CI: failure".into(),
        Some("failure") => "CI: failure".into(),
        Some("pending") | Some("running") if ru => "CI: in progress".into(),
        Some("pending") | Some("running") => "CI: in progress".into(),
        None if ru => "Нет открытого PR для ветки".into(),
        None => "No open PR for branch".into(),
        _ if ru => "CI: —".into(),
        _ => "CI: —".into(),
    };
    let line3 = if !gh_note.is_empty() {
        gh_note.to_string()
    } else {
        String::new()
    };
    (git_part, line2, line3)
}

fn read_once(prev: &GitSnapshot, cfg: &GitStatusConfig, ru: bool) -> GitSnapshot {
    if !cfg.enabled {
        return GitSnapshot {
            status: "absent".into(),
            ..Default::default()
        };
    }
    if !git_ok() {
        return GitSnapshot {
            status: "absent".into(),
            ..Default::default()
        };
    }
    let Some(repo) = resolve_repo(cfg) else {
        return GitSnapshot {
            status: "absent".into(),
            ..Default::default()
        };
    };
    let porcelain = match run_with_timeout(
        "git",
        &["status", "--porcelain", "--branch"],
        Some(&repo),
        GIT_TIMEOUT_SECS,
    ) {
        Some(s) => s,
        None => {
            let repo_s = repo.to_string_lossy().into_owned();
            let same_repo = prev.repo_path == repo_s && !prev.repo_path.is_empty();
            let mut snap = if same_repo {
                prev.clone()
            } else {
                GitSnapshot {
                    repo_path: repo_s.clone(),
                    ..Default::default()
                }
            };
            snap.status = if same_repo && !prev.branch.is_empty() {
                "stale".into()
            } else {
                "error".into()
            };
            snap.repo_path = repo_s;
            let timeout_note = if ru {
                "git status: таймаут"
            } else {
                "git status timed out"
            };
            snap.note.clear();
            if snap.line1.is_empty() {
                snap.line1 = timeout_note.into();
            } else {
                snap.line3 = timeout_note.into();
            }
            return snap;
        }
    };
    let (branch, ahead, behind, dirty) = parse_porcelain_branch(&porcelain);
    let repo_s = repo.to_string_lossy().into_owned();

    let mut gh_note = String::new();
    let mut gh_gray = false;
    let mut ci_state: Option<String> = None;
    let mut rate_wait: Option<u64> = None;

    if origin_is_github(&repo) {
        if let Some(w) = gh_rate_limit_message(&repo) {
            rate_wait = Some(w);
        } else {
            match query_gh_ci(&repo, &branch) {
                GhOutcome::Ok(ci) => ci_state = ci,
                GhOutcome::RateLimited(ra) => {
                    rate_wait = Some(ra.max(BACKOFF_BASE_SECS).min(BACKOFF_CAP_SECS))
                }
                GhOutcome::AuthOrMissing(msg) => {
                    gh_note = msg;
                    gh_gray = true;
                }
                GhOutcome::Other(msg) => {
                    if msg.contains("401") || msg.contains("403") || msg.to_lowercase().contains("auth") {
                        gh_note = if ru {
                            "gh: войдите (gh auth login)".into()
                        } else {
                            "gh: run gh auth login".into()
                        };
                        gh_gray = true;
                    } else if msg.contains("no pull requests") || msg.contains("Could not find") {
                        ci_state = None;
                    } else {
                        gh_note = msg;
                        gh_gray = true;
                    }
                }
            }
        }
    }

    if let Some(wait) = rate_wait {
        let rate_note = if ru {
            format!("GitHub rate limit, повтор через {wait}с")
        } else {
            format!("GitHub rate limit, retry in {wait}s")
        };
        let mut note_for_lines = gh_note.clone();
        if note_for_lines.is_empty() {
            note_for_lines = rate_note;
        }
        let mut snap = GitSnapshot {
            status: "stale".into(),
            repo_path: repo_s,
            branch: branch.clone(),
            dirty,
            ahead,
            behind,
            ci_state: ci_state.clone(),
            level: compute_level(&ci_state, dirty, ahead, behind, false),
            fetched_at: if prev.repo_path == repo.to_string_lossy().as_ref() {
                prev.fetched_at
            } else {
                now_ms()
            },
            backoff_until: now_ms() + wait * 1000,
            note: String::new(),
            ..Default::default()
        };
        let (l1, l2, l3) = build_lines(&branch, dirty, ahead, behind, &ci_state, &note_for_lines, ru);
        snap.line1 = l1;
        snap.line2 = l2;
        snap.line3 = l3;
        return snap;
    }

    let level = compute_level(&ci_state, dirty, ahead, behind, gh_gray);
    let (line1, line2, line3) = build_lines(&branch, dirty, ahead, behind, &ci_state, &gh_note, ru);
    GitSnapshot {
        status: "ok".into(),
        level,
        branch,
        repo_path: repo_s,
        dirty,
        ahead,
        behind,
        ci_state,
        line1,
        line2,
        line3,
        fetched_at: now_ms(),
        note: gh_note,
        backoff_until: 0,
    }
}

fn broadcast(app: &AppHandle, snap: GitSnapshot) {
    let st = app.state::<AppState>();
    *st.git.lock().unwrap() = snap.clone();
    persist(&snap);
    let _ = app.emit("git", &snap);
}

pub fn present(cfg: &GitStatusConfig) -> bool {
    cfg.enabled && git_ok() && resolve_repo(cfg).is_some()
}

pub fn probe() -> String {
    let cfg = crate::config::load().git_status;
    if !present(&cfg) {
        if !cfg.enabled {
            return "Git/CI: disabled in config".into();
        }
        if !git_ok() {
            return "Git/CI: git not in PATH".into();
        }
        return "Git/CI: no repo (set git_status.repo_path or open a folder in Cursor)".into();
    }
    let p = resolve_repo(&cfg).unwrap();
    format!("Git/CI: repo {} (origin github={})", p.display(), origin_is_github(&p))
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        {
            let st = app.state::<AppState>();
            let snap = st.git.lock().unwrap().clone();
            let _ = app.emit("git", &snap);
        }
        loop {
            let cfg = cfg_snapshot(&app);
            let poll = cfg.poll_secs.max(30);
            let bu = {
                let st = app.state::<AppState>();
                let bu = st.git.lock().unwrap().backoff_until;
                bu
            };
            let now = now_ms();
            if bu > now {
                sleep_interruptible(((bu - now) / 1000).clamp(1, 30));
                continue;
            }
            let prev = {
                let st = app.state::<AppState>();
                let prev = st.git.lock().unwrap().clone();
                prev
            };
            let ru = {
                let st = app.state::<AppState>();
                let lang = &st.cfg.lock().unwrap().lang;
                let l = if lang == "auto" {
                    crate::i18n::resolve_auto()
                } else {
                    lang.as_str()
                };
                l == "ru"
            };
            let snap = read_once(&prev, &cfg, ru);
            if snap.status == "error" || snap.status == "stale" {
                crate::applog(&format!("gitstatus: {}", snap.note));
            }
            broadcast(&app, snap);
            sleep_interruptible(poll);
        }
    });
}
