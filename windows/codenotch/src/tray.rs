use crate::hooks_install;
use crate::i18n::tr;
use tauri::menu::{CheckMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Wry};

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let lang = {
        let st = app.state::<crate::AppState>();
        let c = st.cfg.lock().unwrap();
        c.lang.clone()
    };
    let menu = build_menu(app, &lang)?;
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip(concat!("Codenotch v", env!("CARGO_PKG_VERSION")))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, ev| handle(app, ev.id().as_ref()))
        .build(app)?;
    Ok(())
}

pub fn build_menu(app: &AppHandle, lang: &str) -> tauri::Result<Menu<Wry>> {
    let providers = {
        let st = app.state::<crate::AppState>();
        let c = st.cfg.lock().unwrap();
        c.providers.clone()
    };
    let install = MenuItemBuilder::with_id("install", tr(lang, "install")).build(app)?;
    let uninstall = MenuItemBuilder::with_id("uninstall", tr(lang, "uninstall")).build(app)?;
    let l_auto = CheckMenuItemBuilder::with_id("lang-auto", tr(lang, "lang_auto"))
        .checked(lang == "auto")
        .build(app)?;
    let l_zh = CheckMenuItemBuilder::with_id("lang-zh", "中文")
        .checked(lang == "zh")
        .build(app)?;
    let l_en = CheckMenuItemBuilder::with_id("lang-en", "English")
        .checked(lang == "en")
        .build(app)?;
    let l_ja = CheckMenuItemBuilder::with_id("lang-ja", "日本語")
        .checked(lang == "ja")
        .build(app)?;
    let l_ko = CheckMenuItemBuilder::with_id("lang-ko", "한국어")
        .checked(lang == "ko")
        .build(app)?;
    let l_ru = CheckMenuItemBuilder::with_id("lang-ru", "Русский")
        .checked(lang == "ru")
        .build(app)?;
    let lang_menu = SubmenuBuilder::new(app, tr(lang, "language"))
        .items(&[&l_auto, &l_zh, &l_en, &l_ja, &l_ko, &l_ru])
        .build()?;
    let p_claude = CheckMenuItemBuilder::with_id("prov-claude", format!("{} Claude", tr(lang, "show")))
        .checked(providers.claude)
        .build(app)?;
    let p_codex = CheckMenuItemBuilder::with_id("prov-codex", format!("{} Codex", tr(lang, "show")))
        .checked(providers.codex)
        .build(app)?;
    let p_cursor = CheckMenuItemBuilder::with_id("prov-cursor", format!("{} Cursor", tr(lang, "show")))
        .checked(providers.cursor)
        .build(app)?;
    let p_gemini = CheckMenuItemBuilder::with_id(
        "prov-gemini",
        format!("{} Antigravity", tr(lang, "show")),
    )
    .checked(providers.gemini)
    .build(app)?;
    let git_on = {
        let st = app.state::<crate::AppState>();
        let on = st.cfg.lock().unwrap().git_status.enabled;
        on
    };
    let p_git = CheckMenuItemBuilder::with_id("show-git", tr(lang, "show_git"))
        .checked(git_on)
        .build(app)?;
    // Top-level checks: Windows tray submenus often fail to open / deliver clicks.
    let muted = {
        let st = app.state::<crate::AppState>();
        let c = st.cfg.lock().unwrap();
        c.alert_muted.clone()
    };
    // Checked = alerts enabled (not muted)
    let a_claude = CheckMenuItemBuilder::with_id("alert-claude", "Claude")
        .checked(!muted.claude)
        .build(app)?;
    let a_codex = CheckMenuItemBuilder::with_id("alert-codex", "Codex")
        .checked(!muted.codex)
        .build(app)?;
    let a_cursor = CheckMenuItemBuilder::with_id("alert-cursor", "Cursor")
        .checked(!muted.cursor)
        .build(app)?;
    let a_gemini = CheckMenuItemBuilder::with_id("alert-gemini", "Antigravity")
        .checked(!muted.gemini)
        .build(app)?;
    let alerts_menu = SubmenuBuilder::new(app, tr(lang, "alerts"))
        .items(&[&a_claude, &a_codex, &a_cursor, &a_gemini])
        .build()?;
    let edge = {
        let st = app.state::<crate::AppState>();
        let c = st.cfg.lock().unwrap();
        c.edge().to_string()
    };
    let e_right = CheckMenuItemBuilder::with_id("edge-right", tr(lang, "edge_right"))
        .checked(edge == "right")
        .build(app)?;
    let e_left = CheckMenuItemBuilder::with_id("edge-left", tr(lang, "edge_left"))
        .checked(edge == "left")
        .build(app)?;
    let e_top = CheckMenuItemBuilder::with_id("edge-top", tr(lang, "edge_top"))
        .checked(edge == "top")
        .build(app)?;
    let e_bottom = CheckMenuItemBuilder::with_id("edge-bottom", tr(lang, "edge_bottom"))
        .checked(edge == "bottom")
        .build(app)?;
    let edge_menu = SubmenuBuilder::new(app, tr(lang, "placement"))
        .items(&[&e_right, &e_left, &e_top, &e_bottom])
        .build()?;
    let accent_choice = {
        let st = app.state::<crate::AppState>();
        let c = st.cfg.lock().unwrap();
        crate::accent::normalize(&c.accent)
    };
    let ac_system = CheckMenuItemBuilder::with_id("accent-system", tr(lang, "accent_system"))
        .checked(accent_choice == "system")
        .build(app)?;
    let ac_pink = CheckMenuItemBuilder::with_id("accent-ff33e1", format!("● #FF33E1 {}", tr(lang, "accent_pink")))
        .checked(accent_choice == "ff33e1").build(app)?;
    let ac_red = CheckMenuItemBuilder::with_id("accent-eb4236", format!("● #EB4236 {}", tr(lang, "accent_red")))
        .checked(accent_choice == "eb4236").build(app)?;
    let ac_orange = CheckMenuItemBuilder::with_id("accent-eb8436", format!("● #EB8436 {}", tr(lang, "accent_orange")))
        .checked(accent_choice == "eb8436").build(app)?;
    let ac_yellow = CheckMenuItemBuilder::with_id("accent-ffd400", format!("● #FFD400 {}", tr(lang, "accent_yellow")))
        .checked(accent_choice == "ffd400").build(app)?;
    let ac_green = CheckMenuItemBuilder::with_id("accent-00ff88", format!("● #00FF88 {}", tr(lang, "accent_green")))
        .checked(accent_choice == "00ff88").build(app)?;
    let ac_teal = CheckMenuItemBuilder::with_id("accent-00e5cc", format!("● #00E5CC {}", tr(lang, "accent_teal")))
        .checked(accent_choice == "00e5cc").build(app)?;
    let ac_blue = CheckMenuItemBuilder::with_id("accent-36a8eb", format!("● #36A8EB {}", tr(lang, "accent_blue")))
        .checked(accent_choice == "36a8eb").build(app)?;
    let ac_indigo = CheckMenuItemBuilder::with_id("accent-6c5ce7", format!("● #6C5CE7 {}", tr(lang, "accent_indigo")))
        .checked(accent_choice == "6c5ce7").build(app)?;
    let ac_purple = CheckMenuItemBuilder::with_id("accent-b026ff", format!("● #B026FF {}", tr(lang, "accent_purple")))
        .checked(accent_choice == "b026ff").build(app)?;
    let ac_off = CheckMenuItemBuilder::with_id("accent-f7f6f5", format!("● #F7F6F5 {}", tr(lang, "accent_offwhite")))
        .checked(accent_choice == "f7f6f5").build(app)?;
    let accent_menu = SubmenuBuilder::new(app, tr(lang, "accent"))
        .items(&[
            &ac_system, &ac_pink, &ac_red, &ac_orange, &ac_yellow, &ac_green,
            &ac_teal, &ac_blue, &ac_indigo, &ac_purple, &ac_off,
        ])
        .build()?;
    let refresh = MenuItemBuilder::with_id("refresh", tr(lang, "refresh")).build(app)?;
    let reset = MenuItemBuilder::with_id("reset", tr(lang, "reset_pos")).build(app)?;
    let open_data = MenuItemBuilder::with_id("open-data", tr(lang, "open_data")).build(app)?;
    let telegram = MenuItemBuilder::with_id(
        "telegram",
        format!("{} {}", tr(lang, "telegram"), crate::community::TELEGRAM_HANDLE),
    )
    .build(app)?;
    let auto = CheckMenuItemBuilder::with_id("autostart", tr(lang, "autostart"))
        .checked(crate::autostart::is_enabled())
        .build(app)?;
    let quit = MenuItemBuilder::with_id("quit", tr(lang, "quit")).build(app)?;
    MenuBuilder::new(app)
        .items(&[&install, &uninstall])
        .separator()
        .item(&p_claude)
        .item(&p_codex)
        .item(&p_cursor)
        .item(&p_gemini)
        .item(&p_git)
        .separator()
        .item(&lang_menu)
        .item(&alerts_menu)
        .item(&edge_menu)
        .item(&accent_menu)
        .item(&refresh)
        .item(&reset)
        .item(&open_data)
        .item(&telegram)
        .item(&auto)
        .separator()
        .item(&quit)
        .build()
}

