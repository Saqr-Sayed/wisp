use std::sync::{Arc, OnceLock};

use wisp_core::db::Db;
use wisp_core::tracker::{unix_now, SysEvents};

use windows::core::w;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::{WTSRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION};
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, PostQuitMessage,
    RegisterClassW, TranslateMessage, MSG, WNDCLASSW, WM_DESTROY, WM_ENDSESSION,
    WM_QUERYENDSESSION, WS_OVERLAPPED,
};

const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
const WTS_SESSION_LOCK: u32 = 0x7;
const WTS_SESSION_UNLOCK: u32 = 0x8;

/// The system event sink that the hidden window's WndProc pushes into.
static SYS: OnceLock<Arc<SysEvents>> = OnceLock::new();

/// Runs a hidden message window registering for terminal-session change
/// notifications: lock -> sleep, unlock -> wake, system shutdown -> power_off.
/// The WndProc runs on this thread, so the static is safe to set here.
pub fn spawn_session_listener(sys: Arc<SysEvents>) {
    let _ = SYS.set(sys);
    std::thread::spawn(|| unsafe {
        let hinst: HINSTANCE = GetModuleHandleW(None).unwrap_or_default().into();
        let class = w!("WispWtsSession");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinst,
            lpszClassName: class,
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            return;
        }
        // CreateWindowExW returns Result<HWND> in windows 0.61.
        let Ok(hwnd) = CreateWindowExW(
            Default::default(),
            class,
            w!("Wisp"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinst),
            None,
        ) else {
            return;
        };
        let _ = WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION); // Result<()> in 0.61
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }
    });
}

extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_WTSSESSION_CHANGE => match wparam.0 as u32 {
            WTS_SESSION_LOCK => push("sleep"),
            WTS_SESSION_UNLOCK => push("wake"),
            _ => {}
        },
        WM_QUERYENDSESSION => return LRESULT(1),
        WM_ENDSESSION if wparam.0 != 0 => push("power_off"),
        WM_DESTROY => unsafe { PostQuitMessage(0) },
        _ => {}
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn push(kind: &'static str) {
    if let Some(sys) = SYS.get() {
        sys.push(kind, unix_now());
    }
}

/// Mirrors the Linux daemon: if this process instance's boot time differs from
/// what we last recorded, log "boot" at boot time and "login" at now.
pub fn record_boot_login(db: &Db) {
    let up = unsafe { GetTickCount64() } as i64 / 1000;
    let boot_time = unix_now() - up;
    match db.get_setting("last_boot") {
        Some(s) if s == boot_time.to_string() => {}
        _ => {
            db.insert_system_event("boot", "", boot_time, Some(boot_time));
            db.insert_system_event("login", "", unix_now(), Some(unix_now()));
            db.set_setting("last_boot", &boot_time.to_string());
        }
    }
}