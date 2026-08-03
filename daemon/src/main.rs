mod dbus_api; mod gnome; mod logind; mod systemd;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use salmonella_core::db::Db;
use salmonella_core::tracker::{run_tracker_loop, unix_now, SysEvents, WindowSource};
use gnome::GnomeBackend;

fn pick_backend() -> Option<impl WindowSource> {
    if let Some(g) = GnomeBackend::new() {
        println!("GNOME Shell extension backend active");
        return Some(g);
    }
    None
}

/// The daemon starts at login before gnome-shell finishes loading extensions,
/// so retry until the Salmonella extension owns its bus name.
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

fn usable(p: &Path) -> bool {
    if p.file_name().map(|f| f.to_string_lossy().starts_with('.')).unwrap_or(false) {
        return false;
    }
    !p.components().any(|c| matches!(c.as_os_str().to_str(), Some(".git" | "node_modules" | "target")))
}

fn rel_path(home: &Path, p: &Path) -> String {
    match p.strip_prefix(home) {
        Ok(rel) => rel.display().to_string(),
        Err(_) => p.display().to_string(),
    }
}

/// Watch XDG user dirs for file create/delete/rename and log them as system
/// events (debounced 3s, so save-bomb bursts collapse into "N files in dir").
fn watch_files(db: Arc<Db>) {
    use notify_debouncer_full::{
        new_debouncer,
        notify::event::ModifyKind,
        notify::{EventKind, RecursiveMode},
        DebounceEventResult,
    };

    let home = dirs::home_dir().unwrap_or_default();
    let dirs: Vec<_> = ["Desktop", "Documents", "Downloads", "Pictures", "Videos", "Music"]
        .iter()
        .map(|d| home.join(d))
        .filter(|d| d.is_dir())
        .collect();
    if dirs.is_empty() {
        return;
    }
    let listed: Vec<String> = dirs.iter().map(|d| d.display().to_string()).collect();
    println!("file watcher active on: {}", listed.join(", "));

    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel::<DebounceEventResult>();
        let mut debouncer = new_debouncer(Duration::from_secs(3), None, tx).expect("debouncer");
        for d in &dirs {
            if let Err(e) = debouncer.watch(d, RecursiveMode::Recursive) {
                eprintln!("watch {:?}: {e}", d);
            }
        }
        for batch in rx {
            let events = match batch {
                Ok(events) => events,
                Err(e) => {
                    eprintln!("file watcher: {e:?}");
                    continue;
                }
            };
            let mut by_kind: std::collections::HashMap<&'static str, Vec<String>> =
                std::collections::HashMap::new();
            for ev in events {
                let kind = match ev.event.kind {
                    EventKind::Create(_) => "file_created",
                    EventKind::Remove(_) => "file_deleted",
                    EventKind::Modify(ModifyKind::Name(_)) => "file_renamed",
                    _ => continue,
                };
                if ev.event.paths.is_empty() {
                    continue;
                }
                let paths: Vec<&PathBuf> = ev.event.paths.iter().filter(|p| usable(p)).collect();
                if paths.is_empty() {
                    continue;
                }
                by_kind.entry(kind).or_default().extend(paths.iter().map(|p| rel_path(&home, p)));
            }
            if by_kind.is_empty() {
                continue;
            }
            let now = unix_now();
            for (kind, paths) in by_kind {
                let title = match paths.len() {
                    1 => paths[0].clone(),
                    2 if kind == "file_renamed" => format!("{} -> {}", paths[0], paths[1]),
                    n => {
                        let dir = paths[0].rsplit_once('/').map(|(d, _)| d).unwrap_or(&paths[0]);
                        format!("{n} files in {dir}")
                    }
                };
                db.insert_system_event(kind, &title, now, Some(now));
            }
        }
    });
}

#[tokio::main]
async fn main() {
    println!("Salmonella daemon starting...");
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
        run_tracker_loop(db2, backend, &sys2, &|_, _| None, move |app, title, now| {
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
            println!("salmonella daemon exiting ({kind})");
        }
        _ = std::future::pending::<()>() => {}
    }
}
