mod db; mod classifier; mod wayland; mod dbus_api; mod systemd;

fn main() {
    println!("Salmonella daemon starting...");
    let _db = db::Db::new();
    println!("SQLite ready");
}
