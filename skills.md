# skills - upgrade

Read this before editing the upgrade runtime.

## Required Context

- `~/primary/skills/component-triad.md`
- `~/primary/skills/actor-systems.md`
- `~/primary/skills/kameo.md`
- `~/primary/skills/rust-discipline.md`
- `~/primary/skills/testing.md`
- this repo's `ARCHITECTURE.md`
- `signal-upgrade/ARCHITECTURE.md`
- `owner-signal-upgrade/ARCHITECTURE.md`

## Boundary

This repo owns the upgrade runtime: library, CLI, daemon, future actor
tree, policy bootstrap, sema-engine state, migration catalogue, and
handover orchestration.

Contract records stay in `signal-upgrade` and `owner-signal-upgrade`.

## Invariants

- U1 stays scaffold-only. Do not move `sema-upgrade`, Persona
  `HandoverDriver`, or predecessor contract types into this repo in U1.
- `upgrade` and `upgrade-daemon` both take exactly one argument.
- Flag-style arguments are rejected.
- The CLI remains a daemon client when the daemon lands; it must not
  open durable state directly.
- The daemon owns future durable state and bootstrap policy handling.
- U4 is the first step that moves real migration and handover runtime
  code into this crate.
