use serde::Serialize;

#[cfg(target_os = "windows")]
mod windows_backend;

#[derive(Serialize)]
pub struct LogEntry {
    pub id: i64,
    pub event_type: String,
    pub app_name: String,
    pub window_title: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub duration: Option<i64>,
}

#[cfg(target_os = "linux")]
mod commands {
    use super::LogEntry;

    type Row = (i64, String, String, String, i64, i64, i64);

    fn to_log_entry(r: Row) -> LogEntry {
        LogEntry {
            id: r.0,
            event_type: r.1,
            app_name: r.2,
            window_title: r.3,
            start_time: r.4,
            end_time: (r.5 >= 0).then_some(r.5),
            duration: (r.6 >= 0).then_some(r.6),
        }
    }

    async fn call(method: &str, body: &(impl serde::Serialize + zbus::zvariant::Type)) -> Result<zbus::Message, String> {
        let conn = zbus::Connection::session().await.map_err(|e| e.to_string())?;
        conn.call_method(
            Some("com.Saqr.Salomnella"),
            "/com/Saqr/Salomnella",
            Some("com.Saqr.Salomnella"),
            method,
            body,
        )
        .await
        .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn get_timeline(from: i64, to: i64) -> Result<Vec<LogEntry>, String> {
        let reply = call("GetTimeline", &(from, to)).await?;
        let rows: Vec<Row> = reply.body().deserialize().map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(to_log_entry).collect())
    }

    #[tauri::command]
    pub async fn get_status() -> Result<(bool, i64, String, String), String> {
        let reply = call("GetStatus", &()).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn search(query: String) -> Result<Vec<LogEntry>, String> {
        let reply = call("Search", &(query,)).await?;
        let rows: Vec<Row> = reply.body().deserialize().map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(to_log_entry).collect())
    }
}

#[cfg(target_os = "windows")]
mod commands {
    use super::LogEntry;
    use salmonella_core::db::Db;
    use std::sync::Arc;
    use tauri::State;

    fn to_log_entry(e: &salmonella_core::db::LogEntry) -> LogEntry {
        LogEntry {
            id: e.id,
            event_type: e.event_type.clone(),
            app_name: e.app_name.clone(),
            window_title: e.window_title.clone(),
            start_time: e.start_time,
            end_time: e.end_time,
            duration: e.duration,
        }
    }

    #[tauri::command]
    pub fn get_timeline(db: State<'_, Arc<Db>>, from: i64, to: i64) -> Result<Vec<LogEntry>, String> {
        Ok(db.get_timeline(from, to).iter().map(to_log_entry).collect())
    }

    #[tauri::command]
    pub fn get_status(db: State<'_, Arc<Db>>) -> Result<(bool, i64, String, String), String> {
        let (since, app, title) = db.get_status();
        Ok((since > 0, since, app, title))
    }

    #[tauri::command]
    pub fn search(db: State<'_, Arc<Db>>, query: String) -> Result<Vec<LogEntry>, String> {
        Ok(db.search(&query).iter().map(to_log_entry).collect())
    }
}

use commands::{get_timeline, get_status, search};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![get_timeline, get_status, search]);

    #[cfg(target_os = "windows")]
    let builder = {
        use crate::windows_backend::{install_autostart, Win32Backend};
        use salmonella_core::db::Db;
        use salmonella_core::tracker::run_tracker_loop;
        use std::sync::Arc;
        use tauri::tray::TrayIconBuilder;
        use tauri::{menu::{Menu, MenuItem}, Manager};

        builder
            .setup(|app| {
                install_autostart();

                let db = Arc::new(Db::new());
                app.manage(db.clone());

                std::thread::spawn(move || {
                    run_tracker_loop(db, Win32Backend, |_, _, _| {});
                });

                let show = MenuItem::with_id(app, "show", "إظهار", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "خروج", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show, &quit])?;
                let _tray = TrayIconBuilder::new()
                    .icon(app.default_window_icon().unwrap().clone())
                    .menu(&menu)
                    .show_menu_on_left_click(true)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .build(app)?;

                Ok(())
            })
            .on_window_event(|window, event| {
                use tauri::WindowEvent;
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window.hide();
                    api.prevent_close();
                }
            })
    };

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
