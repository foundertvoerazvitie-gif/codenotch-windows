<div align="center">

# Codenotch for Windows

![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4)
![Rust](https://img.shields.io/badge/rust-Tauri%202-orange)
![License](https://img.shields.io/badge/license-MIT-green)
[![Telegram](https://img.shields.io/badge/Telegram-@aistanislav-26A5E4)](https://t.me/aistanislav)

**Usage notch on the screen edge:** how much of your AI allowance is left, and whether Claude is still working.

[Telegram @aistanislav](https://t.me/aistanislav) · updates, tips, Cursor/Codex limits

</div>

Windows port of [vinzdg/codenotch](https://github.com/vinzdg/codenotch) (MIT). Same idea as the macOS original — inverse-rounded pill, colour-graded rings, hover card — rebuilt in **Rust + Tauri 2 / WebView2**. Providers are reimplemented from documented APIs; Swift UI code is not copied.

## What it shows

| Cell | Source |
|---|---|
| **Claude** | Anthropic OAuth usage (`~/.claude/.credentials.json`) + session activity (hooks / transcript) |
| **Codex** | ChatGPT usage API via `~/.codex/auth.json`, else last rollout snapshot |
| **Cursor** | Editor session → `usage-summary`: **Cursor Models** (`autoPercentUsed`), **Other Models** (`apiPercentUsed`), optional **On demand** |
| **Antigravity** | Local bridge / Cloud Code quota, or today’s turn count |
| **Grok** | Grok CLI / SuperGrok credits (`~/.grok/auth.json` → `cli-chat-proxy.grok.com` billing); weekly Grok Build ring |

Hide unused providers from the tray. Missing installs simply omit a cell.

## Windows extras

- Russian / English UI  
- Edge: left / right / top / bottom (position remembered)  
- Accent: system colour or presets  
- Toasts at 80% / 100% (mute per provider)  
- Tray → **Telegram @aistanislav** + one-time first-run card  

## Build

Needs Rust (MSVC) and WebView2 (included on Windows 11).

```powershell
cd windows
cargo build --release
.\target\release\codenotch.exe
.\target\release\codenotch.exe doctor
```

Data / logs: `%APPDATA%\codenotch`.

More detail (layout, icons, tray): [`windows/README.md`](windows/README.md).

## Credits

- Design & macOS app: [vinzdg/codenotch](https://github.com/vinzdg/codenotch)  
- Earlier Windows work: [Im-Midi/codenotch-windows](https://github.com/Im-Midi/codenotch-windows)  
- Session engine lineage: [Im-Midi/Pac-Man](https://github.com/Im-Midi/Pac-Man)  

This repo’s `Sources/` tree is the upstream macOS app kept for reference; **the product here is the Windows port** under `windows/`.

## License

MIT — see [LICENSE](LICENSE) and [windows/LICENSE](windows/LICENSE).  
Codenotch design and name belong to the upstream author.
