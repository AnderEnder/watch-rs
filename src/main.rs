use chrono::offset::Local;
use crossterm::{AlternateScreen, ClearType, Crossterm};
use std::process::Command;
use std::{thread, time};
use structopt::StructOpt;

/// watch - execute a program periodically, showing output fullscreen
#[derive(StructOpt, Debug, Clone)]
#[structopt(name = "watch")]
pub struct WatchOpts {
    #[structopt(long = "difference", short = "d")]
    difference: bool,
    #[structopt(long = "cumulative", short = "c")]
    cumulative: bool,
    #[structopt(long = "no-title", short = "t")]
    no_title: bool,
    #[structopt(long = "interval", short = "n", default_value = "2")]
    interval: u64,
    #[structopt(name = "command", raw(min_values = "1"))]
    command: Vec<String>,
}

fn main() {
    let args = WatchOpts::from_args();
    let interval = time::Duration::from_secs(args.interval);
    let status_begin = format!("Every {0}.0s: ", args.interval);
    let command = args.command.join(" ");
    let crossterm = Crossterm::new();
    let terminal = crossterm.terminal();
    let cursor = crossterm.cursor();

    if let Ok(_alternate) = AlternateScreen::to_alternate(false) {
        loop {
            let output = Command::new("sh").arg("-c").arg(&command).output().unwrap();
            let now = Local::now().format("%c").to_string();

            terminal.clear(ClearType::All).unwrap();
            cursor.goto(0, 0).unwrap();

            let (width, _) = terminal.terminal_size();

            // add spaces to allign with the right side
            let space = String::from_utf8(vec![
                b' ';
                width as usize
                    - status_begin.len()
                    - command.len()
                    - now.len()
                    - 3
            ])
            .unwrap();

            let status = format!("{0}{1}{2}{3:>28}\n\n", status_begin, &command, space, now);

            terminal.write(status).unwrap();
            terminal
                .write(&String::from_utf8(output.stdout).unwrap())
                .unwrap();

            thread::sleep(interval);
        }
    }
}
