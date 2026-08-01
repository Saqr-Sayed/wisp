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
    pub friendly_name: String,
    pub site: String,
    pub category: String,
    pub series: String,
    pub episode: String,
}

pub fn show_notification(body: &str) -> Result<(), String> {
    notify_rust::Notification::new()
        .summary("Salmonella")
        .body(body)
        .show()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
mod commands {
    use super::LogEntry;

    type Row = (i64, String, String, String, i64, i64, i64, String, String, String, String, String);

    fn to_log_entry(r: Row) -> LogEntry {
        LogEntry {
            id: r.0, event_type: r.1, app_name: r.2,
            window_title: r.3, start_time: r.4,
            end_time: (r.5 >= 0).then_some(r.5),
            duration: (r.6 >= 0).then_some(r.6),
            friendly_name: r.7, site: r.8, category: r.9,
            series: r.10, episode: r.11,
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
    pub async fn notify(body: String) -> Result<(), String> {
        super::show_notification(&body)
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

    #[tauri::command]
    pub async fn get_report(from: i64, to: i64, group_by: String) -> Result<Vec<(String, i64)>, String> {
        let reply = call("GetReport", &(from, to, group_by)).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn get_series(from: i64, to: i64) -> Result<Vec<(String, String, i64)>, String> {
        let reply = call("GetSeries", &(from, to)).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn get_limits() -> Result<Vec<(String, String, i64)>, String> {
        let reply = call("GetLimits", &()).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn set_limit(target: String, kind: String, minutes: i64) -> Result<(), String> {
        call("SetLimit", &(target, kind, minutes)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn remove_limit(target: String) -> Result<(), String> {
        call("RemoveLimit", &(target,)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn get_name_overrides() -> Result<Vec<(String, String)>, String> {
        let reply = call("GetNameOverrides", &()).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn set_name_override(app_id: String, friendly: String) -> Result<(), String> {
        call("SetNameOverride", &(app_id, friendly)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn remove_name_override(app_id: String) -> Result<(), String> {
        call("RemoveNameOverride", &(app_id,)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn get_known_apps() -> Result<Vec<(String, String)>, String> {
        let reply = call("GetKnownApps", &()).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn get_known_sites() -> Result<Vec<(String, String)>, String> {
        let reply = call("GetKnownSites", &()).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn get_site_overrides() -> Result<Vec<(String, String)>, String> {
        let reply = call("GetSiteOverrides", &()).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn set_site_override(site: String, friendly: String) -> Result<(), String> {
        call("SetSiteOverride", &(site, friendly)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn remove_site_override(site: String) -> Result<(), String> {
        call("RemoveSiteOverride", &(site,)).await.map(|_| ())
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
            friendly_name: e.friendly_name.clone(),
            site: e.site.clone(),
            category: e.category.clone(),
            series: e.series.clone(),
            episode: e.episode.clone(),
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

    #[tauri::command]
    pub fn get_report(db: State<'_, Arc<Db>>, from: i64, to: i64, group_by: String) -> Result<Vec<(String, i64)>, String> {
        Ok(db.get_report(from, to, &group_by))
    }

    #[tauri::command]
    pub fn get_series(db: State<'_, Arc<Db>>, from: i64, to: i64) -> Result<Vec<(String, String, i64)>, String> {
        Ok(db.get_series(from, to))
    }

    #[tauri::command]
    pub fn get_limits(db: State<'_, Arc<Db>>) -> Result<Vec<(String, String, i64)>, String> {
        Ok(db.get_limits())
    }

    #[tauri::command]
    pub fn set_limit(db: State<'_, Arc<Db>>, target: String, kind: String, minutes: i64) -> Result<(), String> {
        db.set_limit(&target, &kind, minutes);
        Ok(())
    }

    #[tauri::command]
    pub fn remove_limit(db: State<'_, Arc<Db>>, target: String) -> Result<(), String> {
        db.remove_limit(&target);
        Ok(())
    }

    #[tauri::command]
    pub fn get_name_overrides(db: State<'_, Arc<Db>>) -> Result<Vec<(String, String)>, String> {
        Ok(db.get_name_overrides())
    }

    #[tauri::command]
    pub fn notify(body: String) -> Result<(), String> {
        super::show_notification(&body)
    }

    #[tauri::command]
    pub fn set_name_override(db: State<'_, Arc<Db>>, app_id: String, friendly: String) -> Result<(), String> {
        db.set_name_override(&app_id, &friendly);
        Ok(())
    }

    #[tauri::command]
    pub fn remove_name_override(db: State<'_, Arc<Db>>, app_id: String) -> Result<(), String> {
        db.remove_name_override(&app_id);
        Ok(())
    }

    #[tauri::command]
    pub fn get_known_apps(db: State<'_, Arc<Db>>) -> Result<Vec<(String, String)>, String> {
        Ok(db.get_known_apps())
    }

    #[tauri::command]
    pub fn get_known_sites(db: State<'_, Arc<Db>>) -> Result<Vec<(String, String)>, String> {
        Ok(db.get_known_sites())
    }

    #[tauri::command]
    pub fn get_site_overrides(db: State<'_, Arc<Db>>) -> Result<Vec<(String, String)>, String> {
        Ok(db.get_site_overrides())
    }

    #[tauri::command]
    pub fn set_site_override(db: State<'_, Arc<Db>>, site: String, friendly: String) -> Result<(), String> {
        db.set_site_override(&site, &friendly);
        Ok(())
    }

    #[tauri::command]
    pub fn remove_site_override(db: State<'_, Arc<Db>>, site: String) -> Result<(), String> {
        db.remove_site_override(&site);
        Ok(())
    }
}

use commands::{
    get_known_apps, get_known_sites, get_limits, get_name_overrides, get_report, get_series,
    get_site_overrides, get_status, get_timeline, notify, remove_limit, remove_name_override,
    remove_site_override, search, set_limit, set_name_override, set_site_override,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            notify,
            get_timeline, get_status, search,
            get_report, get_series, get_limits, set_limit, remove_limit,
            get_name_overrides, set_name_override, remove_name_override,
            get_known_apps, get_known_sites,
            get_site_overrides, set_site_override, remove_site_override,
        ]);

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
