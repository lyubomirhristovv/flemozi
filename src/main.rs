#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod assets;
mod config;
mod emoji;
mod emoticon;
mod env;
mod search;
mod tabs;

mod ui;
#[cfg(target_os = "windows")]
mod win32;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;

use app::Flemozi;

pub fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    #[cfg(target_os = "macos")]
    unsafe {
        macos::set_activation_policy_accessory();
    }

    #[cfg(target_os = "linux")]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.iter().any(|a| a == "-h" || a == "--help") {
            println!(
                "Usage: flemozi [--toggle | --show | --hidden]\n\n  \
                 --toggle  Show/hide the picker of the running instance (bind this to a shortcut)\n  \
                 --show    Show the picker of the running instance\n  \
                 --hidden  Start in the background (tray only)"
            );
            return Ok(());
        }
        let cmd = if args.iter().any(|a| a == "--toggle") { "toggle" } else { "show" };
        if linux::send_to_running_instance(cmd) {
            return Ok(());
        }
        app::set_start_hidden(args.iter().any(|a| a == "--hidden"));
        linux::start_ipc_server();
        let result = daemon().run();
        linux::cleanup_ipc();
        return result;
    }

    #[cfg(not(target_os = "linux"))]
    application().run()
}

pub fn window_settings() -> iced::window::Settings {
    use iced::window;

    window::Settings {
        size: iced::Size::new(500.0, 500.0),
        #[cfg(target_os = "linux")]
        position: window::Position::Centered,
        #[cfg(not(target_os = "linux"))]
        position: window::Position::Default,
        min_size: None,
        max_size: None,
        visible: cfg!(target_os = "linux"),
        resizable: false,
        closeable: false,
        minimizable: false,
        decorations: false,
        transparent: true,
        level: window::Level::AlwaysOnTop,
        #[cfg(target_os = "linux")]
        platform_specific: window::settings::PlatformSpecific {
            application_id: "flemozi".to_string(),
            override_redirect: false,
        },
        ..Default::default()
    }
}

#[cfg(not(target_os = "linux"))]
fn application() -> iced::Application<impl iced::Program<Message = app::Message, Theme = iced::Theme>> {
    iced::application(Flemozi::new, Flemozi::update, Flemozi::view)
        .subscription(Flemozi::subscription)
        .title(Flemozi::title)
        .window(window_settings())
        .exit_on_close_request(false)
}

#[cfg(target_os = "linux")]
fn daemon() -> iced::Daemon<impl iced::Program<Message = app::Message, Theme = iced::Theme>> {
    iced::daemon(Flemozi::new, Flemozi::update, Flemozi::view_window)
        .subscription(Flemozi::subscription)
        .title(Flemozi::title_window)
        .theme(Flemozi::theme_window)
}
