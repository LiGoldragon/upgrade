use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argument {
    value: String,
}

impl Argument {
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        if value.is_empty() {
            return Err(Error::EmptyArgument);
        }
        if value.starts_with('-') {
            return Err(Error::FlagStyleArgument { argument: value });
        }
        Ok(Self { value })
    }

    pub fn value(&self) -> &str {
        self.value.as_str()
    }

    pub fn kind(&self) -> InvocationKind {
        if self.value.trim_start().starts_with('(') {
            InvocationKind::InlineNota
        } else if self.value.ends_with(".rkyv") {
            InvocationKind::SignalFile
        } else {
            InvocationKind::NotaFile
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationKind {
    InlineNota,
    NotaFile,
    SignalFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    argument: Argument,
}

impl Invocation {
    pub fn from_program_arguments<I, S>(arguments: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut iterator = arguments.into_iter().map(Into::into);
        let _program_name = iterator.next();
        let Some(argument) = iterator.next() else {
            return Err(Error::MissingArgument);
        };
        if iterator.next().is_some() {
            return Err(Error::TooManyArguments);
        }
        Ok(Self {
            argument: Argument::new(argument)?,
        })
    }

    pub fn argument(&self) -> &Argument {
        &self.argument
    }

    pub fn require_signal_file_argument(&self) -> Result<(), Error> {
        match self.argument.kind() {
            InvocationKind::SignalFile => Ok(()),
            InvocationKind::InlineNota | InvocationKind::NotaFile => {
                Err(Error::DaemonExpectedSignalFile)
            }
        }
    }
}
