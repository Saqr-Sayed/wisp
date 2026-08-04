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
    pub detail: String,
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

    type Row = (i64, String, String, String, i64, i64, i64, String, String, String, String, String, String);

    fn to_log_entry(r: Row) -> LogEntry {
        LogEntry {
            id: r.0, event_type: r.1, app_name: r.2,
            window_title: r.3, start_time: r.4,
            end_time: (r.5 >= 0).then_some(r.5),
            duration: (r.6 >= 0).then_some(r.6),
            friendly_name: r.7, site: r.8, category: r.9,
            series: r.10, episode: r.11, detail: r.12,
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
    pub async fn get_content(from: i64, to: i64) -> Result<Vec<(String, String, String, i64)>, String> {
        let reply = call("GetContent", &(from, to)).await?;
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

    #[tauri::command]
    pub async fn get_series_overrides() -> Result<Vec<(String, String)>, String> {
        let reply = call("GetSeriesOverrides", &()).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn set_series_override(pattern: String, name: String) -> Result<(), String> {
        call("SetSeriesOverride", &(pattern, name)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn remove_series_override(pattern: String) -> Result<(), String> {
        call("RemoveSeriesOverride", &(pattern,)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn get_setting(key: String) -> Result<String, String> {
        let reply = call("GetSetting", &(key,)).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn set_setting(key: String, value: String) -> Result<(), String> {
        call("SetSetting", &(key, value)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn get_categories() -> Result<Vec<(i64, String, String, i64, i64, i64)>, String> {
        let reply = call("GetCategories", &()).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn get_category_members(id: i64) -> Result<Vec<(String, String)>, String> {
        let reply = call("GetCategoryMembers", &(id,)).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn add_category(name: String, color: String) -> Result<i64, String> {
        let reply = call("AddCategory", &(name, color)).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn rename_category(id: i64, new_name: String) -> Result<(), String> {
        call("RenameCategory", &(id, new_name)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn set_category_color(id: i64, color: String) -> Result<(), String> {
        call("SetCategoryColor", &(id, color)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn add_category_member(id: i64, kind: String, target: String) -> Result<(), String> {
        call("AddCategoryMember", &(id, kind, target)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn delete_category_member(kind: String, target: String) -> Result<(), String> {
        call("DeleteCategoryMember", &(kind, target)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn delete_category(id: i64) -> Result<bool, String> {
        let reply = call("DeleteCategory", &(id,)).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn list_ignored() -> Result<Vec<(String, String)>, String> {
        let reply = call("ListIgnored", &()).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn ignore_target(kind: String, target: String) -> Result<(), String> {
        call("IgnoreTarget", &(kind, target)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn unignore_target(kind: String, target: String) -> Result<(), String> {
        call("UnignoreTarget", &(kind, target)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn archive_target(kind: String, target: String) -> Result<(), String> {
        call("ArchiveTarget", &(kind, target)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn unarchive_target(kind: String, target: String) -> Result<(), String> {
        call("UnarchiveTarget", &(kind, target)).await.map(|_| ())
    }

    #[tauri::command]
    pub async fn list_archived() -> Result<Vec<(String, String)>, String> {
        let reply = call("ListArchived", &()).await?;
        reply.body().deserialize().map_err(|e| e.to_string())
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
            detail: e.detail.clone(),
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
    pub fn get_content(db: State<'_, Arc<Db>>, from: i64, to: i64) -> Result<Vec<(String, String, String, i64)>, String> {
        Ok(db.get_content(from, to))
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

    #[tauri::command]
    pub fn get_series_overrides(db: State<'_, Arc<Db>>) -> Result<Vec<(String, String)>, String> {
        Ok(db.get_series_overrides())
    }

    #[tauri::command]
    pub fn set_series_override(db: State<'_, Arc<Db>>, pattern: String, name: String) -> Result<(), String> {
        db.set_series_override(&pattern, &name);
        Ok(())
    }

    #[tauri::command]
    pub fn remove_series_override(db: State<'_, Arc<Db>>, pattern: String) -> Result<(), String> {
        db.remove_series_override(&pattern);
        Ok(())
    }

    #[tauri::command]
    pub fn get_setting(db: State<'_, Arc<Db>>, key: String) -> Result<String, String> {
        Ok(db.get_setting(&key).unwrap_or_default())
    }

    #[tauri::command]
    pub fn set_setting(db: State<'_, Arc<Db>>, key: String, value: String) -> Result<(), String> {
        db.set_setting(&key, &value);
        Ok(())
    }

    #[tauri::command]
    pub fn get_categories(db: State<'_, Arc<Db>>) -> Result<Vec<(i64, String, String, i64, i64, i64)>, String> {
        Ok(db.categories().into_iter().map(|c| (c.id, c.name, c.color, c.is_builtin, c.is_deletable, c.sort)).collect())
    }

    #[tauri::command]
    pub fn get_category_members(db: State<'_, Arc<Db>>, id: i64) -> Result<Vec<(String, String)>, String> {
        Ok(db.category_members(id))
    }

    #[tauri::command]
    pub fn add_category(db: State<'_, Arc<Db>>, name: String, color: String) -> Result<i64, String> {
        Ok(db.add_category(&name, &color))
    }

    #[tauri::command]
    pub fn rename_category(db: State<'_, Arc<Db>>, id: i64, new_name: String) -> Result<(), String> {
        db.rename_category(id, &new_name);
        Ok(())
    }

    #[tauri::command]
    pub fn set_category_color(db: State<'_, Arc<Db>>, id: i64, color: String) -> Result<(), String> {
        db.set_category_color(id, &color);
        Ok(())
    }

    #[tauri::command]
    pub fn add_category_member(db: State<'_, Arc<Db>>, id: i64, kind: String, target: String) -> Result<(), String> {
        db.add_category_member(id, &kind, &target);
        Ok(())
    }

    #[tauri::command]
    pub fn delete_category_member(db: State<'_, Arc<Db>>, kind: String, target: String) -> Result<(), String> {
        db.delete_category_member(&kind, &target);
        Ok(())
    }

    #[tauri::command]
    pub fn delete_category(db: State<'_, Arc<Db>>, id: i64) -> Result<bool, String> {
        Ok(db.delete_category(id))
    }

    #[tauri::command]
    pub fn list_ignored(db: State<'_, Arc<Db>>) -> Result<Vec<(String, String)>, String> {
        Ok(db.list_ignored())
    }

    #[tauri::command]
    pub fn ignore_target(db: State<'_, Arc<Db>>, kind: String, target: String) -> Result<(), String> {
        db.ignore_target(&kind, &target);
        Ok(())
    }

    #[tauri::command]
    pub fn unignore_target(db: State<'_, Arc<Db>>, kind: String, target: String) -> Result<(), String> {
        db.unignore_target(&kind, &target);
        Ok(())
    }

    #[tauri::command]
    pub fn archive_target(db: State<'_, Arc<Db>>, kind: String, target: String) -> Result<(), String> {
        db.archive_target(&kind, &target);
        Ok(())
    }

    #[tauri::command]
    pub fn unarchive_target(db: State<'_, Arc<Db>>, kind: String, target: String) -> Result<(), String> {
        db.unarchive_target(&kind, &target);
        Ok(())
    }

    #[tauri::command]
    pub fn list_archived(db: State<'_, Arc<Db>>) -> Result<Vec<(String, String)>, String> {
        Ok(db.list_archived())
    }
}

use commands::{
    add_category, add_category_member, archive_target, delete_category, delete_category_member,
    get_categories, get_category_members, get_content, get_known_apps, get_known_sites, get_limits,
    get_name_overrides, get_report, get_series, get_series_overrides, get_setting, get_site_overrides, get_status,
    get_timeline, ignore_target, list_archived, list_ignored, notify, remove_limit, remove_name_override,
    remove_series_override, remove_site_override, rename_category, search, set_category_color, set_limit,
    set_name_override, set_series_override, set_setting, set_site_override, unarchive_target, unignore_target,
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
            get_series_overrides, set_series_override, remove_series_override,
            get_setting, set_setting,
            get_categories, get_category_members, get_content, add_category, rename_category,
            set_category_color, add_category_member, delete_category_member, delete_category,
            list_ignored, ignore_target, unignore_target,
            list_archived, archive_target, unarchive_target,
        ]);

    #[cfg(target_os = "windows")]
    let builder = {
        use crate::windows_backend::{install_autostart, Win32Backend};
        use salmonella_core::db::Db;
        use salmonella_core::tracker::{run_tracker_loop, unix_now, SysEvents};
        use std::sync::Arc;
        use tauri::tray::TrayIconBuilder;
        use tauri::{menu::{Menu, MenuItem}, Manager};

        builder
            .setup(|app| {
                install_autostart();

                let db = Arc::new(Db::new());
                app.manage(db.clone());
                db.close_dangling(unix_now());

                let sys = SysEvents::new();
                std::thread::spawn(move || {
                    run_tracker_loop(db, Win32Backend, &sys, &|_, _| None, |_, _, _| {});
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
