const MEDIA_APPS: &[&str] = &["mpv", "vlc", "totem", "celluloid", "ffplay", "smplayer", "io.mpv.Mpv",
    "showtime", "org.gnome.Showtime"];
const VIDEO_EXTS: &[&str] = &[".mp4", ".mkv", ".avi", ".mov", ".webm", ".m4v", ".flv"];
const VIDEO_PLAYERS: &[&str] = &["mpv", "vlc", "celluloid", "totem", "smplayer", "io.mpv.mpv",
    "showtime", "org.gnome.Showtime"];
const AUDIO_PLAYERS: &[&str] = &["spotify", "rhythmbox", "audacious",
    "lollypop", "strawberry", "cmus", "ncspot"];
const AUDIO_EXTS: &[&str] = &[".mp3", ".flac", ".ogg", ".m4a", ".opus", ".wav", ".aac"];
const AUDIO_SUFFIXES: &[&str] = &[" - spotify", " - lollypop", " - strawberry",
    " - cmus", " - audacious", " - ncspot", " - rhythmbox"];
const BROWSERS: &[&str] = &["firefox", "chrome", "chromium", "brave", "edge", "tor", "mozilla"];
const READERS: &[&str] = &["evince", "okular", "foliate", "xreader", "zathura", "calibre"];
const DOC_EXTS: &[&str] = &[".pdf", ".epub", ".djvu", ".mobi", ".azw3", ".txt", ".md"];
const GAME_APPS: &[&str] = &["steam", "lutris", "heroic", "wine", "games"];
const PROD_APPS: &[&str] = &["ptyxis", "gnome-terminal", "konsole", "alacritty", "kitty",
    "code", "codium", "cursor", "gedit", "gnome-text-editor", "emacs", "vim", "neovim",
    "libreoffice", "onlyoffice", "jetbrains", "idea", "obsidian"];
const MEDIA_SITES: &[&str] = &["youtube", "netflix", "shahid", "twitch", "tiktok", "vimeo",
    "youtu"];

const BUILTIN_NAMES: &[(&str, &str)] = &[
    ("org.mozilla.firefox.desktop", "فايرفوكس"),
    ("firefox", "فايرفوكس"),
    ("google-chrome.desktop", "كروم"),
    ("chromium.desktop", "كروم"),
    ("chrome", "كروم"),
    ("org.gnome.Ptyxis.desktop", "الطرفية"),
    ("ptyxis", "الطرفية"),
    ("org.gnome.Console.desktop", "الطرفية"),
    ("mpv.desktop", "مشغل الفيديو"),
    ("mpv", "مشغل الفيديو"),
    ("vlc.desktop", "مشغل الفيديو"),
    ("org.gnome.Nautilus.desktop", "الملفات"),
    ("nautilus", "الملفات"),
    ("org.gnome.TextEditor.desktop", "المحرر النصي"),
    ("gedit", "المحرر النصي"),
    ("code.desktop", "محرر الأكواد"),
    ("com.valvesoftware.Steam.desktop", "ستيم"),
    ("steam.desktop", "ستيم"),
    ("org.gnome.Evince.desktop", "قارئ المستندات"),
    ("okular.desktop", "قارئ المستندات"),
    ("org.gnome.Totem.desktop", "مشغل الفيديو"),
];

/// كل ما يمكن اشتقاقه من (تطبيق، عنوان) لحظة التسجيل.
pub struct Enriched {
    pub event_type: &'static str,
    pub category: &'static str,
    pub media_kind: &'static str,
    pub series_weak: bool,   // سلسلة فارغة أو سقوط الكلمة المفتاحية — لا يُخزَّن
    pub site: String,
    pub series: String,
    pub episode: String,
    /// العنوان بعد حذف لاحقة المتصفح/المشغل — ما يُعرض في الواجهة
    title_cleaned: String,
}

impl Enriched {
    pub fn title_cleaned(&self) -> &str {
        &self.title_cleaned
    }
}

pub fn classify(app_name: &str, window_title: &str) -> &'static str {
    enrich(app_name, window_title).event_type
}

