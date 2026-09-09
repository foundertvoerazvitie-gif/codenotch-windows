//! Community / promo links (soft — tray + one-time first-run, never blocking).

pub const TELEGRAM_URL: &str = "https://t.me/aistanislav";
pub const TELEGRAM_HANDLE: &str = "@aistanislav";

pub fn open_telegram() -> Result<String, String> {
    open_url(TELEGRAM_URL)?;
    Ok(format!("Opened {TELEGRAM_HANDLE}"))
}

pub fn open_url(url: &str) -> Result<(), String> {
    // Defense-in-depth: only https:// (caller today always passes TELEGRAM_URL).
    if !url.starts_with("https://") {
        return Err("refusing non-https URL".into());
    }
    let mut cmd = std::process::Command::new("cmd");
    cmd.args(["/C", "start", "", url]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to open browser: {e}"))
}
