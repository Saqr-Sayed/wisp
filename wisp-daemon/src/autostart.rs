use std::path::{Path, PathBuf};

pub fn autostart_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".config/autostart/wisp.desktop")
}

pub fn desktop_content(exe: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Wisp\nExec={exe}\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n"
    )
}

fn current_exe() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn install_to(path: &Path, exe: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, desktop_content(exe))
}

pub fn uninstall_at(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub fn install_autostart() -> std::io::Result<()> {
    install_to(&autostart_path(), &current_exe())
}

pub fn uninstall_autostart() -> std::io::Result<()> {
    uninstall_at(&autostart_path())
}

pub fn autostart_enabled() -> bool {
    autostart_path().exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("wisp-autostart-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn desktop_content_matches_spec_fields() {
        let out = desktop_content("/home/u/.local/bin/wisp-daemon");
        assert_eq!(
            out,
            "[Desktop Entry]\nType=Application\nName=Wisp\nExec=/home/u/.local/bin/wisp-daemon\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n"
        );
        assert!(!out.contains("OnlyShowIn"));
    }

    #[test]
    fn autostart_path_ends_with_config_autostart() {
        assert!(autostart_path().ends_with(".config/autostart/wisp.desktop"));
    }

    #[test]
    fn install_uninstall_roundtrip() {
        let dir = tmp_file("home");
        let file = dir.join(".config/autostart/wisp.desktop");
        install_to(&file, "/opt/wisp/wisp-daemon").unwrap();
        let body = std::fs::read_to_string(&file).unwrap();
        assert!(body.contains("Exec=/opt/wisp/wisp-daemon"));
        assert!(uninstall_at(&file).is_ok());
        assert!(!file.exists());
        assert!(uninstall_at(&file).is_ok()); // idempotent: missing file is not an error
        std::fs::remove_dir_all(&dir).ok();
    }
}