fn strip_suffix(title: &str, suffixes: &[&str]) -> String {
    let lower = title.to_lowercase();
    for s in suffixes {
        let s_lower = s.to_lowercase();
        if lower.ends_with(s_lower.as_str()) {
            return title[..title.len() - s.len()].trim().to_string();
        }
    }
    // ponytail: عنوان مساوٍ للاحقة ذاتها بلا مسافة البادئة (mpv الخامل:
    // "- mpv" مقابل " - mpv") — ends_with لا يطابق، والمقصود قص كامل
    // (القرار 14: "فارغ بعد القص"). لا أثر على عناوين المحتوى الحقيقية.
    let t = lower.trim();
    if suffixes.iter().any(|s| t == s.trim()) {
        return String::new();
    }
    title.trim().to_string()
}

fn is_app(app: &str, list: &[&str]) -> bool {
    let a = app.to_lowercase();
    list.iter().any(|x| a.contains(x))
}

const FILE_MANAGERS: &[&str] = &["nautilus", "nemo", "thunar", "dolphin"];

/// مدير ملفات؟ الأسماء المجردة فقط — الاحتواء is_app يغطي المعرفات الكاملة
/// ("nautilus" ⊂ "org.gnome.nautilus.desktop").
pub fn is_file_manager(app: &str) -> bool { is_app(app, FILE_MANAGERS) }

/// عناوين عامة معروفة لا تمثل محتوى — أسماء نافذة افتراضية لمشغلات لا تعرض
/// اسم الملف. "foliate" حزام أمان زائد (مغطاة بقاعدة الاسم الصغير)؛
/// "video player" ضروري — short_name(Showtime) = "showtime" ≠ "video player".
const GENERIC_TITLES: &[&str] = &["video player", "foliate"];

/// اتحاد لواحق فرعي الصوت والفيديو في enrich — للقص اليدوي في
/// is_generic_title بلا استدعاء clean_title (انظر الملاحظة أدناه).
const PLAYER_SUFFIXES: &[&str] = &[" - mpv", " - VLC media player", " - Celluloid", " - Totem",
    " - spotify", " - lollypop", " - strawberry", " - cmus", " - audacious", " - ncspot",
    " - rhythmbox"];

/// عنوان عام لا يمثل محتوى؟: فارغ بعد قص لاحقة المشغل، أو يساوي الاسم الصغير
/// للتطبيق (آخر مقطع بعد النقطة بعد قص .desktop — غير حساس لحالة الأحرف)،
/// أو في قائمة GENERIC_TITLES (مقارنة بحرف صغير).
pub fn is_generic_title(app: &str, title: &str) -> bool {
    // ponytail: القص اليدوي بدل clean_title (انحراف محسوم عن القرار 14):
    // clean_title = enrich، وفروع enrich المحروسة تستدعي is_generic_title
    // لنفس (app, title) → عودية لا نهائية. نفس النتيجة لكل حالات الحارس —
    // اللواحق هنا هي نفسها التي تقصها فروع المشغلات.
    let cleaned = strip_suffix(title, PLAYER_SUFFIXES);
    let c = cleaned.trim().to_lowercase();
    c.is_empty() || c == short_name(app) || GENERIC_TITLES.iter().any(|g| c == *g)
}

/// مشغل وسائط؟ (فيديو أو صوت) — حارس تجاوز mime في المتتبع (القرار 9):
/// القراءة (Papers/Evince) والمتصفحات خارج القائمتين فلا يُتجاوز تصنيفها أبداً.
pub fn is_media_app(app: &str) -> bool { is_app(app, VIDEO_PLAYERS) || is_app(app, AUDIO_PLAYERS) }

/// يحوّل أرقاماً عربية-هندية (٢٦) إلى لاتينية (26) في النص.
fn normalize_digits(s: &str) -> String {
    s.chars().map(|c| match c {
        '٠' => '0', '١' => '1', '٢' => '2', '٣' => '3', '٤' => '4',
        '٥' => '5', '٦' => '6', '٧' => '7', '٨' => '8', '٩' => '9',
        '۰' => '0', '۱' => '1', '۲' => '2', '۳' => '3', '۴' => '4',
        '۵' => '5', '۶' => '6', '۷' => '7', '۸' => '8', '۹' => '9',
        c => c,
    }).collect()
}

