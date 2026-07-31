use std::sync::Arc;
use crate::db::{Db, LogEntry};
use zbus::{interface, ConnectionBuilder};

type Row = (i64, String, String, String, i64, i64, i64);

fn to_row(e: &LogEntry) -> Row {
    (e.id, e.event_type.clone(), e.app_name.clone(), e.window_title.clone(),
     e.start_time, e.end_time.unwrap_or(0), e.duration.unwrap_or(0))
}

pub struct ActivityTracker {
    db: Arc<Db>,
}

impl ActivityTracker {
    pub fn new(db: Arc<Db>) -> Self { ActivityTracker { db } }
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
}

pub async fn serve(db: Arc<Db>) -> zbus::Result<()> {
    let tracker = ActivityTracker::new(db);
    let _conn = ConnectionBuilder::session()?
        .name("com.Saqr.Salomnella")?
        .serve_at("/com/Saqr/Salomnella", tracker)?
        .build()
        .await?;
    futures::future::pending::<()>().await;
    Ok(())
}
