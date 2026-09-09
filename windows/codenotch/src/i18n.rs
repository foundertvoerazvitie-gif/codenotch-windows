//! Rust-side (tray menu) strings. The page has its own dictionary; keys are kept identical on both sides.

pub fn resolve_auto() -> &'static str {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Globalization::GetUserDefaultLocaleName;
        let mut buf = [0u16; 85];
        let n = GetUserDefaultLocaleName(&mut buf);
        if n > 0 {
            let name = String::from_utf16_lossy(&buf[..(n as usize - 1)]).to_lowercase();
            if name.starts_with("zh") {
                return "zh";
            }
            if name.starts_with("ja") {
                return "ja";
            }
            if name.starts_with("ko") {
                return "ko";
            }
            if name.starts_with("ru") {
                return "ru";
            }
        }
    }
    "en"
}

pub fn tr(lang: &str, key: &str) -> &'static str {
    let l = if lang == "auto" { resolve_auto() } else { lang };
    match (l, key) {
        ("zh", "install") => "安装 Claude Code 钩子",
        ("zh", "uninstall") => "卸载钩子",
        ("zh", "language") => "语言",
        ("zh", "lang_auto") => "跟随系统",
        ("zh", "reset_pos") => "重置悬浮条位置",
        ("zh", "quit") => "退出",
        ("zh", "hooks_missing") => "钩子未安装：右键托盘图标 → 安装 Claude Code 钩子（桌面版无需，已自动兜底）",
        ("zh", "autostart") => "开机自启（静默待命）",
        ("zh", "refresh") => "立即刷新用量",
        ("zh", "open_data") => "打开数据文件夹（日志 / 图标）",

        ("ja", "install") => "Claude Code フックを導入",
        ("ja", "uninstall") => "フックを削除",
        ("ja", "language") => "言語",
        ("ja", "lang_auto") => "システムに従う",
        ("ja", "reset_pos") => "バー位置をリセット",
        ("ja", "quit") => "終了",
        ("ja", "hooks_missing") => "フック未導入：トレイ右クリック → フックを導入（デスクトップ版は自動フォールバック済み）",
        ("ja", "autostart") => "Windows起動時に自動開始",
        ("ja", "refresh") => "使用量を今すぐ更新",
        ("ja", "open_data") => "データフォルダを開く（ログ / アイコン）",

        ("ko", "install") => "Claude Code 후크 설치",
        ("ko", "uninstall") => "후크 제거",
        ("ko", "language") => "언어",
        ("ko", "lang_auto") => "시스템 따르기",
        ("ko", "reset_pos") => "바 위치 초기화",
        ("ko", "quit") => "종료",
        ("ko", "hooks_missing") => "후크 미설치: 트레이 우클릭 → 후크 설치 (데스크톱판은 자동 폴백)",
        ("ko", "autostart") => "Windows 시작 시 자동 실행",
        ("ko", "refresh") => "사용량 지금 새로고침",
        ("ko", "open_data") => "데이터 폴더 열기 (로그 / 아이콘)",

        ("ru", "install") => "Установить хуки Claude Code",
        ("ru", "uninstall") => "Удалить хуки",
        ("ru", "language") => "Язык",
        ("ru", "lang_auto") => "Как в системе",
        ("ru", "reset_pos") => "Сбросить позицию",
        ("ru", "quit") => "Выход",
        ("ru", "hooks_missing") => "Хуки не установлены: ПКМ по трею → Установить хуки Claude Code (для десктоп-приложения есть автозапасной путь)",
        ("ru", "autostart") => "Запускать с Windows (тихо)",
        ("ru", "refresh") => "Обновить лимиты сейчас",
        ("ru", "open_data") => "Открыть папку данных (логи / иконки)",
        ("ru", "telegram") => "Telegram",
        ("ru", "providers") => "Провайдеры",
        ("ru", "show") => "Показать",
        ("ru", "alerts") => "Алерты 80%/100%",
        ("ru", "placement") => "Положение",
        ("ru", "edge_right") => "Справа",
        ("ru", "edge_left") => "Слева",
        ("ru", "edge_top") => "Сверху",
        ("ru", "edge_bottom") => "Снизу",

        ("zh", "providers") => "服务商",
        ("zh", "show") => "显示",
        ("zh", "alerts") => "用量提醒 80%/100%",
        ("zh", "placement") => "位置",
        ("zh", "edge_right") => "右侧",
        ("zh", "edge_left") => "左侧",
        ("zh", "edge_top") => "顶部",
        ("zh", "edge_bottom") => "底部",
        ("ja", "providers") => "プロバイダー",
        ("ja", "show") => "表示",
        ("ja", "alerts") => "上限アラート 80%/100%",
        ("ja", "placement") => "配置",
        ("ja", "edge_right") => "右",
        ("ja", "edge_left") => "左",
        ("ja", "edge_top") => "上",
        ("ja", "edge_bottom") => "下",
        ("ko", "providers") => "프로바이더",
        ("ko", "show") => "표시",
        ("ko", "alerts") => "한도 알림 80%/100%",
        ("ko", "placement") => "위치",
        ("ko", "edge_right") => "오른쪽",
        ("ko", "edge_left") => "왼쪽",
        ("ko", "edge_top") => "위",
        ("ko", "edge_bottom") => "아래",
        (_, "providers") => "Providers",
        (_, "show") => "Show",
        (_, "alerts") => "Alerts 80%/100%",
        (_, "placement") => "Placement",
        (_, "edge_right") => "Right",
        (_, "edge_left") => "Left",
        (_, "edge_top") => "Top",
        (_, "edge_bottom") => "Bottom",
        ("ru", "accent") => "Акцент",
        ("ru", "accent_system") => "Цвет Windows",
        ("ru", "accent_pink") => "Розовый",
        ("ru", "accent_red") => "Красный",
        ("ru", "accent_orange") => "Оранжевый",
        ("ru", "accent_yellow") => "Жёлтый",
        ("ru", "accent_green") => "Зелёный",
        ("ru", "accent_teal") => "Бирюзовый",
        ("ru", "accent_blue") => "Синий",
        ("ru", "accent_indigo") => "Индиго",
        ("ru", "accent_purple") => "Фиолетовый",
        ("ru", "accent_offwhite") => "Светлый",
        ("zh", "accent") => "强调色",
        ("zh", "accent_system") => "跟随系统",
        ("ja", "accent") => "アクセント",
        ("ja", "accent_system") => "システム",
        ("ko", "accent") => "강조색",
        ("ko", "accent_system") => "시스템",
        (_, "accent") => "Accent",
        (_, "accent_system") => "Windows accent",
        (_, "accent_pink") => "Pink",
        (_, "accent_red") => "Red",
        (_, "accent_orange") => "Orange",
        (_, "accent_yellow") => "Yellow",
        (_, "accent_green") => "Green",
        (_, "accent_teal") => "Teal",
        (_, "accent_blue") => "Blue",
        (_, "accent_indigo") => "Indigo",
        (_, "accent_purple") => "Purple",
        (_, "accent_offwhite") => "Off-white",

        (_, "install") => "Install Claude Code hooks",
        (_, "uninstall") => "Uninstall hooks",
        (_, "language") => "Language",
        (_, "lang_auto") => "Follow system",
        (_, "reset_pos") => "Reset bar position",
        (_, "quit") => "Quit",
        (_, "hooks_missing") => "Hooks not installed: tray right-click → Install Claude Code hooks (desktop app auto-fallback active)",
        (_, "autostart") => "Start with Windows (silent)",
        (_, "refresh") => "Refresh usage now",
        (_, "open_data") => "Open data folder (logs / icons)",
        (_, "telegram") => "Telegram",
        _ => "?",
    }
}
