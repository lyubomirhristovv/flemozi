//! Linux backend.
//!
//! Wayland forbids global key hooks, global hotkeys, window positioning and
//! re-showing hidden windows, so on Linux the picker is a normal focused
//! window that receives keys through iced. Showing/hiding is done by opening
//! and closing the window (see `app.rs`), and the "global hotkey" is the
//! `flemozi --toggle` command, which the user binds in their desktop's
//! shortcut settings. It talks to the running instance over a Unix socket.

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tracing::{error, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookKey {
    Char(char),
    Backspace,
    Delete,
    Home,
    End,
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HookKeyEvent {
    pub key: HookKey,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

static HOOK_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_hook_active(active: bool) {
    HOOK_ACTIVE.store(active, Ordering::SeqCst);
}

pub fn is_hook_active() -> bool {
    HOOK_ACTIVE.load(Ordering::SeqCst)
}

/// Keys arrive through iced events, so there is nothing to poll.
pub fn try_recv_hook_key() -> Option<HookKeyEvent> {
    None
}

pub fn hook_event_from_iced(
    key: &iced::keyboard::Key,
    text: Option<&str>,
    modifiers: iced::keyboard::Modifiers,
) -> Option<HookKeyEvent> {
    use iced::keyboard::Key;
    use iced::keyboard::key::Named;

    let ctrl = modifiers.control();
    let shift = modifiers.shift();
    let alt = modifiers.alt();

    let hk = match key {
        Key::Named(Named::Backspace) => HookKey::Backspace,
        Key::Named(Named::Delete) => HookKey::Delete,
        Key::Named(Named::Home) => HookKey::Home,
        Key::Named(Named::End) => HookKey::End,
        Key::Named(Named::ArrowUp) => HookKey::Up,
        Key::Named(Named::ArrowDown) => HookKey::Down,
        Key::Named(Named::ArrowLeft) => HookKey::Left,
        Key::Named(Named::ArrowRight) => HookKey::Right,
        Key::Named(Named::Enter) => HookKey::Enter,
        Key::Named(Named::Escape) => HookKey::Escape,
        Key::Named(Named::Space) => HookKey::Char(' '),
        Key::Character(c) => {
            // With Ctrl held `text` is a control code, so use the key itself.
            let s = if ctrl { Some(c.as_str()) } else { text.or(Some(c.as_str())) };
            let ch = s.and_then(|s| {
                let mut it = s.chars();
                let ch = it.next()?;
                it.next().is_none().then_some(ch)
            })?;
            // The query cursor is byte-based, so only accept ASCII input.
            if !(ch.is_ascii_graphic() || ch == ' ') {
                return None;
            }
            HookKey::Char(if ctrl { ch.to_ascii_lowercase() } else { ch })
        }
        _ => return None,
    };

    Some(HookKeyEvent { key: hk, ctrl, shift, alt })
}

// ── single-instance IPC ──

fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let user = std::env::var("USER").unwrap_or_default();
    dir.join(format!("flemozi-{user}.sock"))
}

/// Sends `cmd` to an already-running instance. Returns `true` if one answered.
pub fn send_to_running_instance(cmd: &str) -> bool {
    match UnixStream::connect(socket_path()) {
        Ok(mut s) => {
            let _ = s.write_all(cmd.as_bytes());
            true
        }
        Err(_) => false,
    }
}

static IPC_RX: OnceLock<Mutex<Receiver<String>>> = OnceLock::new();

pub fn start_ipc_server() {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            error!("start_ipc_server: bind {path:?} failed: {e}");
            return;
        }
    };
    let (tx, rx): (Sender<String>, Receiver<String>) = channel();
    let _ = IPC_RX.set(Mutex::new(rx));
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            let mut buf = String::new();
            let _ = stream.read_to_string(&mut buf);
            let cmd = buf.trim().to_string();
            info!("ipc: received {cmd:?}");
            if tx.send(cmd).is_err() {
                break;
            }
        }
    });
}

pub fn cleanup_ipc() {
    let _ = std::fs::remove_file(socket_path());
}

pub fn try_recv_ipc() -> Option<String> {
    IPC_RX.get()?.lock().ok()?.try_recv().ok()
}

// ── tray ──

/// tray-icon on Linux needs a running GTK main loop on the creating thread.
pub fn spawn_tray(icon_rgba: Vec<u8>, width: u32, height: u32) {
    use tray_icon::TrayIconBuilder;
    use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};

    std::thread::spawn(move || {
        if let Err(e) = gtk::init() {
            warn!("spawn_tray: gtk init failed, no tray icon: {e}");
            return;
        }
        let Ok(icon) = tray_icon::Icon::from_rgba(icon_rgba, width, height) else {
            warn!("spawn_tray: invalid icon");
            return;
        };
        let show = MenuItem::with_id(MenuId::new("show-window"), "Show Window", true, None);
        let exit = MenuItem::with_id(MenuId::new("exit"), "Exit", true, None);
        let Ok(menu) = Menu::with_items(&[&show, &PredefinedMenuItem::separator(), &exit]) else {
            return;
        };
        match TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip("Flemozi")
            .with_menu(Box::new(menu))
            .build()
        {
            Ok(tray) => {
                gtk::main();
                drop(tray);
            }
            Err(e) => warn!("spawn_tray: failed to build tray icon: {e}"),
        }
    });
}

// ── clipboard & paste ──

// arboard on Linux only serves the selection while the Clipboard is alive.
static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

