use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("upgrade expects exactly one NOTA or signal argument")]
    MissingArgument,
    #[error("upgrade expects exactly one NOTA or signal argument")]
    TooManyArguments,
    #[error("argument must not be empty")]
    EmptyArgument,
    #[error("flag-style argument `{argument}` is not accepted; pass one NOTA record or file path")]
    FlagStyleArgument { argument: String },
}
