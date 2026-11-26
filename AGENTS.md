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
- `core/tests/e2e_relay.py` takes `--relay-dir <obs_blade clone>/tool/remote_relay`.

## Where to build and test

- **Maintainer checkout:** if `docs/private/` exists, read
  [`docs/private/maintainer-workflow.md`](docs/private/maintainer-workflow.md)
  first (gitignored, maintainer clone only): which machines build and test
  what, their paths and helper scripts, where the signing material lives,
  and the live relay test allowlist. Without it you're in a contributor
  clone; `README.md` covers building and testing.
- Copying the repo into a separate build folder: exclude `target`, `.deps`,
  `build/`, `build_*/`, `build-*/` - NOT `build*`, that eats
  `build-lib.sh`. `plugin/.deps/obs-deps-qt6-*` is the Qt for the
  standalone and the panel preview. A dev standalone's LaunchAgent points
  at its build folder: turn "Start at login" off before deleting it.
- **Windows over SSH:** GUI programs must start in the desktop session: a
  temporary scheduled task (`LogonType Interactive`); for hidden helpers
  use `conhost.exe --headless powershell ...` (plain powershell opens a
  visible terminal). Clean up after a test: the HKCU `Run` value "OBS Blade
  Remote" and `HKCU\Software\Kounex\OBS Blade Remote`.
- **SteamOS / Linux over SSH:** the tray needs a desktop session (Desktop
  Mode). SteamOS kills an SSH session's processes on logout
  (`KillUserProcesses`): start GUI programs with
  `systemd-run --user --unit=.. --collect <cmd>` (screenshots:
  `systemd-run --user --wait spectacle -b -n -f -o ..`).
- **Live relay tests** (`failover.py`, `portable_start.py`,
  `load_probe.py`) need a test id in the relay's `RELAY_PRO_ALLOW` - add
  it for the run, **remove it right after** (count must be 0 again).

## CI, signing, releases

- `.github/workflows/ci.yml`: Windows (MSVC; the plugin template treats
  warnings as errors - e.g. double->float), Linux AppImage (Ubuntu 22.04,
  xcb only, smoke test under Xvfb), macOS universal signed + notarized.
- macOS secrets `MACOS_DEVID_*` / `MACOS_NOTARY_*` (README "CI builds and
  macOS signing"). Where the signing material lives, how the secrets are
  set and when the Developer ID certificates need renewing: the private
  doc above. Secret values never go in chat or in files here.
- Over ssh, codesign needs the login keychain unlocked first
  (`security unlock-keychain`, else `errSecInternalComponent`).
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
secret, and `docs/private/` (maintainer-only notes; `.gitignore`).
