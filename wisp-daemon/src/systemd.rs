use std::path::PathBuf;

fn service_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
        .join(".config/systemd/user/wisp.service")
}

fn service_content() -> String {
    format!(
        "[Unit]\nDescription=Wisp Activity Tracker\nAfter=default.target\n\n\
         [Service]\nType=dbus\nBusName=com.saqr.wisp\n\
         ExecStart={}/wisp-daemon\nRestart=on-failure\nRestartSec=2\n\n\
         [Install]\nWantedBy=default.target",
        std::env::current_exe().unwrap_or_default().parent().unwrap_or(&PathBuf::from(".")).display()
    )
}

pub fn install() {
    let path = service_path();
    std::fs::create_dir_all(path.parent().unwrap()).ok();
    if !path.exists() {
        std::fs::write(&path, service_content()).ok();
    }
    // ponytail: enable only once; never `--now` — starting the service from
    // within the daemon deadlocks (service waits on D-Bus name, daemon waits on systemctl)
    let is_enabled = std::process::Command::new("systemctl")
        .args(["--user", "is-enabled", "wisp.service"])
        .output()
        .ok();
    let enabled = is_enabled
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "enabled")
        .unwrap_or(false);
    if !enabled {
        std::process::Command::new("systemctl")
            .args(["--user", "enable", "wisp.service"])
            .output()
            .ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_unit_targets_default_target() {
        let content = service_content();
        assert!(content.contains("After=default.target"));
        assert!(content.contains("WantedBy=default.target"));
        assert!(!content.contains("graphical-session"));
        assert!(!content.contains("BindsTo"));
    }
}
