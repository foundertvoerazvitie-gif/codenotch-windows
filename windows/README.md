# Codenotch for Windows

A Windows port of [Codenotch](https://github.com/vinzdg/codenotch) — the usage notch that
sits on the edge of your screen and answers two questions at a glance:
**how much of my AI allowance is left**, and **is Claude still working**.

Same design language as the macOS original (inverse-rounded pill, colour-graded rings,
hover card with per-window bars), rebuilt for Windows in Rust + Tauri 2 / WebView2.
No code is copied from the Swift app; the providers are reimplemented from their
documented behaviour and the wire formats.

**Updates & tips:** [Telegram @aistanislav](https://t.me/aistanislav)

## What it shows

| Cell | Source | How it reads it |
|---|---|---|
| **Claude** | `GET https://api.anthropic.com/api/oauth/usage` with the token Claude Code keeps in `~/.claude/.credentials.json` | Session / weekly windows, 429 back-off with a persisted deadline, stale readings dimmed with their age. A thin arc spins inside the ring while a Claude session is working, and pulses amber when one is waiting on you (Claude Code hooks + transcript watcher, desktop app included). |
| **Codex** | `GET https://chatgpt.com/backend-api/wham/usage` with the session Codex keeps in `~/.codex/auth.json` (read only, never refreshed), falling back to the `rate_limits` snapshot in the newest rollout log | Live primary/secondary windows (5h + weekly on paid plans, a monthly window on free) while Codex is signed in; otherwise the last snapshot, marked stale by its own timestamp. |
| **Cursor** | The editor's own session from `state.vscdb` → `cursor.com/api/usage-summary` | **Cursor Models** ← `autoPercentUsed`, **Other Models** ← `apiPercentUsed` (not the blended `totalPercentUsed`). Reset at billing-cycle end. Nothing to sign into: it borrows the editor's session. Composer is not a separate API field. |
| **Antigravity** | The local `language_server` bridge (quota summary), then Google's Cloud Code API for licensed accounts, then a plain count of today's model turns | Honest degradation: a percentage only when one exists, a `~count` when it does not. |

Providers that are not installed simply do not get a cell. Tray toggles can hide any provider.

## Windows extras (this port)

- Russian / English UI (tray + notch card)
- Edge placement: left / right / top / bottom, with remembered along-edge position
- Accent colour: Windows system accent or macOS-like presets
- Threshold toasts at 80% / 100% (per-provider mute in the tray)
- Soft community link: tray **Telegram @aistanislav** + one-time first-run card

## Install / build

Prerequisites: Rust (MSVC toolchain), WebView2 runtime (ships with Windows 11).

```powershell
# from this directory (`windows/` in the full upstream tree, or the repo root of this port)
cargo build --release
.\target\release\codenotch.exe          # pill on the primary monitor edge
.\target\release\codenotch.exe doctor   # credentials, data sources, icons, hooks
```

Tray menu: refresh, reset position, open data folder (`%APPDATA%\codenotch`), language,
providers, edge, accent, threshold mute, Telegram, start with Windows, Claude Code hooks.

### Icons

Provider marks are the SVGs from [`@lobehub/icons-static-svg`](https://github.com/lobehub/lobe-icons)
(MIT), embedded unmodified — see `codenotch/glyphs/NOTICE.md`. Drop your own
`claude|codex|cursor|gemini.svg` (or `.png`) into `%APPDATA%\codenotch\glyphs\` to override.
The marks remain the trademarks of their owners.

## Layout

```
.
├── codenotch/          Tauri 2 app: window, tray, providers, session engine
│   ├── src/            usage / codex / cursor / antigravity, glyphs, doctor, community
│   ├── ui/notch.html   the pill + hover card (single file, no framework)
│   └── glyphs/         provider marks (+ NOTICE.md)
└── codenotch-hook/     <5 ms hook messenger Claude Code calls; forwards events to the app
```

## Relationship to upstream

This port follows the upstream design spec and provider semantics. Credits:

- Original design / macOS app: [vinzdg/codenotch](https://github.com/vinzdg/codenotch) (MIT)
- Prior Windows work: [Im-Midi/codenotch-windows](https://github.com/Im-Midi/codenotch-windows)
- Session-detection engine lineage: [Im-Midi/Pac-Man](https://github.com/Im-Midi/Pac-Man) (MIT)

This fork adds the Windows extras above and ships community updates via [@aistanislav](https://t.me/aistanislav).

## License

MIT — see `LICENSE`. The Codenotch design and name belong to the upstream author.
