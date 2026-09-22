mod display;
mod running_command;

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
use termion::{async_stdin, clear, color, cursor};

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
    #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
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

/// Compare two strings and return the new content with differences highlighted
fn highlight_differences(old_content: &str, new_content: &str) -> String {
    if old_content == new_content {
        return new_content.to_string();
    }

    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();
    let mut result = Vec::new();

    let max_lines = std::cmp::max(old_lines.len(), new_lines.len());

    for i in 0..max_lines {
        let old_line = old_lines.get(i).copied().unwrap_or("");
        let new_line = new_lines.get(i).copied().unwrap_or("");

        if old_line != new_line {
            // Highlight the entire changed line in red
            result.push(format!(
                "{}{}{}",
                color::Fg(color::Red),
                new_line,
                color::Fg(color::Reset)
            ));
        } else {
            result.push(new_line.to_string());
        }
    }

    result.join("\n")
}

/// Accumulate new content with previous content
fn accumulate_content(old_content: &str, new_content: &str) -> String {
    if old_content.is_empty() {
        new_content.to_string()
    } else {
        format!("{}\n{}", old_content, new_content)
    }
}

fn draw<W: Write>(
    stdout: &mut AlternateScreen<W>,
    status_begin: &str,
    command: &str,
    now: &str,
    content: &str,
    no_title: bool,
) -> io::Result<()> {
    let (width, height) = termion::terminal_size()?;

    if width == 0 || height == 0 {
        return stdout.flush();
    }

    if !no_title {
        let title = format!("{status_begin}{command}");
        let padding = (width as usize).saturating_sub(title.len() + now.len() + 1);
        let status = format!("{title}{}{now}", " ".repeat(padding));
        let status: String = status.chars().take(width as usize).collect();
        write!(stdout, "{status}\r")?;
        if height > 1 {
            writeln!(stdout)?;
        }
    }

    let available_height = if no_title {
        height
    } else {
        height.saturating_sub(2)
    };

    for (n, out) in content.lines().enumerate() {
        if n >= available_height as usize {
            break;
        }
        let line_prefix = if no_title && n == 0 { "\r" } else { "\r\n" };
        write!(
            stdout,
            "{}{}",
            line_prefix,
            display::clip_line(out, width as usize)
        )?;
    }

    write!(stdout, "{}", cursor::Goto(1, 1))?;
    stdout.flush()?;
    Ok(())
}

mod command_output;

fn main() -> io::Result<()> {
    let args = WatchOpts::parse();
    let status_begin = format!("Every {:.2}s: ", args.interval.as_secs_f64());
    let command = args.command.join(" ");

    let mut stdout = cursor::HideCursor::from(stdout().into_raw_mode()?).into_alternate_screen()?;
    let mut key_stream = async_stdin().keys();

    let mut previous_content = String::new();
    let mut cumulative_content = String::new();
    let mut last_display = String::new();

    'outer: loop {
        let mut running = RunningCommand::spawn(&command)?;
        let now = Local::now().format("%c").to_string();
        write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1))?;
        draw(
            &mut stdout,
            &status_begin,
            &command,
            &now,
            &last_display,
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
                    &last_display,
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

        let raw_content = command_output::format_output(&output);

        // Process content based on flags
        let display_content = if args.cumulative {
            let old_cumulative = cumulative_content.clone();
            cumulative_content = accumulate_content(&cumulative_content, &raw_content);
            if args.difference {
                highlight_differences(&old_cumulative, &cumulative_content)
            } else {
                cumulative_content.clone()
            }
        } else if args.difference {
            let highlighted = highlight_differences(&previous_content, &raw_content);
            previous_content = raw_content;
            highlighted
        } else {
            previous_content = raw_content.clone(); // Store for potential difference highlighting
            raw_content
        };

        draw(
            &mut stdout,
            &status_begin,
            &command,
            &now,
            &display_content,
            args.no_title,
        )?;

        last_display = display_content.clone();

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
                    &display_content,
                    args.no_title,
                )?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod cli_tests;
