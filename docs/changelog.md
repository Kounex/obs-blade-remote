# Changelog (agent)

Running log of the work, newest first. Before 2026-11-09 this code lived
in `Kounex/obs_blade` (`desktop/`): see its `docs/changelog-agent.md`.

## 2026-11-15 - 0.4.1: quick reconnect after a relay restart

- `core/src/relay.rs` `Retry`: a relay link that held over 60 s and
  dropped retries every 0.5-1.5 s (jittered) for 15 s, then the old
  backoff. Measured on the VPS: a relay restart leaves no relay for
  ~1.5 s; the old first retry at ~2 s (4 s, 8 s if it came early) kept
  phones waiting. Relay 0.0.9 holds a phone up to 6 s for its bridge, the
  app (after 4.3.0) retries 1 s apart after such a drop. PR 1, CI green
  on all three platforms. No protocol change: works with every relay and
  app version.

## 2026-11-13 - 0.4.0 released

- Tag `v0.4.0`: CI run 43427820392 published the five downloads +
  `SHA256SUMS.txt`; obs-blade.kounex.com/remote links them
  (`releases/latest/download/<file>`, all five checked). Out before the
  app's 4.3 so dogfooding and App Review can use it (user, 2026-11-13).

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
