//! Runtime for the `upgrade` triad.
//!
//! This crate owns compiled migration modules and the production
//! adjacent-version handover client/driver. The ordinary and owner
//! signal vocabularies live in `signal-upgrade` and
//! `owner-signal-upgrade`.

mod catalogue;
mod error;
mod execution;
mod handover;
mod invocation;
mod migrations;
mod placeholder;

pub use catalogue::{
    DatabaseMigration, DatabaseMigrationError, DatabaseMigrationResult, MigrationCatalogue,
    MigrationModule, ModuleResult,
};
pub use error::Error;
pub use execution::{Command, Effect, Engine, EngineError, Lowering, first_reply};
pub use handover::{
    DrivenHandover, HandoverClient, HandoverDriver, HandoverEndpoint, HandoverFrameCodec, Prepared,
    ReceivedHandoverRequest, SocketPath, Target, TargetInput, VersionLabel,
};
pub use invocation::{Argument, Invocation, InvocationKind};
pub use placeholder::{
    daemon_placeholder_response, ordinary_placeholder_reply, ordinary_placeholder_response,
    owner_placeholder_reply,
};
