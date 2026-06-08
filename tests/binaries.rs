use std::process::Command;

#[cfg(feature = "nota-text")]
fn assert_placeholder_reply(binary: &str, argument: &str) {
    let output = Command::new(binary)
        .arg(argument)
        .output()
        .expect("run binary");

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout"),
        "(RequestUnimplemented NotBuiltYet)\n"
    );
    assert!(output.stderr.is_empty());
}

#[cfg(feature = "nota-text")]
#[test]
fn upgrade_cli_accepts_one_argument_and_prints_one_nota_reply() {
    assert_placeholder_reply(env!("CARGO_BIN_EXE_upgrade"), "(Tap All)");
}

#[test]
fn upgrade_daemon_accepts_signal_encoded_argument_and_prints_scaffold_reply() {
    let output = Command::new(env!("CARGO_BIN_EXE_upgrade-daemon"))
        .arg("configuration.rkyv")
        .output()
        .expect("run daemon placeholder");

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout"),
        "upgrade-daemon accepted signal-encoded configuration\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn upgrade_daemon_rejects_nota_arguments() {
    for argument in ["(Tap All)", "configuration.nota"] {
        let output = Command::new(env!("CARGO_BIN_EXE_upgrade-daemon"))
            .arg(argument)
            .output()
            .expect("run daemon placeholder");

        assert!(!output.status.success());
        let standard_error = String::from_utf8(output.stderr).expect("stderr");
        assert!(standard_error.contains("signal-encoded rkyv configuration file"));
        assert!(output.stdout.is_empty());
    }
}

#[cfg(feature = "nota-text")]
#[test]
fn upgrade_binary_rejects_flag_style_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_upgrade"))
        .arg("--help")
        .output()
        .expect("run binary");

    assert!(!output.status.success());
    let standard_error = String::from_utf8(output.stderr).expect("stderr");
    assert!(standard_error.contains("flag-style argument"));
    assert!(output.stdout.is_empty());
}
