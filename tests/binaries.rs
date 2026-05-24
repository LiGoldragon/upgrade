use std::process::Command;

fn assert_placeholder_reply(binary: &str) {
    let output = Command::new(binary)
        .arg("(Tap All)")
        .output()
        .expect("run binary");

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout"),
        "(RequestUnimplemented (NotBuiltYet))\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn upgrade_cli_accepts_one_argument_and_prints_one_nota_reply() {
    assert_placeholder_reply(env!("CARGO_BIN_EXE_upgrade"));
}

#[test]
fn upgrade_daemon_accepts_one_argument_and_prints_one_nota_reply() {
    assert_placeholder_reply(env!("CARGO_BIN_EXE_upgrade-daemon"));
}

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
