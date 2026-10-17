# H1 Violet Focus-Window Logo Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the rose ring-wisp brand with the H1 violet focus-window mark across hero icon, symbolic/tray icon, header mini mark, and Linux packaging in a single pass.

**Architecture:** One 128px hero SVG (back neutral window + violet front window + white focus dot/hand) is the single source that `tauri icon` compiles into every platform icon; a separate 16px monochrome symbolic SVG serves top bar/tray/notifications; a flat 24px mini SVG replaces the App.vue inline mark; hicolor install + metainfo ride the tauri deb/rpm bundle.

**Tech Stack:** SVG (hand-authored, 2px grid), ImageMagick (`magick`), tauri CLI (`npx tauri icon`, `npx tauri build`), Vue 3 SFC, `desktop-file-validate`, `dpkg-deb`/`rpm`.

## Global Constraints

- **No squircle, no baked shadow** — free-form transparent canvas; compositor draws shadows.
- **Canvas**: hero 128px master (`source-logo.svg` 128 viewBox); symbolic 16px.
- **Grid**: 2px base grid; strokes snap to whole px at 16px.
- **Depth**: top-light plane + darker chin **≤4px** on front window; back window flat neutral (no gradient).
- **Palette**: 1–2 hues (violet + neutrals; grays don't count as hues). Flat fills preferred; at most one subtle vertical gradient on front window (light top → base).
- **Deliverables**: `source-logo.svg` (hero), `hicolor` full-color apps icons, `hicolor/symbolic` + tray, header mini SVG, `ico`/`icns` regen.
- Palette (H1, flat): front top-light `#B79FF0` · front base `#7C53C7` · chin (≤4px) `#5B3FA3` · back window `#3A3A3C` · clock/focus dot + hand `#FFFFFF` on violet (symbolic: `currentColor`) · outline (symbolic only) `currentColor`, 2px strokes converted to path.
- Tray (`wisp-ui/src-tauri/src/lib.rs:630`): `TrayIconBuilder` reuses `default_window_icon`, menu show/quit kept — **no tray-code change**, only icon asset changes.
- Tauri bundle: `tauri.conf.json` `bundle.icon` list **unchanged** (still `32x32`, `128x128`, `128x128@2x`, `icns`, `ico`); `ico`/`icns` regenerated from new source.
- Stale / carry-along (`64x64.png`, `icon.png`, `Square*.png`, `StoreLogo.png`, `android/`, `ios/`): regen via `tauri icon` but do not hand-edit; delete nothing.
- No new npm/cargo dependencies; only CLI tooling already installed (`magick`, `npx tauri`, `desktop-file-validate`).
- Spec: `docs/superpowers/specs/2026-09-12-logo-design.md` (supersedes the rose ring-wisp from `docs/superpowers/plans/2026-08-15-logo-redesign.md`).

---

## File map

| File | Responsibility |
|------|---------------|
| `wisp-ui/src-tauri/icons/source-logo.svg` (overwrite) | Hero master: back neutral window + violet front window + white dot/hand, 128 viewBox |
| `wisp-ui/src-tauri/icons/icon-master.png` (regen) | 1024² render of the hero for the repo's master asset |
| `wisp-ui/src-tauri/icons/{32x32.png,128x128.png,128x128@2x.png,icon.icns,icon.ico,…}` (regen via `tauri icon`) | Platform/bundle icon set; `bundle.icon` list unchanged |
| `wisp-ui/src-tauri/icons/hicolor/symbolic/apps/com.saqr.wisp-symbolic.svg` (create) | 16px monochrome symbolic master, `currentColor`, strokes as paths |
| `wisp-ui/src-tauri/icons/hicolor/{16x16,32x32,128x128,256x256}/apps/com.saqr.wisp.png` (create) | hicolor full-color hero PNGs rendered from `icon-master.png` |
| `wisp-ui/src-tauri/icons/hicolor/scalable/apps/com.saqr.wisp.svg` (create) | hicolor scalable hero (copy of hero master) |
| `wisp-ui/src-tauri/icons/com.saqr.wisp.metainfo.xml` (create) | metainfo source shipped to `/usr/share/metainfo/` via bundle files map |
| `wisp-ui/src-tauri/tauri.conf.json` (modify: add `bundle.linux.deb.files` + `bundle.linux.rpm.files`) | Ships hicolor tree + metainfo inside deb/rpm |
| `wisp-ui/src/App.vue:189` (modify: inline SVG only) | Header flat mini redraw; old `ring-g`/`wisp-g` gradient defs deleted by the replacement |
| `wisp-ui/src-tauri/src/lib.rs` (read-only check) | Tray wiring verification — no code change |

---

### Task 1: Redraw the hero `source-logo.svg` (H1 violet focus-window)

**Files:**
- Overwrite: `wisp-ui/src-tauri/icons/source-logo.svg`
- Test: manual GNOME App Icon Preview check (no new test files; asset-only task)

**Interfaces:**
- Consumes: palette + geometry from the spec (§1/3, Global Constraints above).
- Produces: `source-logo.svg` — the single source Task 2 renders from; `hicolor/scalable` copy source for Task 4.

- [ ] **Step 1: Overwrite `source-logo.svg` with the focus-window hero**

Write this exact content to `wisp-ui/src-tauri/icons/source-logo.svg`:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
  <defs>
    <clipPath id="frontClip">
      <rect x="36" y="52" width="64" height="56" rx="8"/>
    </clipPath>
  </defs>
  <rect x="28" y="20" width="64" height="56" rx="8" fill="#3A3A3C"/>
  <g clip-path="url(#frontClip)">
    <rect x="36" y="52" width="64" height="56" fill="#7C53C7"/>
    <rect x="36" y="52" width="64" height="22" fill="#B79FF0"/>
    <rect x="36" y="104" width="64" height="4" fill="#5B3FA3"/>
  </g>
  <path d="M68 80 L78 68" stroke="#FFFFFF" stroke-width="2.5" stroke-linecap="round"/>
  <circle cx="68" cy="80" r="6" fill="#FFFFFF"/>
</svg>
```

Why these numbers (do not "improve" them): back window `x28 y20 w64 h56` and front `x36 y52 w64 h56` are the spec geometry; their union is `x28–100 y20–108`, centered on the 128 canvas (`64,64`) with ~112px optical weight. Top-light band is 22px (~40% of 56). Chin is 4px (`y104 = 52+56−4`), the spec maximum. Front center is `(68,80)`; the hand runs to `(78,68)` (~2 o'clock). Every coordinate is a multiple of 2 (2px grid). Only flat fills — no gradient (spec prefers flat).

- [ ] **Step 2: Sanity-check palette and grid**

Run (from repo root):

```bash
grep -c "#7C53C7\|#B79FF0\|#5B3FA3\|#3A3A3C\|#FFFFFF" wisp-ui/src-tauri/icons/source-logo.svg && grep -c "e13057\|ff9db8\|e94560\|ring-g\|wisp-g" wisp-ui/src-tauri/icons/source-logo.svg; true
```

Expected: first grep prints `7` (back 1 + base 1 + band 1 + chin 1 + hand 1 + dot 1 + clip rect 0 fills… precisely: `#3A3A3C`×1, `#7C53C7`×1, `#B79FF0`×1, `#5B3FA3`×1, `#FFFFFF`×2 = 6; plus `url(#frontClip)` line contains no palette hex — so expected `6`); second grep prints `0` (no rose leftovers). If the first number is not `6`, recount against the Step 1 listing — do not proceed with a pasted-wrong file.

- [ ] **Step 3: App Icon Preview check (spec test 1, hero half)**

Open GNOME App Icon Preview, load `wisp-ui/src-tauri/icons/source-logo.svg`, confirm the artwork renders without clipping on the free-form canvas.

Expected: full back + front windows visible, transparent margins roughly even on all four sides, no squircle/shadow baked in.

- [ ] **Step 4: Commit**

```bash
git add wisp-ui/src-tauri/icons/source-logo.svg
git commit -m "feat(icons): H1 violet focus-window hero source"
```

---

### Task 2: Regenerate the bundle icon set via `tauri icon`

**Files:**
- Regen: everything under `wisp-ui/src-tauri/icons/` (`32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.ico`, `icon.icns`, carry-along sizes)
- Create: `wisp-ui/src-tauri/icons/icon-master.png` (1024²)
- `wisp-ui/src-tauri/tauri.conf.json` — read-only: `bundle.icon` list stays exactly `["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"]`

**Interfaces:**
- Consumes: `source-logo.svg` from Task 1.
- Produces: regenerated `32x32.png` etc. (Task 4 ships them; Task 5 verifies them at 32px and in dock); `icon-master.png` (Task 4 renders hicolor PNGs from it).

- [ ] **Step 1: Render the 1024 master PNG from the new source**

Run (in `wisp-ui/`):

```bash
magick -background none -density 384 src-tauri/icons/source-logo.svg -resize 1024x1024 src-tauri/icons/icon-master.png
```

(`-density 384` = 4× default 96dpi so the vector render is sharp.)

- [ ] **Step 2: Verify the master visually**

Read `wisp-ui/src-tauri/icons/icon-master.png` as an image.

Expected: dark-gray back window up-left, violet front window with lighter violet top band and dark chin, white dot + short hand near front-window center, transparent background. If ImageMagick's internal SVG renderer mangles the clip (band/chin leaking outside the rounded corners), rerun Step 1 with `-density 768`; if still wrong, note it and move on — `tauri icon` (resvg, Step 3) output is authoritative, not ImageMagick's.

- [ ] **Step 3: Regenerate all icons from the SVG**

Run (in `wisp-ui/`):

```bash
npx tauri icon src-tauri/icons/source-logo.svg
```

Expected: exit 0, no errors; `src-tauri/icons/` repopulated (png sizes, `icon.ico`, `icon.icns`, `android/`, `ios/`).

- [ ] **Step 4: Verify the bundle list (spec: all five live icons regen)**

Run (in `wisp-ui/`):

```bash
ls -la src-tauri/icons/32x32.png "src-tauri/icons/128x128.png" "src-tauri/icons/128x128@2x.png" src-tauri/icons/icon.icns src-tauri/icons/icon.ico src-tauri/icons/icon-master.png && grep -n '"icon"' src-tauri/tauri.conf.json
```

Expected: all six files exist with fresh timestamps; grep shows the `bundle.icon` list unchanged: `["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"]`. `ico`/`icns` cover Windows/macOS from the new source — no further Windows-specific work.

- [ ] **Step 5: 32px legibility check (spec accept criterion)**

Run (from repo root):

```bash
magick wisp-ui/src-tauri/icons/32x32.png -resize 800% /tmp/wisp-32-zoom.png
```

then read `/tmp/wisp-32-zoom.png` as an image.

Expected: two window planes still read as two planes; dot/hand merged into a single white dot (intended per spec). If the windows fuse into one blob, the hero geometry is wrong — fix Task 1, do not blur/sharpen the PNG.

- [ ] **Step 6: Commit**

```bash
git add wisp-ui/src-tauri/icons
git commit -m "feat(icons): regenerate set from H1 violet focus-window source"
```

---

### Task 3: Symbolic 16px master + tray wiring check

**Files:**
- Create: `wisp-ui/src-tauri/icons/hicolor/symbolic/apps/com.saqr.wisp-symbolic.svg`
- `wisp-ui/src-tauri/src/lib.rs:630-631` — read-only verification, zero code change

**Interfaces:**
- Consumes: nothing from earlier tasks (independent metaphor reduction of the same spec §2/3).
- Produces: `com.saqr.wisp-symbolic.svg` (Task 4 ships it to `hicolor/symbolic`; Task 5 checks 16px legibility + tray rendering).

- [ ] **Step 1: Write the 16px symbolic master**

Write this exact content to `wisp-ui/src-tauri/icons/hicolor/symbolic/apps/com.saqr.wisp-symbolic.svg`:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor">
  <path fill-rule="evenodd" d="M3 0h5a3 3 0 0 1 3 3v4a3 3 0 0 1-3 3H3a3 3 0 0 1-3-3V3a3 3 0 0 1 3-3Zm0 2a1 1 0 0 0-1 1v4a1 1 0 0 0 1 1h5a1 1 0 0 0 1-1V3a1 1 0 0 0-1-1H3Z"/>
  <path fill-rule="evenodd" d="M8 6h5a3 3 0 0 1 3 3v4a3 3 0 0 1-3 3H8a3 3 0 0 1-3-3V9a3 3 0 0 1 3-3Zm0 2a1 1 0 0 0-1 1v4a1 1 0 0 0 1 1h5a1 1 0 0 0 1-1V9a1 1 0 0 0-1-1H8Z"/>
  <circle cx="11" cy="11" r="2"/>
</svg>
```

Why these numbers: back outline = 2px ring around edge rect `(1,1,9,8)` (outer `0,0,11,10 r3`, inner `2,2,7,6 r1`); front outline = 2px ring around edge rect `(6,7,9,8)` (outer `5,6,11,10 r3`, inner `7,8,7,6 r1`); dot `r2` at `(11,11)`, the front-window center snapped to whole px. Strokes are pre-converted to filled paths (`fill-rule="evenodd"`), round joins approximated by the `a3 3`/`a1 1` arcs. `currentColor` only — no hard-coded hex. (Parent dirs `hicolor/symbolic/apps/` do not exist yet; create them first: `mkdir -p wisp-ui/src-tauri/icons/hicolor/symbolic/apps`.)

- [ ] **Step 2: Verify mono + path-only rules**

Run (from repo root):

```bash
grep -c "<path" wisp-ui/src-tauri/icons/hicolor/symbolic/apps/com.saqr.wisp-symbolic.svg; grep -cE "stroke|#[0-9A-Fa-f]{3,6}" wisp-ui/src-tauri/icons/hicolor/symbolic/apps/com.saqr.wisp-symbolic.svg; true
```

Expected: first command prints `2` (two outline paths); second prints `0` (no `stroke` attributes, no hard-coded colors). If either differs, the file was pasted wrong — rewrite Step 1, do not patch with sed.

- [ ] **Step 3: Tray wiring check — confirm no code change needed**

Run (from repo root):

```bash
grep -n "default_window_icon\|TrayIconBuilder" wisp-ui/src-tauri/src/lib.rs
```

Expected: matches at `lib.rs:599` (`use tauri::tray::TrayIconBuilder;`) and `lib.rs:630-631` (`.icon(app.default_window_icon().unwrap().clone())`) — tray still reuses the window icon with the existing show/quit menu. No edit. Then prove the crate still compiles (in `wisp-ui/src-tauri/`):

```bash
cargo check
```

Expected: `Finished` with no errors.

- [ ] **Step 4: Commit**

```bash
git add wisp-ui/src-tauri/icons/hicolor/symbolic
git commit -m "feat(icons): 16px symbolic focus-window master"
```

---

### Task 4: Header mini redraw + hicolor install + desktop/metainfo validation

**Files:**
- Modify: `wisp-ui/src/App.vue:189` (inline `<svg>` element only — one line)
- Create: `wisp-ui/src-tauri/icons/hicolor/{16x16,32x32,128x128,256x256}/apps/com.saqr.wisp.png`, `wisp-ui/src-tauri/icons/hicolor/scalable/apps/com.saqr.wisp.svg`, `wisp-ui/src-tauri/icons/com.saqr.wisp.metainfo.xml`
- Modify: `wisp-ui/src-tauri/tauri.conf.json` (add `bundle.linux.deb.files` + `bundle.linux.rpm.files` only; `bundle.icon` list untouched)

**Interfaces:**
- Consumes: `icon-master.png` (Task 2), hero SVG (Task 1), symbolic SVG (Task 3).
- Produces: header mini mark; hicolor tree + metainfo shipped in deb/rpm; validated `.desktop` `Icon=` entry (Task 5 does the final visual matrix).

- [ ] **Step 1: Replace the App.vue header mark with the flat mini**

Replace the entire old `<svg …>…</svg>` element on `wisp-ui/src/App.vue:189` (the one containing `<linearGradient id="ring-g"` and `<linearGradient id="wisp-g"`) with exactly:

```html
<svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true"><rect x="3" y="4" width="12" height="10" rx="2" fill="currentColor" opacity="0.55"/><rect x="9" y="10" width="12" height="10" rx="2" fill="#7C53C7"/><circle cx="15" cy="15" r="2.5" fill="#FFFFFF"/></svg>
```

Flat only: back rect `currentColor` at 55% opacity (adapts to light/dark theme), front rect violet `#7C53C7`, white dot `r2.5` at `(15,15)` = front-rect center (`x9–21`, `y10–20`). Same 22px size and position beside the wordmark. The old `ring-g`/`wisp-g` gradient defs disappear with the replaced element — that is the deletion, no second edit.

- [ ] **Step 2: Verify header — no rose leftovers, frontend builds**

Run (from repo root):

```bash
grep -rn "ring-g\|wisp-g\|e13057\|ff9db8\|e94560" wisp-ui/src/ wisp-ui/src-tauri/icons/source-logo.svg; true
```

Expected: no output (zero stale rose references anywhere). Then (in `wisp-ui/`):

```bash
npm run build
```

Expected: vite build succeeds with no errors.

- [ ] **Step 3: Build the hicolor PNG set + scalable copy from the master**

Run (from repo root):

```bash
mkdir -p wisp-ui/src-tauri/icons/hicolor/16x16/apps wisp-ui/src-tauri/icons/hicolor/32x32/apps wisp-ui/src-tauri/icons/hicolor/128x128/apps wisp-ui/src-tauri/icons/hicolor/256x256/apps wisp-ui/src-tauri/icons/hicolor/scalable/apps
magick wisp-ui/src-tauri/icons/icon-master.png -resize 16x16 wisp-ui/src-tauri/icons/hicolor/16x16/apps/com.saqr.wisp.png
magick wisp-ui/src-tauri/icons/icon-master.png -resize 32x32 wisp-ui/src-tauri/icons/hicolor/32x32/apps/com.saqr.wisp.png
magick wisp-ui/src-tauri/icons/icon-master.png -resize 128x128 wisp-ui/src-tauri/icons/hicolor/128x128/apps/com.saqr.wisp.png
magick wisp-ui/src-tauri/icons/icon-master.png -resize 256x256 wisp-ui/src-tauri/icons/hicolor/256x256/apps/com.saqr.wisp.png
cp wisp-ui/src-tauri/icons/source-logo.svg wisp-ui/src-tauri/icons/hicolor/scalable/apps/com.saqr.wisp.svg
ls wisp-ui/src-tauri/icons/hicolor/16x16/apps wisp-ui/src-tauri/icons/hicolor/32x32/apps wisp-ui/src-tauri/icons/hicolor/128x128/apps wisp-ui/src-tauri/icons/hicolor/256x256/apps wisp-ui/src-tauri/icons/hicolor/scalable/apps
```

Expected: each size dir lists exactly `com.saqr.wisp.png`; scalable dir lists exactly `com.saqr.wisp.svg`. (The `symbolic/` dir already exists from Task 3.)

- [ ] **Step 4: Write the metainfo source file**

Write this exact content to `wisp-ui/src-tauri/icons/com.saqr.wisp.metainfo.xml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>com.saqr.wisp</id>
  <name>Wisp</name>
  <summary>Private local screen-time tracker</summary>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>MIT</project_license>
  <description>
    <p>Wisp is a private, offline-first screen-time tracker. All data stays on this machine.</p>
  </description>
  <launchable type="desktop-id">com.saqr.wisp.desktop</launchable>
  <icon type="cached" width="128" height="128">com.saqr.wisp</icon>
</component>
```

(`project_license MIT` matches the repo `LICENSE`; `launchable`/`icon` use the `com.saqr.wisp` id the `.desktop` entry and hicolor install share.)

- [ ] **Step 5: Wire hicolor + metainfo into the deb/rpm bundle**

In `wisp-ui/src-tauri/tauri.conf.json`, add a `linux` section next to the existing `bundle` keys so the file reads (only the `bundle` object changes; everything else byte-identical):

```json
"bundle": { "active": true, "targets": ["deb", "rpm"], "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"], "linux": { "deb": { "files": { "icons/hicolor": "/usr/share/icons/hicolor", "icons/com.saqr.wisp.metainfo.xml": "/usr/share/metainfo/com.saqr.wisp.metainfo.xml" } }, "rpm": { "files": { "icons/hicolor": "/usr/share/icons/hicolor", "icons/com.saqr.wisp.metainfo.xml": "/usr/share/metainfo/com.saqr.wisp.metainfo.xml" } } } },
```

If the installed tauri-cli schema rejects `bundle.linux.deb.files`/`bundle.linux.rpm.files` (build error in Step 6 names the key), look up the correct extra-files key in the schema referenced at the top of this same file (`$schema`) and use exactly that key — same source→dest mapping, no other change.

- [ ] **Step 6: Build the deb and validate the `.desktop` entry (spec tests 4 + `Icon=` rule)**

Run (in `wisp-ui/`):

```bash
npx tauri build --bundles deb
```

Expected: build succeeds; exactly one `target/release/bundle/deb/*.deb` appears. Then (from repo root, replacing `WISP_DEB` with that path):

```bash
WISP_DEB=$(ls -t wisp-ui/src-tauri/target/release/bundle/deb/*.deb | head -1); dpkg-deb -c "$WISP_DEB" | grep -Ei "desktop|icons/hicolor|metainfo"; dpkg-deb -f "$WISP_DEB" Package
```

Expected: listing contains `usr/share/applications/com.saqr.wisp.desktop`, the four `usr/share/icons/hicolor/*/apps/com.saqr.wisp.png` files, `usr/share/icons/hicolor/scalable/apps/com.saqr.wisp.svg`, `usr/share/icons/hicolor/symbolic/apps/com.saqr.wisp-symbolic.svg`, and `usr/share/metainfo/com.saqr.wisp.metainfo.xml`. Then:

```bash
WISP_DEB=$(ls -t wisp-ui/src-tauri/target/release/bundle/deb/*.deb | head -1); rm -rf /tmp/wisp-deb && mkdir -p /tmp/wisp-deb && dpkg-deb -e "$WISP_DEB" /tmp/wisp-deb && grep "^Icon=" /tmp/wisp-deb/../data-tmp 2>/dev/null; dpkg-deb --fsys-tarfile "$WISP_DEB" | tar -xO ./usr/share/applications/com.saqr.wisp.desktop | grep "^Icon="; dpkg-deb --fsys-tarfile "$WISP_DEB" | tar -xO ./usr/share/applications/com.saqr.wisp.desktop > /tmp/com.saqr.wisp.desktop && desktop-file-validate /tmp/com.saqr.wisp.desktop && echo DESKTOP-VALIDATE-CLEAN
```

Expected: `Icon=com.saqr.wisp` (points at hicolor, per spec — not a path, not `wisp`) and the final line `DESKTOP-VALIDATE-CLEAN` (`desktop-file-validate` prints nothing on success; the echo proves it ran clean). Ignore the stray first `grep` (it intentionally finds nothing in the control dir).

- [ ] **Step 7: Commit**

```bash
git add wisp-ui/src/App.vue wisp-ui/src-tauri/icons/hicolor wisp-ui/src-tauri/icons/com.saqr.wisp.metainfo.xml wisp-ui/src-tauri/tauri.conf.json
git commit -m "feat(icons): header mini, hicolor install, metainfo, bundle wiring"
```

---

### Task 5: Final verification matrix (dock light/dark, 32px, tray 16, deb/rpm bundle)

**Files:** none modified. Installs the Task 4 deb on the dev machine (and inspects the rpm payload without installing).

**Interfaces:**
- Consumes: Task 4 deb artifact + installed desktop icon cache.
- Produces: pass/fail for each of the four spec tests (§3) — the definition of done for this plan.

- [ ] **Step 1: Install the deb and refresh the icon cache**

Run:

```bash
WISP_DEB=$(ls -t wisp-ui/src-tauri/target/release/bundle/deb/*.deb | head -1); sudo dpkg -i "$WISP_DEB" && gtk-update-icon-cache -f /usr/share/icons/hicolor 2>/dev/null; touch ~/.local/share/applications/com.saqr.wisp.desktop 2>/dev/null; true
```

Expected: `dpkg -i` sets up `wisp` without errors. (If an older rose-icon Wisp is installed, `dpkg -i` upgrades it in place — that is the intent.)

- [ ] **Step 2: Dock light/dark check (spec tests 1+2)**

With the app installed: open GNOME App Grid, toggle Settings → Appearance → Style to Light, then Dark. At each style, look at the Wisp grid icon and the running-app dock tile.

Expected: hero renders without clipping at both styles (App Icon Preview result from Task 1 holds on-device); back window `#3A3A3C` stays legible on light and distinct on dark. FAIL → revisit back-window neutrality (spec Decision: neutral dark-gray restored the contrast) — do not invent a new hue.

- [ ] **Step 3: 32px + tray-16 check (spec tests 1+3, on-device)**

Shrink the dock / set dash-to-dock tile size to 32px; then launch Wisp so its tray icon appears in the top bar (GNOME Shell with AppIndicator extension) and trigger a notification.

Expected: at 32px the two planes still read and dot/hand merge to one dot (matches the `/tmp/wisp-32-zoom.png` check from Task 2); top-bar tray and notification render the monochrome symbolic (two outlines + solid dot distinguishable at 16px), tinted by the theme via `currentColor`. If the tray shows the full-color hero instead of the symbolic, that is a follow-up code task (tray icon-id selection in `lib.rs`) — record it, do not ad-lib tray code in this plan (spec mandates no tray-code change).

- [ ] **Step 4: rpm payload check (no install)**

Run (in `wisp-ui/`):

```bash
npx tauri build --bundles rpm && rpm -qlp $(ls -t src-tauri/target/release/bundle/rpm/*.rpm | head -1) | grep -E "applications|hicolor|metainfo"
```

Expected: build succeeds; listing shows the `.desktop` file, all hicolor PNGs + scalable SVG + symbolic SVG, and the metainfo XML — mirroring the deb payload from Task 4 Step 6.

- [ ] **Step 5: No-stragglers check and final status**

Run (from repo root):

```bash
git status --short; grep -rn "ring-g\|wisp-g\|e13057\|ff9db8\|e94560" wisp-ui/src/ wisp-ui/src-tauri/icons/source-logo.svg; true
```

Expected: `git status` clean (or only the intended new files if a commit was skipped), and the rose grep prints nothing. Done when Steps 2–4 all pass: report the matrix (dock light ✓/✗, dock dark ✓/✗, 32px ✓/✗, tray 16 ✓/✗, deb payload ✓/✗, rpm payload ✓/✗, desktop-validate ✓/✗).

---

## Self-review

1. **Spec coverage:** Decisions table → plan header + Task geometry (scope C full set: hero T1–T2, symbolic/tray T3, header/packaging T4, matrix T5 ✓; metaphor F focus-window in all three SVGs ✓; Foliate/Whisp HIG: no squircle/shadow, 128/16 canvas, 2px grid, chin ≤4px, flat fills ✓; H1 violet palette hexes verbatim ✓; back-window neutral `#3A3A3C` with contrast rationale ✓). §0 inventory → every bullet addressed (source-logo replaced T1; icon-master regen T2; five live icons regen T2 Step 4; carry-along regen-not-edited via Global Constraints + T2 Step 3; App.vue:189 redraw T4 Steps 1–2; lib.rs:630 no-change T3 Step 3; hicolor + Icon= + metainfo T4 Steps 3–6 ✓). §1/3 hero → T1+T2 incl. 32px accept ✓. §2/3 symbolic+tray → T3 incl. strokes-as-paths, `currentColor`, gap-carrying outlines, usage rule respected (no tray code) ✓. §3/3 header+packaging → T4 (flat mini, old defs deleted, `Icon=com.saqr.wisp`, bundle.icon unchanged, ico/icns regen) ✓. Four tests → App Icon Preview T1 Step 3; light/dark T5 Step 2; 16px legibility T5 Step 3 (+ path/mono gate T3 Step 2); desktop-file-validate T4 Step 6 ✓. Rejected alternatives → nothing to implement; T5 Step 2 FAIL clause guards the back-window decision ✓. Rollout order → task order matches (hero→tauri icon→32px; symbolic→tray; header→hicolor→tests) ✓.
2. **Placeholder scan:** no TBD/TODO/"appropriate handling"/"similar to"/undescribed steps — every code step ships exact SVG/JSON/XML, every command step states exact command + expected output. One intentional schema fallback (T4 Step 5) names the exact lookup source (`$schema` in the same file) and the invariant mapping rather than leaving a gap.
3. **Type consistency:** palette hexes identical in header, T1, T4-mini (`#7C53C7`) and metainfo id == desktop id == hicolor basename (`com.saqr.wisp`); symbolic id suffixed `-symbolic` everywhere; `bundle.icon` list byte-identical to the current `tauri.conf.json:23`; tray lines cited (`lib.rs:599,630-631`) match the verified file.
