mod dbus_api; mod gnome; mod systemd;

use std::sync::Arc;

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

#[tokio::main]
async fn main() {
    println!("Salmonella daemon starting...");
    systemd::install();

    let db = Arc::new(Db::new());

    let (_conn, tracker) = dbus_api::serve(db.clone()).await.unwrap();

    let Some(backend) = pick_backend() else {
        eprintln!("No window backend available — enable the Salmonella GNOME extension");
        std::future::pending::<()>().await;
        return;
    };

    std::thread::spawn(move || {
        let handle = tokio::runtime::Handle::current();
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