fn with_clipboard<R>(f: impl FnOnce(&mut arboard::Clipboard) -> R) -> Option<R> {
    let mut guard = CLIPBOARD.lock().ok()?;
    if guard.is_none() {
        match arboard::Clipboard::new() {
            Ok(c) => *guard = Some(c),
            Err(e) => {
                error!("clipboard: open failed: {e}");
                return None;
            }
        }
    }
    guard.as_mut().map(f)
}

fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

fn run_quiet(prog: &str, args: &[&str]) -> bool {
    Command::new(prog)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Best-effort Ctrl+V into the previously focused window.
fn simulate_paste() -> bool {
    let attempts: &[(&str, &[&str])] = if is_wayland() {
        &[
            ("ydotool", &["key", "29:1", "47:1", "47:0", "29:0"]),
            ("wtype", &["-M", "ctrl", "v", "-m", "ctrl"]),
            ("xdotool", &["key", "--clearmodifiers", "ctrl+v"]),
        ]
    } else {
        &[
            ("xdotool", &["key", "--clearmodifiers", "ctrl+v"]),
            ("ydotool", &["key", "29:1", "47:1", "47:0", "29:0"]),
        ]
    };
    for (prog, args) in attempts {
        if run_quiet(prog, args) {
            info!("simulate_paste: pasted via {prog}");
            return true;
        }
    }
    info!("simulate_paste: no working paste tool; content left on clipboard");
    false
}

pub unsafe fn paste_emoji(text: &str) {
    let text = text.to_owned();
    std::thread::spawn(move || {
        let saved = with_clipboard(|c| c.get_text().ok()).flatten();
        if with_clipboard(|c| c.set_text(text.clone())).and_then(|r| r.ok()).is_none() {
            error!("paste_emoji: failed to set clipboard");
            return;
        }
        // Give the compositor time to refocus the previous window.
        std::thread::sleep(Duration::from_millis(250));
        if simulate_paste() {
            std::thread::sleep(Duration::from_millis(300));
            if let Some(prev) = saved {
                let _ = with_clipboard(|c| c.set_text(prev));
            }
        }
    });
}

/// Puts the GIF on the clipboard the way file managers copy a file (uri-list)
/// plus the raw `image/gif` bytes, with no plain-text fallback so apps can't
/// paste the path as text.
pub unsafe fn paste_gif_file(bytes: &[u8]) {
    let path = match write_temp_gif(bytes) {
        Ok(p) => p,
        Err(e) => {
            error!("paste_gif_file: failed to write temp file: {e}");
            return;
        }
    };
    let uri = file_uri(&path);
    let bytes = bytes.to_vec();
    std::thread::spawn(move || {
        let ok = if is_wayland() {
            set_gif_clipboard_wayland(&uri, bytes)
        } else {
            set_gif_clipboard_x11(&uri)
        };
        if !ok {
            error!("paste_gif_file: failed to set clipboard");
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
        simulate_paste();
    });
}

fn set_gif_clipboard_wayland(uri: &str, gif: Vec<u8>) -> bool {
    use wl_clipboard_rs::copy::{MimeSource, MimeType, Options, Source};

    let mime = |data: Vec<u8>, ty: &str| MimeSource {
        source: Source::Bytes(data.into_boxed_slice()),
        mime_type: MimeType::Specific(ty.to_string()),
    };
    let sources = vec![
        mime(format!("{uri}\r\n").into_bytes(), "text/uri-list"),
        mime(format!("copy\n{uri}").into_bytes(), "x-special/gnome-copied-files"),
        mime(gif, "image/gif"),
    ];
    let mut options = Options::new();
    // text/uri-list counts as text, so the library would otherwise also offer
    // text/plain with the path — exactly what apps then paste.
    options.omit_additional_text_mime_types(true);
    match options.copy_multi(sources) {
        Ok(()) => true,
        Err(e) => {
            error!("set_gif_clipboard_wayland: {e}");
            false
        }
    }
}

/// arboard can't offer custom MIME types on X11, so use xclip when present.
fn set_gif_clipboard_x11(uri: &str) -> bool {
    let child = Command::new("xclip")
        .args(["-selection", "clipboard", "-t", "text/uri-list"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match child {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(format!("{uri}\r\n").as_bytes());
            }
            // xclip forks to serve the selection; the parent exits once stdin closes.
            child.wait().map(|s| s.success()).unwrap_or(false)
        }
        Err(e) => {
            error!("set_gif_clipboard_x11: xclip not available ({e}); install xclip to paste GIFs on X11");
            false
        }
    }
}

/// Writes the GIF to a per-user temp dir under a unique name, so a paste that
/// is still uploading isn't overwritten. Files older than an hour are removed.
fn write_temp_gif(bytes: &[u8]) -> std::io::Result<PathBuf> {
    let user = std::env::var("USER").unwrap_or_default();
    let dir = std::env::temp_dir().join(format!("flemozi-{user}"));
    std::fs::create_dir_all(&dir)?;

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let old = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age > Duration::from_secs(3600));
            if old {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    let path = dir.join(format!("flemozi-{stamp}.gif"));
    std::fs::write(&path, bytes)?;
    Ok(path)
}

fn file_uri(path: &std::path::Path) -> String {
    let mut uri = String::from("file://");
    for b in path.to_string_lossy().bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) {
            uri.push(b as char);
        } else {
            uri.push_str(&format!("%{b:02X}"));
        }
    }
    uri
}
