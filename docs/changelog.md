# Changelog (agent)

Running log of the work, newest first. Before 2026-11-09 this code lived
in `Kounex/obs_blade` (`desktop/`): see its `docs/changelog-agent.md`.

## 2026-11-13 - AppImage on SteamOS

- Tested the CI AppImage on the Steam Machine (SteamOS 3.9, Plasma
  Wayland) with Flatpak OBS 33.0.1: window and tray icon (XWayland),
  autostart entry points at the AppImage file (not its mount), the
  Flatpak's WebSocket settings found, pairing, start after a crash
  without Safe Mode, end to end through the relay
  (`core/tests/flatpak_start.py`, new).
- The AppImage forces Qt's xcb platform: it bundles only that one, so a
  desktop-wide `QT_QPA_PLATFORM=wayland` would have stopped it starting.

## 2026-11-09 - Own repository

- Moved from `obs_blade/desktop/` with history: its own CI
  (`.github/workflows/ci.yml`) and releases here (tag `v<version>`, the
  built-in token - no cross-repo token), so the downloads get their own
  "Latest" without touching the app repo's FOSS APK release.
