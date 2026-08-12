use std::process::{Command, Output};

fn run(binary: &str, args: &[&str]) -> Output {
    Command::new(binary)
        .args(args)
        .output()
        .expect("CLI binary must run")
}

fn normalized(output: Output) -> (i32, Vec<u8>, Vec<u8>) {
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout)
            .expect("CLI output is UTF-8")
            .replace("taskseal", "<entrypoint>")
            .replace("tseal", "<entrypoint>")
            .into_bytes(),
        output.stderr,
    )
}

#[test]
fn both_entrypoints_have_identical_command_and_alias_matrix() {
    // Break caught: wiring one binary to a different parser, alias table, or entrypoint.
    let taskseal = env!("CARGO_BIN_EXE_taskseal");
    let tseal = env!("CARGO_BIN_EXE_tseal");

    let cases: &[&[&str]] = &[
        &[],
        &["help"],
        &["--help"],
        &["-h"],
        &["codex"],
        &["claude"],
        &["--", "fake"],
        &["status"],
        &["starts"],
        &["start"],
        &["scan"],
        &["init"],
        &["prepare"],
        &["check"],
        &["explain"],
        &["inspect"],
        &["doctor"],
        &["unknown-command"],
    ];

    for args in cases {
        assert_eq!(
            normalized(run(taskseal, args)),
            normalized(run(tseal, args)),
            "entrypoints diverged for {args:?}",
        );
    }
}
