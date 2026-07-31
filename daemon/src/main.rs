mod db; mod classifier; mod wayland; mod dbus_api; mod systemd;

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use wayland::WlrBackend;
use db::Db;
use classifier::classify;

#[tokio::main]
async fn main() {
    println!("Salmonella daemon starting...");
    systemd::install();

    let db = Arc::new(Db::new());

    let (_conn, tracker) = dbus_api::serve(db.clone()).await.unwrap();

    if let Some(mut wl) = WlrBackend::new() {
        println!("Wayland backend active");
        let mut prev_app = String::new();
        let mut prev_title = String::new();
        let mut current_log_id: Option<i64> = None;

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let (app, title) = wl.active_window();

            if (app != prev_app || title != prev_title) && !app.is_empty() {
                let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;

                if let Some(id) = current_log_id {
                    db.close_log(id, now);
                }

                let et = classify(&app, &title);
                current_log_id = Some(db.insert_log(et, &app, &title, now));

                if let Err(e) = tracker.emit_window_changed(&app, &title, now).await {
                    eprintln!("window_changed signal failed: {e}");
                }

                prev_app = app.clone();
                prev_title = title.clone();
            }
        }
    } else {
        eprintln!("No Wayland compositor found");
        std::future::pending::<()>().await;
    }
}
