use upgrade::{Error, Invocation, InvocationKind};

#[test]
fn invocation_requires_exactly_one_argument_after_program_name() {
    assert_eq!(
        Invocation::from_program_arguments(["upgrade"]),
        Err(Error::MissingArgument)
    );
    assert_eq!(
        Invocation::from_program_arguments(["upgrade", "(Inspect All)", "extra"]),
        Err(Error::TooManyArguments)
    );
}

#[test]
fn invocation_rejects_empty_and_flag_style_arguments() {
    assert_eq!(
        Invocation::from_program_arguments(["upgrade", ""]),
        Err(Error::EmptyArgument)
    );
    assert_eq!(
        Invocation::from_program_arguments(["upgrade", "--help"]),
        Err(Error::FlagStyleArgument {
            argument: "--help".to_owned()
        })
    );
}

#[test]
fn invocation_classifies_the_three_single_argument_shapes() {
    let inline = Invocation::from_program_arguments(["upgrade", "(Tap All)"]).expect("inline");
    let nota_file =
        Invocation::from_program_arguments(["upgrade", "./request.nota"]).expect("nota file");
    let signal_file =
        Invocation::from_program_arguments(["upgrade", "./request.rkyv"]).expect("signal file");

    assert_eq!(inline.argument().kind(), InvocationKind::InlineNota);
    assert_eq!(nota_file.argument().kind(), InvocationKind::NotaFile);
    assert_eq!(signal_file.argument().kind(), InvocationKind::SignalFile);
}

#[test]
fn daemon_invocation_requires_signal_encoded_file_argument() {
    let signal_file =
        Invocation::from_program_arguments(["upgrade-daemon", "./configuration.rkyv"])
            .expect("signal file");
    assert_eq!(signal_file.require_signal_file_argument(), Ok(()));

    let inline = Invocation::from_program_arguments(["upgrade-daemon", "(Configure Empty)"])
        .expect("inline");
    let nota_file = Invocation::from_program_arguments(["upgrade-daemon", "./configuration.nota"])
        .expect("nota file");

    assert_eq!(
        inline.require_signal_file_argument(),
        Err(Error::DaemonExpectedSignalFile)
    );
    assert_eq!(
        nota_file.require_signal_file_argument(),
        Err(Error::DaemonExpectedSignalFile)
    );
}
