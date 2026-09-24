mod display;
mod running_command;
mod watch_state;

use chrono::offset::Local;
use clap::Parser;
use running_command::RunningCommand;
use std::io;
use std::io::{Write, stdout};
use std::thread;
use std::time::{Duration, Instant};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::{AlternateScreen, IntoAlternateScreen};
use termion::{async_stdin, clear, cursor};
use watch_state::{Frame, WatchState};

/// watch - execute a program periodically, showing output fullscreen
#[derive(Parser, Debug, Clone)]
#[command(name = "watch")]
pub struct WatchOpts {
    #[arg(long = "difference", short = 'd')]
    difference: bool,
    #[arg(long = "cumulative", short = 'c')]
    cumulative: bool,
    #[arg(long = "no-title", short = 't')]
    no_title: bool,
    #[arg(long = "interval", short = 'n', default_value = "2", value_parser = parse_interval)]
    /// Interval
    interval: Duration,
    #[arg(
        required = true,
        trailing_var_arg = true,
        allow_hyphen_values = true,
        help = "Shell command string, or program followed by arguments"
    )]
    command: Vec<String>,
}

fn parse_interval(value: &str) -> Result<Duration, String> {
    let seconds: f64 = value
        .parse()
        .map_err(|_| "interval must be a number".to_string())?;
    let duration = Duration::try_from_secs_f64(seconds)
        .map_err(|_| "interval must be finite, positive, and representable".to_string())?;
    if duration.is_zero() {
        return Err("interval must be positive and at least one nanosecond".to_string());
    }
    Ok(duration)
}

fn draw<W: Write>(
    stdout: &mut AlternateScreen<W>,
    status_begin: &str,
    command: &str,
    now: &str,
    frame: &Frame,
    no_title: bool,
) -> io::Result<()> {
    display::render(
        stdout,
        termion::terminal_size()?,
        status_begin,
        command,
        now,
        frame,
        no_title,
    )
}

mod command_output;

fn main() -> io::Result<()> {
    let args = WatchOpts::parse();
    let status_begin = format!("Every {:.2}s: ", args.interval.as_secs_f64());
    let command = args.command.join(" ");

    let mut stdout = cursor::HideCursor::from(stdout().into_raw_mode()?).into_alternate_screen()?;
    let mut key_stream = async_stdin().keys();

    let mut state = WatchState::new(args.difference, args.cumulative);

    'outer: loop {
        let mut running = RunningCommand::spawn(&args.command)?;
        let now = Local::now().format("%c").to_string();
        write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1))?;
        draw(
            &mut stdout,
            &status_begin,
            &command,
            &now,
            state.frame(),
            args.no_title,
        )?;
        let mut running_size = termion::terminal_size()?;
        let output = loop {
            match key_stream.next() {
                Some(Ok(Key::Ctrl('c'))) | Some(Ok(Key::Char('q'))) => break 'outer,
                _ => {}
            }
            if let Some(output) = running.poll()? {
                break output;
            }
            let size = termion::terminal_size()?;
            if size != running_size {
                running_size = size;
                write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1))?;
                draw(
                    &mut stdout,
                    &status_begin,
                    &command,
                    &now,
                    state.frame(),
                    args.no_title,
                )?;
            }
            thread::sleep(Duration::from_millis(10));
        };
        let now = Local::now().format("%c").to_string();

        write!(
            stdout,
            "{}{}{}",
            clear::All,
            cursor::Hide,
            cursor::Goto(1, 1)
        )?;

        let mut tsize = termion::terminal_size()?;

        state.update(command_output::format_output(&output));

        draw(
            &mut stdout,
            &status_begin,
            &command,
            &now,
            state.frame(),
            args.no_title,
        )?;

        let started = Instant::now();

        loop {
            match key_stream.next() {
                Some(Ok(Key::Ctrl('c'))) | Some(Ok(Key::Char('q'))) => {
                    write!(
                        stdout,
                        "{}{}{}",
                        clear::All,
                        cursor::Show,
                        cursor::Goto(1, 1)
                    )?;
                    break 'outer;
                }
                _ => {}
            }

            let elapsed = started.elapsed();
            if elapsed >= args.interval {
                break;
            }
            thread::sleep(Duration::from_millis(10).min(args.interval - elapsed));

            let csize = termion::terminal_size()?;
            if tsize != csize {
                tsize = csize;
                draw(
                    &mut stdout,
                    &status_begin,
                    &command,
                    &now,
                    state.frame(),
                    args.no_title,
                )?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod cli_tests;
