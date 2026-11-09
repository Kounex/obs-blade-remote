# Changelog (agent)

Running log of the work, newest first. Before 2026-11-09 this code lived
in `Kounex/obs_blade` (`desktop/`): see its `docs/changelog-agent.md`.

## 2026-11-09 - Own repository

- Moved from `obs_blade/desktop/` with history: its own CI
  (`.github/workflows/ci.yml`) and releases here (tag `v<version>`, the
  built-in token - no cross-repo token), so the downloads get their own
  "Latest" without touching the app repo's FOSS APK release.
