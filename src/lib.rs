//! Runtime scaffold for the `upgrade` triad.
//!
//! U1 keeps this crate intentionally small: command-shape validation
//! and typed placeholder replies. Migration catalogue and handover
//! runtime code arrive in U4.

mod error;
mod invocation;
mod placeholder;

pub use error::Error;
pub use invocation::{Argument, Invocation, InvocationKind};
pub use placeholder::{
    daemon_placeholder_response, ordinary_placeholder_reply, ordinary_placeholder_response,
    owner_placeholder_reply,
};
