use std::sync::Mutex;
use wayland_client::{
    Connection, Dispatch, QueueHandle, event_created_child,
    protocol::wl_registry::{self, WlRegistry},
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

static STATE: WlState = WlState {
    app_id: Mutex::new(String::new()),
    title: Mutex::new(String::new()),
};

struct WlState {
    app_id: Mutex<String>,
    title: Mutex<String>,
}

pub struct WlrBackend {
    conn: Connection,
    queue: wayland_client::EventQueue<WlBackend>,
}

struct WlBackend;

impl WlrBackend {
    pub fn new() -> Option<Self> {
        let conn = Connection::connect_to_env().ok()?;
        let mut queue = conn.new_event_queue();
        let qh = queue.handle();
        let _registry = conn.display().get_registry(&qh, ());
        queue.roundtrip(&mut WlBackend).ok()?;
        Some(WlrBackend { conn, queue })
    }

    pub fn active_window(&mut self) -> (String, String) {
        self.queue.dispatch_pending(&mut WlBackend).ok();
        (
            STATE.app_id.lock().unwrap().clone(),
            STATE.title.lock().unwrap().clone(),
        )
    }
}

impl Dispatch<WlRegistry, ()> for WlBackend {
    fn event(
        _state: &mut WlBackend,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<WlBackend>,
    ) {
        if let wl_registry::Event::Global { name, interface, .. } = event {
            if interface == "zwlr_foreign_toplevel_manager_v1" {
                let _ = registry.bind::<ZwlrForeignToplevelManagerV1, (), WlBackend>(name, 3, qh, ());
            }
        }
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for WlBackend {
    fn event(
        _state: &mut WlBackend,
        _proxy: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<WlBackend>,
    ) {
        if matches!(event, zwlr_foreign_toplevel_manager_v1::Event::Finished) {
            *STATE.app_id.lock().unwrap() = String::new();
            *STATE.title.lock().unwrap() = String::new();
        }
    }

    event_created_child!(WlBackend, ZwlrForeignToplevelManagerV1, [
        zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for WlBackend {
    fn event(
        _state: &mut WlBackend,
        proxy: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<WlBackend>,
    ) {
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                *STATE.app_id.lock().unwrap() = app_id;
            }
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                *STATE.title.lock().unwrap() = title;
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                *STATE.app_id.lock().unwrap() = String::new();
                *STATE.title.lock().unwrap() = String::new();
                let _ = proxy.destroy();
            }
            _ => {}
        }
    }
}
