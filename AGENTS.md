# OBS Blade Remote - agent notes

The desktop side of OBS Blade's remote access (Pro): a Rust core (`core/`)
that links OBS's local obs-websocket to the relay, wrapped as an OBS plugin
(`plugin/`) and a standalone tray / menu bar app (`standalone/`), sharing
one Qt settings panel (`ui/`). `README.md` is the human front page (downloads,
build commands, tests); this file holds what an agent needs on top.

Moved out of `Kounex/obs_blade` (`desktop/`) on 2026-11-09 with its history
(`git subtree split`). Work before that: `obs_blade`'s
`docs/changelog-agent.md` (entries 2026-11-07 .. 2026-11-09 "OBS Blade
Remote ..."). From now on: `docs/changelog.md` here.

## The other half lives in obs_blade

- The **relay** (`tool/remote_relay/`, deployed on the Hetzner VPS) and its
  protocol contract `tool/remote_relay/README.md` (frames, close codes,
  pairing, end-to-end encryption) - `core/src/wire.rs` mirrors
  `protocol.py`. A protocol change touches both repos: change the relay
  README first, then the core, then the app (`lib/utils/remote/`).
- The **app side** (pairing UI, remote sessions, `noise_ik.dart` - the
  phone's half of `core/src/e2e.rs`).
- The local clone is `~/agent/obs-blade`; `core/tests/e2e_relay.py` takes
  `--relay-dir ../obs-blade/tool/remote_relay`.

## Where to build and test

- **Never build on the NAS** (it's prod, `/tmp` is RAM). The MacBook
  (`ssh macbook`) is the dev machine: `rustup` shims at
  `/opt/homebrew/opt/rustup/bin`, scratch copy `~/agent-build/desktop`
  (rsync this repo there, excluding `target`, `.deps`, `build/`, `build_*/`,
  `build-*/` - NOT `build*`, that eats `build-lib.sh`). Its
  `plugin/.deps/obs-deps-qt6-*` is the Qt for the standalone and the panel
  preview. The dev standalone's LaunchAgent points at that build folder:
  turn "Start at login" off before deleting it.
- **Windows PC** (`ssh windows`, PowerShell; `it-env/docs/machines/windows.md`):
  CI installers, test folder `C:\obb-test` (portable OBS 32.2.2 with its own
  port 4466). GUI programs must start in the desktop session: a temporary
  scheduled task (`LogonType Interactive`) - helper
  `C:\Users\kounex\obb-launch.ps1 -Exe .. -Dir .. [-Arguments ..]`; for
  hidden helpers use `conhost.exe --headless powershell ...` (plain
  powershell opens a visible terminal). Clean up after a test: the HKCU
  `Run` value "OBS Blade Remote" and `HKCU\Software\Kounex\OBS Blade Remote`.
- **Steam Machine** (`ssh steammachine`, SteamOS, user `deck`): the Linux
  target (AppImage, Flatpak OBS). Often asleep - ask the user to wake it
  and switch to Desktop Mode (the tray needs a desktop session). SteamOS
  kills an SSH session's processes on logout (`KillUserProcesses`): start
  GUI programs with `systemd-run --user --unit=.. --collect <cmd>`
  (screenshots: `systemd-run --user --wait spectacle -b -n -f -o ..`).
  Test folder `~/obb-test` (AppImage, venv, `obs-pw`); Flatpak OBS
  `com.obsproject.Studio` is installed for the user.
- **Live relay tests** (`failover.py`, `portable_start.py`,
  `load_probe.py`) need a test id in `RELAY_PRO_ALLOW` in
  `/etc/remote-relay.env` on `hetzner-vps` (restart `remote-relay`) - add
  it for the run, **remove it right after** (count must be 0 again).

## CI, signing, releases

- `.github/workflows/ci.yml`: Windows (MSVC; the plugin template treats
  warnings as errors - e.g. double->float), Linux AppImage (Ubuntu 22.04,
  xcb only, smoke test under Xvfb), macOS universal signed + notarized.
- macOS secrets `MACOS_DEVID_*` / `MACOS_NOTARY_*` (README "CI builds and
  macOS signing"). Source: the MacBook's `~/.config/obs-blade/` (a link
  into `~/NASync/obs-blade`, backed up on the NAS): `developer-id/*.p12`
  + `p12-password`, `asc-key.p8` (App Store Connect team key QJ5A6U8X72,
  ids in `~/.localrc` `OBS_BLADE_ASC_*`). Set them by piping from the
  MacBook into `gh secret set` - values never in chat or files here.
  Developer ID certificates expire 2027-12-09; Xcode can't make new ones
  (shows the role as Admin) - developer.apple.com with an openssl CSR.
- Over ssh, codesign needs `security unlock-keychain -p "$(cat ~/.keychain)"`
  first (else `errSecInternalComponent`).
- Windows installers are unsigned (SmartScreen warns) - a signing option
  (Azure Trusted Signing or a certificate) is still the user's decision.
- Release: bump the version in `core/Cargo.toml`, `plugin/buildspec.json`,
  `standalone/CMakeLists.txt` (`./version.sh` checks), write
  `release-notes/<version>.md`, tag `v<version>`, push the tag. The
  release job checks all five files and publishes with fixed names - the
  site and the app link `releases/latest/download/<file>`; never rename
  them.

## Gotchas worth knowing

- Qt panel: check native macOS rendering, not only Fusion
  (`ui/preview` `--native` over ssh with `QT_SCALE_FACTOR=2`); no literal
  non-ASCII in C++ sources (MSVC) - use `\u` escapes.
- OBS facts used here are from OBS's source (shallow clone into the
  scratchpad, delete after): portable config (`frontend/OBSApp.cpp`
  BASE_PATH), crash marks `.sentinel/run_*` (Safe Mode dialog), the
  Windows mutex `OBSStudioCore`.
- New scripts: check `git ls-files -s` shows `100755`.

## Git

Public `github.com/Kounex/obs-blade-remote`, branch `main`. Commit each
verified unit and push. Not tracked: build dirs, `venv/`, anything
secret (`.gitignore`).
