---
name: Wisp
description: Warm, playful desktop screen-time tracker with a rose-ring wisp mark
colors:
  wisp-rose: "#e94560"
  blush-milk: "#fdf0ec"
  peach-sand: "#f9e0d5"
  blossom-border: "#f5d5c9"
  surface: "#ffffff"
  ink: "#1c1917"
  ink-muted: "#6b6459"
  accent-ink: "#ffffff"
  danger: "#dc2626"
  danger-soft: "#fef2f2"
  dark-bg: "#17130e"
  dark-surface: "#241e17"
  dark-surface-soft: "#201a12"
  dark-border: "#33291f"
  dark-ink: "#f5f0e8"
  dark-ink-muted: "#b5a992"
  dark-accent: "#ff5c8a"
  dark-accent-ink: "#1c1917"
  cat-reading: "#16a34a"
  cat-games: "#9333ea"
  cat-entertainment: "#f59e0b"
  cat-productivity: "#2563eb"
  cat-browsing: "#64748b"
  cat-other: "#8a7f6e"
typography:
  display:
    fontFamily: "Cairo, system-ui, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 900
    fontFeature: "tabular-nums"
  title:
    fontFamily: "Cairo, system-ui, sans-serif"
    fontSize: "1.05rem"
    fontWeight: 900
    letterSpacing: "-0.02em"
  body:
    fontFamily: "Cairo, system-ui, sans-serif"
    fontSize: "0.9rem"
    fontWeight: 600
  label:
    fontFamily: "Cairo, system-ui, sans-serif"
    fontSize: "0.8rem"
    fontWeight: 700
rounded:
  sm: "10px"
  md: "14px"
  full: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
components:
  button-primary:
    backgroundColor: "{colors.wisp-rose}"
    textColor: "{colors.accent-ink}"
    rounded: "{rounded.sm}"
    padding: "0.45rem 1.1rem"
    typography: "{typography.label}"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.sm}"
    padding: "0.45rem 1.1rem"
    typography: "{typography.label}"
  pill:
    backgroundColor: "{colors.peach-sand}"
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.full}"
    padding: "0.3rem 0.9rem"
    typography: "{typography.label}"
  pill-on:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.blush-milk}"
    rounded: "{rounded.full}"
    padding: "0.3rem 0.9rem"
    typography: "{typography.label}"
  period-switch:
    backgroundColor: "{colors.peach-sand}"
    rounded: "{rounded.full}"
    padding: "0.15rem"
  card:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.md}"
  icon-button:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.sm}"
    size: "34px"
  day-pill-today:
    backgroundColor: "{colors.wisp-rose}"
    textColor: "{colors.accent-ink}"
    rounded: "{rounded.full}"
    size: "26px"
---

# Design System: Wisp

## 1. Overview

**Creative North Star: "The Wisp's Corner"**

Wisp lives in the corner of the desktop like a small, friendly companion: it keeps watch over time, and when it speaks, it speaks softly. The whole system is built on that posture: warm rose-blush surfaces, pill-shaped controls that beg to be clicked, and an interface dense enough to answer "where did my time go?" in five seconds, yet soft enough that a limit warning never feels like a scolding.

The aesthetic philosophy is *squeezable utility*. Every control is rounded, plump, and inviting; every surface is warm and tinted toward the rose hue; nothing in the app is allowed to feel corporate, clinical, or surveilling. Depth is conveyed through gentle rose-tinted lift rather than hard gray shadows, and the accent rose is spent sparingly so it always marks something live: today, the selected day, the primary action, the brand mark.

This system explicitly rejects: generic SaaS dashboards (centered hero branding, gray-blue palettes, template card grids), system-monitor clutter (widget-everywhere density, technical overload), and anything that reads "webapp" inside a desktop shell (centered wordmark heroes, no persistent chrome, mouse-only paths).

**Key Characteristics:**
- Warm blush-milk surfaces, everything tinted toward rose — never neutral gray-beige
- Pill-shaped controls with plump radii (10px minimum, 999px for pills)
- Rose-tinted ambient shadows, max 12px blur, gentle lift on hover
- RTL-first (Arabic), Cairo as the single typeface
- Desktop-native toolbar chrome: brand, period switch, week strip, persistent settings gear
- One accent rose spent sparingly: today, selection, primary actions, the mark

## 2. Colors

A warm rose-blush palette: the surfaces are milk-warm with rose undertones, the accent is a vivid berry rose, and every shadow carries a rosy blush instead of gray.

