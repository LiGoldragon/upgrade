# upgrade

`upgrade` is the runtime crate for the upgrade triad.

This U1 crate is a scaffold only. It ships the library, `upgrade`
CLI, `upgrade-daemon` placeholder, bootstrap policy seed, and tests
that enforce the one-argument command shape: NOTA at the CLI edge,
signal-encoded rkyv configuration at the daemon edge. U4 moves the
real migration catalogue and handover driver into this crate.
