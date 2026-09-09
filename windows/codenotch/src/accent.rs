//! Accent colour for the ample usage band and busy indicators.
//! Matches macOS AccentColorChoice presets; "system" reads the Windows accent.

use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_READ, REG_DWORD,
    REG_VALUE_TYPE,
};

/// Persistence keys match the macOS app so choices stay recognizable across ports.
pub const PRESETS: &[(&str, &str, &str)] = &[
    ("system", "system", ""), // resolved at runtime
    ("ff33e1", "pink", "#FF33E1"),
    ("eb4236", "red", "#EB4236"),
    ("eb8436", "orange", "#EB8436"),
    ("ffd400", "yellow", "#FFD400"),
    ("00ff88", "green", "#00FF88"),
    ("00e5cc", "teal", "#00E5CC"),
    ("36a8eb", "blue", "#36A8EB"),
    ("6c5ce7", "indigo", "#6C5CE7"),
    ("b026ff", "purple", "#B026FF"),
    ("f7f6f5", "offwhite", "#F7F6F5"),
];

const FALLBACK: &str = "#00FF88";

pub fn normalize(choice: &str) -> String {
    let c = choice.trim().trim_start_matches('#').to_ascii_lowercase();
    if c.is_empty() || c == "system" {
        return "system".into();
    }
    if PRESETS.iter().any(|(id, _, _)| *id == c) {
        return c;
    }
    // Accept bare hex like FF33E1
    if c.len() == 6 && c.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return c;
    }
    "system".into()
}

pub fn resolve(choice: &str) -> String {
    let id = normalize(choice);
    if id == "system" {
        return system_accent().unwrap_or_else(|| FALLBACK.to_string());
    }
    for (pid, _, hex) in PRESETS {
        if *pid == id && !hex.is_empty() {
            return (*hex).to_string();
        }
    }
    format!("#{id}")
}

/// Windows stores DWM AccentColor as 0xAABBGGRR.
fn system_accent() -> Option<String> {
    unsafe {
        let mut hkey = Default::default();
        let sub = wide("Software\\Microsoft\\Windows\\DWM");
        if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(sub.as_ptr()), 0, KEY_READ, &mut hkey).is_err()
        {
            return None;
        }
        let mut color = read_dword(hkey, "AccentColor")
            .or_else(|| read_dword(hkey, "ColorizationColor"));
        let _ = RegCloseKey(hkey);
        let dword = color.take()?;
        // ABGR → #RRGGBB
        let r = dword & 0xFF;
        let g = (dword >> 8) & 0xFF;
        let b = (dword >> 16) & 0xFF;
        Some(format!("#{r:02X}{g:02X}{b:02X}"))
    }
}

unsafe fn read_dword(hkey: windows::Win32::System::Registry::HKEY, name: &str) -> Option<u32> {
    let wname = wide(name);
    let mut ty = REG_VALUE_TYPE::default();
    let mut data = [0u8; 4];
    let mut len = data.len() as u32;
    let status = RegQueryValueExW(
        hkey,
        PCWSTR(wname.as_ptr()),
        None,
        Some(&mut ty),
        Some(data.as_mut_ptr()),
        Some(&mut len),
    );
    if status != ERROR_SUCCESS || ty != REG_DWORD || len < 4 {
        return None;
    }
    Some(u32::from_le_bytes(data))
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
