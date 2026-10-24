use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub launch_at_startup: bool,
    pub shortcut_ctrl: bool,
    pub shortcut_alt: bool,
    pub shortcut_shift: bool,
    pub shortcut_code: String,
    #[serde(default)]
    pub klipy_api_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            launch_at_startup: false,
            shortcut_ctrl: true,
            shortcut_alt: true,
            shortcut_shift: false,
            shortcut_code: "Period".to_string(),
            klipy_api_key: String::new(),
        }
    }
}

impl Config {
    #[cfg(target_os = "linux")]
    fn path() -> PathBuf {
        xdg_config_home().join("flemozi").join("config.json")
    }

    #[cfg(not(target_os = "linux"))]
    fn path() -> PathBuf {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(appdata).join("flemozi").join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }

        let serialized = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let mut options = fs::OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        let mut file = options.open(&path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(&serialized)
    }

    #[cfg(target_os = "linux")]
    pub fn apply_launch_at_startup(&self) {
        let dir = xdg_config_home().join("autostart");
        let file = dir.join("flemozi.desktop");
        if self.launch_at_startup {
            let exe = std::env::current_exe()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "flemozi".to_string());
            let entry = format!(
                "[Desktop Entry]\nType=Application\nName=Flemozi\nExec=\"{exe}\" --hidden\n\
                 Icon=flemozi\nX-GNOME-Autostart-enabled=true\n"
            );
            let _ = fs::create_dir_all(&dir);
            let _ = fs::write(&file, entry);
        } else {
            let _ = fs::remove_file(&file);
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn apply_launch_at_startup(&self) {
        let startup = std::env::var("APPDATA")
            .map(|a| PathBuf::from(a).join(r"Microsoft\Windows\Start Menu\Programs\Startup"))
            .unwrap_or_default();
        let lnk = startup.join("Flemozi.lnk");

        if self.launch_at_startup {
            if let Ok(exe) = std::env::current_exe() {
                let _ = fs::create_dir_all(&startup);
                let script = format!(
                    "$ws=(New-Object -ComObject WScript.Shell).CreateShortcut('{}');\
                     $ws.TargetPath='{}';\
                     $ws.Save()",
                    lnk.to_string_lossy().replace('\'', "''"),
                    exe.to_string_lossy().replace('\'', "''"),
                );
                let _ = std::process::Command::new("powershell")
                    .args(["-NoProfile", "-Command", &script])
                    .output();
            }
        } else {
            let _ = fs::remove_file(&lnk);
        }
    }
}

#[cfg(target_os = "linux")]
fn xdg_config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn missing_klipy_key_defaults_to_empty() {
        let config: Config = serde_json::from_str(
            r#"{
                "launch_at_startup": false,
                "shortcut_ctrl": true,
                "shortcut_alt": true,
                "shortcut_shift": false,
                "shortcut_code": "Period"
            }"#,
        )
        .unwrap();

        assert!(config.klipy_api_key.is_empty());
    }
}
