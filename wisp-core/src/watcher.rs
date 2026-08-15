use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::db::Db;
use crate::tracker::unix_now;

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

/// Watch the given dirs for file create/delete/rename and log them as system
/// events (debounced 3s, so save-bomb bursts collapse into "N files in dir").
/// Used by the Linux daemon and the Windows in-process tracker.
pub fn spawn_file_watcher(db: Arc<Db>, dirs: Vec<PathBuf>) {
    if dirs.is_empty() {
        return;
    }
    let home = dirs::home_dir().unwrap_or_default();
    let listed: Vec<String> = dirs.iter().map(|d| d.display().to_string()).collect();
    println!("file watcher active on: {}", listed.join(", "));

    std::thread::spawn(move || {
        use notify_debouncer_full::{
            new_debouncer,
            notify::event::ModifyKind,
            notify::{EventKind, RecursiveMode},
            DebounceEventResult,
        };

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
                by_kind.entry(kind).or_default()
                    .extend(paths.iter().map(|p| rel_path(&home, p)));
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