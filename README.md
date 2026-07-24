# ETS2 Dispatch

External web dashboard and dispatch system for Euro Truck Simulator 2.

## Initial Scope

- Read live truck and job telemetry through the SCS Telemetry SDK.
- Expose a local WebSocket/HTTP bridge to browser clients.
- Show a browser dashboard with vehicle state, current job, position, and navigation summary.
- Support external dispatch jobs and verify their progress from telemetry.

Save-game synchronization and custom map routing are later milestones. They are intentionally outside the first prototype.

## Project Layout

```text
apps/       Web UI and local bridge applications
crates/     Shared Rust modules and protocol types
assets/     Map and other project-owned assets
fixtures/   Telemetry and save-game test fixtures
third-party/ SDK sources and license notices
tmp/        Working design and investigation notes
```

The SCS SDK license is kept with the SDK distribution and must be preserved when SDK code or substantial portions are redistributed.

The current scope and staged milestones are documented in [`tmp/project-goals.md`](tmp/project-goals.md).
