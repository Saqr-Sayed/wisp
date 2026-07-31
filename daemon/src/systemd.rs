use std::path::PathBuf;

fn service_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
        .join(".config/systemd/user/salmonella.service")
}

fn service_content() -> String {
    format!(
        "[Unit]\nDescription=Salmonella Activity Tracker\n\
         After=graphical-session.target\nBindsTo=graphical-session.target\n\n\
         [Service]\nType=dbus\nBusName=com.Saqr.Salomnella\n\
         ExecStart={}/salmonella-daemon\nRestart=on-failure\nRestartSec=2\n\n\
         [Install]\nWantedBy=graphical-session.target",
        std::env::current_exe().unwrap_or_default().parent().unwrap_or(&PathBuf::from(".")).display()
    )
}

pub fn install() {
    let path = service_path();
    std::fs::create_dir_all(path.parent().unwrap()).ok();
    std::fs::write(&path, service_content()).ok();
    std::process::Command::new("systemctl")
        .args(["--user", "enable", "--now", "salmonella.service"])
        .output().ok();
}
