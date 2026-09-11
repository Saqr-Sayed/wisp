# Linux Autostart + Tray Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Daemon-owned autostart (`--install`/`--uninstall` writing `~/.config/autostart/wisp.desktop`) plus a shared tray icon and Settings toggle, working on every Linux DE.

**Architecture:** New `wisp-daemon/src/autostart.rs` module owns all `.desktop` file writes (daemon is the single writer); `main()` parses install flags before startup. `wisp.service` is loosened to `default.target` as best-effort fallback. Tray builder moves from Windows-only setup into a shared `setup_tray()` helper called by both Linux and Windows setups. Settings toggle is two thin tauri commands that spawn the daemon binary with the flag.

**Tech Stack:** Rust (std only, `dirs` 5 already a dependency — no new crates), Tauri 2 tray/menu APIs, Vue + `invoke`, `cargo test`.

## Global Constraints

- `.desktop` entry MUST contain exactly: `Type=Application`, `Exec=<absolute path of current wisp-daemon>`, `NoDisplay=true`, `X-GNOME-Autostart-enabled=true` — and MUST NOT contain `OnlyShowIn`.
- `Exec` is resolved from `std::env::current_exe()` at install time.
- `wisp-daemon --install` writes the file and exits 0; `wisp-daemon --uninstall` removes it and exits 0; both parsed in `main()` before normal startup.
- Daemon is the single writer of the autostart file; the UI toggle calls it via a tauri command that spawns the daemon binary with the flag.
- `wisp.service` uses `After=default.target` / `WantedBy=default.target`, no `BindsTo`/`graphical-session.target`; install failures stay ignored; the `.desktop` entry is the primary autostart path.
- Tray keeps at least the existing show/quit menu items (an empty menu breaks some Linux hosts); Linux needs `libayatana-appindicator3` documented.
- Verify with `cargo test` plus the manual DE matrix in Task 6.

---

## File Structure Map

- `wisp-daemon/src/autostart.rs` (CREATE) — sole owner of the autostart file: path resolution, `.desktop` content builder, install/uninstall/enabled fns, unit tests. No D-Bus, no tokio, std only + `dirs`.
- `wisp-daemon/src/main.rs` (MODIFY: line 1 `mod` list, top of `main()`) — flag parsing + early return. Nothing else changes.
- `wisp-daemon/src/systemd.rs` (MODIFY: `service_content()`) — loosen unit to `default.target` + unit test. `install()` logic untouched.
- `packaging/wisp.service` (MODIFY: whole file, 14 lines) — same loosening for the shipped unit file.
- `wisp-ui/src-tauri/src/lib.rs` (MODIFY) — new shared `setup_tray()` helper; Linux setup calls it; Windows setup calls it instead of its inline block; two new tauri commands (`get_autostart`, `set_autostart`) + two private path helpers.
- `wisp-ui/src/lib/dbus.ts` (MODIFY: append 2 fns) — `getAutostart` / `setAutostart` invoke wrappers.
- `wisp-ui/src/components/SettingsPage.vue` (MODIFY: general card only) — autostart toggle, hidden on non-Linux (command missing ⇒ hide).
- `wisp-ui/src/i18n/en.json`, `wisp-ui/src/i18n/ar.json` (MODIFY: 4 keys each) — toggle labels.
- `README.md` (MODIFY: 1 note block after the install commands) — tray runtime dependency.

**Scope note:** the parent spec (`docs/superpowers/specs/2026-09-11-linux-de-support-design.md`) covers three subsystems (backend chain, detection/fallback, autostart+tray+testing). This plan covers ONLY the autostart+tray third. `detect_desktop()` tests and backend work belong to separate plans.

---

### Task 1: Autostart module with unit tests

**Files:**
- Create: `wisp-daemon/src/autostart.rs`
- Modify: `wisp-daemon/src/main.rs:1` (add `mod autostart;`)
- Test: in-file `#[cfg(test)] mod tests` in `wisp-daemon/src/autostart.rs`

