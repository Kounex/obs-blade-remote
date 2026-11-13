#!/usr/bin/env python3
"""Flatpak OBS on Linux with the standalone (the AppImage): the standalone
reads the Flatpak's WebSocket settings, a phone pairs, OBS crashes, the
phone starts it again through the relay - the Flatpak, without the Safe
Mode question - and reaches it end to end.

Runs on the Linux machine (e.g. the Steam Machine in Desktop Mode) with the
standalone running and Flatpak OBS (`com.obsproject.Studio`) running with
its WebSocket server on, against a deployed relay:

    venv/bin/python3 flatpak_start.py --relay-url https://relay.obs-blade.com \
        --user <id in RELAY_PRO_ALLOW> --obs-password <pw>

The test device is removed at the end. Needs aiohttp, noiseprotocol,
cryptography.
"""

from __future__ import annotations

import argparse
import asyncio
import os
import subprocess

import aiohttp

from e2e_relay import Obs, Sealed, fail, ok
from portable_start import pair_standalone, wait_for

APP_ID = "com.obsproject.Studio"
SENTINEL = os.path.expanduser(f"~/.var/app/{APP_ID}/config/obs-studio/.sentinel")


def flatpak_obs_running() -> bool:
    out = subprocess.run(["flatpak", "ps", "--columns=application"], capture_output=True, text=True)
    return APP_ID in out.stdout.split()


def crash_marks() -> set[str]:
    return {n for n in os.listdir(SENTINEL) if n.startswith("run_")} if os.path.isdir(SENTINEL) else set()


async def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--relay-url", required=True)
    parser.add_argument("--user", required=True)
    parser.add_argument("--obs-port", type=int, default=4455)
    parser.add_argument("--obs-password", required=True)
    args = parser.parse_args()
    relay = args.relay_url.rstrip("/")

    if not flatpak_obs_running():
        fail(f"start Flatpak OBS ({APP_ID}) first")
    async with aiohttp.ClientSession() as http:
        collected, app_private, bridge_key = await pair_standalone(
            http, relay, args.user, f"ws://127.0.0.1:{args.obs_port}", args.obs_password
        )
        bridge_id, device_id = collected["bridge"]["id"], collected["device_id"]
        auth = {"Authorization": f"Bearer {collected['app_token']}"}
        ok(f"paired through the standalone ({collected['bridge']['kind']}, {collected['bridge']['name']})")
        try:
            await run(http, relay, bridge_id, auth, app_private, bridge_key)
        finally:
            async with http.delete(f"{relay}/v1/devices/{device_id}", headers=auth) as r:
                print(f"     test device removed ({r.status})", flush=True)


async def run(http, relay, bridge_id, auth, app_private, bridge_key) -> None:
    async def obs_is(state: str):
        async with http.get(f"{relay}/v1/bridges/{bridge_id}", headers=auth) as r:
            info = await r.json()
        return info["online"] and info["kind"] == "standalone" and info["obs"] == state

    await wait_for("the standalone online with OBS running", 30, lambda: obs_is("running"))
    ok("the standalone reaches Flatpak OBS (settings from ~/.var/app)")

    # The sandboxed process shows up on the host as plain "obs"
    subprocess.run(["pkill", "-9", "-x", "obs"], check=False)
    await wait_for("OBS gone", 10, lambda: asyncio.sleep(0, result=not flatpak_obs_running()))
    marks = crash_marks()
    if not marks:
        fail("no crash mark left in the Flatpak config - the test can't show they're cleared")
    ok(f"OBS killed (a crash), {len(marks)} crash mark(s) in the Flatpak config")
    await wait_for("the relay hearing OBS closed", 30, lambda: obs_is("closed"))

    async with http.post(f"{relay}/v1/bridges/{bridge_id}/start-obs", headers=auth) as r:
        if r.status != 202:
            fail(f"start-obs: {r.status} {await r.text()}")
    ok("phone asked the standalone to start OBS (202)")

    await wait_for("OBS running again (stuck at the Safe Mode question?)", 90, lambda: obs_is("running"))
    if not flatpak_obs_running():
        fail("OBS runs again, but not the Flatpak")
    left = marks & crash_marks()
    if left:
        fail(f"old crash marks still there: {left}")
    ok("Flatpak OBS started, crash marks cleared, no Safe Mode question")

    app_socket = f"{relay.replace('http', 'ws', 1)}/v1/app?bridge={bridge_id}"
    async with http.ws_connect(app_socket, headers=auth, protocols=("obswebsocket.json",)) as phone_ws:
        sealed = Sealed(phone_ws, app_private, bridge_key)
        await sealed.handshake()
        phone = Obs(sealed)
        await phone.identify(None)
        version = await phone.request("GetVersion")
        ok(f"phone reaches OBS {version['responseData']['obsVersion']} through the relay, end to end")


if __name__ == "__main__":
    asyncio.run(main())
