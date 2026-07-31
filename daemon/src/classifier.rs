const MEDIA_APPS: &[&str] = &["mpv", "vlc", "totem", "celluloid", "ffplay", "smplayer", "io.mpv.Mpv"];
const VIDEO_EXTS: &[&str] = &[".mp4", ".mkv", ".avi", ".mov", ".webm", ".m4v", ".flv"];

pub fn classify(app_name: &str, window_title: &str) -> &'static str {
    let app = app_name.to_lowercase();
    let title = window_title.to_lowercase();

    if matches!(title.as_str(), "__boot__" | "__shutdown__" | "__sleep__" | "__wake__") {
        return "system";
    }
    if MEDIA_APPS.iter().any(|m| app.contains(m)) {
        return "media";
    }
    if VIDEO_EXTS.iter().any(|e| title.contains(e)) {
        return "media";
    }
    "app"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn media_app() { assert_eq!(classify("mpv", "x.mp4"), "media"); }
    #[test] fn media_title() { assert_eq!(classify("firefox", "x.mp4"), "media"); }
    #[test] fn regular_app() { assert_eq!(classify("Code", "main.rs"), "app"); }
    #[test] fn system_event() { assert_eq!(classify("", "__boot__"), "system"); }
}
