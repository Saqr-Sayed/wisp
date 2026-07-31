use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LogEntry {
    pub id: i64, pub event_type: String, pub app_name: String,
    pub window_title: String, pub start_time: i64,
    pub end_time: i64, pub duration: i64,
}

type Row = (i64, String, String, String, i64, i64, i64);

#[tauri::command]
async fn get_timeline(from: i64, to: i64) -> Result<Vec<LogEntry>, String> {
    let conn = zbus::Connection::session().await.map_err(|e| e.to_string())?;
    let reply = conn.call_method(
        Some(zbus::names::BusName::WellKnown(zbus::names::WellKnownName::from_static_str("com.Saqr.Salomnella").unwrap())),
        "/com/Saqr/Salomnella",
        Some(zbus::names::InterfaceName::from_static_str("com.Saqr.Salomnella").unwrap()),
        "GetTimeline",
        &(from, to),
    ).await.map_err(|e| e.to_string())?;
    let rows: Vec<Row> = reply.body().deserialize().map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|r| LogEntry {
        id: r.0, event_type: r.1, app_name: r.2,
        window_title: r.3, start_time: r.4,
        end_time: r.5, duration: r.6,
    }).collect())
}

#[tauri::command]
async fn get_status() -> Result<(bool, i64, String, String), String> {
    let conn = zbus::Connection::session().await.map_err(|e| e.to_string())?;
    let reply = conn.call_method(
        Some(zbus::names::BusName::WellKnown(zbus::names::WellKnownName::from_static_str("com.Saqr.Salomnella").unwrap())),
        "/com/Saqr/Salomnella",
        Some(zbus::names::InterfaceName::from_static_str("com.Saqr.Salomnella").unwrap()),
        "GetStatus",
        &(),
    ).await.map_err(|e| e.to_string())?;
    let status: (bool, i64, String, String) = reply.body().deserialize().map_err(|e| e.to_string())?;
    Ok(status)
}

#[tauri::command]
async fn search(query: String) -> Result<Vec<LogEntry>, String> {
    let conn = zbus::Connection::session().await.map_err(|e| e.to_string())?;
    let reply = conn.call_method(
        Some(zbus::names::BusName::WellKnown(zbus::names::WellKnownName::from_static_str("com.Saqr.Salomnella").unwrap())),
        "/com/Saqr/Salomnella",
        Some(zbus::names::InterfaceName::from_static_str("com.Saqr.Salomnella").unwrap()),
        "Search",
        &(query,),
    ).await.map_err(|e| e.to_string())?;
    let rows: Vec<Row> = reply.body().deserialize().map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|r| LogEntry {
        id: r.0, event_type: r.1, app_name: r.2,
        window_title: r.3, start_time: r.4,
        end_time: r.5, duration: r.6,
    }).collect())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![get_timeline, get_status, search])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
