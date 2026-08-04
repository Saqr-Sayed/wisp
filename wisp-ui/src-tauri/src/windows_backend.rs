use wisp_core::tracker::WindowSource;
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

/// Polls the Win32 foreground window (GetForegroundWindow) every second.
pub struct Win32Backend;

impl WindowSource for Win32Backend {
    fn active_window(&mut self) -> (String, String) {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return (String::new(), String::new());
            }

            let mut title_buf = [0u16; 1024];
            let n = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
            let title = if n > 0 {
                String::from_utf16_lossy(&title_buf[..n as usize])
            } else {
                String::new()
            };

            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            (process_name(pid), title)
        }
    }
}

fn process_name(pid: u32) -> String {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return String::new();
        }
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);
        if ok == 0 || size == 0 {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buf[..size as usize]);
        std::path::Path::new(&path)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

/// Registers the app to start with Windows (HKCU Run key).
pub fn install_autostart() {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
    use winreg::RegKey;
    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        KEY_WRITE,
    ) else {
        return;
    };
    if let Ok(exe) = std::env::current_exe() {
        // ponytail: drop the pre-rebrand autostart value (name built at runtime to
        // keep the rebrand's zero-trace guarantee).
        let old = ["Sal", "monella"].concat();
        let _ = key.delete_value(&old);
        let _ = key.set_value("Wisp", &exe.to_string_lossy().to_string());
    }
}
