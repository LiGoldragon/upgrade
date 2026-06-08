# INTENT — upgrade

*What the psyche has explicitly intended for this project. Synthesised
from psyche statements and the applicable workspace constraints; not
embellished. Maintenance: `primary/skills/repo-intent.md`.*

`upgrade` is the runtime leg of the upgrade triad: the `upgrade` CLI,
the `upgrade-daemon`, and the daemon's state engine for schema/version
migration orchestration. The full daemon will own the migration
catalogue, policy state, migration history, the active-version event
log, the quarantine list, and handover orchestration. Paired with the
contract repos `signal-upgrade` (ordinary migration attempts) and
`meta-signal-upgrade` (meta policy administration, selector control,
rollback, quarantine).

## Repo-scope only

This file carries daemon-side intent for `upgrade`. Wire vocabulary
stays in `signal-upgrade/INTENT.md` and `meta-signal-upgrade/INTENT.md`.
Workspace-shape intent stays in `primary/INTENT.md`.

## Goals

- Be the runtime that registers schema fingerprints and migration
  paths, gates handovers on them, and quarantines on failure — the
  natural home for the schema-engine pipeline's persistent registry.
- Build the U4 runtime against the schema-derived stack from the start:
  the macro library is the substrate, not a later conversion of
  hand-written contracts.

## Constraints

- **Both binaries take exactly one argument.** The CLI is the human
  edge and may accept NOTA. The daemon is a machine process and accepts
  only a signal-encoded rkyv configuration file. Flag-style arguments
  are rejected rather than treated as request data.
- **The runtime depends on the contracts; it does not duplicate them.**
  Wire records live in `signal-upgrade` and `meta-signal-upgrade`; the
  runtime imports them. Historical Spirit data migrations import the
  current Spirit contract from `signal-spirit`, not the retired
  `signal-persona-spirit` crate name.
- **Process lifecycle authority stays with Persona.** The upgrade daemon
  asks Persona to start next-version units rather than talking to
  systemd directly.
- **Inter-component traffic is Signal; NOTA renders only at edges.**
  Schema-derived planes carry the runtime. NOTA is the CLI/debug/audit
  text projection behind `nota-text`; the daemon default graph stays
  binary/rkyv-only and does not pull `nota-next`.
- **Skeleton honesty.** While skeletal, the binaries return typed
  CLI `RequestUnimplemented` replies rather than faking behaviour; the
  daemon validates its signal-encoded configuration argument and returns
  only a scaffold acknowledgement. The CLI and daemon open no durable
  state until the runtime substance lands.

## Anti-patterns

- Do not hand-roll the contract record definitions, dispatcher,
  catalogue, or storage descriptors that the schema pipeline emits — the
  upgrade triad orchestrates its own schema cutover as part of the macro
  library landing, given the self-reference between the schema-language
  substrate and the runtime that owns migration.

## See also

- `ARCHITECTURE.md` — role, boundaries, the skeletal U1 shape, the
  pending schema-engine upgrade, and code map.
- `../signal-upgrade/INTENT.md` — ordinary migration-attempt contract.
- `../meta-signal-upgrade/INTENT.md` — meta policy contract.
- `primary/skills/component-triad.md` — triad structure and the
  compile-time module index for migration dispatch.