**Interfaces:**
- Consumes: `dirs::home_dir()` (`dirs` 5, already in `wisp-daemon/Cargo.toml`), `std::env::current_exe()`.
- Produces (used by Task 2): `pub fn install_autostart() -> std::io::Result<()>`, `pub fn uninstall_autostart() -> std::io::Result<()>`, `pub fn autostart_enabled() -> bool`.

- [ ] **Step 1: Write the failing test.** Create `wisp-daemon/src/autostart.rs` with stub functions and the full test module:

```rust
use std::path::{Path, PathBuf};

pub fn autostart_path() -> PathBuf {
    PathBuf::new()
}

pub fn desktop_content(_exe: &str) -> String {
    String::new()
}

pub fn install_to(_path: &Path, _exe: &str) -> std::io::Result<()> {
    Ok(())
}

pub fn uninstall_at(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

pub fn install_autostart() -> std::io::Result<()> {
    Ok(())
}

pub fn uninstall_autostart() -> std::io::Result<()> {
    Ok(())
}

pub fn autostart_enabled() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("wisp-autostart-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn desktop_content_matches_spec_fields() {
        let out = desktop_content("/home/u/.local/bin/wisp-daemon");
        assert_eq!(
            out,
            "[Desktop Entry]\nType=Application\nName=Wisp\nExec=/home/u/.local/bin/wisp-daemon\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n"
        );
        assert!(!out.contains("OnlyShowIn"));
    }

    #[test]
    fn install_uninstall_roundtrip() {
        let dir = tmp_file("home");
        let file = dir.join(".config/autostart/wisp.desktop");
        install_to(&file, "/opt/wisp/wisp-daemon").unwrap();
        let body = std::fs::read_to_string(&file).unwrap();
        assert!(body.contains("Exec=/opt/wisp/wisp-daemon"));
        assert!(uninstall_at(&file).is_ok());
        assert!(!file.exists());
        assert!(uninstall_at(&file).is_ok()); // idempotent: missing file is not an error
        std::fs::remove_dir_all(&dir).ok();
    }
}
```

- [ ] **Step 2: Run tests to verify they fail.**

Run: `cargo test -p wisp-daemon autostart`
Expected: FAIL — `desktop_content_matches_spec_fields` (empty string mismatch). (Note: this fails to compile until `mod autostart;` is wired — add it in Step 3 first if the compiler complains, then re-run to see the assertion FAIL.)

- [ ] **Step 3: Write minimal implementation.** Replace the stubs with:

```rust
use std::path::{Path, PathBuf};

pub fn autostart_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".config/autostart/wisp.desktop")
}

pub fn desktop_content(exe: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Wisp\nExec={exe}\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n"
    )
}

fn current_exe() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn install_to(path: &Path, exe: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, desktop_content(exe))
}

pub fn uninstall_at(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub fn install_autostart() -> std::io::Result<()> {
    install_to(&autostart_path(), &current_exe())
}

pub fn uninstall_autostart() -> std::io::Result<()> {
    uninstall_at(&autostart_path())
}

pub fn autostart_enabled() -> bool {
    autostart_path().exists()
}
```

Also edit `wisp-daemon/src/main.rs:1`:

```rust
// old:
mod dbus_api; mod gnome; mod logind; mod mpris; mod systemd;
// new:
mod autostart; mod dbus_api; mod gnome; mod logind; mod mpris; mod systemd;
```

- [ ] **Step 4: Run tests to verify they pass.**

Run: `cargo test -p wisp-daemon autostart`
Expected: PASS — both tests ok.

- [ ] **Step 5: Commit.**

```bash
git add wisp-daemon/src/autostart.rs wisp-daemon/src/main.rs
git commit -m "feat(daemon): autostart .desktop module with roundtrip tests"
```

---

### Task 2: `--install` / `--uninstall` CLI flags in daemon main

