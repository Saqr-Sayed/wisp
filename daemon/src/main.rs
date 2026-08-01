mod dbus_api; mod gnome; mod systemd;

use std::sync::Arc;
use std::time::Duration;

use salmonella_core::db::Db;
use salmonella_core::tracker::{run_tracker_loop, WindowSource};
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

#[tokio::main]
async fn main() {
    println!("Salmonella daemon starting...");
    systemd::install();

    let db = Arc::new(Db::new());
    db.backfill_sites();

    let (_conn, tracker) = dbus_api::serve(db.clone()).await.unwrap();

    let handle = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let backend = wait_for_backend();
        run_tracker_loop(db, backend, move |app, title, now| {
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

    std::future::pending::<()>().await;
}
