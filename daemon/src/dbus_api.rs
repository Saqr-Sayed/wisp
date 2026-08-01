use std::sync::{Arc, Mutex};
use salmonella_core::db::{Db, LogEntry};
use zbus::{interface, object_server::SignalContext, ConnectionBuilder};

type Row = (i64, String, String, String, i64, i64, i64, String, String, String, String, String);

fn to_row(e: &LogEntry) -> Row {
    (e.id, e.event_type.clone(), e.app_name.clone(), e.window_title.clone(),
     e.start_time, e.end_time.unwrap_or(-1), e.duration.unwrap_or(-1),
     e.friendly_name.clone(), e.site.clone(), e.category.clone(),
     e.series.clone(), e.episode.clone())
}

#[derive(Clone)]
pub struct ActivityTracker {
    db: Arc<Db>,
    conn: Arc<Mutex<Option<zbus::Connection>>>,
}

impl ActivityTracker {
    pub fn new(db: Arc<Db>) -> Self {
        ActivityTracker { db, conn: Arc::new(Mutex::new(None)) }
    }

    pub fn set_connection(&self, conn: zbus::Connection) {
        *self.conn.lock().unwrap() = Some(conn);
    }

    pub async fn emit_window_changed(&self, app_name: &str, window_title: &str, since: i64) -> zbus::Result<()> {
        let Some(conn) = self.conn.lock().unwrap().clone() else { return Ok(()) };
        let ctxt = SignalContext::new(&conn, "/com/Saqr/Salomnella")?;
        self.window_changed(&ctxt, app_name, window_title, since).await
    }
}

#[interface(name = "com.Saqr.Salomnella")]
impl ActivityTracker {
    async fn ping(&self) -> bool { true }

    async fn get_status(&self) -> (bool, i64, String, String) {
        let (since, app, title) = self.db.get_status();
        (since > 0, since, app, title)
    }

    async fn get_timeline(&self, from: i64, to: i64) -> Vec<Row> {
        self.db.get_timeline(from, to).iter().map(to_row).collect()
    }

    async fn search(&self, query: &str) -> Vec<Row> {
        self.db.search(query).iter().map(to_row).collect()
    }

    async fn get_report(&self, from: i64, to: i64, group_by: &str) -> Vec<(String, i64)> {
        self.db.get_report(from, to, group_by)
    }

    async fn get_series(&self, from: i64, to: i64) -> Vec<(String, String, i64)> {
        self.db.get_series(from, to)
    }

    async fn get_limits(&self) -> Vec<(String, String, i64)> {
        self.db.get_limits()
    }

    async fn set_limit(&self, target: &str, kind: &str, minutes: i64) {
        self.db.set_limit(target, kind, minutes);
    }

    async fn remove_limit(&self, target: &str) {
        self.db.remove_limit(target);
    }

    async fn get_name_overrides(&self) -> Vec<(String, String)> {
        self.db.get_name_overrides()
    }

    async fn set_name_override(&self, app_id: &str, friendly: &str) {
        self.db.set_name_override(app_id, friendly);
    }

    async fn remove_name_override(&self, app_id: &str) {
        self.db.remove_name_override(app_id);
    }

    async fn get_setting(&self, key: String) -> zbus::fdo::Result<String> {
        Ok(self.db.get_setting(&key).unwrap_or_default())
    }

    async fn set_setting(&self, key: String, value: String) -> zbus::fdo::Result<()> {
        self.db.set_setting(&key, &value);
        Ok(())
    }

    async fn list_custom_categories(&self) -> zbus::fdo::Result<Vec<(i64, String, String, String)>> {
        Ok(self.db.list_custom_categories())
    }

    async fn add_custom_category(&self, kind: String, target: String, display_name: String)
        -> zbus::fdo::Result<i64>
    {
        Ok(self.db.add_custom_category(&kind, &target, &display_name).unwrap_or(0))
    }

    async fn remove_custom_category(&self, id: i64) -> zbus::fdo::Result<bool> {
        self.db.remove_custom_category(id);
        Ok(true)
    }

    #[zbus(signal)]
    async fn window_changed(&self, ctxt: &zbus::object_server::SignalContext<'_>, app_name: &str, window_title: &str, since: i64) -> zbus::Result<()>;
}

pub async fn serve(db: Arc<Db>) -> zbus::Result<(zbus::Connection, ActivityTracker)> {
    let tracker = ActivityTracker::new(db);
    let emitter = tracker.clone();
    let conn = ConnectionBuilder::session()?
        .name("com.Saqr.Salomnella")?
        .serve_at("/com/Saqr/Salomnella", tracker)?
        .build()
        .await?;
    emitter.set_connection(conn.clone());
    Ok((conn, emitter))
}
