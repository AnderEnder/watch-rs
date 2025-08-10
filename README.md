# watch-rs

A Rust implementation of the classic Unix `watch` utility that executes programs periodically and displays output fullscreen.

## Features

- Execute commands at specified intervals (default: 2 seconds)
- Full-screen terminal display with alternate screen buffer
- Real-time keyboard input handling (Ctrl+C or 'q' to quit)
- Dynamic terminal resizing support
- Command output display with timestamp and interval information

## Dependencies

- **clap 4.0**: Modern command-line argument parsing
- **chrono 0.4**: Date and time handling for timestamps
- **termion 4**: Terminal I/O and raw mode handling

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

## Command-line Options

- `-n, --interval <INTERVAL>`: Set execution interval in seconds (default: 2)
- `-d, --difference`: Highlight differences between updates
- `-c, --cumulative`: Cumulative mode
- `-t, --no-title`: Disable title display
- `-h, --help`: Print help information

## Controls

- **Ctrl+C** or **q**: Exit the program
- Terminal automatically redraws on resize

## Architecture

Single-file Rust application that:
- Uses clap for argument parsing with derive macros
- Implements terminal UI using termion's alternate screen and raw mode
- Executes shell commands via `sh -c` 
- Handles real-time input and display updates in a main event loop