**Files:**
- Modify: `wisp-daemon/src/main.rs:56-60` (top of `main()`)
- Test: manual CLI run (no new unit test — thin glue over Task 1's tested fns)

**Interfaces:**
- Consumes: `autostart::install_autostart`, `autostart::uninstall_autostart` (Task 1).
- Produces: process exit semantics — flag consumed ⇒ print + exit 0; failure ⇒ stderr + exit 1; no flag ⇒ normal startup unchanged.

- [ ] **Step 1: Add the flag handler.** In `wisp-daemon/src/main.rs`, insert before `main()`:

```rust
/// Handles `wisp-daemon --install` / `--uninstall` (autostart .desktop entry).
/// Returns true when a flag was consumed and the daemon must exit.
fn handle_install_flags() -> bool {
    let flag = std::env::args().nth(1);
    match flag.as_deref() {
        Some("--install") => {
            match autostart::install_autostart() {
                Ok(()) => println!("autostart installed"),
                Err(e) => {
                    eprintln!("autostart install failed: {e}");
                    std::process::exit(1);
                }
            }
            true
        }
        Some("--uninstall") => {
            match autostart::uninstall_autostart() {
                Ok(()) => println!("autostart removed"),
                Err(e) => {
                    eprintln!("autostart uninstall failed: {e}");
                    std::process::exit(1);
                }
            }
            true
        }
        _ => false,
    }
}
```

And change the top of `main()`:

```rust
// old:
    println!("Wisp daemon starting...");
    systemd::install();
// new:
    println!("Wisp daemon starting...");
    if handle_install_flags() {
        return;
    }
    systemd::install();
```

- [ ] **Step 2: Verify manually.**

Run: `cargo build -p wisp-daemon && ./target/debug/wisp-daemon --install && cat ~/.config/autostart/wisp.desktop && ./target/debug/wisp-daemon --uninstall && ls ~/.config/autostart/wisp.desktop`
Expected: install prints `autostart installed`, `cat` shows the 6-line entry with `Exec=<repo>/target/debug/wisp-daemon` and no `OnlyShowIn`; uninstall prints `autostart removed` and the final `ls` fails (file gone). Re-run `--install` afterwards to leave the dev machine clean, then `--uninstall` again if you installed it only for the check.

- [ ] **Step 3: Commit.**

```bash
git add wisp-daemon/src/main.rs
git commit -m "feat(daemon): --install/--uninstall autostart flags"
```

---

### Task 3: Loosen `wisp.service` to `default.target`

**Files:**
- Modify: `wisp-daemon/src/systemd.rs:8-17` (`service_content()`)
- Modify: `packaging/wisp.service` (whole file)
- Test: in-file `#[cfg(test)] mod tests` in `wisp-daemon/src/systemd.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: unit file content valid outside GNOME; `install()` behavior unchanged (best-effort, failures ignored).

- [ ] **Step 1: Write the failing test.** Append to `wisp-daemon/src/systemd.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_unit_targets_default_target() {
        let content = service_content();
        assert!(content.contains("After=default.target"));
        assert!(content.contains("WantedBy=default.target"));
        assert!(!content.contains("graphical-session"));
        assert!(!content.contains("BindsTo"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails.**

Run: `cargo test -p wisp-daemon systemd`
Expected: FAIL with assertion on `After=default.target`.

- [ ] **Step 3: Loosen both unit files.** In `wisp-daemon/src/systemd.rs`, replace `service_content()`:

```rust
fn service_content() -> String {
    format!(
        "[Unit]\nDescription=Wisp Activity Tracker\nAfter=default.target\n\n\
         [Service]\nType=dbus\nBusName=com.saqr.wisp\n\
         ExecStart={}/wisp-daemon\nRestart=on-failure\nRestartSec=2\n\n\
         [Install]\nWantedBy=default.target",
        std::env::current_exe().unwrap_or_default().parent().unwrap_or(&PathBuf::from(".")).display()
    )
}
```

Replace `packaging/wisp.service` entirely with:

```ini
[Unit]
Description=Wisp Activity Tracker
After=default.target

[Service]
Type=dbus
BusName=com.saqr.wisp
ExecStart=%h/.local/bin/wisp-daemon
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
```

- [ ] **Step 4: Run test to verify it passes.**

Run: `cargo test -p wisp-daemon systemd`
Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add wisp-daemon/src/systemd.rs packaging/wisp.service
git commit -m "feat(daemon): loosen wisp.service to default.target for all DEs"
```

---

### Task 4: Shared tray setup (Linux + Windows)

**Files:**
- Modify: `wisp-ui/src-tauri/src/lib.rs` (add `setup_tray()` helper after `disable_pinch_zoom` at line 563; Linux setup at lines 584-588; Windows setup imports at lines 599-600 and tray block at lines 627-644)
- Modify: `README.md` (tray runtime note after the install commands)
- Test: `cargo check -p wisp-ui` + manual tray check (Task 6 matrix)

**Interfaces:**
- Consumes: existing show (`إظهار`)/quit (`خروج`) menu item ids and `default_window_icon()` — behavior identical to current Windows tray.
- Produces: `setup_tray(app: &mut tauri::App) -> tauri::Result<()>` used by both setups.

- [ ] **Step 1: Extract the shared tray helper.** Insert after `disable_pinch_zoom` (after line 563) in `wisp-ui/src-tauri/src/lib.rs`:

```rust
#[cfg(any(target_os = "windows", target_os = "linux"))]
fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder, Manager};
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
}
```

- [ ] **Step 2: Wire both setups to it.** Change the Linux setup:

```rust
// old:
    #[cfg(target_os = "linux")]
    let builder = builder.setup(|app| {
        disable_pinch_zoom(app.handle());
        Ok(())
    });
// new:
    #[cfg(target_os = "linux")]
    let builder = builder.setup(|app| {
        disable_pinch_zoom(app.handle());
        setup_tray(app)?;
        Ok(())
    });
```

In the Windows setup, delete the two import lines:

```rust
// old (lib.rs:599-600):
        use tauri::tray::TrayIconBuilder;
        use tauri::{menu::{Menu, MenuItem}, Manager};
// new:
        use tauri::Manager;
```

and replace the inline tray block (lib.rs:627-644):

```rust
// old:
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
// new:
                setup_tray(app)?;

                Ok(())
```

- [ ] **Step 3: Document the Linux tray runtime dep.** In `README.md`, directly after the `sudo apt install ./wisp-v1-linux-x86_64.deb` code block (line 71), insert:

```markdown
> **Note:** the tray icon needs `libayatana-appindicator3` at runtime
> (Debian/Ubuntu: `libayatana-appindicator3-1`; Fedora: `libappindicator-gtk3`).
> If the tray icon is missing, install that package and restart Wisp.
```

(Build-time headers are already documented at `README.md:99` and `CONTRIBUTING.md:20` — do not duplicate them.)

- [ ] **Step 4: Verify it compiles.**

Run: `cargo check -p wisp-ui`
Expected: success with no errors (warnings pre-existing only).

- [ ] **Step 5: Commit.**

```bash
git add wisp-ui/src-tauri/src/lib.rs README.md
git commit -m "feat(ui): shared tray setup for Linux and Windows"
```

---

### Task 5: Settings UI autostart toggle

**Files:**
- Modify: `wisp-ui/src-tauri/src/lib.rs` (linux `commands` mod, `use commands::{...}` list at lines 534-541, `invoke_handler![...]` at lines 569-582)
- Modify: `wisp-ui/src/lib/dbus.ts` (append 2 wrappers)
- Modify: `wisp-ui/src/components/SettingsPage.vue` (general card + script)
- Modify: `wisp-ui/src/i18n/en.json`, `wisp-ui/src/i18n/ar.json` (4 keys each)
- Test: `cargo check -p wisp-ui` + manual toggle roundtrip (Task 6)

**Interfaces:**
- Consumes: daemon `--install`/`--uninstall` flags (Task 2); UI spawns the daemon binary — daemon stays the single writer.
- Produces: tauri commands `get_autostart() -> bool`, `set_autostart(enabled: bool)`; TS `getAutostart()` / `setAutostart(enabled)`.

- [ ] **Step 1: Add the tauri commands.** Inside the `#[cfg(target_os = "linux")] mod commands` block in `wisp-ui/src-tauri/src/lib.rs` (after `set_setting`, lines 191-194), add:

```rust
    #[tauri::command]
    pub async fn get_autostart() -> Result<bool, String> {
        Ok(super::daemon_autostart_path().exists())
    }

    #[tauri::command]
    pub async fn set_autostart(enabled: bool) -> Result<(), String> {
        let flag = if enabled { "--install" } else { "--uninstall" };
        let output = std::process::Command::new(super::daemon_binary()?)
            .arg(flag)
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                Err(format!("wisp-daemon {flag} failed"))
            } else {
                Err(stderr)
            }
        }
    }
```

Add the two private helpers right after the linux `mod commands` block closes (after line 287):

```rust
#[cfg(target_os = "linux")]
fn daemon_binary() -> Result<std::path::PathBuf, String> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let next = dir.join("wisp-daemon");
            if next.exists() {
                return Ok(next);
            }
        }
    }
    // ponytail: PATH lookup — Command resolves bare names via PATH, covers /usr/bin + ~/.local/bin installs
    Ok(std::path::PathBuf::from("wisp-daemon"))
}

#[cfg(target_os = "linux")]
fn daemon_autostart_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".config/autostart/wisp.desktop")
}
```

Register both commands: add `get_autostart, set_autostart` to the `use commands::{...}` list (line 534-541) and to the `invoke_handler![...]` list (lines 569-582, next to `get_setting, set_setting`).

- [ ] **Step 2: Add the TS wrappers.** Append to `wisp-ui/src/lib/dbus.ts`:

```ts
export async function getAutostart(): Promise<boolean> { return invoke('get_autostart') }
export async function setAutostart(enabled: boolean): Promise<void> { return invoke('set_autostart', { enabled }) }
```

- [ ] **Step 3: Add the toggle to Settings.** In `wisp-ui/src/components/SettingsPage.vue`:
  1. Extend the `../lib/dbus` import with `getAutostart, setAutostart`.
  2. In `<script setup>`, after the `setLang` function (line 33), add:

```ts
const autostart = ref(false)
const showAutostart = ref(false)
async function refreshAutostart() {
  try {
    autostart.value = await getAutostart()
    showAutostart.value = true
  } catch {
    showAutostart.value = false // non-Linux: command missing, hide the toggle
  }
}
async function toggleAutostart() {
  const next = !autostart.value
  await setAutostart(next)
  autostart.value = next
}
```

  3. Add `refreshAutostart()` to the `Promise.all([...])` in `onMounted` (line 27).
  4. In the general card `.row` (after the theme block, lines 230-237), add:

```vue
          <div v-if="showAutostart">
            <h4>{{ t('settings.autostart.label') }}</h4>
            <div class="theme-toggle">
              <button class="pill" :class="{ on: autostart }" @click="toggleAutostart">{{ autostart ? t('settings.autostart.on') : t('settings.autostart.off') }}</button>
            </div>
            <p class="hint">{{ t('settings.autostart.hint') }}</p>
          </div>
```

- [ ] **Step 4: Add i18n keys.** In `wisp-ui/src/i18n/en.json` (after `settings.language.en`, line 64):

```json
  "settings.autostart.label": "Start automatically",
  "settings.autostart.hint": "Launch Wisp at login on any desktop.",
  "settings.autostart.on": "On",
  "settings.autostart.off": "Off",
```

In `wisp-ui/src/i18n/ar.json` (after its `settings.language.en` line):

```json
  "settings.autostart.label": "بدء التشغيل التلقائي",
  "settings.autostart.hint": "تشغيل Wisp عند تسجيل الدخول على أي سطح مكتب.",
  "settings.autostart.on": "مفعّل",
  "settings.autostart.off": "مغلق",
```

- [ ] **Step 5: Verify compile + toggle roundtrip.**

Run: `cargo check -p wisp-ui`
Expected: success with no errors.
Then run the app, open Settings → General, flip the toggle on: `~/.config/autostart/wisp.desktop` appears; flip off: it disappears. (Full matrix in Task 6.)

- [ ] **Step 6: Commit.**

```bash
git add wisp-ui/src-tauri/src/lib.rs wisp-ui/src/lib/dbus.ts wisp-ui/src/components/SettingsPage.vue wisp-ui/src/i18n/en.json wisp-ui/src/i18n/ar.json
git commit -m "feat(ui): autostart toggle wired to daemon --install/--uninstall"
```

---

### Task 6: Full test suite + manual DE matrix

**Files:** none (verification only — no commit).

- [ ] **Step 1: Run the full workspace test suite.**

Run: `cargo test --workspace`
Expected: all suites PASS, including `desktop_content_matches_spec_fields`, `install_uninstall_roundtrip`, `service_unit_targets_default_target`.

- [ ] **Step 2: Run the manual matrix before release.** For each DE — GNOME 50, Plasma 6 Wayland, Plasma 6 X11, Hyprland, Sway, XFCE, COSMIC:

| Check | Command / action | Expected |
|---|---|---|
| Autostart install | `wisp-daemon --install; cat ~/.config/autostart/wisp.desktop` | 6-line entry, `Exec=<abs path>`, no `OnlyShowIn`, exit 0 |
| Reboot autostart | reboot, then `ps aux \| grep wisp-daemon` | daemon running without manual start |
| Tray icon | log in, look at panel/tray | icon visible, left-click shows show/quit menu |
| Tray show | click show | main window appears and focuses |
| Tray quit | click quit | app exits 0 |
| UI toggle off | Settings → toggle off | `.desktop` file removed |
| UI toggle on | Settings → toggle on | `.desktop` file re-created |

Record results per DE; any failure is a release blocker for that DE.

---

## Self-Review

1. **Spec coverage (§3/3):** `.desktop` fields + `Exec` from `current_exe()` + no `OnlyShowIn` → Task 1 (asserted byte-exact). CLI flags + exit 0 + parsed before startup → Task 2. UI toggle spawning daemon, daemon single writer → Task 5. `wisp.service` loosened in both `systemd.rs` and `packaging/` → Task 3. Tray shared setup + show/quit kept + libayatana doc → Task 4. `cargo test` autostart roundtrip → Tasks 1, 6. Manual matrix (winner log line belongs to the detection plan; reboot autostart + tray icon checks included) → Task 6. `detect_desktop()` table tests are detection-plan scope, explicitly out of scope here.
2. **Placeholder scan:** no TBD/TODO/"similar to"/"appropriate handling" — every code step shows complete blocks; error paths return concrete messages; temp dirs use `std::process::id()` instead of a new `tempfile` dep.
3. **Type consistency:** `desktop_content(&str) -> String` used by `install_to(&Path, &str)`; `install_autostart()/uninstall_autostart() -> io::Result<()>` consumed by `handle_install_flags() -> bool`; tauri `get_autostart`/`set_autostart` names match `dbus.ts` invoke strings, the `use commands` list, and `invoke_handler!`; Vue refs `autostart`/`showAutostart` match the template. Windows setup keeps `Manager` import (still needed by `app.manage` + `get_webview_window`); tray imports move into the helper. `setup_tray(&mut App)` matches the `setup(|app| ...)` closure parameter type on both platforms.
