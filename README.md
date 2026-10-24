<p align="center"><img width="160" src="assets/logo.png"></p>
<h1 align="center">Flemozi for Arch Linux</h1>

<p align="center">
  A fast, lightweight emoji, emoticon and GIF picker — packaged for Arch Linux.
</p>

This is a fork of [KRTirtho/flemozi](https://github.com/KRTirtho/flemozi) (the
Rust/[iced](https://iced.rs/) rewrite) that adds a Linux backend and a local
Arch package. The `flemozi` package on the AUR builds the old Flutter version
and no longer builds.

Tested on CachyOS / KDE Plasma (Wayland).

![Screenshot](assets/screenshots/one.png)

## Build & install

```sh
git clone https://github.com/lyubomirhristovv/flemozi.git
cd flemozi
packaging/arch/build-and-install.sh
```

The script installs missing build dependencies (`base-devel`, `rustup`, `gtk3`,
`libayatana-appindicator`, …), builds the package with `makepkg`, and installs
it with `sudo pacman -U`. If Flemozi is running, it is restarted.

| Option         | Effect                                        |
| -------------- | --------------------------------------------- |
| `--no-install` | Only build; package lands in `packaging/arch/` |
| `--clean`      | Discard cached build output (full rebuild)    |

### GitHub Actions package

Publishing a GitHub release builds an x86_64 Arch package and attaches the
`.pkg.tar.zst` package to that release. To install it:

1. Open the repository's [Releases](https://github.com/lyubomirhristovv/flemozi/releases).
2. Open the release and, after the **Build Arch package** workflow finishes,
   download the `.pkg.tar.zst` file from **Assets**.
3. Open a terminal in the downloaded file's folder and run:

```sh
sudo pacman -U ./flemozi-*.pkg.tar.zst
```

Install the package from a newer release the same way to update. To uninstall:
`sudo pacman -R flemozi`.

### GIF API key

The GIF tab uses [Klipy](https://klipy.com). Create a free app key at
<https://partner.klipy.com>, open **Settings**, paste it into the **Klipy API
key** field, and select **Save key**. Flemozi stores it in the user's
`config.json` as plain text, with owner-only file permissions on Unix; the key
is not embedded in the executable. Alternatively, set `KLIPY_API_KEY` in
Flemozi's runtime environment.

## Usage

- Launch **Flemozi** from the app menu, or run `flemozi`. It lives in the tray.
- **Shortcut:** Wayland does not allow apps to grab global hotkeys. In
  *System Settings → Keyboard → Shortcuts → Add New → Command*, add
  `flemozi --toggle` and bind it (e.g. <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>.</kbd>).
  On X11 the built-in shortcut (<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>.</kbd>) also works.
- Type to search, arrow keys to move, <kbd>Enter</kbd> to pick, <kbd>Esc</kbd> to close.
- The pick is copied to the clipboard. For automatic pasting into the previous
  window install one of:
  - `ydotool` (Wayland, any compositor; needs `ydotoold` running), or
  - `wtype` (wlroots compositors), or
  - `xdotool` (X11).

  Otherwise just press <kbd>Ctrl</kbd>+<kbd>V</kbd>.
- **GIFs** are copied as a file (like copying it in Dolphin) plus raw
  `image/gif` data, so chat apps attach the GIF instead of pasting a path.
  Temporary files live in `/tmp/flemozi-$USER/` and are cleaned after an hour.
  On X11 this needs `xclip`.
- **Settings → Launch at startup** writes `~/.config/autostart/flemozi.desktop`
  (starts hidden in the tray).
- Follows the system light/dark theme.

| CLI flag   | Effect                                         |
| ---------- | ---------------------------------------------- |
| `--toggle` | Show/hide the picker of the running instance   |
| `--show`   | Show the picker (default when already running) |
| `--hidden` | Start in the tray without opening the picker   |

Config: `~/.config/flemozi/config.json`.

## Development

```sh
cargo run      # debug build; needs the same deps as above
```

See [AGENTS.md](AGENTS.md) for architecture notes.

## Credits & license

Original app by Kingkor Roy Tirtho ([@KRTirtho](https://github.com/KRTirtho)).
Emoji rendering uses [Noto Color Emoji](https://github.com/googlefonts/noto-emoji).

Licensed under the [GPLv3](LICENSE).
