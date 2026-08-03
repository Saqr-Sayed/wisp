use salmonella_core::classifier;
use salmonella_core::tracker::{MediaMeta, MediaMime};
use zbus::blocking::{Connection, Proxy};

/// اشتقاق اسم ناقل MPRIS من معرف التطبيق: قص .desktop ثم آخر مقطع بعد النقطة
/// (يحتفظ بحالة الأحرف الأصلية). "mpv.desktop" → org.mpris.MediaPlayer2.mpv؛
/// "org.gnome.Showtime.desktop" → org.mpris.MediaPlayer2.Showtime.
fn derive_bus_name(app_id: &str) -> Option<String> {
    let base = app_id.strip_suffix(".desktop").unwrap_or(app_id);
    let last = base.rsplit(['.', '/', '\\']).next().unwrap_or(base);
    if last.is_empty() { return None; }
    Some(format!("org.mpris.MediaPlayer2.{last}"))
}

/// أسماء ناقل الجلسة المطابقة للاحقة org.mpris.MediaPlayer2.* (غير حساسة
/// لحالة الأحرف) — سقوط اكتشاف الاسم عند اختلاف التسمية عن المشتق.
fn list_mpris_names(conn: &Connection) -> Vec<String> {
    let dbus = match Proxy::new(conn, "org.freedesktop.DBus", "/org/freedesktop/DBus",
                                "org.freedesktop.DBus") {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let names: Vec<String> = match dbus.call("ListNames", &()) {
        Ok(n) => n,
        Err(_) => return Vec::new(),
    };
    const PREFIX: &str = "org.mpris.MediaPlayer2.";
    names.into_iter().filter(|n| {
        n.get(..PREFIX.len()).map(|p| p.eq_ignore_ascii_case(PREFIX)).unwrap_or(false)
    }).collect()
}

/// قراءة سلسلة من قاموس Metadata (a{sv}) — مفاتيح xesam:*.
/// (zvariant 4: downcast_ref::<str>() → Result<&str, _>؛ Dict::iter →
/// (&Value, &Value).)
fn meta_str(metadata: &zvariant::OwnedValue, key: &str) -> Option<String> {
    let zvariant::Value::Dict(dict) = &**metadata else { return None };
    for (k, v) in dict.iter() {
        if let zvariant::Value::Str(k) = k {
            if k.as_str() == key {
                if let Ok(s) = v.downcast_ref::<&str>() {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

/// contentType بادئة audio/ → Audio، video/ → Video، وإلا None.
fn mime_of(ct: &str) -> Option<MediaMime> {
    if ct.starts_with("audio/") { Some(MediaMime::Audio) }
    else if ct.starts_with("video/") { Some(MediaMime::Video) }
    else { None }
}

/// توقيع ملف عبر infer (يعمل على مسارات file:// فقط؛ روابط http(s) → None).
fn sniff_mime(path: &str) -> Option<MediaMime> {
    let t = infer::get_from_path(path).ok()??;
    mime_of(t.mime_type())
}

fn url_mime(url: &str) -> Option<MediaMime> {
    sniff_mime(url.strip_prefix("file://").unwrap_or(url))
}

/// basename المسار بعد قص file:// — سقوط xesam:title الفارغ.
fn url_basename(url: &str) -> Option<String> {
    let path = url.strip_prefix("file://").unwrap_or(url);
    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    if base.is_empty() { None } else { Some(base.to_string()) }
}

/// قراءة ميتاداتا المشغل عبر الاسم المعطى — None إذا غاب الاسم أو
/// PlaybackStatus == "Stopped" أو كانت Metadata فارغة (القرار 10).
fn meta_for(conn: &Connection, bus: &str) -> Option<MediaMeta> {
    let player = Proxy::new(conn, bus, "/org/mpris/MediaPlayer2",
                            "org.mpris.MediaPlayer2.Player").ok()?;
    let status: zvariant::OwnedValue = player.get_property("PlaybackStatus").ok()?;
    if let Ok(s) = status.downcast_ref::<&str>() {
        if s == "Stopped" { return None; }
    }
    let metadata: zvariant::OwnedValue = player.get_property("Metadata").ok()?;
    let zvariant::Value::Dict(dict) = &*metadata else { return None };
    if dict.iter().next().is_none() { return None; }
    let title = meta_str(&metadata, "xesam:title").filter(|t| !t.is_empty())
        .or_else(|| meta_str(&metadata, "xesam:url").and_then(|u| url_basename(&u)));
    let mime = meta_str(&metadata, "xesam:contentType").and_then(|c| mime_of(&c))
        .or_else(|| meta_str(&metadata, "xesam:url").and_then(|u| url_mime(&u)));
    Some(MediaMeta { title, mime })
}

/// خطاف الميتاداتا للـ daemon — zbus حاجب في خيط المتتبع (القرار 11: يُطلق
/// عند تغيّر النافذة فقط، لا كل ثانية؛ سقف موثّق في «المخاطر»). الاستبعاد
/// المبكر لغير المشغلات هنا في الطبقة الحرة (القرار 12) — نواة المتتبع
/// تستدعي metadata بلا شرط (عقد التصميم).
pub fn probe(app_id: &str) -> Option<MediaMeta> {
    if !classifier::is_media_app(app_id) { return None; }
    let bus = derive_bus_name(app_id)?;
    let conn = Connection::session().ok()?;
    let names = list_mpris_names(&conn);
    // الاسم المشتق موجود → عُد بميتاداتاه كما هي (None لموقوف/فارغ) بلا سقوط
    // لمشغل آخر — يمنع إسناد ميتاداتا mpv إلى Showtime المتوقف (القرار 10).
    if names.iter().any(|n| n.eq_ignore_ascii_case(&bus)) {
        return meta_for(&conn, &bus);
    }
    // سقوط: أي مشغل MPRIS آخر يطابق اللاحقة (يغطي اختلاف تسمية Showtime وغيره)
    for alt in names {
        if let Some(m) = meta_for(&conn, &alt) { return Some(m); }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn sniff_mime_mp3_magic() {
        let path = std::env::temp_dir().join(format!("salmonella-sniff-{}.bin", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"ID3\x03\x00\x00\x00\x00\x00\x00").unwrap();
        drop(f);
        assert_eq!(sniff_mime(&path.to_string_lossy()), Some(MediaMime::Audio),
            "توقيع ID3 يشم حتى بامتداد وهمي — التوقيع لا الامتداد");
        let _ = std::fs::remove_file(&path);
    }
}
