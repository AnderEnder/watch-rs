# watch-rs

A Rust implementation of the classic Unix `watch` utility that executes programs periodically and displays output fullscreen.

## Features

- Execute commands at specified intervals (default: 2 seconds)
- Full-screen terminal display with alternate screen buffer
- Real-time keyboard input handling (Ctrl+C or 'q' to quit)
- Dynamic terminal resizing support
- Command output display with timestamp and interval information
- Captured stderr displayed after stdout, with an explicit message for unsuccessful exit status
- Captured stdout and stderr are limited to 1 MiB each; excess bytes are drained and truncation is reported on screen

## Dependencies

- **clap 4.6.7**: Modern command-line argument parsing
- **chrono 0.4.45**: Date and time handling for timestamps
- **termion 4.0.6**: Terminal I/O and raw mode handling

## Building

```bash
# Build the project
cargo build

# Build optimized release version
cargo build --release
```

## Usage

```bash
# Basic usage - watch a command every 2 seconds
cargo run -- ls -la

# Custom interval - watch every 5 seconds
cargo run -- -n 5 "ps aux | grep rust"

# Show help
cargo run -- --help
```

Place watch options before the command. With multiple command arguments, watch runs the first as a program and passes the remaining arguments unchanged, including flags such as `-la` and `--help`. A single quoted command string runs through `sh -c`; use that form for pipelines, environment assignments, and other shell syntax.

## Command-line Options

- `-n, --interval <INTERVAL>`: Set execution interval in seconds (default: 2; must be finite, positive, and representable with nanosecond precision)
- `-d, --difference`: Highlight differences between updates
- `-c, --cumulative`: Cumulative mode
- `-t, --no-title`: Disable title display
- `-h, --help`: Print help information

## Controls

- **Ctrl+C** or **q**: Exit the program
- Terminal automatically redraws on resize

## Architecture

Rust application that:
- Uses clap for argument parsing with derive macros
- Implements terminal UI using termion's alternate screen and raw mode
- Runs a single command string as shell syntax, or executes multiple command words with their argument boundaries preserved
- Keeps cumulative and difference state in `watch_state`, independent of terminal I/O
- Renders frames at a supplied terminal size in `display`, while the main loop handles input and redraw timing

## Testing

Run `cargo test`. Terminal regression tests require Python 3 and a Unix pseudo-terminal; they use only the Python standard library.
