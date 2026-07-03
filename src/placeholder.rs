use meta_signal_upgrade::{
    Output as OwnerOutput, RequestUnimplemented as OwnerRequestUnimplemented,
    UnimplementedReason as OwnerUnimplementedReason,
};
#[cfg(feature = "nota-text")]
use nota::NotaEncode;
use signal_upgrade::{
    Output as OrdinaryOutput, RequestUnimplemented as OrdinaryRequestUnimplemented,
    UnimplementedReason as OrdinaryUnimplementedReason,
};

use crate::{Error, Invocation};

pub fn ordinary_placeholder_reply() -> OrdinaryOutput {
    OrdinaryOutput::RequestUnimplemented(OrdinaryRequestUnimplemented::new(
        OrdinaryUnimplementedReason::NotBuiltYet,
    ))
}

pub fn owner_placeholder_reply() -> OwnerOutput {
    OwnerOutput::RequestUnimplemented(OwnerRequestUnimplemented::new(
        OwnerUnimplementedReason::NotBuiltYet,
    ))
}

#[cfg(feature = "nota-text")]
pub fn ordinary_placeholder_response<I, S>(arguments: I) -> Result<String, Error>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let _invocation = Invocation::from_program_arguments(arguments)?;
    Ok(encode_nota(&ordinary_placeholder_reply()))
}

pub fn daemon_placeholder_response<I, S>(arguments: I) -> Result<String, Error>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let invocation = Invocation::from_program_arguments(arguments)?;
    invocation.require_signal_file_argument()?;
    Ok(String::from(
        "upgrade-daemon accepted signal-encoded configuration",
    ))
}

#[cfg(feature = "nota-text")]
fn encode_nota(value: &impl NotaEncode) -> String {
    value.to_nota()
}
