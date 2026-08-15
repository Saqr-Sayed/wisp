mod dbus_api; mod gnome; mod logind; mod mpris; mod systemd;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use wisp_core::db::Db;
use wisp_core::tracker::{run_tracker_loop, unix_now, SysEvents, WindowSource};
use wisp_core::watcher::spawn_file_watcher;
use gnome::GnomeBackend;

fn pick_backend() -> Option<impl WindowSource> {
    if let Some(g) = GnomeBackend::new() {
        println!("GNOME Shell extension backend active");
        return Some(g);
    }
    None
}

/// The daemon starts at login before gnome-shell finishes loading extensions,
/// so retry until the Wisp extension owns its bus name.
fn wait_for_backend() -> impl WindowSource {
    loop {
        if let Some(b) = pick_backend() {
            return b;
        }
        println!("waiting for the GNOME Shell extension...");
        std::thread::sleep(Duration::from_secs(5));
    }
}

/// System boot time in unix seconds, from /proc/stat. 0 on any failure.
fn boot_time() -> i64 {
    let Ok(stat) = std::fs::read_to_string("/proc/stat") else { return 0 };
    for line in stat.lines() {
        if let Some(rest) = line.strip_prefix("btime ") {
            return rest.trim().parse().unwrap_or(0);
        }
    }
    0
}

/// مراقبة مجلدات XDG للمستخدم (Desktop/Documents/Downloads/Pictures/Videos/Music)
/// عبر المراقب المشترك في wisp-core (يستخدمه أيضاً مسار ويندوز داخل التطبيق).
fn watch_files(db: Arc<Db>) {
    let home = dirs::home_dir().unwrap_or_default();
    let dirs: Vec<PathBuf> = ["Desktop", "Documents", "Downloads", "Pictures", "Videos", "Music"]
        .iter()
        .map(|d| home.join(d))
        .filter(|d| d.is_dir())
        .collect();
    spawn_file_watcher(db, dirs);
}

#[tokio::main]
async fn main() {
    println!("Wisp daemon starting...");
    systemd::install();

    let db = Arc::new(Db::new());
    db.backfill_sites();
    db.close_dangling(unix_now());

    let btime = boot_time();
    let last_boot = db.get_setting("last_boot").unwrap_or_default();
    if last_boot != btime.to_string() {
        if btime > 0 {
            db.insert_system_event("boot", "", btime, Some(btime));
        }
        db.insert_system_event("login", "", unix_now(), Some(unix_now()));
        db.set_setting("last_boot", &btime.to_string());
    }

    let shutdown = Arc::new(AtomicBool::new(false));
    let sys = Arc::new(SysEvents::new());
    logind::spawn_listener(sys.clone(), shutdown.clone());

    watch_files(db.clone());

    let (_conn, tracker) = dbus_api::serve(db.clone()).await.unwrap();

    let handle = tokio::runtime::Handle::current();
    let db2 = db.clone();
    let sys2 = sys.clone();
    std::thread::spawn(move || {
        let backend = wait_for_backend();
        run_tracker_loop(db2, backend, &sys2, &|app, _| mpris::probe(app), move |app, title, now| {
            let handle = handle.clone();
            let app = app.to_string();
            let title = title.to_string();
            handle.block_on(async {
                if let Err(e) = tracker.emit_window_changed(&app, &title, now).await {
                    eprintln!("window_changed signal failed: {e}");
                }
            });
        });
    });

    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("sigterm handler");
    tokio::select! {
        _ = sigterm.recv() => {
            let now = unix_now();
            db.close_dangling(now);
            let kind = if shutdown.load(Ordering::Relaxed) { "power_off" } else { "logout" };
            db.insert_system_event(kind, "", now, Some(now));
            println!("wisp daemon exiting ({kind})");
        }
        _ = std::future::pending::<()>() => {}
    }
}
