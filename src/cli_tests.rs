use super::*;

#[test]
fn forwards_command_flags_in_the_documented_invocation() {
    let opts = WatchOpts::try_parse_from(["watch", "ls", "-la"]).unwrap();
    assert_eq!(opts.command, ["ls", "-la"]);
}

#[test]
fn watch_flags_apply_only_before_the_command() {
    let opts = WatchOpts::try_parse_from([
        "watch", "-n", "5", "-d", "program", "-n", "7", "--help", "-t",
    ])
    .unwrap();
    assert!(opts.difference);
    assert!(!opts.no_title);
    assert_eq!(opts.command, ["program", "-n", "7", "--help", "-t"]);
}

#[test]
fn supports_explicit_separator_and_quoted_shell_command() {
    let opts = WatchOpts::try_parse_from(["watch", "--", "ls", "-la"]).unwrap();
    assert_eq!(opts.command, ["ls", "-la"]);
    let opts = WatchOpts::try_parse_from(["watch", "ps aux | grep rust"]).unwrap();
    assert_eq!(opts.command, ["ps aux | grep rust"]);
}

#[test]
fn retains_watch_help_and_requires_a_command() {
    let error = WatchOpts::try_parse_from(["watch", "--help"]).unwrap_err();
    assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
    assert!(WatchOpts::try_parse_from(["watch"]).is_err());
}
