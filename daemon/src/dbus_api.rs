use std::sync::{Arc, Mutex};
use crate::db::{Db, LogEntry};
use zbus::{interface, object_server::SignalContext, ConnectionBuilder};

type Row = (i64, String, String, String, i64, i64, i64);

fn to_row(e: &LogEntry) -> Row {
    (e.id, e.event_type.clone(), e.app_name.clone(), e.window_title.clone(),
     e.start_time, e.end_time.unwrap_or(-1), e.duration.unwrap_or(-1))
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
