use std::process::Command;

fn hoogle_tui() -> Command {
    Command::new(env!("CARGO_BIN_EXE_hoogle-tui"))
}

#[test]
fn help_prints_usage() {
    let output = hoogle_tui()
        .arg("--help")
        .output()
        .expect("failed to run hoogle-tui --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Terminal UI for Hoogle"));
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--backend"));
}

#[test]
fn version_prints_package_version() {
    let output = hoogle_tui()
        .arg("--version")
        .output()
        .expect("failed to run hoogle-tui --version");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn completions_print_shell_script_without_starting_tui() {
    let output = hoogle_tui()
        .args(["--completions", "bash"])
        .output()
        .expect("failed to run hoogle-tui --completions bash");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("_hoogle-tui"));
    assert!(stdout.contains("--backend"));
}

#[test]
fn invalid_flag_exits_with_error() {
    let output = hoogle_tui()
        .arg("--not-a-real-flag")
        .output()
        .expect("failed to run hoogle-tui with invalid flag");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unexpected argument"));
}

#[test]
fn invalid_backend_exits_with_error() {
    let output = hoogle_tui()
        .args(["--backend", "foobar"])
        .output()
        .expect("failed to run hoogle-tui with invalid backend");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value"));
    assert!(stderr.contains("auto"));
    assert!(stderr.contains("local"));
    assert!(stderr.contains("web"));
}
