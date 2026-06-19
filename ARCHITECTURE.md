# ARCHITECTURE

## Role

`upgrade` is the runtime leg of the upgrade triad. The full daemon will
own the migration catalogue, policy state, migration history, active
version event log, quarantine list, and handover orchestration.

The ordinary contract lives in `signal-upgrade`; the meta policy
contract lives in `meta-signal-upgrade`.

## Boundaries

This repo owns the `upgrade` library, the `upgrade` CLI, the
`upgrade-daemon` process, and the daemon's future state engine. It does
not own the contract record definitions.

Persona keeps process lifecycle authority. The future upgrade daemon
asks Persona to start next-version units rather than talking to systemd
directly.

## Present Shape

`upgrade` is a real, building, tested runtime library with a placeholder
daemon binary. The runtime substance is present and green:

- `MigrationCatalogue` (`src/catalogue.rs`) holds the migration modules,
  matches an `Attempt` by `(component, source, target)`, and runs both the
  in-memory and the durable sema-store migration paths.
- The Nexus/SEMA `Engine` (`src/execution.rs`) implements the generated
  `NexusEngine` and `SemaEngine` traits and runs `Inspect`,
  `AttemptUpgrade`, and `Report` through the generated Nexus runner.
- The `HandoverDriver` (`src/handover.rs`) drives the current-side
  handover protocol over `UnixStream` through a length-prefixed frame
  codec, with a marker drift-guard (component / state_sequence /
  mirrored_write_count / record_frontier) and a recovery fallback on
  completion failure.
- One proven field-migration module
  (`src/migrations/persona_spirit/version_0_1_0_to_0_1_1.rs`) demonstrates
  the layout-5 two-submodule pattern: a frozen `historical` shape, a
  `current_shape` shape, and a `From`-chain between them, reading and
  writing across two separate `.sema` stores via sema-engine.
- Active-version-change and quarantine record types (`src/event.rs`) are
  the rkyv event-log shapes the daemon will persist.

The remaining placeholders are honest and named. The CLI returns typed
`RequestUnimplemented` NOTA output. The daemon binary accepts only a
signal-encoded rkyv configuration-file argument and returns a scaffold
acknowledgement; it does not yet construct the `Engine`, open the
ordinary/meta sockets, or run the dispatch loop. Not-yet-built meta-policy
SEMA verbs (Register/Allow/Block/ForceFlip/Rollback/Quarantine and the
handover write/read verbs) return typed `NotBuiltYet` replies. The
remaining work is the daemon mount (see `## Runtime Substance Path`), the
daemon's own durable policy/history/quarantine state, the per-component
migration modules beyond `persona_spirit`, and the Persona handover wiring.

## Code Map

- `schema/lib.schema` declares the first real schema-next source for
  the runtime-side upgrade surface, including ordinary requests,
  meta-policy requests, and generated Signal/Nexus/SEMA roots.
- `src/schema/lib.rs` is the checked-in generated Rust interface;
  `build.rs` deserializes `schema/lib.schema` into `SchemaSource`,
  validates the schema-in-Rust value through text and rkyv round-trips,
  and fails the build when the generated Rust is stale.
- `src/invocation.rs` classifies the single argument as inline NOTA,
  a NOTA file path, or a signal-encoded rkyv file path. Daemon
  invocation rejects the NOTA forms.
- `src/placeholder.rs` emits the CLI's typed placeholder reply through
  the ordinary contract when `nota-text` is enabled; the daemon
  placeholder performs only binary configuration argument validation.
- `src/bin/upgrade.rs` is the thin CLI placeholder.
- `src/bin/upgrade-daemon.rs` is the daemon placeholder.
- `tests/generated_schema.rs` executes the generated runtime roots:
  short header/frame round-trip, ordinary and meta Signal -> Nexus ->
  SEMA projections, SEMA -> Signal reply projection, and typed trace
  object naming.
- `tests/` also holds command-shape, runtime, handover, and binary
  witnesses.
- `bootstrap-policy.nota` is the empty first-start policy seed.

## Invariants

- Both binaries take exactly one argument.
- Flag-style arguments are rejected rather than treated as request
  data.
- The daemon accepts only a signal-encoded rkyv configuration-file
  argument. It does not decode inline NOTA or `.nota` files.
- The default daemon/runtime dependency graph does not pull
  `nota-next`, `nota-codec`, or `signal-core`; `nota-text` is an
  explicit CLI/debug/audit projection feature.
- The migration catalogue runs a durable sema-store migration path; the
  daemon does not yet open durable state for its own policy, migration
  history, active-version log, or quarantine list, and the CLI opens none.
- The runtime depends on `signal-upgrade` and
  `meta-signal-upgrade`; it does not carry parallel hand-written
  contract records.
- Historical Spirit store migrations carry frozen source/target record
  shapes inside the migration module. They do not depend on today's
  moving `signal-spirit` contract for old table layouts.
- The contracts are schema-derived; the runtime imports their generated
  roots and projects them, rather than carrying a parallel surface.

## Status

Current-stack dependency refresh landed. The runtime carries checked-in
schema-next/schema-rust-next artifacts, uses current signal-frame
contracts for the ordinary and meta-policy sockets, routes durable
database work through sema-engine, and has no signal-core, nota-codec,
or default nota-next dependency path. The migration catalogue engine now
implements the generated `NexusEngine` and `SemaEngine` traits, and the
runtime tests drive `Inspect`, `AttemptUpgrade`, and `Report` through the
generated Nexus runner instead of the retired `signal-executor` path.

The generated module is not yet the process daemon's socket dispatch
path; the binaries still return placeholder replies. The external
contract crates are schema-derived, and `upgrade` projects their
generated `Input`/`Output` roots into the daemon's generated runtime
schema instead of depending on any hand-written channel surface.

## Runtime Substance Path

The generated schema surface is present. The next substantive cutover is
not a schema-stack migration; it is replacing the placeholder binaries
with a real daemon that uses the emitted Signal/Nexus/SEMA roots as its
load-bearing dispatch path, opens sema-engine state for the migration
catalogue, active-version event log, and quarantine list, and drives
handover orchestration through the upgrade contracts.
