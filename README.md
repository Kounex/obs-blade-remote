# OBS Blade Remote

Control OBS from the [OBS Blade](https://obs-blade.kounex.com) app from
anywhere - end-to-end encrypted, no port forwarding. This is the part next
to OBS: an OBS plugin and a standalone tray / menu bar app around one Rust
core. It connects OBS to the relay; the app and the relay live in
[Kounex/obs_blade](https://github.com/Kounex/obs_blade) (relay protocol
contract: [`tool/remote_relay/README.md`](https://github.com/Kounex/obs_blade/blob/master/tool/remote_relay/README.md),
design: [`docs/superpowers/specs/2026-11-07-remote-relay-design.md`](https://github.com/Kounex/obs_blade/blob/master/docs/superpowers/specs/2026-11-07-remote-relay-design.md)).

## Download

Install **one** of the two on the PC running OBS (both at once is fine too -
they hand over to each other):

| | What it is | Download |
|---|---|---|
| **OBS Blade Remote** (recommended) | A small tray / menu bar app. Sees whether OBS runs, and lets the app start OBS. | [macOS](https://github.com/Kounex/obs-blade-remote/releases/latest/download/OBS-Blade-Remote-macos.dmg) · [Windows](https://github.com/Kounex/obs-blade-remote/releases/latest/download/OBS-Blade-Remote-windows-x64-setup.exe) · [Linux (AppImage)](https://github.com/Kounex/obs-blade-remote/releases/latest/download/OBS-Blade-Remote-x86_64.AppImage) |
| **OBS Blade Remote plugin** | Runs inside OBS (Tools → OBS Blade Remote): nothing extra running. | [macOS](https://github.com/Kounex/obs-blade-remote/releases/latest/download/obs-blade-remote-plugin-macos.pkg) · [Windows](https://github.com/Kounex/obs-blade-remote/releases/latest/download/obs-blade-remote-plugin-windows-x64-setup.exe) |

Remote access is part of OBS Blade Pro. Set it up in the app: connect to
OBS on your Wi-Fi, tap the globe on the dashboard, then **Set Up Remote
Access**. Issues and feedback: [Kounex/obs_blade issues](https://github.com/Kounex/obs_blade/issues).

## Releases

One version for everything: `core/Cargo.toml`, `plugin/buildspec.json` and
`standalone/CMakeLists.txt` (`./version.sh` checks they match). Write
`release-notes/<version>.md`, then push the tag `v<version>`: CI builds,
signs and notarizes (macOS), and publishes the release here with fixed
file names, so the `releases/latest/download/<file>` links above (also used
by the site and the app) always give the newest.

## Development

| Folder | What | State |
|---|---|---|
| `core/` | Rust: relay link, one local OBS websocket per phone session, preview cap, OBS monitor (state, status snapshot, pairing via CustomEvent), stored pairing; C ABI `core/include/obs_blade_bridge.h`; `obb` CLI (headless host) | built, tested |
| `plugin/` | OBS plugin shell (C++, obs-plugintemplate 2025-12, OBS 31.1+): starts the core after load, vendor `obs-blade-bridge` `Pair` / `Status` / `OpenPanel`, Tools -> OBS Blade Remote opens the panel, clean stop on OBS exit | builds + runs on macOS (OBS 32.2.2) |
| `standalone/` | Tray / menu bar app (C++/Qt6): the core as `standalone`, OBS watcher (process: `obs64.exe` / `OBS` / `obs`), Start OBS (the executable last seen running - portable installs, with `--portable` when they have no marker file; else macOS bundle id, Windows install dir from the uninstall key + `bin\\64bit`, Linux PATH / Flatpak), start at login (LaunchAgent / HKCU Run / XDG autostart, on by default), log `standalone.log` next to `bridge.json` | builds + runs on macOS and Windows (CI: `.github/workflows/ci.yml`) |
| `ui/` | Shared Qt settings panel (`remote_panel.*`); `ui/preview/` renders every state to PNGs offscreen | built |

## Plugin (macOS)

```bash
core/build-lib.sh                 # universal libobb.a
cd plugin
cmake --preset macos                      # first run downloads OBS sources + obs-deps + Qt6 into .deps
cmake --build --preset macos --config RelWithDebInfo
cp -R build_macos/RelWithDebInfo/obs-blade-remote.plugin \
  ~/Library/Application\ Support/obs-studio/plugins/
```

Notes: the template left `PLUGIN_BUILD_NUMBER` empty outside CI (fixed in
`cmake/common/buildnumber.cmake`); obs-deps' Qt6 links AGL, which the
macOS 26 SDK dropped (filtered in `CMakeLists.txt`). The plugin and the
standalone share `bridge.json` - same bridge, same keys; the standalone
checks every 5 s whether the plugin answers `Status` and then steps back
(`relay: deferred`).

## Panel preview

```bash
cd ui/preview
cmake -B build -DCMAKE_PREFIX_PATH=<Qt6, e.g. plugin/.deps/obs-deps-qt6-*> && cmake --build build
QT_QPA_PLATFORM=offscreen QT_PLUGIN_PATH=<Qt6>/plugins build/panel_preview shots
# The platform's own style (what the standalone shows) - on the Mac itself,
# also over ssh; check both, Fusion hides native-style problems
QT_SCALE_FACTOR=2 build/panel_preview native --native
```

Each run includes `panel_*_advanced.png` with the Advanced section open.

## Core

Build and test on a dev machine (not the NAS):

```bash
cd core
cargo test && cargo clippy --all-targets
cargo build            # target/debug/obb, target/debug/libobb.a
```

End to end with a real OBS (macOS: `tool/obs_local/obs_test_env.sh start` in an
obs_blade checkout next to this one)
and the relay running locally, this script playing the phone:

```bash
python3 -m venv venv && venv/bin/pip install aiohttp==3.14.5
venv/bin/python3 core/tests/e2e_relay.py --relay-dir ../obs-blade/tool/remote_relay \
  --obb core/target/debug/obb \
  [--obs-password PW]   # when OBS runs with --websocket_password PW
  [--stop-obs-cmd "../obs-blade/tool/obs_local/obs_test_env.sh stop"]
```

It pairs through a `CustomEvent`, runs a full obs-websocket session through
the relay (no password on the phone), checks the preview cap (size and
aspect), `InputVolumeMeters`, the status snapshot and the 4901 close after
OBS quits. Last green run: OBS 32.2.2 / obs-websocket 5.7.4, macOS,
2026-11-07, with and without OBS auth.

Failover with the plugin **and** the standalone installed (real OBS, the
standalone running, a deployed relay with a test id in `RELAY_PRO_ALLOW`):

```bash
venv/bin/python3 core/tests/failover.py --relay-url https://relay.obs-blade.com --user <id>
```

Pairs a test device through the plugin, kills OBS (SIGKILL), checks the
standalone takes over, starts OBS through the relay (no Safe Mode
question - the standalone clears OBS's crash marks for phone starts),
checks the plugin is back and a phone reaches OBS end to end, removes the
device. Last green run: OBS 32.2.2, macOS, 2026-11-09 (takeover 1.7 s,
plugin back 4.3 s after the start).

A portable OBS on Windows with the standalone (CI build from
`ci.yml`, a portable OBS with `portable_mode.txt` whose WebSocket
port and password differ from the regular install's, both running, a test
id in `RELAY_PRO_ALLOW`):

```bash
py core/tests/portable_start.py --relay-url https://relay.obs-blade.com --user <id> \
  --obs-dir C:\obb-test\OBS-portable --obs-port 4466 --obs-password <pw>
```

Pairs through the standalone over the portable OBS, kills OBS, starts it
through the relay and checks the portable copy came up without the Safe
Mode question and is reachable end to end. Last green run: OBS 32.2.2
portable, Windows 11, 2026-11-09. GUI programs started over SSH land in
a session without a desktop: start the standalone and OBS through a
temporary scheduled task (`LogonType Interactive`).

Flatpak OBS on Linux with the AppImage (both running in the desktop
session, OBS's WebSocket server on, a test id in `RELAY_PRO_ALLOW`):

```bash
venv/bin/python3 core/tests/flatpak_start.py --relay-url https://relay.obs-blade.com --user <id> \
  --obs-password <pw>
```

The same steps with the Flatpak: pairs, kills OBS, starts it through the
relay (`flatpak run`), checks the crash marks in `~/.var/app/...` are
cleared and a phone reaches OBS. Last green run: OBS 33.0.1 Flatpak,
SteamOS 3.9 (KDE Plasma, Wayland), 2026-11-13. Over SSH, start programs
with `systemd-run --user` - SteamOS kills an SSH session's processes when
it ends.

The core reads obs-websocket's password from
`<OBS config>/plugin_config/obs-websocket/config.json` - for the
standalone also a portable install's `config/obs-studio` next to the OBS
executable it saw running (`bridge.json` `obs_exe`; with a marker file
it wins, after `--portable` the newest config does). Found nowhere: it
tries OBS's default port without a password and the panel asks for the
port and password under Advanced. It stores its
pairing in `<user config>/obs-blade-remote/bridge.json` (0600), shared by
the plugin and the standalone.

## CI builds and macOS signing

`.github/workflows/ci.yml` builds on every push and pull request:
Windows x64 (Inno Setup installers in `windows/`, unsigned for now), Linux
(the standalone as an AppImage, `standalone/linux/appimage.sh`) and macOS universal - the plugin as `.pkg` (installs into
`~/Library/Application Support/obs-studio/plugins`) and the standalone as
`.dmg` (Qt bundled by `standalone/macos/package.sh`), both Developer ID
signed with the hardened runtime, notarized and stapled
(`notarize.sh`). Secrets: `MACOS_DEVID_APP_P12_BASE64`,
`MACOS_DEVID_INSTALLER_P12_BASE64`, `MACOS_DEVID_P12_PASSWORD` (from
the maintainer's `~/.config/obs-blade/developer-id/`) and
`MACOS_NOTARY_KEY_BASE64` / `_KEY_ID` / `_ISSUER_ID` (the App Store
Connect team key the app releases use). Without them (forks) the macOS
builds are ad hoc signed and not notarized. Developer ID certificates
can't be made in Xcode here (it sees the role as Admin): developer.apple.com
with a CSR.

## Standalone (macOS dev build)

```bash
core/build-lib.sh
cd standalone
cmake -B build -DCMAKE_PREFIX_PATH=<Qt6, e.g. plugin/.deps/obs-deps-qt6-*> -DCMAKE_OSX_ARCHITECTURES=arm64
cmake --build build
open "build/OBS Blade Remote.app"
```

Menu bar only (`LSUIElement`); closing the window keeps it running; one
instance per user (lock file). A dev build's "Start at login" points the
LaunchAgent at the build folder - turn it off in the panel before
deleting the build.

## License

GNU GPL v3 (see LICENSE.md). The plugin links OBS Studio (GPL v2 or later).
