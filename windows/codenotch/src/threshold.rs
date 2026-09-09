//! Threshold alerts: announce when a provider's headline limit crosses 80% or 100%.
//! Crossing rule matches the macOS ThresholdNotifier — once per level, again only after a rollover.

use crate::config::{self, ProviderVisibility};
use crate::usage::UsageSnapshot;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tauri::{AppHandle, Manager};

struct State {
    /// Highest threshold currently crossed per provider (0 / 80 / 100).
    crossed: HashMap<String, i32>,
    /// First observe seeds memory from disk.
    warmed: bool,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
    Mutex::new(State {
        crossed: HashMap::new(),
        warmed: false,
    })
});

#[derive(Serialize, Deserialize, Default)]
struct Persisted {
    crossed: HashMap<String, i32>,
}

fn state_path() -> std::path::PathBuf {
    config::config_path().with_file_name("alerts_state.json")
}

fn load_persisted() -> HashMap<String, i32> {
    std::fs::read_to_string(state_path())
        .ok()
        .and_then(|t| serde_json::from_str::<Persisted>(&t).ok())
        .map(|p| p.crossed)
        .unwrap_or_default()
}

fn save_persisted(crossed: &HashMap<String, i32>) {
    let path = state_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let p = Persisted {
        crossed: crossed.clone(),
    };
    if let Ok(txt) = serde_json::to_string_pretty(&p) {
        let _ = std::fs::write(path, txt);
    }
}

fn headline(snap: &UsageSnapshot) -> Option<&crate::usage::LimitWindow> {
    let metered: Vec<_> = snap.windows.iter().filter(|w| w.count.is_none()).collect();
    if metered.is_empty() {
        return None;
    }
    metered.into_iter().max_by(|a, b| {
        a.used
            .partial_cmp(&b.used)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

fn level_of(used: f64) -> i32 {
    let pct = used * 100.0;
    if pct >= 100.0 {
        100
    } else if pct >= 80.0 {
        80
    } else {
        0
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn show_toast(title: &str, body: &str) {
    let title = escape_xml(title);
    let body = escape_xml(body);
    let xml = format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{title}</text><text>{body}</text></binding></visual></toast>"
    );
    let path = std::env::temp_dir().join(format!(
        "codenotch-toast-{}.xml",
        std::process::id()
    ));
    if std::fs::write(&path, xml.as_bytes()).is_err() {
        return;
    }
    let path_str = path.to_string_lossy().replace('\'', "''");
    let ps = format!(
        "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null; \
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom, ContentType = WindowsRuntime] | Out-Null; \
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument; \
$xml.LoadXml((Get-Content -LiteralPath '{path_str}' -Raw -Encoding UTF8)); \
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml); \
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Codenotch').Show($toast); \
Remove-Item -LiteralPath '{path_str}' -Force -ErrorAction SilentlyContinue"
    );
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let _ = cmd.spawn();
}

fn format_reset(ms: Option<u64>, lang: &str) -> String {
    let Some(ms) = ms else {
        return String::new();
    };
    let Some(d) = chrono::DateTime::from_timestamp_millis(ms as i64) else {
        return String::new();
    };
    let local = d.with_timezone(&chrono::Local);
    if lang == "ru" {
        format!(" — сброс {}", local.format("%d.%m %H:%M"))
    } else {
        format!(" — resets {}", local.format("%b %d %H:%M"))
    }
}

fn deliver(
    lang: &str,
    provider: &str,
    window_label: &str,
    threshold: i32,
    used_pct: i32,
    resets_at: Option<u64>,
) {
    let reset = format_reset(resets_at, lang);
    let (title, body) = if lang == "ru" {
        if threshold >= 100 {
            (
                format!("{provider}: лимит исчерпан"),
                format!("«{window_label}» потрачен{reset}."),
            )
        } else {
            (
                format!("{provider}: {used_pct}%"),
                format!("Использовано {used_pct}% лимита «{window_label}»{reset}."),
            )
        }
    } else if threshold >= 100 {
        (
            format!("{provider} limit reached"),
            format!("Its {window_label} limit is spent{reset}."),
        )
    } else {
        (
            format!("{provider} is at {used_pct}%"),
            format!("{used_pct}% of its {window_label} limit used{reset}."),
        )
    };
    show_toast(&title, &body);
}

/// Call after each usage broadcast for a provider.
pub fn observe(app: &AppHandle, provider_id: &str, provider_name: &str, snap: &UsageSnapshot) {
    if snap.status == "absent" || snap.status == "none" || snap.status == "needsAuth" {
        return;
    }
    let (muted, visible, lang) = {
        let st = app.state::<crate::AppState>();
        let c = st.cfg.lock().unwrap();
        let resolved = if c.lang == "auto" {
            crate::i18n::resolve_auto().to_string()
        } else {
            c.lang.clone()
        };
        (
            c.alert_muted.get(provider_id),
            c.providers.get(provider_id),
            resolved,
        )
    };
    if !visible {
        return;
    }

    let Some(h) = headline(snap) else {
        return;
    };
    let level = level_of(h.used);
    let used_pct = (h.used * 100.0).round() as i32;
    let label = h.label.clone();
    let resets = h.resets_at;

    let mut st = STATE.lock().unwrap();
    if !st.warmed {
        st.crossed = load_persisted();
        st.warmed = true;
    }
    let previous = *st.crossed.get(provider_id).unwrap_or(&0);
    // Always remember the level (mute still advances memory — unmute must not replay).
    st.crossed.insert(provider_id.to_string(), level);
    save_persisted(&st.crossed);

    if level <= previous || muted {
        return;
    }

    for threshold in [80, 100] {
        if threshold > previous && threshold <= level {
            deliver(&lang, provider_name, &label, threshold, used_pct, resets);
        }
    }
}

pub fn toggle_mute(app: &AppHandle, id: &str) {
    {
        let st = app.state::<crate::AppState>();
        let mut c = st.cfg.lock().unwrap();
        let on = !c.alert_muted.get(id);
        c.alert_muted.set(id, on);
        config::save(&c);
    }
    crate::tray::refresh_menu(app);
}

pub fn muted_map(app: &AppHandle) -> ProviderVisibility {
    app.state::<crate::AppState>()
        .cfg
        .lock()
        .unwrap()
        .alert_muted
        .clone()
}