pub fn refresh_menu(app: &AppHandle) {
    let lang = {
        let st = app.state::<crate::AppState>();
        let c = st.cfg.lock().unwrap();
        c.lang.clone()
    };
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = build_menu(app, &lang) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn handle(app: &AppHandle, id: &str) {
    match id {
        "install" => notice(app, hooks_install::install()),
        "uninstall" => notice(app, hooks_install::uninstall()),
        "reset" => crate::reset_bar(app),
        "open-data" => {
            let dir = crate::config::config_path().parent().map(|p| p.to_path_buf()).unwrap_or_default();
            let _ = std::fs::create_dir_all(crate::glyphs::user_dir());
            let mut cmd = std::process::Command::new("explorer");
            cmd.arg(dir.as_os_str());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0800_0000);
            }
            let _ = cmd.spawn();
        }
        "telegram" => notice(app, crate::community::open_telegram()),
        "refresh" => {
            {
                let st = app.state::<crate::AppState>();
                let mut u = st.usage.lock().unwrap();
                u.backoff_until = 0;
            }
            crate::usage::request_refresh();
            crate::codex::request_refresh();
            crate::cursor::request_refresh();
            crate::antigravity::request_refresh();
            crate::gitstatus::request_refresh();
            let a = app.clone();
            std::thread::spawn(move || crate::reload_glyphs(&a));
        }
        "autostart" => {
            let r = if crate::autostart::is_enabled() {
                crate::autostart::disable()
            } else {
                crate::autostart::enable()
            };
            notice(app, r);
            refresh_menu(app); // refresh the check marks
        }
        "quit" => app.exit(0),
        _ if id.starts_with("lang-") => crate::apply_lang(app, &id[5..]),
        "show-git" => crate::toggle_git_status(app),
        _ if id.starts_with("prov-") => crate::toggle_provider(app, &id[5..]),
        _ if id.starts_with("alert-") => crate::threshold::toggle_mute(app, &id[6..]),
        _ if id.starts_with("edge-") => crate::apply_edge(app, &id[5..]),
        _ if id.starts_with("accent-") => crate::apply_accent(app, &id[7..]),
        _ => {}
    }
}

fn notice(app: &AppHandle, r: Result<String, String>) {
    let msg = match r {
        Ok(m) => m,
        Err(e) => format!("Error: {e}"),
    };
    let _ = app.emit("notice", &msg);
}
