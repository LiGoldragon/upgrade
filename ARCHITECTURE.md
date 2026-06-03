# ARCHITECTURE

## Role

`upgrade` is the runtime leg of the upgrade triad. The full daemon will
own the migration catalogue, policy state, migration history, active
version event log, quarantine list, and handover orchestration.

The ordinary contract lives in `signal-upgrade`; the owner-only
contract lives in `owner-signal-upgrade`.

## Boundaries

This repo owns the `upgrade` library, the `upgrade` CLI, the
`upgrade-daemon` process, and the daemon's future state engine. It does
not own the contract record definitions.

Persona keeps process lifecycle authority. The future upgrade daemon
asks Persona to start next-version units rather than talking to systemd
directly.

## U1 Shape

U1 is intentionally skeletal. The binaries enforce the component
single-argument rule and return typed `RequestUnimplemented` NOTA
output. No `sema-upgrade` migration modules, no Persona
`HandoverDriver`, and no durable database code are present in U1.

## Code Map

- `schema/lib.schema` declares the first real schema-next source for
  the runtime-side upgrade surface, including ordinary requests,
  owner-policy requests, and generated Signal/Nexus/SEMA roots.
- `schema/lib.asschema` and `src/schema/lib.rs` are checked-in
  generated artifacts; `build.rs` fails the build when they are stale.
- `src/invocation.rs` classifies the single argument as inline NOTA,
  a NOTA file path, or a signal-encoded file path.
- `src/placeholder.rs` emits typed placeholder replies through the new
  contracts.
- `src/bin/upgrade.rs` is the thin CLI placeholder.
- `src/bin/upgrade-daemon.rs` is the daemon placeholder.
- `tests/generated_schema.rs` executes the generated runtime roots:
  short header/frame round-trip, ordinary and owner Signal -> Nexus ->
  SEMA projections, SEMA -> Signal reply projection, and typed trace
  object naming.
- `tests/` also holds command-shape, runtime, handover, and binary
  witnesses.
- `bootstrap-policy.nota` is the empty first-start policy seed.

## Invariants

- Both binaries take exactly one argument.
- Flag-style arguments are rejected rather than treated as request
  data.
- The CLI and daemon do not open durable state in U1.
- The runtime depends on `signal-upgrade` and
  `owner-signal-upgrade`; it does not duplicate their wire records.
- U2 and U3 populate the contracts before U4 moves runtime substance.

## Status

Next-stack port started. The runtime now carries checked-in
schema-next artifacts beside the existing hand-written execution and
handover code. The generated module is executable through tests but is
not yet the daemon's load-bearing dispatch path; the binaries still
return placeholder replies.

## Pending schema-engine upgrade

**Status:** scheduled for migration to schema-language-based contract per `reports/designer/326-v13-spirit-complete-schema-vision.md` + `reports/designer/324-migration-mvp-spirit-handover-re-specification.md`.

**Target:** this triad's hand-written contract records, dispatcher state, persistent migration catalogue, active-version event log, quarantine list, and handover orchestration state all convert to a single `upgrade/upgrade.schema` file (with the wire surface split across `signal-upgrade` + `owner-signal-upgrade` schemas; see those files). The brilliant macro library (`primary-ezqx.1`) reads the schema(s) + emits wire types + ShortHeader projection + dispatcher + VersionProjection + storage descriptors.

**Sequence:** This component is uniquely positioned — **the upgrade triad orchestrates its own schema cutover as part of the brilliant macro library landing**. Schema-daemon's persistent registry (per `reports/designer/326-v13-spirit-complete-schema-vision.md` §4) is the upgrade triad's natural home: the schema-engine pipeline produces schema fingerprints and migration paths, and the upgrade daemon is exactly the runtime that registers them, gates handovers on them, and quarantines on failure. Spirit pilots `primary-ezqx.1` first; the upgrade triad's own schema cutover then folds into the upgrade-triad-as-schema-host work, not as a separate operator pass.

**Per-component concerns:**
- Just-landed merged triad per operator's /318 Wave-4 (upgrade@2f56e37d); the schema cutover lands on a daemon that is currently U1-skeletal. U2 + U3 + U4 work and the schema cutover may interleave rather than sequence strictly: the macro library can be the substrate the U4 runtime is built against from the start, rather than U4 landing hand-written first and converting later.
- The persistent migration catalogue, active-version event log, and quarantine list are exactly the storage shape the schema-language MVP exists to express; the upgrade triad is the natural pilot/early-adopter once Spirit clears the pilot path.
- This component's schema describes what migrations look like and how the runtime executes them — there is a self-reference between the schema-language substrate and the upgrade triad that owns runtime migration. The brilliant macro library landing should land both together.

**References:**
- `reports/designer/326-v13-spirit-complete-schema-vision.md` — uniform header form + schema-language design (§4 schema-daemon registry)
- `reports/designer/324-migration-mvp-spirit-handover-re-specification.md` — migration MVP + handover state
- `reports/designer/322-spirit-mvp-positional-schema-worked-example.md` — Spirit MVP worked example
- `reports/operator/174-schema-import-header-design-critique-2026-05-24.md` — header/body/feature separation + lowering rules