/// `الدرس ٢٦ - mpv` → `(series="الدرس", episode="26", weak=false)`
/// weak=true: لا تطابق (سلسلة فارغة) أو سقوط الكلمة المفتاحية اسماً بلا اسم سابق.
fn parse_episode(raw: &str) -> (String, String, bool) {
    let t = normalize_digits(raw.trim());
    // أولوية: SxxEyy ثم EPn/Episode ثم نمط قديم 3x05 ثم كلمة عربية (قد تكون الكلمة نفسها اسم المسلسل)
    let patterns = [r"(?i)^(.+?)[\s.\-–—]*s(\d{1,2})e(\d{1,3})$",
                    r"(?i)^(.+?)[\s.\-–—]*ep(?:\.|isode)?\s*(\d{1,3})$",
                    r"^(?:(.+?)[\s.\-–—]*)?(\d{1,2})x(\d{1,3})$",
                    r"^(?:(.+?)[\s.\-–—]*)?(الحلقة|الدرس|الجزء)\s*(\d+)$"];
    for (i, pat) in patterns.iter().enumerate() {
        let re = regex::Regex::new(pat).unwrap();
        if let Some(c) = re.captures(&t) {
            let series = c.get(1).map(|m| m.as_str().trim().trim_end_matches('-').trim()).unwrap_or("")
                .to_string();
            if i == 3 {
                // النمط العربي: الكلمة تعمل اسماً للمسلسل إن لم يسبقها اسم
                let kw = c.get(2).map(|m| m.as_str()).unwrap_or("");
                let ep = c.get(3).map(|m| m.as_str()).unwrap_or("");
                let series = if series.is_empty() { kw.to_string() } else { series };
                let weak = c.get(1).is_none();   // الكلمة استُخدمت اسماً — سلسلة هشّة
                return (series, ep.to_string(), weak);
            }
            let (a, b) = (c.get(2).map(|m| m.as_str()), c.get(3).map(|m| m.as_str()));
            let num = |s: &str| s.parse::<u32>().unwrap_or(0);
            let episode = match (a, b) {
                (Some(s), Some(e)) => format!("{}x{}", num(s), num(e)),
                (Some(e), None) => num(e).to_string(),
                _ => "".to_string(),
            };
            return (series, episode, false);
        }
    }
    (String::new(), String::new(), true)
}

pub fn is_browser_app(app: &str) -> bool { is_app(app, BROWSERS) }

/// عناوين لا تمثل مواقع — صفحة البداية، التبويب الجديد، أخطاء المتصفح، وأسماء المتصفحات نفسها.
const JUNK_SITES: &[&str] = &[
    "new tab", "new page", "home", "plank", "problem loading page",
    "mozilla firefox", "google chrome", "chromium", "brave", "microsoft edge", "tor",
    "calculator",
];

pub fn is_junk_site(site: &str) -> bool {
    let s = site.trim().to_lowercase();
    JUNK_SITES.iter().any(|j| s == *j)
}

/// يزيل علامات الاتجاه الثنائية (RLM/LRM وما شابهها) من طرفي النص.
fn trim_bidi(s: &str) -> &str {
    s.trim_matches(['\u{200E}', '\u{200F}', '\u{202A}', '\u{202B}', '\u{202C}',
                    '\u{202D}', '\u{202E}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}'])
}

