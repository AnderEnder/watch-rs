use chrono::offset::Local;
use clap::Parser;
use std::cmp::min;
use std::io;
use std::io::{stdout, Write};
use std::process::Command;
use std::thread;
use std::time::Duration;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::{AlternateScreen, IntoAlternateScreen};
use termion::{async_stdin, clear, cursor};

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

fn draw<W: Write>(
    stdout: &mut AlternateScreen<W>,
    status_begin: &str,
    command: &str,
    now: &str,
    content: &str,
) -> Result<(), std::io::Error> {
    let (width, height) = termion::terminal_size()?;

    let space = String::from_utf8(vec![
        b' ';
        width as usize
            - status_begin.len()
            - command.len()
            - now.len()
            - 4
    ])
    .map_err(|e| io::Error::other(e.to_string()))?;

    let status = format!("{0}{1}{2}{3:>28}", status_begin, &command, space, now);

    writeln!(stdout, "{}\r", status)?;

    for (n, out) in content.lines().enumerate() {
        if n > (height - 3) as usize {
            break;
        }
        if out.len() > width as usize {
            write!(stdout, "\r\n{}", &out[0..width as usize])?
        } else {
            write!(stdout, "\r\n{}", out)?;
        }
    }

    write!(stdout, "{}", cursor::Goto(1, 1))?;
    stdout.flush()?;
    Ok(())
}

fn main() -> Result<(), std::io::Error> {
    let args = WatchOpts::parse();
    let status_begin = format!("Every {:.2}s: ", args.interval);
    let command = args.command.join(" ");

    let mut stdout = stdout().into_raw_mode()?.into_alternate_screen()?;
    let mut key_stream = async_stdin().keys();

    let delta_ms = min(10, (args.interval * 1000_f32) as u64 / 4);
    let delta = delta_ms as f32 / 1000_f32;

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

        let content =
            String::from_utf8(output.stdout).map_err(|e| io::Error::other(e.to_string()))?;

        draw(&mut stdout, &status_begin, &command, &now, &content)?;

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
                draw(&mut stdout, &status_begin, &command, &now, &content)?;
            }
        }
    }
    Ok(())
}
