# ARCHITECTURE

## Role

`upgrade` is the runtime leg of the upgrade triad. The full daemon will
own the migration catalogue, policy state, migration history, active
version event log, quarantine list, and handover orchestration.

The ordinary contract lives in `signal-upgrade`; the meta policy
contract lives in `meta-signal-upgrade`.

`sema-upgrade` is the universal stateful schema-upgrade component for
every sema database (the persona-component typed-record stores), not a
spirit-only tool. It boots first, as engine-pre-zero infrastructure
owned by the engine manager, ahead of any persona daemon. At boot it
checks each daemon's stored schema-version-hash against the hash that
daemon's code declares, and the SEMA interface owns the transitory
dual-database concurrency that exists while a live transition is in
flight. The first test case treats the legacy intent log as a `0.01`
spirit database; the schema-spec-language eventually drives the
transforms from declarative diffs.

## Boundaries

This repo owns the `upgrade` library, the `upgrade` CLI, the
`upgrade-daemon` process, and the daemon's future state engine. It does
not own the contract record definitions.

Persona keeps process lifecycle authority. The future upgrade daemon
asks Persona to start next-version units rather than talking to systemd
directly.

## Migration as a Deployment Prerequisite

Schema migration is a workspace-wide structural prerequisite for every
deployed persona triad, not just spirit. A contract or storage-schema
change requires the running daemon and its `redb` to migrate coherently.
The deploy-restart-update flow means a daemon meeting an existing `redb`
must either find no drift or have a `sema-upgrade` path; without one, any
contract edit after first deploy breaks the next restart. This is why the
upgrade leg exists before most triads have moved any data: it is the
piece that lets a contract evolve at all once a daemon has written
durable state.

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

## Schema-Diff-Driven Upgrade Codegen

The migration surface is derived from the schema diff between a
`main`-`next` version pair, compile-time-optionally per pair. Unchanged
types emit no upgrade code at all; changed types are the ones that need
hand-written upgrade behavior. On database load the runtime runs the
needed upgrades, and an old-version message is upgraded, accepted, and
logged rather than rejected. Upgrade knowledge belongs to `next`: the
`next` crate declares the `next` schema crate as a Cargo dependency so the
generating macro sees both schemas in one place and emits the
`VersionProjection`.

Schema diffs can infer the standard migrations on their own, but
ambiguous transforms still need explicit annotations or traits — the
inferred path covers the unambiguous shape changes, and the author
supplies the rest.

The `VersionProjection` home crate is named `version-projection`, a peer
of `signal-sema`. This keeps the projection vocabulary out of the daemon
crate root: daemons should mostly carry component logic and the
algorithms engines run, while repetitive startup, runner, and transport
boilerplate lives behind libraries or macros. Generated internal
Nexus-plane nouns are not casually promoted to the daemon crate root or
the public contract surface; they belong in their plane schema or module,
and future crate boundaries keep internal Nexus vocabulary separate from
the wire-facing signal APIs.

When an old-version message is upgraded and accepted, the runtime emits
an observable event so introspection, routers, or agents can notify the
source to upgrade its own client schema.

## Code Map

- `schema/lib.schema` declares the first real schema source for
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
  `nota`, `nota-next`, `nota-codec`, or `signal-core`; `nota-text` is an
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
- The runtime does not hand-roll the contract record definitions,
  dispatcher, catalogue, or storage descriptors that the schema pipeline
  emits. The upgrade triad orchestrates its own schema cutover as part of
  the macro-library landing, given the self-reference between the
  schema-language substrate and the runtime that owns migration: the U4
  runtime is built against the schema-derived stack from the start, with
  the macro library as the substrate rather than a later conversion of
  hand-written contracts.
- Schema and upgrade work carries explicit provenance, so a migration's
  origin is recoverable rather than implied.

## Upgrade-Testing Pipeline

A schema change that affects stored data is accepted only through an
upgrade-testing pipeline rather than applied directly. The pipeline
derives the new code, starts a newly compiled daemon, tests it against a
minimal specified database first, and then tests it against a disposable
copy of the live database before the schema change is accepted. The
minimal-database pass catches the obvious shape errors cheaply; the
live-copy pass exercises the migration against real data without
endangering the running store.

## Version-Divergence Recovery

When the `next` version fails — catastrophically or with only partial
support — `main` recovers the caller's intent. It reconstructs what it
can from the original message through partial application and records the
divergence, treating partial support the same as a flat
cannot-do-at-all so that caller intent is preserved across a version
divergence rather than silently degraded.

Dual-version upgrade replies distinguish an old-database write failure
from a new-database write failure, so a caller can tell which side of a
live transition broke.

## Upgrade Substrate

The component upgrade substrate is Nix-flake versions, universal to every
component. Each component's flake captures running-production (the last
known-working `main`, pinned), the local in-development version, and named
variants such as `unstable` and `testing` as flake inputs; every upgrade
sequence is expressed in this flake-input-as-deployed-version structure,
and a release tags the whole dependency surface it uses. Because tests run
through Nix, the discipline is commit-first: uncommitted local state is
invisible to the evaluator.

The endgame is a workspace-owned content-addressed vertical stack. A
workspace content-addressed store replaces Nix store-signing, composing
Criome for authentication, `forge` for build (replacing Nix builders),
and `sema-upgrade` for database migration, so every layer from build
through authentication to distribution lives in the persona vocabulary.

## Status

Current-stack dependency refresh landed. The runtime carries checked-in
schema/schema-rust artifacts, uses current signal-frame
contracts for the ordinary and meta-policy sockets, routes durable
database work through sema-engine, and has no signal-core, nota-codec,
or default nota dependency path. The migration catalogue engine now
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
