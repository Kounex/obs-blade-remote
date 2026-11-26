# Changelog (agent)

Running log of the work, newest first. Before 2026-11-09 this code lived
in `Kounex/obs_blade` (`desktop/`): see its `docs/changelog-agent.md`.

## 2026-11-26 - 0.4.3: the relay tells of a newer release

- The relay (obs_blade `tool/remote_relay`, commit af8fa5af) reads this
  repo's latest GitHub release every 6 h and sends `update_available
  {version}` to a bridge behind it (after the welcome, or when a release
  comes out while it's connected). `relay.rs` hands it to
  `Shared::offer_update`: kept only if it's a plain release newer than
  this build, logged once, `state.update_available`. Panel: a hint with a
  download link (also while deferred to the plugin - an old standalone
  needs it too); standalone tray: "Download OBS Blade Remote X..." item.
  Bridges before this release ignore the message: they only hear of
  updates through the app (bridge info `update_available`).
- Checked on the MacBook (OBS 32.2.2, plugin moved out for the run):
  `e2e_relay.py` plain, and with `--obs-running-false --latest-bridge
  9.9.9 --stop-obs-cmd` (new flags) - all green: 0.4.2's fix on a device
  for the first time (session opens, warning logged, 4901 after OBS
  quit) and the offer in the bridge info and the bridge's state. Panel
  rendered (Fusion dark + native macOS, `ui/preview` states
  `update_available`, `deferred_update_available`). The tray item is
  only compile-checked: the standalone has no relay URL override.
- The local relay picked 0.4.2 up from GitHub by itself during the run.
- PR 3, CI green on all three platforms; released as 0.4.3 (below).

## 2026-11-22 - 0.4.2: a live OBS connection beats the process check

- Relay logs 2026-11-20..22 (obs_blade `docs/changelog-agent.md`
  2026-11-22): a standalone bridge (0.4.1) closed every phone session with
  4901 "obs not running" within 0 s. `session.rs` sent that from the
  shell's process check alone (`obs_process.cpp`: `obs64.exe` / `pgrep -ix
  obs`) without trying OBS, while the monitor's own OBS connection can be
  identified ("running" at the relay). A missed exe name locks phones out.
  One of two candidates - the other is the app's Start OBS wait pausing in
  the background (fixed in the app); relay 0.0.14 counts 4901 sent while
  the bridge reports running, which tells them apart for bridges <= 0.4.1.
- `session.rs` `refuse_without_trying()`: no early 4901 while the
  monitor is identified (`Params.obs_answering`); close codes after a
  failed connect unchanged (a closed OBS still gets 4901, so the app still
  offers Start OBS). `monitor.rs` logs a warning when OBS answers but its
  process wasn't found. Standalone: a phone's Start OBS (and the tray
  item) doesn't start OBS / clear crash marks while OBS answers.
- PR 2, CI green on all three platforms (unit tests for the predicate).
  Not tested on a device: MacBook, Windows PC and Steam Machine were all
  unreachable on 2026-11-22. Check on the next device session: `obb
  --obs-running false` with OBS up -> the warning, and a phone session
  opens.
- Released: tag `v0.4.2`, CI run 45056477969 published the five downloads
  + `SHA256SUMS.txt`; `releases/latest/download/<file>` serves 0.4.2.

## 2026-11-15 - 0.4.1: quick reconnect after a relay restart

- `core/src/relay.rs` `Retry`: a relay link that held over 60 s and
  dropped retries every 0.5-1.5 s (jittered) for 15 s, then the old
  backoff. Measured on the VPS: a relay restart leaves no relay for
  ~1.5 s; the old first retry at ~2 s (4 s, 8 s if it came early) kept
  phones waiting. Relay 0.0.9 holds a phone up to 6 s for its bridge, the
  app (after 4.3.0) retries 1 s apart after such a drop. PR 1, CI green
  on all three platforms. No protocol change: works with every relay and
  app version.
- Released: tag `v0.4.1`, CI run 43656924115 published the five downloads
  + `SHA256SUMS.txt`; all six answer at `releases/latest/download/<file>`.

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
