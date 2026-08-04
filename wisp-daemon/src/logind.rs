use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use wisp_core::tracker::{unix_now, SysEvents};
use zbus::blocking::Connection;

const DEST: &str = "org.freedesktop.login1";
const PATH: &str = "/org/freedesktop/login1";
const IFACE: &str = "org.freedesktop.login1.Manager";

/// Connect to logind, wait for `signal`, and call `handle(arg)` per emission.
/// On any failure or stream end, sleep 5s and reconnect forever.
fn listen(signal: &'static str, mut handle: impl FnMut(bool)) -> ! {
    loop {
        let conn = match Connection::system() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("logind: connect: {e}");
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }
        };
        let proxy = match zbus::blocking::proxy::Proxy::new(&conn, DEST, PATH, IFACE) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("logind: proxy: {e}");
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }
        };
        let stream = match proxy.receive_signal(signal) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("logind: {signal}: {e}");
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }
        };
        for msg in stream {
            match msg.body().deserialize::<(bool,)>() {
                Ok((v,)) => handle(v),
                Err(e) => eprintln!("logind: {signal} body: {e}"),
            }
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

pub fn spawn_listener(sys: Arc<SysEvents>, shutdown: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        listen("PrepareForSleep", |asleep| {
            sys.push(if asleep { "sleep" } else { "wake" }, unix_now());
        })
    });
    std::thread::spawn(move || {
        listen("PrepareForShutdown", |down| {
            if down {
                shutdown.store(true, Ordering::Relaxed);
            }
        })
    });
}
