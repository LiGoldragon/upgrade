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

## U1 Shape

U1 is intentionally skeletal. The CLI enforces the component
single-argument rule and returns typed `RequestUnimplemented` NOTA
output. The daemon placeholder enforces the daemon side of the same
rule by accepting only a signal-encoded rkyv configuration-file
argument before returning a plain scaffold acknowledgement. No `sema-upgrade`
migration modules, no Persona `HandoverDriver`, and no durable
database code are present in U1.

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
- The CLI and daemon do not open durable state in U1.
- The runtime depends on `signal-upgrade` and
  `meta-signal-upgrade`; it does not duplicate their wire records.
- Historical Spirit store migrations depend on `signal-spirit`, the
  renamed ordinary Spirit contract, not on `signal-persona-spirit`.
- U2 and U3 populate the contracts before U4 moves runtime substance.

## Status

Current-stack dependency refresh landed. The runtime carries checked-in
schema-next/schema-rust-next artifacts, uses current signal-frame
contracts for the ordinary and meta-policy sockets, routes durable
database work through sema-engine, and has no signal-core, nota-codec,
or default nota-next dependency path. The generated module is executable through tests but
is not yet the daemon's load-bearing dispatch path; the binaries still
return placeholder replies.

## Runtime Substance Path

The generated schema surface is present. The next substantive cutover is
not a schema-stack migration; it is replacing the placeholder binaries
with a real daemon that uses the emitted Signal/Nexus/SEMA roots as its
load-bearing dispatch path, opens sema-engine state for the migration
catalogue, active-version event log, and quarantine list, and drives
handover orchestration through the upgrade contracts.
