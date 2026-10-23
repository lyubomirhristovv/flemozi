# Flemozi (Arch Linux fork)

Rust + [iced] 0.14 emoji / emoticon / GIF picker. Fork whose purpose is Linux
(Arch) support; Windows/macOS code from upstream is kept but not built or
tested here. **Not Flutter** — ignore the Flutter-era `Makefile` and
`CHANGELOG.md`.

## Build & install (Arch)

```sh
packaging/arch/build-and-install.sh            # deps, makepkg, sudo pacman -U
packaging/arch/build-and-install.sh --no-install
cargo build / cargo run                         # dev
```

- `packaging/arch/PKGBUILD` builds from the local checkout (not a git source);
  `pkgver` comes from `Cargo.toml`. Installs the binary, `packaging/linux/flemozi.desktop`,
  icons and license.
- Edition 2024 → Rust ≥1.85. No tests, no CI, no linter/formatter config.

## `.env` / API key

`build.rs` reads `.env` and embeds values XOR-obfuscated via
`OUT_DIR/env_vars.rs`. Only `KLIPY_API_KEY` is used (GIF tab, Klipy API
`https://api.klipy.com/api/v1/{key}/gifs/{trending,search}`); the
`KLIPY_API_KEY` env var is a runtime fallback. `.env` is gitignored — never commit it.

## Linux design (`src/linux.rs`)

Wayland forbids global key hooks, global hotkeys, window positioning and
re-showing hidden windows, so on Linux:

- The app is an `iced::daemon`; "show" opens a window, "hide" closes it.
- Keys come from iced keyboard events, mapped to the same `HookKeyEvent` the
  Win/mac hooks produce, so `app.rs` key handling is shared.
- Single instance via Unix socket `$XDG_RUNTIME_DIR/flemozi-$USER.sock`;
  `flemozi --toggle` / `--show` signal the running instance. Users bind
  `--toggle` in their DE shortcut settings. `global-hotkey` still registers
  (works on X11 only).
- Tray: `tray-icon` with `gtk` feature on its own thread running `gtk::main()`.
- Paste: clipboard via `arboard` (kept alive in a static, required on Linux),
  then best-effort Ctrl+V via `ydotool` / `wtype` / `xdotool`.
- Theme: iced resets the system theme to `None` when a Wayland window opens;
  `State::system_theme` keeps the last real value from `system::theme_changes()`.
- Config: `$XDG_CONFIG_HOME/flemozi/config.json`; autostart via
  `~/.config/autostart/flemozi.desktop` (`--hidden`).

## Architecture

| Path | Role |
|---|---|
| `src/main.rs` | Entrypoint; Linux CLI/IPC + `iced::daemon`, else `iced::application` |
| `src/app.rs` | State, messages, update & subscriptions (`show`/`hide` per platform) |
| `src/linux.rs` | Linux backend (see above) |
| `src/win32.rs` / `src/macos.rs` | Upstream Windows / macOS backends |
| `src/config.rs` | Config load/save, launch-at-startup |
| `src/tabs/` | Emoji, emoticon, GIF (Klipy) and settings tabs |
| `src/ui.rs`, `src/ui/` | Views and styles |
| `src/assets/emojis.rs` | Static emoji dataset; rendered with embedded `assets/NotoColorEmoji.ttf` |
