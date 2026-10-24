# Flemozi (Linux/Arch fork)

Rust + [iced] 0.14 emoji / emoticon / GIF picker. This fork supports Linux and
is packaged for Arch Linux. Other operating systems are not supported.

## Build & install (Arch)

```sh
packaging/arch/build-and-install.sh            # deps, makepkg, sudo pacman -U
packaging/arch/build-and-install.sh --no-install
cargo build / cargo run                         # dev
```

- `packaging/arch/PKGBUILD` builds from the local checkout (not a git source);
  `pkgver` comes from `Cargo.toml`. Installs the binary, `packaging/linux/flemozi.desktop`,
  icons and license.
- Edition 2024 → Rust ≥1.85. A small config test runs with `cargo test`; there
  is no linter/formatter config. GitHub Actions builds an x86_64 Arch package
  when a GitHub release is published.

## Klipy API key

The GIF tab uses the Klipy API
`https://api.klipy.com/api/v1/{key}/gifs/{trending,search}`. Users enter the
key in Settings; it is saved in the user config and is not embedded at build
time. `KLIPY_API_KEY` is an optional runtime environment fallback.

## Linux design (`src/linux.rs`)

Wayland forbids global key hooks, global hotkeys, window positioning and
re-showing hidden windows, so on Linux:

- The app is an `iced::daemon`; "show" opens a window, "hide" closes it.
- Keys come from iced keyboard events and are mapped to `HookKeyEvent` for
  handling in `app.rs`.
- Single instance via Unix socket `$XDG_RUNTIME_DIR/flemozi-$USER.sock`;
  `flemozi --toggle` / `--show` signal the running instance. Users bind
  `--toggle` in their DE shortcut settings. `global-hotkey` still registers
  (works on X11 only).
- Tray: `tray-icon` with `gtk` feature on its own thread running `gtk::main()`.
- Paste: clipboard via `arboard` (kept alive in a static, required on Linux),
  then best-effort Ctrl+V via `ydotool` / `wtype` / `xdotool`. GIFs go through
  `wl-clipboard-rs` as `text/uri-list` + `x-special/gnome-copied-files` +
  `image/gif` with **no** text/plain (else apps paste the path); X11 uses `xclip`.
- Theme: iced resets the system theme to `None` when a Wayland window opens;
  `State::system_theme` keeps the last real value from `system::theme_changes()`.
- Config: `$XDG_CONFIG_HOME/flemozi/config.json`; autostart via
  `~/.config/autostart/flemozi.desktop` (`--hidden`).

## Architecture

| Path | Role |
|---|---|
| `src/main.rs` | Linux entrypoint; CLI/IPC + `iced::daemon` |
| `src/app.rs` | State, messages, update & subscriptions |
| `src/linux.rs` | Linux backend (see above) |
| `src/config.rs` | Config load/save, launch-at-startup |
| `src/tabs/` | Emoji, emoticon and GIF (Klipy) logic |
| `src/ui.rs`, `src/ui/` | Views and styles |
| `src/assets/emojis.rs` | Static emoji dataset; rendered with embedded `assets/NotoColorEmoji.ttf` |