### Primary
- **Wisp Rose** (#e94560): The accent. Used for the brand mark and wordmark, "today" in the week strip, the selected state of the segmented period switch, primary buttons, limit warnings in charts, and the active settings gear. Spent sparingly: one dominant rose element per screen.
- **Dark Wisp Rose** (#ff5c8a): The dark-mode accent, lifted in lightness to hold 3:1+ contrast against dark surfaces.

### Neutral
- **Blush Milk** (#fdf0ec): The app background. A warm off-white tinted toward the rose hue — never plain beige, never pure white.
- **Peach Sand** (#f9e0d5): The soft surface layer: pill backgrounds, hover fills, skeleton pulse, segmented-switch track.
- **Blossom Border** (#f5d5c9): Hairline borders on cards, icon buttons, and ghost buttons. A border must be a rose-tinted whisper, never gray.
- **Surface** (#ffffff): Cards and elevated containers.
- **Ink** (#1c1917): Primary text, selected pill fill.
- **Ink Muted** (#6b6459): Labels, secondary text, unselected pill text. 4.5:1+ on Blush Milk.
- **Danger** (#dc2626) / **Danger Soft** (#fef2f2): Limit-exceeded banners only.
- **Dark mode**: dark-bg #17130e, dark-surface #241e17, dark-surface-soft #201a12, dark-border #33291f, dark-ink #f5f0e8, dark-ink-muted #b5a992 — the same rose-warm story in low light.

### Category Colors
Seven data colors for app/category breakdowns: Media = Wisp Rose, Reading = green (#16a34a), Games = violet (#9333ea), Entertainment = amber (#f59e0b), Productivity = blue (#2563eb), Browsing = slate (#64748b), Other = taupe (#8a7f6e). Dark mode uses the lifted variants (#34d399, #c084fc, #fbbf24, #60a5fa, #94a3b8, #a8a29e).

### Named Rules
**The Rose Rarity Rule.** Wisp Rose covers at most ~10–15% of any given screen. Its rarity is the point: it marks *now* and *action*, so the eye always knows where the live thing is.

**The Blush-Not-Beige Rule.** Every neutral is tinted toward the rose hue. A surface that looks neutral-gray in context is wrong. If it looks like a SaaS palette, the warmth has leaked out.

## 3. Typography

**Display Font:** Cairo (with system-ui, sans-serif fallback)
**Body Font:** Cairo (same family, weight contrast carries hierarchy)

**Character:** One rounded, humanist Arabic-capable face used at every size; hierarchy comes entirely from weight (600/700/800/900) and size, not from adding a second font. Warm, geometric, friendly — a wordmark and a data number should feel like siblings.

### Hierarchy
- **Display** (900, 1.5rem, tabular-nums): The headline numbers — today's and week's totals in the overview card. Always `font-variant-numeric: tabular-nums` so durations don't jitter on refresh.
- **Title** (900, 1.05rem, -0.02em): The brand wordmark in the header toolbar.
- **Body** (600, 0.9rem): Buttons, banners, general content.
- **Label** (700–800, 0.7–0.8rem): Pills, section titles, card labels, week strip letters, tooltips, badges.

### Named Rules
**The One-Face Rule.** Cairo is the only typeface. No display pairing, no mono, no second family — weight contrast is the entire hierarchy machine.

**The Tabular Numbers Rule.** Every duration figure uses tabular numerals. Numbers that shift width on every refresh read as broken.

## 4. Elevation

Gentle lift. The system is warm and flat by default; cards sit on the Blush Milk ground with a whisper of rose-tinted ambient shadow, and interactive elements rise 1px with a slightly deeper blush when hovered. Shadows are never neutral gray and never hard: they are the rose hue at low opacity, blurred softly.

### Shadow Vocabulary
- **Ambient** (`0 1px 2px rgba(233,69,96,.08), 0 1px 3px rgba(233,69,96,.06)`): Cards at rest. A quiet suggestion that the surface is slightly above the background.
- **Lift** (`0 4px 12px rgba(233,69,96,.14)`): Hovered primary buttons and anything temporarily raised. Max 12px blur — a blush, not a crater.

### Named Rules
**The Gentle Lift Rule.** Shadows are always rose-tinted (rgba(233, 69, 96, …)) and never exceed 12px blur. If a shadow reads as gray or deep, it's wrong. If an element looks like a 2014-era app, the shadow is too dark and the blur is too small.

## 5. Components

Every component is soft and squeezable: plump radii, pill silhouettes, and a 1px hover lift. Nothing is allowed to feel sharp or corporate.

### Header Toolbar
The desktop-native chrome: brand mark + wordmark (inline-start), segmented period switch (يوم/أسبوع/شهر), week strip (‹ day letters ›) with tooltips, and a persistent settings gear (inline-end, active state = Wisp Rose fill). Settings also opens with Ctrl+,. The bar is 44px and earns its height with navigation, never hero branding.

### Buttons
- **Shape:** Gently curved (10px radius), plump padding (0.45rem 1.1rem).
- **Primary:** Wisp Rose fill, white text (accent-ink). Hover: rises 1px (translateY(-1px)) with the Lift shadow, 120–150ms.
- **Ghost:** Transparent, 1px Blossom Border, Ink text; hover fills Peach Sand.
- **Focus:** 2px Wisp Rose outline, 2px offset, on :focus-visible only.

### Chips / Pills
- **Style:** Full-pill (999px), Peach Sand fill, Ink Muted text, 700 weight.
- **State:** Selected (`.on`) flips to Ink fill with Blush Milk text (dashboard tabs) or Wisp Rose fill with white text (period switch). Hover darkens the text to Ink.

### Segmented Period Switch
A Peach Sand track (999px) holding transparent pills; the selected pill fills Wisp Rose. Reads as one physical control, not three buttons.

### Week Strip (day pills)
26px circles, full-pill. Default: Ink Muted letter on transparent. Today: Wisp Rose fill, white letter, weight 900. Selected: 2px Wisp Rose inset ring. Hover: Peach Sand fill. Tooltips carry the full date and day total.

### Cards / Containers
- **Corner Style:** Rounded (14px).
- **Background:** Surface white; overview card uses the same.
- **Shadow Strategy:** Ambient shadow at rest (see Elevation).
- **Border:** 1px Blossom Border.
- **Internal Padding:** 0.85–1.2rem.

### Inputs / Fields
- **Style:** Inherits ghost treatment: 1px Blossom Border, 10px radius, Ink text.
- **Focus:** 2px Wisp Rose outline, 2px offset.
- **Error:** Danger text + Danger Soft fill where an error banner is needed.

### Icon Buttons
34×34 (mini 26×26, full-pill), 1px Blossom Border, Ink icon; hover fills Peach Sand and lifts 1px. Used for week arrows, the settings gear, and dismiss actions.

### Banners
Danger Soft fill, 1px Danger border, 10px radius, 600-weight 0.85rem text, slide-in 200ms. Only for fetch errors and limit warnings.

### Skeleton / Loading
Peach Sand bars with a 1.2s pulse at 50% opacity. Never spinners in the analysis card — bars match the content shape.

## 6. Do's and Don'ts

### Do:
- **Do** keep the header a working toolbar: brand, period switch, week strip, gear in one 44px row.
- **Do** spend Wisp Rose sparingly — today, selection, primary action, the mark. One dominant rose per screen.
- **Do** tint every neutral and every shadow toward the rose hue (Blush Milk, Peach Sand, Blossom Border, rgba(233,69,96,…)).
- **Do** use full-pill radii for pills, day pills, and the period switch; 10–14px for cards and buttons.
- **Do** set durations in tabular-nums so they don't jitter on the 5-second refresh.
- **Do** keep the settings gear visible in every view, with Ctrl+, as the keyboard path.
- **Do** respect prefers-reduced-motion: all transitions and pulses collapse to instant.

### Don't:
- **Don't** build generic SaaS dashboards: no centered hero branding, no gray-blue palettes, no template card grids.
- **Don't** build system-monitor clutter: no widget-everywhere density, no technical overload, no Activity-monitor styling.
- **Don't** make it read as a webapp in a desktop shell: no centered wordmark heroes, no mouse-only paths, no chrome that disappears per view.
- **Don't** use neutral gray shadows or beige surfaces — warmth-by-hue is the brand; warmth-by-default-beige is the tell.
- **Don't** add a second typeface, ever. Weight contrast is the hierarchy.
- **Don't** use borders as color stripes (no colored border-left/right accents on cards or alerts).
- **Don't** use gradient text, glassmorphism, or hand-drawn illustrations.
- **Don't** ship any text below 4.5:1 contrast against its background, or 3:1 for large text.
