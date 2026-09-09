use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderVisibility {
    #[serde(default = "default_true")]
    pub claude: bool,
    #[serde(default = "default_true")]
    pub codex: bool,
    #[serde(default = "default_true")]
    pub cursor: bool,
    #[serde(default = "default_true")]
    pub gemini: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ProviderVisibility {
    fn default() -> Self {
        Self {
            claude: true,
            codex: true,
            cursor: true,
            gemini: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub repo_path: Option<String>,
    #[serde(default = "default_git_poll_secs")]
    pub poll_secs: u64,
}

fn default_git_poll_secs() -> u64 {
    120
}

impl Default for GitStatusConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            repo_path: None,
            poll_secs: default_git_poll_secs(),
        }
    }
}

impl ProviderVisibility {
    pub fn get(&self, id: &str) -> bool {
        match id {
            "claude" => self.claude,
            "codex" => self.codex,
            "cursor" => self.cursor,
            "gemini" => self.gemini,
            _ => true,
        }
    }

    pub fn set(&mut self, id: &str, on: bool) {
        match id {
            "claude" => self.claude = on,
            "codex" => self.codex = on,
            "cursor" => self.cursor = on,
            "gemini" => self.gemini = on,
            _ => {}
        }
    }

    /// All false — used for alert_muted defaults (alerts on).
    pub fn all_false() -> Self {
        Self {
            claude: false,
            codex: false,
            cursor: false,
            gemini: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    /// "auto" | "zh" | "en" | "ja" | "ko" | "ru"
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default)]
    pub bar_x: Option<i32>,
    #[serde(default)]
    pub bar_y: Option<i32>,
    /// Logical width of the bar (wheel-adjustable, 220-520); None = default 360
    #[serde(default)]
    pub bar_w: Option<u32>,
    /// Allow dragging + wheel resizing (tray toggle, off by default to prevent accidental drags)
    #[serde(default)]
    pub drag_enabled: bool,
    /// Vertical position of the notch along a side edge (0 = top, 1 = bottom). Kept for older configs; mirrored into along_right on load.
    #[serde(default = "default_along")]
    pub notch_y: f64,
    /// Which screen edge the notch is welded to: right | left | top | bottom
    #[serde(default = "default_edge")]
    pub edge: String,
    /// Along-edge position (0–1) remembered per edge so switching edges does not lose the place you chose.
    #[serde(default = "default_along")]
    pub along_right: f64,
    #[serde(default = "default_along")]
    pub along_left: f64,
    #[serde(default = "default_along")]
    pub along_top: f64,
    #[serde(default = "default_along")]
    pub along_bottom: f64,
    /// Which provider rings are shown in the notch
    #[serde(default)]
    pub providers: ProviderVisibility,
    /// Per-provider mute for 80%/100% threshold toasts (`true` = muted)
    #[serde(default = "default_alert_muted")]
    pub alert_muted: ProviderVisibility,
    /// Accent for the ample usage band: "system" or a 6-char hex key (same as macOS).
    #[serde(default = "default_accent")]
    pub accent: String,
    /// First-run Telegram / community card has been dismissed
    #[serde(default)]
    pub welcome_seen: bool,
    #[serde(default)]
    pub git_status: GitStatusConfig,
}

fn default_along() -> f64 {
    0.5
}
fn default_edge() -> String {
    "right".into()
}

fn default_port() -> u16 {
    48666
}
fn default_lang() -> String {
    "auto".into()
}
fn default_alert_muted() -> ProviderVisibility {
    ProviderVisibility::all_false()
}
fn default_accent() -> String {
    "system".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: default_port(),
            lang: default_lang(),
            bar_x: None,
            bar_y: None,
            bar_w: None,
            drag_enabled: false,
            notch_y: default_along(),
            edge: default_edge(),
            along_right: default_along(),
            along_left: default_along(),
            along_top: default_along(),
            along_bottom: default_along(),
            providers: ProviderVisibility::default(),
            alert_muted: ProviderVisibility::all_false(),
            accent: default_accent(),
            welcome_seen: false,
            git_status: GitStatusConfig::default(),
        }
    }
}

impl Config {
    pub fn normalize_edge(edge: &str) -> &'static str {
        match edge {
            "left" => "left",
            "top" => "top",
            "bottom" => "bottom",
            _ => "right",
        }
    }

    pub fn edge(&self) -> &'static str {
        Self::normalize_edge(&self.edge)
    }

    pub fn is_horizontal(&self) -> bool {
        matches!(self.edge(), "top" | "bottom")
    }

    pub fn along(&self) -> f64 {
        match self.edge() {
            "left" => self.along_left,
            "top" => self.along_top,
            "bottom" => self.along_bottom,
            _ => self.along_right,
        }
        .clamp(0.0, 1.0)
    }

    pub fn set_along(&mut self, ratio: f64) {
        let r = ratio.clamp(0.0, 1.0);
        match self.edge() {
            "left" => self.along_left = r,
            "top" => self.along_top = r,
            "bottom" => self.along_bottom = r,
            _ => {
                self.along_right = r;
                self.notch_y = r;
            }
        }
    }

    pub fn set_edge(&mut self, edge: &str) {
        self.edge = Self::normalize_edge(edge).to_string();
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("codenotch")
        .join("config.json")
}

pub fn load() -> Config {
    let path = config_path();
    let mut cfg = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str::<Config>(&t).ok())
        .unwrap_or_default();
    // Older builds only had notch_y for the right edge — promote it once if along_right is still the default mid and notch_y was moved.
    if (cfg.along_right - 0.5).abs() < f64::EPSILON && (cfg.notch_y - 0.5).abs() > f64::EPSILON {
        cfg.along_right = cfg.notch_y.clamp(0.0, 1.0);
    }
    cfg.edge = Config::normalize_edge(&cfg.edge).to_string();
    cfg
}

pub fn save(cfg: &Config) {
    let path = config_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(txt) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(path, txt);
    }
}