/// `عنوان - يوتيوب — Mozilla Firefox` → (site="YouTube", title="عنوان")
fn parse_browser_title(app: &str, title: &str) -> (String, String) {
    let suffix = BROWSERS.iter()
        .map(|b| format!(" — {}", b.to_uppercase()))
        .find(|s| title.to_lowercase().ends_with(&s.to_lowercase()))
        .or_else(|| {
            title.rfind(" — ").and_then(|i| {
                let rest = &title[i + " — ".len()..];
                let first = rest.split_whitespace().next().unwrap_or("");
                let _ = app;
                if BROWSERS.iter().any(|b| first.to_lowercase().contains(b)) { Some(title[i..].to_string()) } else { None }
            })
        })
        .map(|s| s.to_string());
    let stripped = match suffix {
        Some(s) => title[..title.len() - s.len()].trim().to_string(),
        None => title.trim().to_string(),
    };
    let stripped = trim_bidi(&stripped);
    // آخر فاصل بين " - " و " / " و " \ " و " — " (تنسيق فايرفوكس: "عنوان — موقع — Mozilla Firefox")
    let sep = [" - ", " / ", " \\ ", " — "].iter()
        .filter_map(|s| stripped.rfind(s).map(|i| (i, s.len())))
        .max_by_key(|(i, _)| *i);
    let (site, title) = match sep {
        Some((i, sep_len)) => {
            let site = stripped[i + sep_len..].trim();
            // ponytail: مقطع أخير طويل = منشور إعادة تغريد يعرض النص كاملاً؛
            // نرجع للمقطع الأول (X \ محتوى طويل → X). عتبة 40 حرفاً، غيّرها عند الحاجة.
            let site = if site.len() > 40 { &stripped[..i] } else { site };
            (site.to_string(), stripped[..i].trim().to_string())
        }
        None => (stripped.to_string(), String::new()),
    };
    let site = if is_junk_site(&site) { String::new() } else { site.to_string() };
    (site, title)
}

fn builtin_category_for_site(site: &str) -> &'static str {
    let s = site.to_lowercase();
    if MEDIA_SITES.iter().any(|m| s.contains(m)) { "media" } else { "browsing" }
}

pub fn builtin_name(app_name: &str) -> Option<String> {
    let a = app_name.to_lowercase();
    BUILTIN_NAMES.iter()
        .find(|(k, _)| a == k.to_lowercase() || a.contains(&k.to_lowercase()))
        .map(|(_, v)| v.to_string())
}

pub fn short_name(app_name: &str) -> String {
    let base = app_name.strip_suffix(".desktop").or_else(|| app_name.strip_suffix(".exe")).unwrap_or(app_name);
    let last = base.rsplit(['.', '/', '\\']).next().unwrap_or(base);
    last.to_lowercase()
}

pub fn enrich(app_name: &str, window_title: &str) -> Enriched {
    let title_lower = window_title.to_lowercase();

    if matches!(title_lower.as_str(), "__boot__" | "__shutdown__" | "__sleep__" | "__wake__") {
        return Enriched { event_type: "system", category: "other", media_kind: "", series_weak: true,
            site: String::new(), series: String::new(), episode: String::new(),
            title_cleaned: window_title.to_string() };
    }

    // قراءة: امتداد مستند غير مشروط بالحارس (عنوان ".pdf" محتوى حتى من تطبيق
    // عامّي — القرار 15)، أو تطبيق قارئ بعنوان غير عام
    if DOC_EXTS.iter().any(|e| title_lower.contains(e))
        || (is_app(app_name, READERS) && !is_generic_title(app_name, window_title)) {
        return Enriched { event_type: "app", category: "reading", media_kind: "reading", series_weak: true,
            site: String::new(), series: String::new(), episode: String::new(),
            title_cleaned: window_title.to_string() };
    }

    // استماع: امتداد صوتي في العنوان غير مشروط (يسبق فرع مشغلات الفيديو —
    // mpv + "song.mp3" → استماع بغلبة الامتداد)، أو مشغل صوت بعنوان غير عام
    if AUDIO_EXTS.iter().any(|e| title_lower.contains(e))
        || (is_app(app_name, AUDIO_PLAYERS) && !is_generic_title(app_name, window_title)) {
        let title = strip_suffix(window_title, AUDIO_SUFFIXES);
        return Enriched { event_type: "media", category: "listening", media_kind: "listening",
            series_weak: true, site: String::new(), series: String::new(), episode: String::new(),
            title_cleaned: title };
    }

    // مشغلات فيديو: لاحقة المشغل تُحذف ثم كشف الحلقة — بعنوان غير عام فقط؛
    // حجب الحارس يسقط إلى خريطة التطبيقات أدناه (media_kind="")
    if is_app(app_name, VIDEO_PLAYERS) && !is_generic_title(app_name, window_title) {
        let title = strip_suffix(window_title, &[" - mpv", " - VLC media player", " - Celluloid", " - Totem"]);
        let (series, episode, weak) = parse_episode(&title);
        let category = "media";
        return Enriched { event_type: "media", category, media_kind: "watching",
            series_weak: series.is_empty() || weak, site: String::new(), series, episode,
            title_cleaned: title };
    }
    if VIDEO_EXTS.iter().any(|e| title_lower.contains(e)) {
        return Enriched { event_type: "media", category: "media", media_kind: "watching", series_weak: true,
            site: String::new(), series: String::new(), episode: String::new(),
            title_cleaned: window_title.to_string() };
    }

    // متصفح: استخراج الموقع
    if is_browser_app(app_name) {
        let (site, title) = parse_browser_title(app_name, window_title);
        return Enriched { event_type: "app", category: builtin_category_for_site(&site), media_kind: "",
            series_weak: true, site, series: String::new(), episode: String::new(), title_cleaned: title };
    }

    // تطبيقات حسب الخريطة
    let category = if is_app(app_name, GAME_APPS) { "games" }
        else if is_app(app_name, PROD_APPS) { "productivity" }
        else if is_app(app_name, MEDIA_APPS) { "media" }
        else { "other" };

    Enriched { event_type: if category == "media" { "media" } else { "app" }, category,
        media_kind: "", series_weak: true, site: String::new(), series: String::new(),
        episode: String::new(), title_cleaned: window_title.to_string() }
}

