# Changelog

All notable changes to Wisp, per release (tags `v1`, `v2`, …).

## v1 — 2026-08-15

First stable release.

**Platforms**: Linux (GNOME) and Windows.

**Added**
- App & window tracking with friendly names and per-app overrides
- Media tracking: MPRIS (Linux, mime-based watching/listening) and SMTC (Windows)
- File event tracking in Desktop/Documents/Downloads/Pictures/Videos/Music (shared watcher, 3s debounce)
- System session events: boot, login, sleep, wake, power-off, logout
- Analytics: timeline, daily/weekly overview + averages, per-app reports, series/episode trees, search
- Controls: per-app/site limits with alerts, categories, archive & ignore
- Arabic + English UI, follow-system theming, desktop notifications
- SQLite storage, 100+ unit tests, Windows CI (smoke test + installer builds)
- Release artifacts: Windows NSIS/MSI installers, Linux DEB/RPM + daemon tarball