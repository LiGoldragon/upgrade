use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("signal frame: {0}")]
    SignalFrame(#[from] signal_frame::FrameError),
    #[error("upgrade expects exactly one argument")]
    MissingArgument,
    #[error("upgrade expects exactly one argument")]
    TooManyArguments,
    #[error("argument must not be empty")]
    EmptyArgument,
    #[error("flag-style argument `{argument}` is not accepted; pass one NOTA record or file path")]
    FlagStyleArgument { argument: String },
    #[error("upgrade-daemon expects a signal-encoded rkyv configuration file")]
    DaemonExpectedSignalFile,
    #[error("daemon frame is too large: {bytes} bytes")]
    DaemonFrameTooLarge { bytes: usize },
    #[error("socket path is occupied by a non-socket file: {path}")]
    SocketPathOccupied { path: PathBuf },
    #[error("unexpected Signal frame: {got}")]
    UnexpectedSignalFrame { got: String },
    #[error("handover marker component mismatch: expected {expected}, got {actual}")]
    HandoverMarkerComponentMismatch { expected: String, actual: String },
    #[error("next handover marker mismatch on {field}: expected {expected}, got {actual}")]
    NextHandoverMarkerMismatch {
        field: &'static str,
        expected: String,
        actual: String,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::MissingArgument, Self::MissingArgument)
            | (Self::TooManyArguments, Self::TooManyArguments)
            | (Self::EmptyArgument, Self::EmptyArgument)
            | (Self::DaemonExpectedSignalFile, Self::DaemonExpectedSignalFile) => true,
            (
                Self::FlagStyleArgument { argument: left },
                Self::FlagStyleArgument { argument: right },
            ) => left == right,
            (
                Self::DaemonFrameTooLarge { bytes: left },
                Self::DaemonFrameTooLarge { bytes: right },
            ) => left == right,
            (Self::SocketPathOccupied { path: left }, Self::SocketPathOccupied { path: right }) => {
                left == right
            }
            (
                Self::UnexpectedSignalFrame { got: left },
                Self::UnexpectedSignalFrame { got: right },
            ) => left == right,
            (
                Self::HandoverMarkerComponentMismatch {
                    expected: left_expected,
                    actual: left_actual,
                },
                Self::HandoverMarkerComponentMismatch {
                    expected: right_expected,
                    actual: right_actual,
                },
            ) => left_expected == right_expected && left_actual == right_actual,
            (
                Self::NextHandoverMarkerMismatch {
                    field: left_field,
                    expected: left_expected,
                    actual: left_actual,
                },
                Self::NextHandoverMarkerMismatch {
                    field: right_field,
                    expected: right_expected,
                    actual: right_actual,
                },
            ) => {
                left_field == right_field
                    && left_expected == right_expected
                    && left_actual == right_actual
            }
            _ => false,
        }
    }
}

impl Eq for Error {}