/// العنوان المعروض بعد إزالة لاحقة المشغل — مصدر الحقيقة هو enrich
pub fn clean_title(app_name: &str, window_title: &str) -> String {
    enrich(app_name, window_title).title_cleaned().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn media_app() { assert_eq!(classify("mpv", "x.mp4"), "media"); }
    #[test] fn media_title() { assert_eq!(classify("firefox", "x.mp4"), "media"); }
    #[test] fn regular_app() { assert_eq!(classify("Code", "main.rs"), "app"); }
    #[test] fn system_event() { assert_eq!(classify("", "__boot__"), "system"); }

    #[test] fn friendly_builtin() {
        assert_eq!(builtin_name("org.mozilla.firefox.desktop").unwrap(), "فايرفوكس");
        assert_eq!(builtin_name("org.gnome.Ptyxis.desktop").unwrap(), "الطرفية");
        assert!(builtin_name("unknown.app.desktop").is_none());
    }

    #[test] fn short_name_strips_desktop_and_exe() {
        assert_eq!(short_name("org.gnome.Nautilus.desktop"), "nautilus");
        assert_eq!(short_name("firefox.exe"), "firefox");
        assert_eq!(short_name("mpv"), "mpv");
    }

    #[test] fn site_from_browser_title() {
        let e = enrich("org.mozilla.firefox.desktop",
            "عالم مارفل السينمائي هو عملية نفسية سريعة - YouTube — Mozilla Firefox");
        assert_eq!(e.site, "YouTube");
        assert_eq!(e.title_cleaned(), "عالم مارفل السينمائي هو عملية نفسية سريعة");
        assert_eq!(e.category, "media");
    }

    #[test] fn site_only_title() {
        let e = enrich("org.mozilla.firefox.desktop", "YouTube — Mozilla Firefox");
        assert_eq!(e.site, "YouTube");
        assert_eq!(e.category, "media");
    }

    #[test] fn browser_title_with_case_fold_before_separator() {
        let e = enrich("org.mozilla.firefox.desktop", "İstanbul - Müze — Mozilla Firefox");
        assert_eq!(e.site, "Müze");
        assert_eq!(e.title_cleaned(), "İstanbul");
    }

    #[test] fn junk_title_is_not_a_site() {
        assert_eq!(enrich("org.mozilla.firefox.desktop", "Calculator — Mozilla Firefox").site, "");
        assert_eq!(enrich("org.mozilla.firefox.desktop", "New Tab — Mozilla Firefox").site, "");
        assert_eq!(enrich("org.mozilla.firefox.desktop", "Problem loading page").site, "");
        assert_eq!(enrich("org.mozilla.firefox.desktop", "Home / X — Mozilla Firefox").site, "X");
    }

    #[test] fn twitter_slash_separator() {
        assert_eq!(enrich("org.mozilla.firefox.desktop", "منشور ما / X — Mozilla Firefox").site, "X");
        assert_eq!(enrich("org.mozilla.firefox.desktop", "منشور ما \\ X — Mozilla Firefox").site, "X");
        assert_eq!(enrich("org.mozilla.firefox.desktop", "منشور ما - X — Mozilla Firefox").site, "X");
    }

    #[test] fn bidi_marks_trimmed_from_site() {
        let e = enrich("org.mozilla.firefox.desktop", "\u{200F}Google Gemini — Mozilla Firefox");
        assert_eq!(e.site, "Google Gemini");
    }

    #[test] fn long_twitter_quote_falls_back_to_first_segment() {
        let e = enrich("org.mozilla.firefox.desktop",
            "X \\ DeepSeek على X: \"🚀 Official API is now LIVE in public beta\" — Mozilla Firefox");
        assert_eq!(e.site, "X");
    }

    #[test] fn double_em_dash_title_uses_last_segment_as_site() {
        let e = enrich("org.mozilla.firefox.desktop", "Agent Skill — shieldcn — Mozilla Firefox");
        assert_eq!(e.site, "shieldcn");
        assert_eq!(e.title_cleaned(), "Agent Skill");
        let e = enrich("org.mozilla.firefox.desktop", "الواحة — تغذية — Mozilla Firefox");
        assert_eq!(e.site, "تغذية");
    }

    #[test] fn junk_site_list() {
        assert!(is_junk_site("New Tab"));
        assert!(is_junk_site("plank"));
        assert!(is_junk_site("Mozilla Firefox"));
        assert!(is_junk_site("Calculator"));
        assert!(!is_junk_site("YouTube"));
    }

    #[test] fn episode_latin() {
        let e = enrich("mpv.desktop", "SpongeBob S01E03 - mpv");
        assert_eq!(e.series, "SpongeBob");
        assert_eq!(e.episode, "1x3");
    }

    #[test] fn episode_arabic_with_arabic_digits() {
        let e = enrich("mpv.desktop", "الدرس ٢٦ - mpv");
        assert_eq!(e.series, "الدرس");
        assert_eq!(e.episode, "26");
    }

    #[test] fn episode_plain_number() {
        let e = enrich("mpv.desktop", "الدرس 26 - mpv");
        assert_eq!(e.series, "الدرس");
        assert_eq!(e.episode, "26");
    }

    #[test] fn episode_ep_pattern() {
        let e = enrich("vlc.desktop", "Show EP3 - VLC media player");
        assert_eq!(e.series, "Show");
        assert_eq!(e.episode, "3");
    }

    #[test] fn episode_word() {
        let e = enrich("mpv.desktop", "Show episode 3");
        assert_eq!(e.series, "Show");
        assert_eq!(e.episode, "3");
    }

    #[test] fn episode_old_style() {
        let e = enrich("mpv.desktop", "Show 3x05");
        assert_eq!(e.series, "Show");
        assert_eq!(e.episode, "3x5");
    }

    #[test] fn reading_pdf() {
        let e = enrich("org.gnome.Evince.desktop", "الرياضيات.pdf - Evince");
        assert_eq!(e.category, "reading");
        assert_eq!(e.event_type, "app");
    }

    #[test] fn productivity_terminal() {
        assert_eq!(enrich("org.gnome.Ptyxis.desktop", "bash").category, "productivity");
    }

    #[test] fn browsing_default() {
        assert_eq!(enrich("org.mozilla.firefox.desktop", "صفحة عادية - موقع عادي — Mozilla Firefox").category, "browsing");
    }

    #[test] fn game() {
        assert_eq!(enrich("steam.desktop", "Counter-Strike 2").category, "games");
    }

    #[test] fn system_kept() {
        assert_eq!(enrich("", "__boot__").event_type, "system");
    }

    #[test] fn audio_player_is_listening() {
        let e = enrich("spotify.desktop", "أغنية - spotify");
        assert_eq!(e.event_type, "media");
        assert_eq!(e.category, "listening");
        assert_eq!(e.media_kind, "listening");
    }

    #[test] fn audio_ext_in_browser_title_is_listening() {
        let e = enrich("org.mozilla.firefox.desktop", "song.mp3 - YouTube");
        assert_eq!(e.media_kind, "listening");
    }

    #[test] fn audio_check_precedes_video_branch() {
        let e = enrich("mpv.desktop", "song.mp3 - mpv");
        assert_eq!(e.media_kind, "listening");
    }

    #[test] fn video_mkv_stays_watching() {
        let e = enrich("vlc.desktop", "movie.mkv - VLC media player");
        assert_eq!(e.media_kind, "watching");
    }

    #[test] fn clean_title_strips_player_suffix() {
        assert_eq!(clean_title("spotify.desktop", "أغنية - spotify"), "أغنية");
        assert_eq!(clean_title("org.gnome.Evince.desktop", "كتاب.pdf - Evince"), "كتاب.pdf - Evince");
    }

    #[test] fn reading_stays_reading_without_audio() {
        let e = enrich("org.gnome.Evince.desktop", "الرياضيات.pdf - Evince");
        assert_eq!(e.media_kind, "reading");
        assert_eq!(e.category, "reading");
    }

    #[test]
    fn showtime_generic_title_is_not_content() {
        // (أعيدت تسمية showtime_video_reaches_watching — تغيّر المعنى: حارس
        // العنوان العام يوقف فرع المشغل ويسقط إلى خريطة التطبيقات)
        let e = enrich("org.gnome.Showtime.desktop", "Video Player");
        assert_eq!(e.event_type, "media");
        assert_eq!(e.media_kind, "");
        assert_eq!(e.series, "");
    }

    #[test] fn series_weak_true_on_keyword_fall() {
        let e = enrich("mpv.desktop", "الدرس ٢٦ - mpv");
        assert!(e.series_weak, "سقوط الكلمة المفتاحية — سلسلة هشّة");
    }

    #[test] fn series_weak_true_on_no_match() {
        let e = enrich("mpv.desktop", "movie.mp4 - mpv");
        assert!(e.series_weak, "بلا تطابق — سلسلة فارغة");
    }

    #[test] fn series_weak_false_on_strong_series() {
        assert!(!enrich("mpv.desktop", "SpongeBob S01E03 - mpv").series_weak);
        assert!(!enrich("mpv.desktop", "Show 3x05").series_weak);
    }

    #[test] fn is_file_manager_covers_full_ids() {
        assert!(is_file_manager("org.gnome.Nautilus.desktop"));
        assert!(is_file_manager("org.nemo.Nemo"));
        assert!(is_file_manager("org.kde.dolphin"));
        assert!(!is_file_manager("mpv"));
    }

    #[test]
    fn showtime_real_title_still_watching() {
        let e = enrich("org.gnome.Showtime.desktop", "فيلم");
        assert_eq!(e.media_kind, "watching");
    }

    #[test]
    fn generic_title_empty_after_clean() {
        let e = enrich("mpv.desktop", "- mpv");
        assert_eq!(e.media_kind, "", "منظف فارغ بعد القص — عنوان عام");
    }

    #[test]
    fn generic_title_matches_short_name() {
        assert_eq!(enrich("com.github.johnfactotum.Foliate.desktop", "Foliate").media_kind, "",
            "الاسم الصغير = foliate");
        assert_eq!(enrich("mpv.desktop", "Mpv").media_kind, "", "مطابقة بحرف صغير");
    }

    #[test]
    fn generic_title_false_for_real_titles() {
        assert!(!is_generic_title("mpv.desktop", "الدرس 2 - mpv"));
        assert!(!is_generic_title("mpv.desktop", "تائية أبو إسحاق الإلبيري"));
    }

    #[test]
    fn doc_ext_not_blocked_by_guard() {
        let e = enrich("com.github.johnfactotum.Foliate.desktop", "كتاب.pdf");
        assert_eq!(e.media_kind, "reading", "الامتداد غير مشروط بالحارس");
    }
}
