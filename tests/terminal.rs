#[test]
fn terminal_regressions() {
    let status = std::process::Command::new("python3")
        .args([
            "-B",
            "-m",
            "unittest",
            "discover",
            "-s",
            "tests",
            "-p",
            "test_*.py",
            "-v",
        ])
        .env("WATCH_BINARY", env!("CARGO_BIN_EXE_watch"))
        .status()
        .expect("terminal regression tests require Python 3 on Unix");
    assert!(status.success(), "terminal regression tests failed");
}
