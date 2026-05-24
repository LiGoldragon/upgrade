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

- `src/invocation.rs` classifies the single argument as inline NOTA,
  a NOTA file path, or a signal-encoded file path.
- `src/placeholder.rs` emits typed placeholder replies through the new
  contracts.
- `src/bin/upgrade.rs` is the thin CLI placeholder.
- `src/bin/upgrade-daemon.rs` is the daemon placeholder.
- `tests/` holds command-shape and binary witnesses.
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

Scaffold only. U4 consumes this crate after U2 and U3 have populated the
working and owner contracts.
