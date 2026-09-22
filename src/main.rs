use chrono::offset::Local;
use clap::Parser;
use std::cmp::min;
use std::io;
use std::io::{Write, stdout};
use std::process::Command;
use std::thread;
use std::time::Duration;
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
    #[arg(long = "interval", short = 'n', default_value = "2")]
    /// Interval
    interval: f32,
    #[arg(required = true)]
    command: Vec<String>,
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
        if out.len() > width as usize {
            write!(stdout, "{}{}", line_prefix, &out[0..width as usize])?
        } else {
            write!(stdout, "{}{}", line_prefix, out)?;
        }
    }

    write!(stdout, "{}", cursor::Goto(1, 1))?;
    stdout.flush()?;
    Ok(())
}

mod command_output;

fn main() -> io::Result<()> {
    let args = WatchOpts::parse();
    let status_begin = format!("Every {:.2}s: ", args.interval);
    let command = args.command.join(" ");

    let mut stdout = stdout().into_raw_mode()?.into_alternate_screen()?;
    let mut key_stream = async_stdin().keys();

    let delta_ms = min(10, (args.interval * 1000_f32) as u64 / 4);
    let delta = delta_ms as f32 / 1000_f32;

    let mut previous_content = String::new();
    let mut cumulative_content = String::new();

    'outer: loop {
        let output = Command::new("sh").arg("-c").arg(&command).output()?;
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

        let mut ctime = 0_f32;

        while ctime < args.interval {
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

            thread::sleep(Duration::from_millis(delta_ms));
            ctime += delta;

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
